//! The resolver is the third stage. It checks the names of the AST and returns the resolved AST.
//!
//! It checks that function names are unique, that there is a `main` function,
//! and that each variable is defined before it is used.
//! A `let` can reuse the name of a variable: it defines a new variable,
//! and the name refers to the new variable after this `let`.
//!
//! Places have their own names, separate from functions and variables.
//! In a function header, a place name refers to a place parameter of the function,
//! or else to the built-in place `host`. So a place parameter named `host` hides the built-in place.
//! A function whose header does not give its places runs on the built-in place `host`.
//! The `main` function has no place parameters, so it runs on `host` only.
//! In `thread(f@a)`, `f` is a function of the program and `a` is a place `f` runs on,
//! so the place name is resolved in the header of `f`, not in the enclosing function.

use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::ast;
use crate::resolved_ast::{self, FunctionId, FunctionPlaces, Place, PlaceParameterId, VariableId};

#[derive(Debug, PartialEq, Eq)]
pub enum ResolveError {
    MissingMain,
    DuplicateFunction(String),
    UnknownVariable(String),
    UnknownFunction(String),
    /// The function named `function` does not run on the place named `place`.
    FunctionDoesNotRunOnPlace {
        function: String,
        place: String,
    },
    /// Two place parameters of a function have this name.
    DuplicatePlaceParameter(String),
    UnknownPlace(String),
    /// A function header lists this place twice in the places the function runs on.
    DuplicatePlace(String),
    MainWithPlaceParameters,
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolveError::MissingMain => write!(f, "no `main` function found"),
            ResolveError::DuplicateFunction(name) => {
                write!(f, "function `{name}` is defined more than once")
            }
            ResolveError::UnknownVariable(name) => write!(f, "unknown variable `{name}`"),
            ResolveError::UnknownFunction(name) => write!(f, "unknown function `{name}`"),
            ResolveError::FunctionDoesNotRunOnPlace { function, place } => {
                write!(f, "function `{function}` does not run on place `@{place}`")
            }
            ResolveError::DuplicatePlaceParameter(name) => {
                write!(f, "place parameter `@{name}` is declared more than once")
            }
            ResolveError::UnknownPlace(name) => write!(f, "unknown place `@{name}`"),
            ResolveError::DuplicatePlace(name) => {
                write!(f, "place `@{name}` is listed more than once")
            }
            ResolveError::MainWithPlaceParameters => {
                write!(f, "the `main` function cannot have place parameters")
            }
        }
    }
}

/// Checks the names of `program` and returns its resolved AST.
pub fn resolve(program: ast::Program) -> Result<resolved_ast::Program, ResolveError> {
    check_no_duplicate_functions(&program)?;
    let main = program
        .functions
        .iter()
        .position(|function| function.name == "main")
        .ok_or(ResolveError::MissingMain)?;
    if !program.functions[main].place_parameters.is_empty() {
        return Err(ResolveError::MainWithPlaceParameters);
    }
    // The places of every function are resolved first,
    // so that a body can refer to the part of any function on any of its places.
    let all_places = program
        .functions
        .iter()
        .map(resolve_function_places)
        .collect::<Result<Vec<_>, _>>()?;
    let functions = Functions::new(&program.functions, all_places);
    let resolved = program
        .functions
        .into_iter()
        .enumerate()
        .map(|(index, function)| resolve_function(function, &functions, FunctionId(index)))
        .collect::<Result<_, _>>()?;
    Ok(resolved_ast::Program::new(resolved, FunctionId(main)))
}

fn check_no_duplicate_functions(program: &ast::Program) -> Result<(), ResolveError> {
    let mut names = HashSet::new();
    for function in &program.functions {
        if !names.insert(function.name.as_str()) {
            return Err(ResolveError::DuplicateFunction(function.name.clone()));
        }
    }
    Ok(())
}

/// Resolves the places in the header of `function`.
fn resolve_function_places(function: &ast::Function) -> Result<FunctionPlaces, ResolveError> {
    check_no_duplicate_place_parameters(&function.place_parameters)?;
    let run_on = match &function.places {
        Some(names) => resolve_places(names, &function.place_parameters)?,
        None => vec![Place::Host],
    };
    Ok(FunctionPlaces::new(
        function.place_parameters.clone(),
        run_on,
    ))
}

/// Resolves the body of `function`, whose id is `id`.
/// `functions` contains the places of every function of the program.
fn resolve_function(
    function: ast::Function,
    functions: &Functions,
    id: FunctionId,
) -> Result<resolved_ast::Function, ResolveError> {
    let mut variables = Variables::default();
    let body = function
        .body
        .into_iter()
        .map(|statement| resolve_statement(statement, &mut variables, functions))
        .collect::<Result<_, _>>()?;
    Ok(resolved_ast::Function::new(
        function.name,
        functions.places(id).clone(),
        body,
        variables.names,
    ))
}

fn check_no_duplicate_place_parameters(names: &[String]) -> Result<(), ResolveError> {
    let mut seen = HashSet::new();
    for name in names {
        if !seen.insert(name.as_str()) {
            return Err(ResolveError::DuplicatePlaceParameter(name.clone()));
        }
    }
    Ok(())
}

/// Resolves the places a function runs on, named `names`.
/// `place_parameters` contains the names of the place parameters of the function.
fn resolve_places(
    names: &[String],
    place_parameters: &[String],
) -> Result<Vec<Place>, ResolveError> {
    let mut places = Vec::new();
    for name in names {
        let place = resolve_place(name, place_parameters)?;
        if places.contains(&place) {
            return Err(ResolveError::DuplicatePlace(name.clone()));
        }
        places.push(place);
    }
    Ok(places)
}

/// Resolves the place named `name`.
/// `place_parameters` contains the names of the place parameters of its function.
fn resolve_place(name: &str, place_parameters: &[String]) -> Result<Place, ResolveError> {
    if let Some(index) = place_parameters
        .iter()
        .position(|parameter| parameter == name)
    {
        Ok(Place::Parameter(PlaceParameterId(index)))
    } else if name == "host" {
        Ok(Place::Host)
    } else {
        Err(ResolveError::UnknownPlace(name.to_string()))
    }
}

/// Resolves `statement`. `variables` contains the variables defined before it in its function.
/// `functions` contains the places of every function of the program.
fn resolve_statement(
    statement: ast::Statement,
    variables: &mut Variables,
    functions: &Functions,
) -> Result<resolved_ast::Statement, ResolveError> {
    match statement {
        ast::Statement::Print(value) => Ok(resolved_ast::Statement::Print(resolve_expression(
            value, variables, functions,
        )?)),
        ast::Statement::Delete(value) => Ok(resolved_ast::Statement::Delete(resolve_expression(
            value, variables, functions,
        )?)),
        ast::Statement::Join(value) => Ok(resolved_ast::Statement::Join(resolve_expression(
            value, variables, functions,
        )?)),
        ast::Statement::Let { name, value } => {
            // The value is resolved before the variable is defined,
            // so in `let x = x;` the value refers to the previous `x`.
            let value = resolve_expression(value, variables, functions)?;
            let variable = variables.define(name);
            Ok(resolved_ast::Statement::Let { variable, value })
        }
    }
}

/// Resolves `expression`. `variables` contains the variables defined before it in its function.
/// `functions` contains the places of every function of the program.
fn resolve_expression(
    expression: ast::Expression,
    variables: &Variables,
    functions: &Functions,
) -> Result<resolved_ast::Expression, ResolveError> {
    match expression {
        ast::Expression::Integer(value) => Ok(resolved_ast::Expression::Integer(value)),
        ast::Expression::NewSimDevice => Ok(resolved_ast::Expression::NewSimDevice),
        ast::Expression::Variable(name) => match variables.lookup(&name) {
            Some(variable) => Ok(resolved_ast::Expression::Variable(variable)),
            None => Err(ResolveError::UnknownVariable(name)),
        },
        ast::Expression::NewThread { function, place } => {
            let (function, place) = functions.lookup_part(&function, &place)?;
            Ok(resolved_ast::Expression::NewThread { function, place })
        }
    }
}

/// The places of every function of the program, by name and by id.
struct Functions {
    ids_by_name: HashMap<String, FunctionId>,
    /// The places of each function, indexed by its [`FunctionId`].
    places: Vec<FunctionPlaces>,
}

impl Functions {
    /// Creates the places of the functions `functions`, whose places are `places`.
    fn new(functions: &[ast::Function], places: Vec<FunctionPlaces>) -> Self {
        let ids_by_name = functions
            .iter()
            .enumerate()
            .map(|(index, function)| (function.name.clone(), FunctionId(index)))
            .collect();
        Functions {
            ids_by_name,
            places,
        }
    }

    /// The places of the function `id`.
    fn places(&self, id: FunctionId) -> &FunctionPlaces {
        self.places
            .get(id.0)
            .expect("a function id should refer to a function of the program")
    }

    /// Resolves the part of the function named `function_name` on the place named `place_name`.
    fn lookup_part(
        &self,
        function_name: &str,
        place_name: &str,
    ) -> Result<(FunctionId, Place), ResolveError> {
        let id = *self
            .ids_by_name
            .get(function_name)
            .ok_or_else(|| ResolveError::UnknownFunction(function_name.to_string()))?;
        let places = self.places(id);
        let place = resolve_place(place_name, places.parameter_names())?;
        if !places.run_on().contains(&place) {
            return Err(ResolveError::FunctionDoesNotRunOnPlace {
                function: function_name.to_string(),
                place: place_name.to_string(),
            });
        }
        Ok((id, place))
    }
}

/// The variables defined so far in a function.
#[derive(Default)]
struct Variables {
    /// The variable that each name refers to.
    by_name: HashMap<String, VariableId>,
    /// The name of each variable defined so far, indexed by its id.
    names: Vec<String>,
}

impl Variables {
    /// Defines a new variable named `name` and returns its id.
    /// After this, `name` refers to the new variable.
    fn define(&mut self, name: String) -> VariableId {
        let variable = VariableId(self.names.len());
        self.names.push(name.clone());
        self.by_name.insert(name, variable);
        variable
    }

    /// Returns the variable that `name` refers to, or `None` if no variable has this name.
    fn lookup(&self, name: &str) -> Option<VariableId> {
        self.by_name.get(name).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A program with empty functions named `function_names`.
    fn program(function_names: &[&str]) -> ast::Program {
        ast::Program {
            functions: function_names
                .iter()
                .map(|name| ast_function(name, vec![]))
                .collect(),
        }
    }

    /// A program with only a `main` function, whose body is `body`.
    fn main_with_body(body: Vec<ast::Statement>) -> ast::Program {
        ast::Program {
            functions: vec![ast_function("main", body)],
        }
    }

    /// An AST function named `name` with the body `body`, whose header does not give places.
    fn ast_function(name: &str, body: Vec<ast::Statement>) -> ast::Function {
        ast::Function {
            name: name.to_string(),
            place_parameters: vec![],
            places: None,
            body,
        }
    }

    /// An empty AST function named `name`, with the place parameters `place_parameters`
    /// and the places `places`, as written in its header.
    fn ast_function_with_places(
        name: &str,
        place_parameters: &[&str],
        places: Option<&[&str]>,
    ) -> ast::Function {
        let to_strings = |names: &[&str]| names.iter().map(|name| name.to_string()).collect();
        ast::Function {
            name: name.to_string(),
            place_parameters: to_strings(place_parameters),
            places: places.map(to_strings),
            body: vec![],
        }
    }

    /// Resolves a program with an empty `main` and the function `function`.
    fn resolve_with(function: ast::Function) -> Result<resolved_ast::Program, ResolveError> {
        resolve(ast::Program {
            functions: vec![ast_function("main", vec![]), function],
        })
    }

    /// Resolves a program with an empty `main` and the function `f`, as in [`ast_function_with_places`].
    /// Returns the places `f` runs on.
    fn places_of_f(
        place_parameters: &[&str],
        places: Option<&[&str]>,
    ) -> Result<Vec<Place>, ResolveError> {
        let function = ast_function_with_places("f", place_parameters, places);
        Ok(resolve_with(function)?.functions()[1]
            .places()
            .run_on()
            .to_vec())
    }

    /// Resolves a program whose only function is `main`.
    fn resolve_main(main: ast::Function) -> Result<resolved_ast::Program, ResolveError> {
        resolve(ast::Program {
            functions: vec![main],
        })
    }

    /// A resolved place parameter.
    fn parameter(index: usize) -> Place {
        Place::Parameter(PlaceParameterId(index))
    }

    /// Resolves `program` and returns the id of its `main` function.
    fn main_id(program: ast::Program) -> Result<FunctionId, ResolveError> {
        Ok(resolve(program)?.main())
    }

    /// Checks that a `main` function with the body `body` resolves to the body `expected`.
    fn assert_main_body_resolves_to(
        body: Vec<ast::Statement>,
        expected: Vec<resolved_ast::Statement>,
    ) {
        let program = resolve(main_with_body(body)).expect("the program should be valid");
        assert_eq!(program.function(program.main()).body(), expected);
    }

    /// Returns the error found when resolving a `main` function with the body `body`.
    fn main_body_error(body: Vec<ast::Statement>) -> ResolveError {
        resolve(main_with_body(body))
            .err()
            .expect("the program should be invalid")
    }

    /// An AST statement that defines the variable `name` with the value `value`.
    fn ast_let(name: &str, value: ast::Expression) -> ast::Statement {
        ast::Statement::Let {
            name: name.to_string(),
            value,
        }
    }

    /// An AST statement that prints the variable `name`.
    fn ast_print_variable(name: &str) -> ast::Statement {
        ast::Statement::Print(ast::Expression::Variable(name.to_string()))
    }

    /// A resolved statement that sets `variable` to the integer `value`.
    fn resolved_let_integer(variable: usize, value: i64) -> resolved_ast::Statement {
        resolved_ast::Statement::Let {
            variable: VariableId(variable),
            value: resolved_ast::Expression::Integer(value),
        }
    }

    /// A resolved statement that prints `variable`.
    fn resolved_print_variable(variable: usize) -> resolved_ast::Statement {
        resolved_ast::Statement::Print(resolved_ast::Expression::Variable(VariableId(variable)))
    }

    /// A program with a `main` function with the body `body`,
    /// and a function `work` that runs on its place parameter `a`.
    fn program_with_work(body: Vec<ast::Statement>) -> ast::Program {
        ast::Program {
            functions: vec![
                ast_function("main", body),
                ast_function_with_places("work", &["a"], Some(&["a"])),
            ],
        }
    }

    /// An AST expression that creates a thread for the part of `function` on `place`.
    fn ast_thread(function: &str, place: &str) -> ast::Expression {
        ast::Expression::NewThread {
            function: function.to_string(),
            place: place.to_string(),
        }
    }

    /// Returns the error found when resolving a program as in [`program_with_work`].
    fn work_body_error(body: Vec<ast::Statement>) -> ResolveError {
        resolve(program_with_work(body))
            .err()
            .expect("the program should be invalid")
    }

    #[test]
    fn thread_refers_to_a_function_defined_later() {
        let body = vec![
            ast_let("t", ast_thread("work", "a")),
            ast::Statement::Join(ast::Expression::Variable("t".to_string())),
        ];
        let program = resolve(program_with_work(body)).expect("the program should be valid");
        assert_eq!(
            program.function(program.main()).body(),
            [
                resolved_ast::Statement::Let {
                    variable: VariableId(0),
                    value: resolved_ast::Expression::NewThread {
                        function: FunctionId(1),
                        place: parameter(0),
                    },
                },
                resolved_ast::Statement::Join(resolved_ast::Expression::Variable(VariableId(0))),
            ]
        );
    }

    #[test]
    fn thread_of_an_unknown_function_is_an_error() {
        assert_eq!(
            work_body_error(vec![ast_let("t", ast_thread("other", "a"))]),
            ResolveError::UnknownFunction("other".to_string())
        );
    }

    #[test]
    fn thread_on_an_unknown_place_is_an_error() {
        assert_eq!(
            work_body_error(vec![ast_let("t", ast_thread("work", "z"))]),
            ResolveError::UnknownPlace("z".to_string())
        );
    }

    #[test]
    fn thread_on_a_place_the_function_does_not_run_on_is_an_error() {
        assert_eq!(
            work_body_error(vec![ast_let("t", ast_thread("work", "host"))]),
            ResolveError::FunctionDoesNotRunOnPlace {
                function: "work".to_string(),
                place: "host".to_string(),
            }
        );
    }

    #[test]
    fn main_is_found() {
        assert_eq!(main_id(program(&["main"])), Ok(FunctionId(0)));
    }

    #[test]
    fn main_is_found_among_other_functions() {
        assert_eq!(
            main_id(program(&["helper", "main", "other"])),
            Ok(FunctionId(1))
        );
    }

    #[test]
    fn missing_main_is_an_error() {
        assert_eq!(
            resolve(program(&["helper"])).err(),
            Some(ResolveError::MissingMain)
        );
    }

    #[test]
    fn empty_program_has_no_main() {
        assert_eq!(resolve(program(&[])).err(), Some(ResolveError::MissingMain));
    }

    #[test]
    fn function_bodies_are_resolved() {
        let program = main_with_body(vec![ast::Statement::Print(ast::Expression::Integer(42))]);
        let program = resolve(program).expect("the program should be valid");
        assert_eq!(
            program.function(program.main()),
            &resolved_ast::Function::new(
                "main".to_string(),
                FunctionPlaces::host(),
                vec![resolved_ast::Statement::Print(
                    resolved_ast::Expression::Integer(42)
                )],
                vec![],
            )
        );
    }

    #[test]
    fn duplicate_function_is_an_error() {
        assert_eq!(
            resolve(program(&["main", "helper", "helper"])).err(),
            Some(ResolveError::DuplicateFunction("helper".to_string()))
        );
    }

    #[test]
    fn duplicate_main_is_an_error() {
        assert_eq!(
            resolve(program(&["main", "main"])).err(),
            Some(ResolveError::DuplicateFunction("main".to_string()))
        );
    }

    #[test]
    fn variable_refers_to_its_let() {
        assert_main_body_resolves_to(
            vec![
                ast_let("x", ast::Expression::Integer(1)),
                ast_print_variable("x"),
            ],
            vec![resolved_let_integer(0, 1), resolved_print_variable(0)],
        );
    }

    #[test]
    fn each_let_defines_a_new_variable() {
        assert_main_body_resolves_to(
            vec![
                ast_let("x", ast::Expression::Integer(1)),
                ast_let("y", ast::Expression::Integer(2)),
                ast_print_variable("x"),
                ast_print_variable("y"),
            ],
            vec![
                resolved_let_integer(0, 1),
                resolved_let_integer(1, 2),
                resolved_print_variable(0),
                resolved_print_variable(1),
            ],
        );
    }

    #[test]
    fn let_with_the_same_name_hides_the_previous_variable() {
        assert_main_body_resolves_to(
            vec![
                ast_let("x", ast::Expression::Integer(1)),
                ast_print_variable("x"),
                ast_let("x", ast::Expression::Integer(2)),
                ast_print_variable("x"),
            ],
            vec![
                resolved_let_integer(0, 1),
                resolved_print_variable(0),
                resolved_let_integer(1, 2),
                resolved_print_variable(1),
            ],
        );
    }

    #[test]
    fn value_of_a_let_refers_to_the_previous_variable_with_the_same_name() {
        assert_main_body_resolves_to(
            vec![
                ast_let("x", ast::Expression::Integer(1)),
                ast_let("x", ast::Expression::Variable("x".to_string())),
            ],
            vec![
                resolved_let_integer(0, 1),
                resolved_ast::Statement::Let {
                    variable: VariableId(1),
                    value: resolved_ast::Expression::Variable(VariableId(0)),
                },
            ],
        );
    }

    #[test]
    fn variable_count_counts_each_let() {
        let program = resolve(main_with_body(vec![
            ast_let("x", ast::Expression::Integer(1)),
            ast_let("x", ast::Expression::Integer(2)),
        ]))
        .expect("the program should be valid");
        assert_eq!(program.function(program.main()).variable_count(), 2);
    }

    #[test]
    fn unknown_variable_is_an_error() {
        assert_eq!(
            main_body_error(vec![ast_print_variable("x")]),
            ResolveError::UnknownVariable("x".to_string())
        );
    }

    #[test]
    fn variable_used_before_its_let_is_an_error() {
        assert_eq!(
            main_body_error(vec![
                ast_print_variable("x"),
                ast_let("x", ast::Expression::Integer(1)),
            ]),
            ResolveError::UnknownVariable("x".to_string())
        );
    }

    #[test]
    fn let_value_cannot_refer_to_its_own_variable() {
        assert_eq!(
            main_body_error(vec![ast_let(
                "x",
                ast::Expression::Variable("x".to_string())
            )]),
            ResolveError::UnknownVariable("x".to_string())
        );
    }

    #[test]
    fn variables_of_another_function_are_unknown() {
        let program = ast::Program {
            functions: vec![
                ast_function("helper", vec![ast_let("x", ast::Expression::Integer(1))]),
                ast_function("main", vec![ast_print_variable("x")]),
            ],
        };
        assert_eq!(
            resolve(program).err(),
            Some(ResolveError::UnknownVariable("x".to_string()))
        );
    }

    #[test]
    fn variable_in_delete_is_resolved() {
        assert_main_body_resolves_to(
            vec![
                ast_let("d", ast::Expression::NewSimDevice),
                ast::Statement::Delete(ast::Expression::Variable("d".to_string())),
            ],
            vec![
                resolved_ast::Statement::Let {
                    variable: VariableId(0),
                    value: resolved_ast::Expression::NewSimDevice,
                },
                resolved_ast::Statement::Delete(resolved_ast::Expression::Variable(VariableId(0))),
            ],
        );
    }

    #[test]
    fn variable_names_are_kept_for_each_let() {
        let program = resolve(main_with_body(vec![
            ast_let("x", ast::Expression::Integer(1)),
            ast_let("y", ast::Expression::Integer(2)),
            ast_let("x", ast::Expression::Integer(3)),
        ]))
        .expect("the program should be valid");
        assert_eq!(
            program.function(program.main()).variable_names(),
            ["x", "y", "x"]
        );
    }

    #[test]
    fn function_without_places_runs_on_host() {
        assert_eq!(places_of_f(&[], None), Ok(vec![Place::Host]));
    }

    #[test]
    fn host_is_a_built_in_place() {
        assert_eq!(places_of_f(&[], Some(&["host"])), Ok(vec![Place::Host]));
    }

    #[test]
    fn places_refer_to_place_parameters() {
        assert_eq!(
            places_of_f(&["a", "b"], Some(&["b", "a"])),
            Ok(vec![parameter(1), parameter(0)])
        );
    }

    #[test]
    fn function_can_run_on_host_and_a_place_parameter() {
        assert_eq!(
            places_of_f(&["a"], Some(&["host", "a"])),
            Ok(vec![Place::Host, parameter(0)])
        );
    }

    #[test]
    fn function_with_place_parameters_and_without_places_runs_on_host() {
        assert_eq!(places_of_f(&["a"], None), Ok(vec![Place::Host]));
    }

    #[test]
    fn place_parameter_named_host_hides_the_built_in_place() {
        assert_eq!(
            places_of_f(&["host"], Some(&["host"])),
            Ok(vec![parameter(0)])
        );
    }

    #[test]
    fn place_parameter_names_are_kept() {
        let function = ast_function_with_places("f", &["a", "b"], Some(&["a"]));
        let program = resolve_with(function).expect("the program should be valid");
        assert_eq!(
            program.functions()[1].places().parameter_names(),
            ["a", "b"]
        );
    }

    #[test]
    fn unknown_place_is_an_error() {
        assert_eq!(
            places_of_f(&["a"], Some(&["b"])),
            Err(ResolveError::UnknownPlace("b".to_string()))
        );
    }

    #[test]
    fn duplicate_place_parameter_is_an_error() {
        assert_eq!(
            places_of_f(&["a", "a"], Some(&["a"])),
            Err(ResolveError::DuplicatePlaceParameter("a".to_string()))
        );
    }

    #[test]
    fn place_listed_twice_is_an_error() {
        assert_eq!(
            places_of_f(&["a"], Some(&["a", "a"])),
            Err(ResolveError::DuplicatePlace("a".to_string()))
        );
    }

    #[test]
    fn main_can_be_written_on_host() {
        let main = ast_function_with_places("main", &[], Some(&["host"]));
        let program = resolve_main(main).expect("the program should be valid");
        assert_eq!(
            program.function(program.main()).places(),
            &FunctionPlaces::host()
        );
    }

    #[test]
    fn main_with_place_parameters_is_an_error() {
        let main = ast_function_with_places("main", &["a"], Some(&["a"]));
        assert_eq!(
            resolve_main(main).err(),
            Some(ResolveError::MainWithPlaceParameters)
        );
    }

    #[test]
    fn main_with_place_parameters_is_an_error_even_on_host() {
        let main = ast_function_with_places("main", &["a"], None);
        assert_eq!(
            resolve_main(main).err(),
            Some(ResolveError::MainWithPlaceParameters)
        );
    }
}
