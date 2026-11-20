//! The projection splits each function into parts, one for each place it runs on.
//!
//! Today every value of a function is known by every place the function runs on.
//! So each part has the whole body of the function.

use crate::parts_ir::{self, Part};
use crate::typed_ast::{Expression, Function, Program, Statement};

/// Returns `program` with each function split into parts.
pub fn project(program: &Program) -> parts_ir::Program {
    let functions = program.functions().iter().map(project_function).collect();
    parts_ir::Program::new(functions, program.main())
}

fn project_function(function: &Function) -> parts_ir::Function {
    let parts = function
        .places()
        .run_on()
        .iter()
        .map(|&place| {
            let body = function.body().iter().map(project_statement).collect();
            (place, Part::new(body, function.variable_count()))
        })
        .collect();
    parts_ir::Function::new(function.name().to_string(), parts)
}

fn project_statement(statement: &Statement) -> parts_ir::Statement {
    match statement {
        Statement::Print(value) => parts_ir::Statement::Print(project_expression(value)),
        Statement::Delete(value) => parts_ir::Statement::Delete(project_expression(value)),
        Statement::Join(value) => parts_ir::Statement::Join(project_expression(value)),
        Statement::Let { variable, value } => parts_ir::Statement::Let {
            variable: *variable,
            value: project_expression(value),
        },
    }
}

fn project_expression(expression: &Expression) -> parts_ir::Expression {
    match expression {
        Expression::Integer(value) => parts_ir::Expression::Integer(*value),
        Expression::Variable(variable) => parts_ir::Expression::Variable(*variable),
        Expression::NewSimDevice => parts_ir::Expression::NewSimDevice,
        Expression::NewThread { function, place } => parts_ir::Expression::NewThread {
            function: *function,
            place: *place,
        },
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::typed_ast::{
        FunctionId, FunctionPlaces, Place, PlaceParameterId, Variable, VariableId,
    };
    use crate::types::Type;

    /// A function that runs on `host`, with the body `body` and one variable of type `Int`.
    fn host_function(name: &str, body: Vec<Statement>) -> Function {
        function_on(name, FunctionPlaces::host(), body)
    }

    /// A function that runs on `places`, with the body `body` and one variable of type `Int`.
    fn function_on(name: &str, places: FunctionPlaces, body: Vec<Statement>) -> Function {
        let variables = vec![Variable {
            name: "x".to_string(),
            variable_type: Type::Int,
        }];
        Function::new(name.to_string(), places, body, variables)
    }

    /// A statement that sets the variable to `value`.
    fn let_integer(value: i64) -> Statement {
        Statement::Let {
            variable: VariableId(0),
            value: Expression::Integer(value),
        }
    }

    /// The projection of a `let_integer(value)` statement.
    fn projected_let_integer(value: i64) -> parts_ir::Statement {
        parts_ir::Statement::Let {
            variable: VariableId(0),
            value: parts_ir::Expression::Integer(value),
        }
    }

    /// A part with the body `body` and one variable.
    fn part(body: Vec<parts_ir::Statement>) -> Part {
        Part::new(body, 1)
    }

    /// A projected function named `name` with one host part, whose body is `body`.
    fn projected_host_function(name: &str, body: Vec<parts_ir::Statement>) -> parts_ir::Function {
        parts_ir::Function::new(name.to_string(), HashMap::from([(Place::Host, part(body))]))
    }

    #[test]
    fn function_on_host_gives_one_host_part_with_the_same_statements() {
        let body = vec![
            let_integer(1),
            Statement::Print(Expression::Variable(VariableId(0))),
        ];
        let program = Program::new(vec![host_function("main", body)], FunctionId(0));

        let expected_body = vec![
            projected_let_integer(1),
            parts_ir::Statement::Print(parts_ir::Expression::Variable(VariableId(0))),
        ];
        assert_eq!(
            project(&program).functions(),
            [projected_host_function("main", expected_body)]
        );
    }

    #[test]
    fn function_on_two_places_gives_one_part_per_place_with_the_whole_body() {
        let a = Place::Parameter(PlaceParameterId(0));
        let b = Place::Parameter(PlaceParameterId(1));
        let places = FunctionPlaces::new(vec!["a".to_string(), "b".to_string()], vec![a, b]);
        let program = Program::new(
            vec![
                host_function("main", vec![let_integer(0)]),
                function_on("hello", places, vec![let_integer(1)]),
            ],
            FunctionId(0),
        );

        let expected_parts = HashMap::from([
            (a, part(vec![projected_let_integer(1)])),
            (b, part(vec![projected_let_integer(1)])),
        ]);
        assert_eq!(project(&program).functions()[1].parts(), &expected_parts);
    }

    #[test]
    fn every_function_is_projected_and_main_is_kept() {
        let program = Program::new(
            vec![
                host_function("helper", vec![let_integer(1)]),
                host_function("main", vec![let_integer(2)]),
            ],
            FunctionId(1),
        );

        let projected = project(&program);
        assert_eq!(
            projected.functions(),
            [
                projected_host_function("helper", vec![projected_let_integer(1)]),
                projected_host_function("main", vec![projected_let_integer(2)]),
            ]
        );
        assert_eq!(projected.main(), FunctionId(1));
    }
}
