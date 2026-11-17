//! The resolver is the third stage. It checks the names of the AST and returns the resolved AST.
//!
//! It checks that function names are unique, that there is a `main` function,
//! and that each variable is defined before it is used.
//! A `let` can reuse the name of a variable: it defines a new variable,
//! and the name refers to the new variable after this `let`.

use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::ast;
use crate::resolved_ast::{self, FunctionId, VariableId};

#[derive(Debug, PartialEq, Eq)]
pub enum ResolveError {
    MissingMain,
    DuplicateFunction(String),
    UnknownVariable(String),
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolveError::MissingMain => write!(f, "no `main` function found"),
            ResolveError::DuplicateFunction(name) => {
                write!(f, "function `{name}` is defined more than once")
            }
            ResolveError::UnknownVariable(name) => write!(f, "unknown variable `{name}`"),
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
    let functions = program
        .functions
        .into_iter()
        .map(resolve_function)
        .collect::<Result<_, _>>()?;
    Ok(resolved_ast::Program::new(functions, FunctionId(main)))
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

fn resolve_function(function: ast::Function) -> Result<resolved_ast::Function, ResolveError> {
    let mut variables = Variables::default();
    let body = function
        .body
        .into_iter()
        .map(|statement| resolve_statement(statement, &mut variables))
        .collect::<Result<_, _>>()?;
    Ok(resolved_ast::Function::new(
        function.name,
        body,
        variables.count,
    ))
}

/// Resolves `statement`. `variables` contains the variables defined before it in its function.
fn resolve_statement(
    statement: ast::Statement,
    variables: &mut Variables,
) -> Result<resolved_ast::Statement, ResolveError> {
    match statement {
        ast::Statement::Print(value) => Ok(resolved_ast::Statement::Print(resolve_expression(
            value, variables,
        )?)),
        ast::Statement::Let { name, value } => {
            // The value is resolved before the variable is defined,
            // so in `let x = x;` the value refers to the previous `x`.
            let value = resolve_expression(value, variables)?;
            let variable = variables.define(name);
            Ok(resolved_ast::Statement::Let { variable, value })
        }
    }
}

/// Resolves `expression`. `variables` contains the variables defined before it in its function.
fn resolve_expression(
    expression: ast::Expression,
    variables: &Variables,
) -> Result<resolved_ast::Expression, ResolveError> {
    match expression {
        ast::Expression::Integer(value) => Ok(resolved_ast::Expression::Integer(value)),
        ast::Expression::Variable(name) => match variables.lookup(&name) {
            Some(variable) => Ok(resolved_ast::Expression::Variable(variable)),
            None => Err(ResolveError::UnknownVariable(name)),
        },
    }
}

/// The variables defined so far in a function.
#[derive(Default)]
struct Variables {
    /// The variable that each name refers to.
    by_name: HashMap<String, VariableId>,
    /// The number of variables defined so far.
    count: usize,
}

impl Variables {
    /// Defines a new variable named `name` and returns its id.
    /// After this, `name` refers to the new variable.
    fn define(&mut self, name: String) -> VariableId {
        let variable = VariableId(self.count);
        self.count += 1;
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
                .map(|name| ast::Function {
                    name: name.to_string(),
                    body: vec![],
                })
                .collect(),
        }
    }

    /// A program with only a `main` function, whose body is `body`.
    fn main_with_body(body: Vec<ast::Statement>) -> ast::Program {
        ast::Program {
            functions: vec![ast::Function {
                name: "main".to_string(),
                body,
            }],
        }
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
                vec![resolved_ast::Statement::Print(
                    resolved_ast::Expression::Integer(42)
                )],
                0,
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
                ast::Function {
                    name: "helper".to_string(),
                    body: vec![ast_let("x", ast::Expression::Integer(1))],
                },
                ast::Function {
                    name: "main".to_string(),
                    body: vec![ast_print_variable("x")],
                },
            ],
        };
        assert_eq!(
            resolve(program).err(),
            Some(ResolveError::UnknownVariable("x".to_string()))
        );
    }
}
