//! The interpreter is the last stage. It executes a program from its resolved AST.
//!
//! The previous stages check the program, so running it cannot fail.

use std::io::Write;

use crate::resolved_ast::{Expression, Function, Program, Statement, VariableId};

/// Runs the `main` function of `program`. What the program prints is written to `output`.
pub fn run(program: &Program, output: &mut impl Write) {
    call(program.function(program.main()), output);
}

/// Calls `function`.
fn call(function: &Function, output: &mut impl Write) {
    let mut variables = Variables::new(function.variable_count);
    for statement in &function.body {
        match statement {
            Statement::Print(expression) => {
                writeln!(output, "{}", evaluate(expression, &variables))
                    .expect("writing the program output should succeed")
            }
            Statement::Let { variable, value } => {
                let value = evaluate(value, &variables);
                variables.set(*variable, value);
            }
        }
    }
}

/// Returns the value of `expression`. `variables` contains the variables of its function.
fn evaluate(expression: &Expression, variables: &Variables) -> i64 {
    match expression {
        Expression::Integer(value) => *value,
        Expression::Variable(variable) => variables.get(*variable),
    }
}

/// The values of the variables of one function call.
struct Variables {
    /// The value of each variable, indexed by its id. `None` if the variable is not set yet.
    values: Vec<Option<i64>>,
}

impl Variables {
    /// Creates `count` variables that are not set yet.
    fn new(count: usize) -> Self {
        Variables {
            values: vec![None; count],
        }
    }

    fn set(&mut self, variable: VariableId, value: i64) {
        *self
            .values
            .get_mut(variable.0)
            .expect("a variable id should be smaller than the variable count of its function") =
            Some(value);
    }

    fn get(&self, variable: VariableId) -> i64 {
        self.values
            .get(variable.0)
            .expect("a variable id should be smaller than the variable count of its function")
            .expect("the resolver should ensure that a variable is set before it is read")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolved_ast::FunctionId;

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

    fn function_with_variables(
        name: &str,
        body: Vec<Statement>,
        variable_count: usize,
    ) -> Function {
        Function {
            name: name.to_string(),
            body,
            variable_count,
        }
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
            "42
"
        );
    }

    #[test]
    fn let_can_copy_another_variable() {
        let body = vec![
            let_statement(0, Expression::Integer(1)),
            let_statement(1, Expression::Variable(VariableId(0))),
            let_statement(0, Expression::Integer(2)),
            print_variable(0),
            print_variable(1),
        ];
        assert_eq!(
            run_to_string(&main_with_body_and_variables(body, 2)),
            "2
1
"
        );
    }

    #[test]
    #[should_panic(
        expected = "the resolver should ensure that a variable is set before it is read"
    )]
    fn reading_a_variable_that_is_not_set_panics() {
        run_to_string(&main_with_body_and_variables(vec![print_variable(0)], 1));
    }

    #[test]
    #[should_panic(
        expected = "a variable id should be smaller than the variable count of its function"
    )]
    fn reading_a_variable_that_does_not_exist_panics() {
        run_to_string(&main_with_body(vec![print_variable(0)]));
    }
}
