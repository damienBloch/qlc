//! The place checker is the sixth stage. It checks where the values of the typed AST are known.
//!
//! Today every value of a function is known by every place the function runs on.
//! A value of kind `Resource` cannot be copied, so it cannot be known by several places.
//! So a function that runs on several places cannot use a value of kind `Resource`.

use std::fmt;

use crate::typed_ast::{Expression, Function, Program, Statement};
use crate::types::{Kind, Type};

/// An error in where the values of a program are known.
#[derive(Debug, PartialEq, Eq)]
pub enum PlaceError {
    /// The function named `function` runs on several places and uses a value of type `value_type`,
    /// whose kind is `Resource`.
    ResourceOnSeveralPlaces { function: String, value_type: Type },
}

impl fmt::Display for PlaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlaceError::ResourceOnSeveralPlaces {
                function,
                value_type,
            } => write!(
                f,
                "function `{function}` runs on several places, \
                 so it cannot use a value of type `{value_type}`, which cannot be copied"
            ),
        }
    }
}

/// Checks where the values of `program` are known.
pub fn check(program: &Program) -> Result<(), PlaceError> {
    for function in program.functions() {
        check_function(function)?;
    }
    Ok(())
}

fn check_function(function: &Function) -> Result<(), PlaceError> {
    if function.places().run_on().len() == 1 {
        return Ok(());
    }
    for statement in function.body() {
        match statement {
            Statement::Print(value)
            | Statement::Delete(value)
            | Statement::Let { variable: _, value } => {
                let value_type = expression_type(value, function);
                if value_type.kind() == Kind::Resource {
                    return Err(PlaceError::ResourceOnSeveralPlaces {
                        function: function.name().to_string(),
                        value_type,
                    });
                }
            }
        }
    }
    Ok(())
}

/// The type of `expression`, which is in the body of `function`.
fn expression_type(expression: &Expression, function: &Function) -> Type {
    match expression {
        Expression::Integer(_) => Type::Int,
        Expression::NewSimDevice => Type::SimDevice,
        Expression::Variable(variable) => {
            function
                .variables()
                .get(variable.0)
                .expect("`typed_ast::Function::new` checks variable ids")
                .variable_type
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typed_ast::{
        FunctionId, FunctionPlaces, Place, PlaceParameterId, Variable, VariableId,
    };

    /// Checks a program with an empty `main` and a function `f` that runs on `run_on`,
    /// with the body `body` and the variables `variables`, given by their name and type.
    /// `f` has two place parameters.
    fn check_f(
        run_on: Vec<Place>,
        body: Vec<Statement>,
        variables: &[(&str, Type)],
    ) -> Result<(), PlaceError> {
        let variables = variables
            .iter()
            .map(|(name, variable_type)| Variable {
                name: name.to_string(),
                variable_type: *variable_type,
            })
            .collect();
        let places = FunctionPlaces::new(vec!["a".to_string(), "b".to_string()], run_on);
        let f = Function::new("f".to_string(), places, body, variables);
        let main = Function::new("main".to_string(), FunctionPlaces::host(), vec![], vec![]);
        check(&Program::new(vec![main, f], FunctionId(0)))
    }

    fn a() -> Place {
        Place::Parameter(PlaceParameterId(0))
    }

    fn b() -> Place {
        Place::Parameter(PlaceParameterId(1))
    }

    /// A statement that sets `variable` to the value of `value`.
    fn let_statement(variable: usize, value: Expression) -> Statement {
        Statement::Let {
            variable: VariableId(variable),
            value,
        }
    }

    /// A statement that deletes `variable`.
    fn delete_variable(variable: usize) -> Statement {
        Statement::Delete(Expression::Variable(VariableId(variable)))
    }

    /// The error for a `SimDevice` in the function `f`.
    fn sim_device_error() -> PlaceError {
        PlaceError::ResourceOnSeveralPlaces {
            function: "f".to_string(),
            value_type: Type::SimDevice,
        }
    }

    #[test]
    fn integers_on_several_places_are_accepted() {
        let body = vec![
            let_statement(0, Expression::Integer(1)),
            Statement::Print(Expression::Variable(VariableId(0))),
        ];
        assert_eq!(check_f(vec![a(), b()], body, &[("x", Type::Int)]), Ok(()));
    }

    #[test]
    fn sim_device_on_one_place_is_accepted() {
        let body = vec![
            let_statement(0, Expression::NewSimDevice),
            delete_variable(0),
        ];
        assert_eq!(check_f(vec![a()], body, &[("d", Type::SimDevice)]), Ok(()));
    }

    #[test]
    fn new_sim_device_on_several_places_is_an_error() {
        let body = vec![Statement::Delete(Expression::NewSimDevice)];
        assert_eq!(check_f(vec![a(), b()], body, &[]), Err(sim_device_error()));
    }

    #[test]
    fn sim_device_in_a_variable_on_several_places_is_an_error() {
        let body = vec![
            let_statement(0, Expression::NewSimDevice),
            delete_variable(0),
        ];
        assert_eq!(
            check_f(vec![a(), b()], body, &[("d", Type::SimDevice)]),
            Err(sim_device_error())
        );
    }
}
