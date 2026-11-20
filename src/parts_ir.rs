//! The parts IR. The projection produces it and the interpreter runs it.
//!
//! Each function is split into parts. A part is the code of a function that one place runs.

use std::collections::HashMap;

pub use crate::resolved_ast::{FunctionId, Place, PlaceParameterId, VariableId};

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

    /// The function `id`. Panics if the program has no such function.
    pub fn function(&self, id: FunctionId) -> &Function {
        self.functions
            .get(id.0)
            .expect("a function id should refer to a function of the program")
    }
}

/// A function, as one part for each place it runs on.
///
/// It has at least one part.
#[derive(Debug, PartialEq, Eq)]
pub struct Function {
    name: String,
    parts: HashMap<Place, Part>,
}

impl Function {
    /// Creates a function whose part on each place is `parts[place]`.
    /// Panics if `parts` is empty.
    pub fn new(name: String, parts: HashMap<Place, Part>) -> Self {
        assert!(
            !parts.is_empty(),
            "a function should have at least one part"
        );
        Function { name, parts }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The part of the function on each place it runs on.
    pub fn parts(&self) -> &HashMap<Place, Part> {
        &self.parts
    }

    /// The part of the function on `place`. Panics if the function does not run on `place`.
    pub fn part(&self, place: Place) -> &Part {
        self.parts
            .get(&place)
            .expect("a function should only be asked for a part on a place it runs on")
    }
}

/// The code of a function that one place runs.
///
/// Its body follows these rules about variables:
///
/// 1. each [`VariableId`] is smaller than the number of variables of the part;
/// 2. each variable is set by at most one `let`;
/// 3. each variable is set before it is read.
#[derive(Debug, PartialEq, Eq)]
pub struct Part {
    body: Vec<Statement>,
    variable_count: usize,
}

impl Part {
    /// Creates a part with `variable_count` variables.
    /// Panics if `body` does not follow the rules about variables of [`Part`].
    pub fn new(body: Vec<Statement>, variable_count: usize) -> Self {
        check_variables(&body, variable_count);
        Part {
            body,
            variable_count,
        }
    }

    pub fn body(&self) -> &[Statement] {
        &self.body
    }

    /// The number of variables of the part. Their ids are `0..variable_count`.
    pub fn variable_count(&self) -> usize {
        self.variable_count
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Statement {
    /// Prints the value of an expression on its own line.
    Print(Expression),
    /// Deletes the device that is the value of an expression.
    Delete(Expression),
    /// Waits for the thread that is the value of an expression.
    Join(Expression),
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
    /// Creates a thread that runs the part of `function` on `place`.
    NewThread { function: FunctionId, place: Place },
}

/// Panics if `body` does not follow the rules about variables of [`Part`].
/// `variable_count` is the number of variables of the part.
fn check_variables(body: &[Statement], variable_count: usize) {
    let mut is_set = vec![false; variable_count];
    for statement in body {
        match statement {
            Statement::Print(value) | Statement::Delete(value) | Statement::Join(value) => {
                check_variables_read(value, &is_set)
            }
            Statement::Let { variable, value } => {
                check_variables_read(value, &is_set);
                let variable_is_set = is_set.get_mut(variable.0).expect(VARIABLE_ID_IN_RANGE);
                assert!(
                    !*variable_is_set,
                    "each variable should be set by at most one `let`"
                );
                *variable_is_set = true;
            }
        }
    }
}

/// Panics if `expression` reads a variable that is not set.
/// `is_set` tells, for each variable, whether it is set.
fn check_variables_read(expression: &Expression, is_set: &[bool]) {
    match expression {
        Expression::Integer(_) | Expression::NewSimDevice | Expression::NewThread { .. } => {}
        Expression::Variable(variable) => assert!(
            *is_set.get(variable.0).expect(VARIABLE_ID_IN_RANGE),
            "a variable should be set before it is read"
        ),
    }
}

const VARIABLE_ID_IN_RANGE: &str = "variable id should be smaller than the number of variables";

#[cfg(test)]
mod tests {
    use super::*;

    /// A part with the body `body` and `variable_count` variables.
    fn part(body: Vec<Statement>, variable_count: usize) -> Part {
        Part::new(body, variable_count)
    }

    /// A function named `name` with an empty host part.
    fn function(name: &str) -> Function {
        Function::new(
            name.to_string(),
            HashMap::from([(Place::Host, part(vec![], 0))]),
        )
    }

    /// A statement that sets `variable` to the integer `value`.
    fn let_integer(variable: usize, value: i64) -> Statement {
        Statement::Let {
            variable: VariableId(variable),
            value: Expression::Integer(value),
        }
    }

    /// A statement that prints `variable`.
    fn print_variable(variable: usize) -> Statement {
        Statement::Print(Expression::Variable(VariableId(variable)))
    }

    #[test]
    fn part_that_follows_the_rules_is_created() {
        let body = vec![let_integer(0, 1), print_variable(0)];
        assert_eq!(part(body, 1).body().len(), 2);
    }

    #[test]
    fn variable_without_let_is_allowed() {
        assert_eq!(part(vec![], 1).variable_count(), 1);
    }

    #[test]
    #[should_panic(expected = "variable id should be smaller than the number of variables")]
    fn let_of_a_variable_out_of_range_panics() {
        part(vec![let_integer(1, 1)], 1);
    }

    #[test]
    #[should_panic(expected = "variable id should be smaller than the number of variables")]
    fn read_of_a_variable_out_of_range_panics() {
        part(vec![let_integer(0, 1), print_variable(1)], 1);
    }

    #[test]
    #[should_panic(expected = "each variable should be set by at most one `let`")]
    fn variable_set_twice_panics() {
        part(vec![let_integer(0, 1), let_integer(0, 2)], 1);
    }

    #[test]
    #[should_panic(expected = "a variable should be set before it is read")]
    fn variable_read_before_its_let_panics() {
        part(vec![print_variable(0), let_integer(0, 1)], 1);
    }

    #[test]
    #[should_panic(expected = "a variable should be set before it is read")]
    fn delete_of_a_variable_before_its_let_panics() {
        part(
            vec![
                Statement::Delete(Expression::Variable(VariableId(0))),
                let_integer(0, 1),
            ],
            1,
        );
    }

    #[test]
    #[should_panic(expected = "a function should have at least one part")]
    fn function_without_parts_panics() {
        Function::new("main".to_string(), HashMap::new());
    }

    #[test]
    fn part_on_a_place_of_the_function_is_found() {
        assert_eq!(function("main").part(Place::Host), &part(vec![], 0));
    }

    #[test]
    #[should_panic(expected = "a function should only be asked for a part on a place it runs on")]
    fn part_on_another_place_panics() {
        function("main").part(Place::Parameter(PlaceParameterId(0)));
    }

    #[test]
    #[should_panic(expected = "the `main` function should be one of the functions of the program")]
    fn program_with_an_invalid_main_id_panics() {
        Program::new(vec![], FunctionId(0));
    }

    #[test]
    #[should_panic(expected = "a function id should refer to a function of the program")]
    fn function_with_an_invalid_id_panics() {
        Program::new(vec![function("main")], FunctionId(0)).function(FunctionId(1));
    }
}
