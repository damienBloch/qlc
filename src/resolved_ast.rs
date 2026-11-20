//! The resolved AST. The resolver produces it and the type checker consumes it.
//!
//! It is the AST of a program whose names have been checked:
//! function names are unique, the `main` function exists,
//! and each variable is defined before it is used.
//! Variables are referred to by a [`VariableId`] instead of their name.
//! Places are referred to by a [`Place`] instead of their name.
//!
//! The body of each function follows these rules about variables:
//!
//! 1. each [`VariableId`] is smaller than the number of variables of the function;
//! 2. each variable is set by exactly one `let`;
//! 3. each variable is set before it is read.

pub struct Program {
    functions: Vec<Function>,
    main: FunctionId,
}

impl Program {
    /// Creates a program whose `main` function is `functions[main]`.
    pub fn new(functions: Vec<Function>, main: FunctionId) -> Self {
        assert!(
            main.0 < functions.len(),
            "the `main` function should be one of the functions of the program"
        );
        Program { functions, main }
    }

    pub fn functions(&self) -> &[Function] {
        &self.functions
    }

    /// The id of the `main` function.
    pub fn main(&self) -> FunctionId {
        self.main
    }

    /// Returns the functions and the id of the `main` function.
    pub fn into_parts(self) -> (Vec<Function>, FunctionId) {
        (self.functions, self.main)
    }

    /// The function `id`. Panics if the program has no such function.
    pub fn function(&self, id: FunctionId) -> &Function {
        self.functions
            .get(id.0)
            .expect("a function id should refer to a function of the program")
    }
}

/// A function whose body follows the rules about variables of the module documentation.
///
/// The places it runs on follow these rules:
///
/// 1. there is at least one place;
/// 2. no place is listed twice;
/// 3. each [`PlaceParameterId`] is smaller than the number of place parameters.
#[derive(Debug, PartialEq, Eq)]
pub struct Function {
    name: String,
    /// The name of each place parameter, without the `@`, indexed by its [`PlaceParameterId`].
    place_parameter_names: Vec<String>,
    /// The places the function runs on.
    places: Vec<Place>,
    body: Vec<Statement>,
    /// The name of each variable, indexed by its [`VariableId`].
    /// Several variables can have the same name.
    variable_names: Vec<String>,
}

impl Function {
    /// Creates a function whose place parameters have the names `place_parameter_names`,
    /// which runs on `places`, and whose variables have the names `variable_names`.
    /// Panics if `places` does not follow the rules of [`Function`],
    /// or if `body` does not follow the rules about variables of the module documentation.
    pub fn new(
        name: String,
        place_parameter_names: Vec<String>,
        places: Vec<Place>,
        body: Vec<Statement>,
        variable_names: Vec<String>,
    ) -> Self {
        check_places(&places, place_parameter_names.len());
        check_variables(&body, variable_names.len());
        Function {
            name,
            place_parameter_names,
            places,
            body,
            variable_names,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The name of each place parameter of the function, indexed by its [`PlaceParameterId`].
    pub fn place_parameter_names(&self) -> &[String] {
        &self.place_parameter_names
    }

    /// The places the function runs on.
    pub fn places(&self) -> &[Place] {
        &self.places
    }

    pub fn body(&self) -> &[Statement] {
        &self.body
    }

    /// The name of each variable of the function, indexed by its [`VariableId`].
    /// Several variables can have the same name.
    pub fn variable_names(&self) -> &[String] {
        &self.variable_names
    }

    /// The number of variables of the function. Their ids are `0..variable_count`.
    pub fn variable_count(&self) -> usize {
        self.variable_names.len()
    }

    /// Returns the name, the body and the names of the variables of the function.
    /// The places are not returned.
    pub fn into_parts(self) -> (String, Vec<Statement>, Vec<String>) {
        (self.name, self.body, self.variable_names)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Statement {
    /// Prints the value of an expression on its own line.
    Print(Expression),
    /// Deletes the device that is the value of an expression.
    Delete(Expression),
    /// Sets `variable` to the value of `value`.
    Let {
        variable: VariableId,
        value: Expression,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum Expression {
    /// An integer literal, such as `42`.
    Integer(i64),
    /// The value of a variable.
    Variable(VariableId),
    /// Creates a simulated device.
    NewSimDevice,
}

/// A place that a function runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// The built-in place of the computer that runs qlc.
    Host,
    /// A place parameter of the function.
    Parameter(PlaceParameterId),
}

/// The index of a place parameter in the place parameters of its function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaceParameterId(pub usize);

/// The index of a function in the functions of a program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionId(pub usize);

/// The index of a variable in the variables of its function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VariableId(pub usize);

/// Panics if `places` does not follow the rules of [`Function`].
/// `place_parameter_count` is the number of place parameters of the function.
fn check_places(places: &[Place], place_parameter_count: usize) {
    assert!(
        !places.is_empty(),
        "a function should run on at least one place"
    );
    for (index, place) in places.iter().enumerate() {
        assert!(
            !places[..index].contains(place),
            "a function should not run twice on the same place"
        );
        if let Place::Parameter(parameter) = place {
            assert!(
                parameter.0 < place_parameter_count,
                "place parameter id should be smaller than the number of place parameters"
            );
        }
    }
}

/// Panics if `body` does not follow the rules about variables of the module documentation.
/// `variable_count` is the number of variables of the function.
fn check_variables(body: &[Statement], variable_count: usize) {
    let mut is_set = vec![false; variable_count];
    for statement in body {
        match statement {
            Statement::Print(value) | Statement::Delete(value) => {
                check_variables_read(value, &is_set)
            }
            Statement::Let { variable, value } => {
                check_variables_read(value, &is_set);
                let variable_is_set = is_set.get_mut(variable.0).expect(VARIABLE_ID_IN_RANGE);
                assert!(
                    !*variable_is_set,
                    "each variable should be set by only one `let`"
                );
                *variable_is_set = true;
            }
        }
    }
    assert!(
        is_set.iter().all(|&is_set| is_set),
        "each variable should be set by a `let`"
    );
}

/// Panics if `expression` reads a variable that is not set.
/// `is_set` tells, for each variable, whether it is set.
fn check_variables_read(expression: &Expression, is_set: &[bool]) {
    match expression {
        Expression::Integer(_) | Expression::NewSimDevice => {}
        Expression::Variable(variable) => assert!(
            *is_set.get(variable.0).expect(VARIABLE_ID_IN_RANGE),
            "a variable should be set before it is read"
        ),
    }
}

const VARIABLE_ID_IN_RANGE: &str = "variable id should have a name in `variable_names`";

#[cfg(test)]
mod tests {
    use super::*;

    /// A function named `main` that runs on the host, with the body `body` and `variable_count` variables.
    fn function(body: Vec<Statement>, variable_count: usize) -> Function {
        let variable_names = (0..variable_count).map(|i| format!("v{i}")).collect();
        Function::new(
            "main".to_string(),
            vec![],
            vec![Place::Host],
            body,
            variable_names,
        )
    }

    /// An empty function with `place_parameter_count` place parameters, which runs on `places`.
    fn function_on_places(place_parameter_count: usize, places: Vec<Place>) -> Function {
        let place_parameter_names = (0..place_parameter_count)
            .map(|i| format!("p{i}"))
            .collect();
        Function::new(
            "f".to_string(),
            place_parameter_names,
            places,
            vec![],
            vec![],
        )
    }

    #[test]
    fn function_on_the_host_and_a_place_parameter_is_created() {
        let places = vec![Place::Host, Place::Parameter(PlaceParameterId(0))];
        assert_eq!(function_on_places(1, places.clone()).places(), places);
    }

    #[test]
    #[should_panic(expected = "a function should run on at least one place")]
    fn function_without_places_panics() {
        function_on_places(0, vec![]);
    }

    #[test]
    #[should_panic(expected = "a function should not run twice on the same place")]
    fn function_on_the_same_place_twice_panics() {
        let parameter = Place::Parameter(PlaceParameterId(0));
        function_on_places(1, vec![parameter, parameter]);
    }

    #[test]
    #[should_panic(
        expected = "place parameter id should be smaller than the number of place parameters"
    )]
    fn place_parameter_out_of_range_panics() {
        function_on_places(1, vec![Place::Parameter(PlaceParameterId(1))]);
    }

    /// A statement that sets `variable` to the integer `value`.
    fn let_integer(variable: usize, value: i64) -> Statement {
        Statement::Let {
            variable: VariableId(variable),
            value: Expression::Integer(value),
        }
    }

    /// A statement that sets `variable` to the value of `source`.
    fn let_variable(variable: usize, source: usize) -> Statement {
        Statement::Let {
            variable: VariableId(variable),
            value: Expression::Variable(VariableId(source)),
        }
    }

    /// A statement that prints `variable`.
    fn print_variable(variable: usize) -> Statement {
        Statement::Print(Expression::Variable(VariableId(variable)))
    }

    #[test]
    fn function_that_follows_the_rules_is_created() {
        let body = vec![let_integer(0, 1), let_variable(1, 0), print_variable(1)];
        assert_eq!(function(body, 2).body().len(), 3);
    }

    #[test]
    #[should_panic(expected = "variable id should have a name in `variable_names`")]
    fn let_of_a_variable_out_of_range_panics() {
        function(vec![let_integer(1, 1)], 1);
    }

    #[test]
    #[should_panic(expected = "variable id should have a name in `variable_names`")]
    fn read_of_a_variable_out_of_range_panics() {
        function(vec![let_integer(0, 1), print_variable(1)], 1);
    }

    #[test]
    #[should_panic(expected = "each variable should be set by only one `let`")]
    fn variable_set_twice_panics() {
        function(vec![let_integer(0, 1), let_integer(0, 2)], 1);
    }

    #[test]
    #[should_panic(expected = "a variable should be set before it is read")]
    fn variable_read_before_its_let_panics() {
        function(vec![print_variable(0), let_integer(0, 1)], 1);
    }

    #[test]
    #[should_panic(expected = "a variable should be set before it is read")]
    fn let_that_reads_its_own_variable_panics() {
        function(vec![let_variable(0, 0)], 1);
    }

    #[test]
    #[should_panic(expected = "each variable should be set by a `let`")]
    fn variable_without_let_panics() {
        function(vec![], 1);
    }

    #[test]
    #[should_panic(expected = "the `main` function should be one of the functions of the program")]
    fn program_with_an_invalid_main_id_panics() {
        Program::new(vec![], FunctionId(0));
    }

    #[test]
    #[should_panic(expected = "a function id should refer to a function of the program")]
    fn function_with_an_invalid_id_panics() {
        Program::new(vec![function(vec![], 0)], FunctionId(0)).function(FunctionId(1));
    }

    #[test]
    #[should_panic(expected = "a variable should be set before it is read")]
    fn delete_of_a_variable_before_its_let_panics() {
        function(
            vec![
                Statement::Delete(Expression::Variable(VariableId(0))),
                let_integer(0, 1),
            ],
            1,
        );
    }

    #[test]
    fn variable_count_is_the_number_of_variable_names() {
        assert_eq!(
            function(vec![let_integer(0, 1), let_integer(1, 2)], 2).variable_count(),
            2
        );
    }
}
