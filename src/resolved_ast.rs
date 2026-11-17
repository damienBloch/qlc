//! The resolved AST. The resolver produces it and the type checker consumes it.
//!
//! It is the AST of a program whose names have been checked:
//! function names are unique, the `main` function exists,
//! and each variable is defined before it is used.
//! Variables are referred to by a [`VariableId`] instead of their name.
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
#[derive(Debug, PartialEq, Eq)]
pub struct Function {
    name: String,
    body: Vec<Statement>,
    variable_count: usize,
}

impl Function {
    /// Creates a function with `variable_count` variables.
    /// Panics if `body` does not follow the rules about variables of the module documentation.
    pub fn new(name: String, body: Vec<Statement>, variable_count: usize) -> Self {
        check_variables(&body, variable_count);
        Function {
            name,
            body,
            variable_count,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn body(&self) -> &[Statement] {
        &self.body
    }

    /// The number of variables of the function. Their ids are `0..variable_count`.
    pub fn variable_count(&self) -> usize {
        self.variable_count
    }

    /// Returns the name, the body and the number of variables of the function.
    pub fn into_parts(self) -> (String, Vec<Statement>, usize) {
        (self.name, self.body, self.variable_count)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Statement {
    /// Prints the value of an expression on its own line.
    Print(Expression),
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

/// The index of a function in the functions of a program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionId(pub usize);

/// The index of a variable in the variables of its function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VariableId(pub usize);

/// Panics if `body` does not follow the rules about variables of the module documentation.
/// `variable_count` is the number of variables of the function.
fn check_variables(body: &[Statement], variable_count: usize) {
    let mut is_set = vec![false; variable_count];
    for statement in body {
        match statement {
            Statement::Print(value) => check_variables_read(value, &is_set),
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

const VARIABLE_ID_IN_RANGE: &str = "variable id should be smaller than `variable_count`";

#[cfg(test)]
mod tests {
    use super::*;

    /// A function named `main` with the body `body` and `variable_count` variables.
    fn function(body: Vec<Statement>, variable_count: usize) -> Function {
        Function::new("main".to_string(), body, variable_count)
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
    #[should_panic(expected = "variable id should be smaller than `variable_count`")]
    fn let_of_a_variable_out_of_range_panics() {
        function(vec![let_integer(1, 1)], 1);
    }

    #[test]
    #[should_panic(expected = "variable id should be smaller than `variable_count`")]
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
}
