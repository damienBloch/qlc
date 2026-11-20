//! The place checker is the sixth stage. It checks where the values of the typed AST are known.
//!
//! Today every value of a function is known by every place the function runs on.
//! A value of kind `Resource` cannot be copied, so it cannot be known by several places.
//! So a function that runs on several places cannot use a value of kind `Resource`.
//!
//! A thread runs the part of a function on one place, so only the host can create one,
//! and the part it runs cannot be the part on the host.

use std::fmt;

use crate::typed_ast::{Expression, Function, Place, Program, Statement};
use crate::types::{Kind, Type};

/// An error in where the values of a program are known.
#[derive(Debug, PartialEq, Eq)]
pub enum PlaceError {
    /// The function named `function` runs on several places and uses a value of type `value_type`,
    /// whose kind is `Resource`.
    ResourceOnSeveralPlaces { function: String, value_type: Type },
    /// The function named `function` creates a thread, but it does not run on the host only.
    ThreadOutsideHost { function: String },
    /// A thread would run the part of the function named `function` on the host.
    ThreadOnHost { function: String },
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
            PlaceError::ThreadOutsideHost { function } => write!(
                f,
                "function `{function}` does not run on the host, so it cannot create a thread"
            ),
            PlaceError::ThreadOnHost { function } => write!(
                f,
                "a thread cannot run the part of function `{function}` on the host"
            ),
        }
    }
}

/// Checks where the values of `program` are known.
pub fn check(program: &Program) -> Result<(), PlaceError> {
    for function in program.functions() {
        check_function(function, program)?;
    }
    Ok(())
}

fn check_function(function: &Function, program: &Program) -> Result<(), PlaceError> {
    for statement in function.body() {
        let value = match statement {
            Statement::Print(value)
            | Statement::Delete(value)
            | Statement::Join(value)
            | Statement::Let { variable: _, value } => value,
        };
        check_thread(value, function, program)?;
        check_resource(value, function)?;
    }
    Ok(())
}

/// Checks a thread created by `expression`, which is in the body of `function`.
fn check_thread(
    expression: &Expression,
    function: &Function,
    program: &Program,
) -> Result<(), PlaceError> {
    let Expression::NewThread {
        function: part,
        place,
    } = expression
    else {
        return Ok(());
    };
    if function.places().run_on() != [Place::Host] {
        return Err(PlaceError::ThreadOutsideHost {
            function: function.name().to_string(),
        });
    }
    if *place == Place::Host {
        return Err(PlaceError::ThreadOnHost {
            function: program.function(*part).name().to_string(),
        });
    }
    Ok(())
}

/// Checks that `expression`, which is in the body of `function`,
/// is not a resource known by several places.
fn check_resource(expression: &Expression, function: &Function) -> Result<(), PlaceError> {
    if function.places().run_on().len() == 1 {
        return Ok(());
    }
    let value_type = expression_type(expression, function);
    if value_type.kind() == Kind::Resource {
        return Err(PlaceError::ResourceOnSeveralPlaces {
            function: function.name().to_string(),
            value_type,
        });
    }
    Ok(())
}

/// The type of `expression`, which is in the body of `function`.
fn expression_type(expression: &Expression, function: &Function) -> Type {
    match expression {
        Expression::Integer(_) => Type::Int,
        Expression::NewSimDevice => Type::SimDevice,
        Expression::NewThread { .. } => Type::Thread,
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

    /// Checks a program with an empty `main`, an empty function `f` that runs on `target_run_on`,
    /// and a function `spawner` that runs on `spawner_run_on`
    /// and creates a thread for the part of `f` on `place`.
    fn check_thread(
        spawner_run_on: Vec<Place>,
        target_run_on: Vec<Place>,
        place: Place,
    ) -> Result<(), PlaceError> {
        let place_names = vec!["a".to_string(), "b".to_string()];
        let main = Function::new("main".to_string(), FunctionPlaces::host(), vec![], vec![]);
        let f = Function::new(
            "f".to_string(),
            FunctionPlaces::new(place_names.clone(), target_run_on),
            vec![],
            vec![],
        );
        let thread = Expression::NewThread {
            function: FunctionId(1),
            place,
        };
        let spawner = Function::new(
            "spawner".to_string(),
            FunctionPlaces::new(place_names, spawner_run_on),
            vec![Statement::Join(thread)],
            vec![],
        );
        check(&Program::new(vec![main, f, spawner], FunctionId(0)))
    }

    #[test]
    fn thread_created_by_the_host_is_accepted() {
        assert_eq!(check_thread(vec![Place::Host], vec![a()], a()), Ok(()));
    }

    #[test]
    fn thread_created_outside_the_host_is_an_error() {
        assert_eq!(
            check_thread(vec![a()], vec![a()], a()),
            Err(PlaceError::ThreadOutsideHost {
                function: "spawner".to_string()
            })
        );
    }

    #[test]
    fn thread_of_a_part_on_the_host_is_an_error() {
        assert_eq!(
            check_thread(vec![Place::Host], vec![Place::Host], Place::Host),
            Err(PlaceError::ThreadOnHost {
                function: "f".to_string()
            })
        );
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
