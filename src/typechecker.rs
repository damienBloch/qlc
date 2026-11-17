//! The type checker is the fourth stage. It checks the types of the resolved AST
//! and returns the typed AST.
//!
//! The type of a variable is the type of the value of its `let`.
//! `print` only accepts an `Int`, and `delete` only accepts a `SimDevice`.

use std::fmt;

use crate::resolved_ast;
use crate::typed_ast;
use crate::types::Type;

/// An error in the types of a program.
#[derive(Debug, PartialEq, Eq)]
pub enum TypeError {
    /// A value of type `found` is used where a value of type `expected` is needed.
    Mismatch { expected: Type, found: Type },
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeError::Mismatch { expected, found } => {
                write!(f, "expected a value of type `{expected}`, found `{found}`")
            }
        }
    }
}

/// Checks the types of `program` and returns its typed AST.
pub fn check(program: resolved_ast::Program) -> Result<typed_ast::Program, TypeError> {
    let (functions, main) = program.into_parts();
    let functions = functions
        .into_iter()
        .map(check_function)
        .collect::<Result<_, _>>()?;
    Ok(typed_ast::Program::new(functions, main))
}

fn check_function(function: resolved_ast::Function) -> Result<typed_ast::Function, TypeError> {
    let (name, body, variable_count) = function.into_parts();
    // The type of each variable, indexed by its id. `None` until its `let` is checked.
    let mut variable_types = vec![None; variable_count];
    let body = body
        .into_iter()
        .map(|statement| check_statement(statement, &mut variable_types))
        .collect::<Result<_, _>>()?;
    let variable_types = variable_types
        .into_iter()
        .map(|variable_type| {
            variable_type.expect(
                "`resolved_ast::Function` should ensure that each variable is set by a `let`",
            )
        })
        .collect();
    Ok(typed_ast::Function::new(name, body, variable_types))
}

/// Checks `statement`. `variable_types` contains the type of each variable of its function,
/// or `None` if the variable is not set before `statement`.
fn check_statement(
    statement: resolved_ast::Statement,
    variable_types: &mut [Option<Type>],
) -> Result<typed_ast::Statement, TypeError> {
    match statement {
        resolved_ast::Statement::Print(value) => {
            let (value, value_type) = check_expression(value, variable_types);
            match value_type {
                Type::Int => Ok(typed_ast::Statement::Print(value)),
                Type::SimDevice => Err(TypeError::Mismatch {
                    expected: Type::Int,
                    found: value_type,
                }),
            }
        }
        resolved_ast::Statement::Delete(value) => {
            let (value, value_type) = check_expression(value, variable_types);
            match value_type {
                Type::SimDevice => Ok(typed_ast::Statement::Delete(value)),
                Type::Int => Err(TypeError::Mismatch {
                    expected: Type::SimDevice,
                    found: value_type,
                }),
            }
        }
        resolved_ast::Statement::Let { variable, value } => {
            let (value, value_type) = check_expression(value, variable_types);
            *variable_types
                .get_mut(variable.0)
                .expect(VARIABLE_ID_IN_RANGE) = Some(value_type);
            Ok(typed_ast::Statement::Let { variable, value })
        }
    }
}

/// Checks `expression` and returns it with its type.
/// `variable_types` contains the type of each variable of its function,
/// or `None` if the variable is not set before `expression`.
fn check_expression(
    expression: resolved_ast::Expression,
    variable_types: &[Option<Type>],
) -> (typed_ast::Expression, Type) {
    match expression {
        resolved_ast::Expression::Integer(value) => {
            (typed_ast::Expression::Integer(value), Type::Int)
        }
        resolved_ast::Expression::NewSimDevice => {
            (typed_ast::Expression::NewSimDevice, Type::SimDevice)
        }
        resolved_ast::Expression::Variable(variable) => (
            typed_ast::Expression::Variable(variable),
            variable_types
                .get(variable.0)
                .expect(VARIABLE_ID_IN_RANGE)
                .expect("`resolved_ast::Function` should ensure that a variable is set before it is read"),
        ),
    }
}

const VARIABLE_ID_IN_RANGE: &str = "`resolved_ast::Function::new` checks variable ids";

#[cfg(test)]
mod tests {
    use super::*;
    use resolved_ast::{FunctionId, VariableId};

    /// Checks a program whose only function is `main`, with the body `body`
    /// and `variable_count` variables.
    fn check_main_result(
        body: Vec<resolved_ast::Statement>,
        variable_count: usize,
    ) -> Result<typed_ast::Program, TypeError> {
        let main = resolved_ast::Function::new("main".to_string(), body, variable_count);
        check(resolved_ast::Program::new(vec![main], FunctionId(0)))
    }

    /// Like [`check_main_result`], for a program that has no type error.
    fn check_main(body: Vec<resolved_ast::Statement>, variable_count: usize) -> typed_ast::Program {
        check_main_result(body, variable_count).expect("the program should have no type error")
    }

    /// Like [`check_main_result`], for a program that has a type error. Returns the error.
    fn check_main_error(body: Vec<resolved_ast::Statement>, variable_count: usize) -> TypeError {
        check_main_result(body, variable_count)
            .err()
            .expect("the program should have a type error")
    }

    /// The types of the variables of the `main` function of `program`.
    fn main_variable_types(program: &typed_ast::Program) -> &[Type] {
        program.function(program.main()).variable_types()
    }

    /// A resolved statement that sets `variable` to the integer `value`.
    fn let_integer(variable: usize, value: i64) -> resolved_ast::Statement {
        resolved_ast::Statement::Let {
            variable: VariableId(variable),
            value: resolved_ast::Expression::Integer(value),
        }
    }

    /// A resolved statement that sets `variable` to the value of `source`.
    fn let_variable(variable: usize, source: usize) -> resolved_ast::Statement {
        resolved_ast::Statement::Let {
            variable: VariableId(variable),
            value: resolved_ast::Expression::Variable(VariableId(source)),
        }
    }

    /// A resolved statement that sets `variable` to a new simulated device.
    fn let_sim_device(variable: usize) -> resolved_ast::Statement {
        resolved_ast::Statement::Let {
            variable: VariableId(variable),
            value: resolved_ast::Expression::NewSimDevice,
        }
    }

    /// A resolved statement that deletes `variable`.
    fn delete_variable(variable: usize) -> resolved_ast::Statement {
        resolved_ast::Statement::Delete(resolved_ast::Expression::Variable(VariableId(variable)))
    }

    /// A resolved statement that prints `variable`.
    fn print_variable(variable: usize) -> resolved_ast::Statement {
        resolved_ast::Statement::Print(resolved_ast::Expression::Variable(VariableId(variable)))
    }

    #[test]
    fn function_bodies_are_checked() {
        let program = check_main(vec![let_integer(0, 42), print_variable(0)], 1);
        assert_eq!(
            program.function(program.main()),
            &typed_ast::Function::new(
                "main".to_string(),
                vec![
                    typed_ast::Statement::Let {
                        variable: VariableId(0),
                        value: typed_ast::Expression::Integer(42),
                    },
                    typed_ast::Statement::Print(typed_ast::Expression::Variable(VariableId(0))),
                ],
                vec![Type::Int],
            )
        );
    }

    #[test]
    fn main_stays_the_same() {
        let functions =
            ["helper", "main"].map(|name| resolved_ast::Function::new(name.to_string(), vec![], 0));
        let program = check(resolved_ast::Program::new(functions.into(), FunctionId(1)))
            .expect("the program should have no type error");
        assert_eq!(program.main(), FunctionId(1));
    }

    #[test]
    fn variable_set_to_an_integer_is_an_int() {
        let program = check_main(vec![let_integer(0, 1)], 1);
        assert_eq!(main_variable_types(&program), [Type::Int]);
    }

    #[test]
    fn variable_set_to_a_variable_has_its_type() {
        let program = check_main(vec![let_integer(0, 1), let_variable(1, 0)], 2);
        assert_eq!(main_variable_types(&program), [Type::Int, Type::Int]);
    }

    #[test]
    fn variable_set_to_a_new_sim_device_is_a_sim_device() {
        let program = check_main(vec![let_sim_device(0)], 1);
        assert_eq!(main_variable_types(&program), [Type::SimDevice]);
    }

    #[test]
    fn print_of_an_int_is_accepted() {
        check_main(vec![let_integer(0, 1), print_variable(0)], 1);
    }

    #[test]
    fn print_of_a_sim_device_is_an_error() {
        assert_eq!(
            check_main_error(vec![let_sim_device(0), print_variable(0)], 1),
            TypeError::Mismatch {
                expected: Type::Int,
                found: Type::SimDevice,
            }
        );
    }

    #[test]
    fn delete_of_a_sim_device_is_accepted() {
        check_main(vec![let_sim_device(0), delete_variable(0)], 1);
    }

    #[test]
    fn delete_of_an_int_is_an_error() {
        assert_eq!(
            check_main_error(vec![let_integer(0, 1), delete_variable(0)], 1),
            TypeError::Mismatch {
                expected: Type::SimDevice,
                found: Type::Int,
            }
        );
    }
}
