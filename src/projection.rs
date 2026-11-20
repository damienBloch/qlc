//! The projection splits each function into parts, one for each place it runs on.
//!
//! Today every function runs on the host only, so each function has one part.

use std::collections::BTreeMap;

use crate::parts_ir::{self, Part, Place};
use crate::typed_ast::{Expression, Function, Program, Statement};

/// Returns `program` with each function split into parts.
pub fn project(program: &Program) -> parts_ir::Program {
    let functions = program.functions().iter().map(project_function).collect();
    parts_ir::Program::new(functions, program.main())
}

fn project_function(function: &Function) -> parts_ir::Function {
    let body = function.body().iter().map(project_statement).collect();
    let host_part = Part::new(body, function.variable_count());
    parts_ir::Function::new(
        function.name().to_string(),
        BTreeMap::from([(Place::Host, host_part)]),
    )
}

fn project_statement(statement: &Statement) -> parts_ir::Statement {
    match statement {
        Statement::Print(value) => parts_ir::Statement::Print(project_expression(value)),
        Statement::Delete(value) => parts_ir::Statement::Delete(project_expression(value)),
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typed_ast::{FunctionId, Variable, VariableId};
    use crate::types::Type;

    /// A function with the body `body` and one variable of type `Int`.
    fn function(name: &str, body: Vec<Statement>) -> Function {
        let variables = vec![Variable {
            name: "x".to_string(),
            variable_type: Type::Int,
        }];
        Function::new(name.to_string(), body, variables)
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

    /// A projected function named `name` with one host part, whose body is `body`.
    fn host_function(name: &str, body: Vec<parts_ir::Statement>) -> parts_ir::Function {
        parts_ir::Function::new(
            name.to_string(),
            BTreeMap::from([(Place::Host, Part::new(body, 1))]),
        )
    }

    #[test]
    fn function_gives_one_host_part_with_the_same_statements() {
        let body = vec![
            let_integer(1),
            Statement::Print(Expression::Variable(VariableId(0))),
        ];
        let program = Program::new(vec![function("main", body)], FunctionId(0));

        let expected_body = vec![
            projected_let_integer(1),
            parts_ir::Statement::Print(parts_ir::Expression::Variable(VariableId(0))),
        ];
        assert_eq!(
            project(&program).functions(),
            [host_function("main", expected_body)]
        );
    }

    #[test]
    fn every_function_is_projected_and_main_is_kept() {
        let program = Program::new(
            vec![
                function("helper", vec![let_integer(1)]),
                function("main", vec![let_integer(2)]),
            ],
            FunctionId(1),
        );

        let projected = project(&program);
        assert_eq!(
            projected.functions(),
            [
                host_function("helper", vec![projected_let_integer(1)]),
                host_function("main", vec![projected_let_integer(2)]),
            ]
        );
        assert_eq!(projected.main(), FunctionId(1));
    }
}
