//! The interpreter is the last stage. It executes a program from its typed AST.
//!
//! The previous stages check the program, so running it cannot fail.

use std::io::Write;

use crate::typed_ast::{Expression, Function, Program, Statement};

/// Runs the `main` function of `program`. What the program prints is written to `output`.
pub fn run(program: &Program, output: &mut impl Write) {
    call(program.function(program.main()), output);
}

/// Calls `function`.
fn call(function: &Function, output: &mut impl Write) {
    // The value of each variable, indexed by its id. `None` until the variable is set.
    let mut variables = vec![None; function.variable_count()];
    for statement in function.body() {
        match statement {
            Statement::Print(expression) => match evaluate(expression, &variables) {
                Value::Int(value) => {
                    writeln!(output, "{value}").expect("writing the program output should succeed")
                }
                Value::SimDevice => {
                    panic!("the type checker should ensure that `print` gets an `Int`")
                }
            },
            Statement::Delete(expression) => match evaluate(expression, &variables) {
                Value::SimDevice => {}
                Value::Int(_) => {
                    panic!("the type checker should ensure that `delete` gets a `SimDevice`")
                }
            },
            Statement::Let { variable, value } => {
                let value = evaluate(value, &variables);
                *variables.get_mut(variable.0).expect(VARIABLE_ID_IN_RANGE) = Some(value);
            }
        }
    }
}

/// Returns the value of `expression`.
/// `variables` contains the value of each variable of its function, or `None` if it is not set.
fn evaluate(expression: &Expression, variables: &[Option<Value>]) -> Value {
    match expression {
        Expression::Integer(value) => Value::Int(*value),
        Expression::NewSimDevice => Value::SimDevice,
        Expression::Variable(variable) => variables
            .get(variable.0)
            .expect(VARIABLE_ID_IN_RANGE)
            .expect("`typed_ast::Function` should ensure that a variable is set before it is read"),
    }
}

/// A value computed by the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Value {
    Int(i64),
    /// A simulated device.
    SimDevice,
}

const VARIABLE_ID_IN_RANGE: &str = "`typed_ast::Function::new` checks variable ids";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typed_ast::{FunctionId, Variable, VariableId};
    use crate::types::Type;

    /// Runs `program` and returns what it printed.
    fn run_to_string(program: &Program) -> String {
        let mut output = Vec::new();
        run(program, &mut output);
        String::from_utf8(output).expect("the output should be UTF-8")
    }

    /// A function without variables.
    fn function(name: &str, body: Vec<Statement>) -> Function {
        function_with_variables(name, body, 0)
    }

    /// A function with `variable_count` variables of type `Int`.
    fn function_with_variables(
        name: &str,
        body: Vec<Statement>,
        variable_count: usize,
    ) -> Function {
        let variables = (0..variable_count)
            .map(|i| Variable {
                name: format!("v{i}"),
                variable_type: Type::Int,
            })
            .collect();
        Function::new(name.to_string(), body, variables)
    }

    /// A statement that prints the integer `value`.
    fn print(value: i64) -> Statement {
        Statement::Print(Expression::Integer(value))
    }

    /// A statement that prints `variable`.
    fn print_variable(variable: usize) -> Statement {
        Statement::Print(Expression::Variable(VariableId(variable)))
    }

    /// A statement that sets `variable` to the value of `value`.
    fn let_statement(variable: usize, value: Expression) -> Statement {
        Statement::Let {
            variable: VariableId(variable),
            value,
        }
    }

    /// A program with only a `main` function, whose body is `body`.
    fn main_with_body(body: Vec<Statement>) -> Program {
        main_with_body_and_variables(body, 0)
    }

    /// A program with only a `main` function, whose body is `body` and which has `variable_count` variables.
    fn main_with_body_and_variables(body: Vec<Statement>, variable_count: usize) -> Program {
        Program::new(
            vec![function_with_variables("main", body, variable_count)],
            FunctionId(0),
        )
    }

    #[test]
    fn runs_empty_main() {
        assert_eq!(run_to_string(&main_with_body(vec![])), "");
    }

    #[test]
    fn runs_only_the_main_function() {
        let program = Program::new(
            vec![
                function("helper", vec![print(1)]),
                function("main", vec![print(2)]),
            ],
            FunctionId(1),
        );
        assert_eq!(run_to_string(&program), "2\n");
    }

    #[test]
    fn print_writes_the_value_on_its_own_line() {
        assert_eq!(run_to_string(&main_with_body(vec![print(42)])), "42\n");
    }

    #[test]
    fn statements_run_in_order() {
        let body = vec![print(1), print(i64::MAX), print(0)];
        assert_eq!(
            run_to_string(&main_with_body(body)),
            "1\n9223372036854775807\n0\n"
        );
    }

    #[test]
    fn print_writes_the_value_of_a_variable() {
        let body = vec![let_statement(0, Expression::Integer(42)), print_variable(0)];
        assert_eq!(
            run_to_string(&main_with_body_and_variables(body, 1)),
            "42\n"
        );
    }

    #[test]
    fn let_can_copy_another_variable() {
        let body = vec![
            let_statement(0, Expression::Integer(1)),
            let_statement(1, Expression::Variable(VariableId(0))),
            print_variable(0),
            print_variable(1),
        ];
        assert_eq!(
            run_to_string(&main_with_body_and_variables(body, 2)),
            "1\n1\n"
        );
    }

    #[test]
    fn new_sim_device_can_be_stored_in_a_variable() {
        let body = vec![let_statement(0, Expression::NewSimDevice), print(1)];
        assert_eq!(run_to_string(&main_with_body_and_variables(body, 1)), "1\n");
    }

    #[test]
    fn delete_of_a_sim_device_prints_nothing() {
        let body = vec![
            let_statement(0, Expression::NewSimDevice),
            Statement::Delete(Expression::Variable(VariableId(0))),
        ];
        assert_eq!(run_to_string(&main_with_body_and_variables(body, 1)), "");
    }
}
