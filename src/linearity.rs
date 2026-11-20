//! The linearity checker is the fifth stage. It checks how the typed AST uses resources.
//!
//! Reading a variable whose type is of kind `Resource` moves its value.
//! A variable cannot be read after its value is moved.
//! The value of each such variable must be moved before the end of its function.

use std::fmt;

use crate::typed_ast::{Expression, Function, Program, Statement};
use crate::types::Kind;

/// An error in the use of resources.
#[derive(Debug, PartialEq, Eq)]
pub enum LinearityError {
    /// The variable named `variable` is read after its value is moved.
    UseAfterMove { variable: String },
    /// The value of the variable named `variable` is not moved before the end of its function.
    ValueNotUsed { variable: String },
}

impl fmt::Display for LinearityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LinearityError::UseAfterMove { variable } => {
                write!(f, "variable `{variable}` is used after its value is moved")
            }
            LinearityError::ValueNotUsed { variable } => {
                write!(f, "the value of variable `{variable}` is never used")
            }
        }
    }
}

/// Checks how `program` uses resources.
pub fn check(program: &Program) -> Result<(), LinearityError> {
    for function in program.functions() {
        check_function(function)?;
    }
    Ok(())
}

fn check_function(function: &Function) -> Result<(), LinearityError> {
    // Whether the value of each variable is moved, indexed by its id.
    let mut is_moved = vec![false; function.variable_count()];
    for statement in function.body() {
        match statement {
            Statement::Print(value)
            | Statement::Delete(value)
            | Statement::Let { variable: _, value } => {
                check_expression(value, function, &mut is_moved)?
            }
        }
    }
    let not_used = function
        .variables()
        .iter()
        .zip(is_moved)
        .find(|(variable, is_moved)| variable.variable_type.kind() == Kind::Resource && !is_moved);
    match not_used {
        Some((variable, _)) => Err(LinearityError::ValueNotUsed {
            variable: variable.name.clone(),
        }),
        None => Ok(()),
    }
}

/// Checks `expression`, which is in the body of `function`.
/// `is_moved` tells, for each variable, whether its value is moved before `expression`.
fn check_expression(
    expression: &Expression,
    function: &Function,
    is_moved: &mut [bool],
) -> Result<(), LinearityError> {
    match expression {
        Expression::Integer(_) | Expression::NewSimDevice => Ok(()),
        Expression::Variable(variable) => {
            let variable_info = function
                .variables()
                .get(variable.0)
                .expect(VARIABLE_ID_IN_RANGE);
            if variable_info.variable_type.kind() == Kind::Data {
                return Ok(());
            }
            let variable_is_moved = is_moved.get_mut(variable.0).expect(VARIABLE_ID_IN_RANGE);
            if *variable_is_moved {
                return Err(LinearityError::UseAfterMove {
                    variable: variable_info.name.clone(),
                });
            }
            *variable_is_moved = true;
            Ok(())
        }
    }
}

const VARIABLE_ID_IN_RANGE: &str = "`typed_ast::Function::new` checks variable ids";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typed_ast::{FunctionId, FunctionPlaces, Variable, VariableId};
    use crate::types::Type;

    /// Checks a program whose only function is `main`, with the body `body`
    /// and the variables `variables`, given by their name and type.
    fn check_main(body: Vec<Statement>, variables: &[(&str, Type)]) -> Result<(), LinearityError> {
        let variables = variables
            .iter()
            .map(|(name, variable_type)| Variable {
                name: name.to_string(),
                variable_type: *variable_type,
            })
            .collect();
        let main = Function::new("main".to_string(), FunctionPlaces::host(), body, variables);
        check(&Program::new(vec![main], FunctionId(0)))
    }

    /// A statement that sets `variable` to the integer `value`.
    fn let_integer(variable: usize, value: i64) -> Statement {
        Statement::Let {
            variable: VariableId(variable),
            value: Expression::Integer(value),
        }
    }

    /// A statement that sets `variable` to a new simulated device.
    fn let_sim_device(variable: usize) -> Statement {
        Statement::Let {
            variable: VariableId(variable),
            value: Expression::NewSimDevice,
        }
    }

    /// A statement that sets `variable` to the value of `source`.
    fn let_variable(variable: usize, source: usize) -> Statement {
        Statement::Let {
            variable: VariableId(variable),
            value: Expression::Variable(VariableId(source)),
        }
    }

    /// A statement that deletes `variable`.
    fn delete_variable(variable: usize) -> Statement {
        Statement::Delete(Expression::Variable(VariableId(variable)))
    }

    /// A statement that prints `variable`.
    fn print_variable(variable: usize) -> Statement {
        Statement::Print(Expression::Variable(VariableId(variable)))
    }

    fn use_after_move(variable: &str) -> LinearityError {
        LinearityError::UseAfterMove {
            variable: variable.to_string(),
        }
    }

    fn value_not_used(variable: &str) -> LinearityError {
        LinearityError::ValueNotUsed {
            variable: variable.to_string(),
        }
    }

    #[test]
    fn int_can_be_read_several_times() {
        assert_eq!(
            check_main(
                vec![let_integer(0, 1), print_variable(0), print_variable(0)],
                &[("x", Type::Int)]
            ),
            Ok(())
        );
    }

    #[test]
    fn sim_device_can_be_deleted() {
        assert_eq!(
            check_main(
                vec![let_sim_device(0), delete_variable(0)],
                &[("d", Type::SimDevice)]
            ),
            Ok(())
        );
    }

    #[test]
    fn sim_device_deleted_twice_is_an_error() {
        assert_eq!(
            check_main(
                vec![let_sim_device(0), delete_variable(0), delete_variable(0)],
                &[("d", Type::SimDevice)]
            ),
            Err(use_after_move("d"))
        );
    }

    #[test]
    fn sim_device_moved_to_another_variable_can_be_deleted_from_it() {
        assert_eq!(
            check_main(
                vec![let_sim_device(0), let_variable(1, 0), delete_variable(1)],
                &[("d", Type::SimDevice), ("e", Type::SimDevice)]
            ),
            Ok(())
        );
    }

    #[test]
    fn sim_device_moved_to_another_variable_cannot_be_deleted_from_the_first() {
        assert_eq!(
            check_main(
                vec![let_sim_device(0), let_variable(1, 0), delete_variable(0)],
                &[("d", Type::SimDevice), ("e", Type::SimDevice)]
            ),
            Err(use_after_move("d"))
        );
    }

    #[test]
    fn error_names_the_variable_even_when_it_is_shadowed() {
        assert_eq!(
            check_main(
                vec![
                    let_sim_device(0),
                    let_variable(1, 0),
                    delete_variable(1),
                    delete_variable(1),
                ],
                &[("d", Type::SimDevice), ("d", Type::SimDevice)]
            ),
            Err(use_after_move("d"))
        );
    }

    #[test]
    fn every_function_is_checked() {
        let helper = Function::new(
            "helper".to_string(),
            FunctionPlaces::host(),
            vec![let_sim_device(0), delete_variable(0), delete_variable(0)],
            vec![Variable {
                name: "d".to_string(),
                variable_type: Type::SimDevice,
            }],
        );
        let main = Function::new("main".to_string(), FunctionPlaces::host(), vec![], vec![]);
        assert_eq!(
            check(&Program::new(vec![helper, main], FunctionId(1))),
            Err(use_after_move("d"))
        );
    }

    #[test]
    fn int_can_be_unused() {
        assert_eq!(
            check_main(vec![let_integer(0, 1)], &[("x", Type::Int)]),
            Ok(())
        );
    }

    #[test]
    fn sim_device_not_deleted_is_an_error() {
        assert_eq!(
            check_main(vec![let_sim_device(0)], &[("d", Type::SimDevice)]),
            Err(value_not_used("d"))
        );
    }

    #[test]
    fn sim_device_moved_to_another_variable_not_deleted_is_an_error() {
        assert_eq!(
            check_main(
                vec![let_sim_device(0), let_variable(1, 0)],
                &[("d", Type::SimDevice), ("e", Type::SimDevice)]
            ),
            Err(value_not_used("e"))
        );
    }

    #[test]
    fn shadowed_sim_device_not_deleted_is_an_error() {
        assert_eq!(
            check_main(
                vec![let_sim_device(0), let_sim_device(1), delete_variable(1)],
                &[("d", Type::SimDevice), ("d", Type::SimDevice)]
            ),
            Err(value_not_used("d"))
        );
    }
}
