//! The interpreter is the last stage. It executes a program from its resolved AST.
//!
//! The previous stages check the program, so running it cannot fail.

use std::io::Write;

use crate::ast::{Expression, Function, Statement};
use crate::resolved_ast::Program;

/// Runs the `main` function of `program`. What the program prints is written to `output`.
pub fn run(program: &Program, output: &mut impl Write) {
    call(program.function(program.main()), output);
}

/// Calls `function`.
fn call(function: &Function, output: &mut impl Write) {
    for statement in &function.body {
        match statement {
            Statement::Print(expression) => writeln!(output, "{}", evaluate(expression))
                .expect("writing the program output should succeed"),
        }
    }
}

/// Returns the value of `expression`.
fn evaluate(expression: &Expression) -> i64 {
    match expression {
        Expression::Integer(value) => *value,
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

    fn function(name: &str, body: Vec<Statement>) -> Function {
        Function {
            name: name.to_string(),
            body,
        }
    }

    /// A statement that prints the integer `value`.
    fn print(value: i64) -> Statement {
        Statement::Print(Expression::Integer(value))
    }

    /// A program with only a `main` function, whose body is `body`.
    fn main_with_body(body: Vec<Statement>) -> Program {
        Program::new(vec![function("main", body)], FunctionId(0))
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
}
