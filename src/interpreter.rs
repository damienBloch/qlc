//! The interpreter is the last stage. It executes a program from its AST.

use std::collections::HashSet;
use std::fmt;
use std::io::Write;

use crate::ast::{Function, Program, Statement};

#[derive(Debug, PartialEq, Eq)]
pub enum RunError {
    MissingMain,
    DuplicateFunction(String),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::MissingMain => write!(f, "no `main` function found"),
            RunError::DuplicateFunction(name) => {
                write!(f, "function `{name}` is defined more than once")
            }
        }
    }
}

/// Runs the `main` function of `program`. What the program prints is written to `output`.
pub fn run(program: &Program, output: &mut impl Write) -> Result<(), RunError> {
    check_no_duplicate_functions(program)?;
    let main = program
        .functions
        .iter()
        .find(|function| function.name == "main")
        .ok_or(RunError::MissingMain)?;
    call(main, output);
    Ok(())
}

fn check_no_duplicate_functions(program: &Program) -> Result<(), RunError> {
    let mut names = HashSet::new();
    for function in &program.functions {
        if !names.insert(function.name.as_str()) {
            return Err(RunError::DuplicateFunction(function.name.clone()));
        }
    }
    Ok(())
}

/// Calls `function`.
fn call(function: &Function, output: &mut impl Write) {
    for statement in &function.body {
        match statement {
            Statement::Print(value) => {
                writeln!(output, "{value}").expect("writing the program output should succeed")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs `program` and returns what it printed.
    fn run_to_string(program: &Program) -> Result<String, RunError> {
        let mut output = Vec::new();
        run(program, &mut output)?;
        Ok(String::from_utf8(output).expect("the output should be UTF-8"))
    }

    /// A program with only a `main` function, whose body is `body`.
    fn main_with_body(body: Vec<Statement>) -> Program {
        Program {
            functions: vec![Function {
                name: "main".to_string(),
                body,
            }],
        }
    }

    /// A program with empty functions named `function_names`.
    fn program(function_names: &[&str]) -> Program {
        Program {
            functions: function_names
                .iter()
                .map(|name| Function {
                    name: name.to_string(),
                    body: vec![],
                })
                .collect(),
        }
    }

    #[test]
    fn runs_program_with_main() {
        assert_eq!(run_to_string(&program(&["main"])), Ok(String::new()));
    }

    #[test]
    fn runs_program_with_main_and_other_functions() {
        assert_eq!(
            run_to_string(&program(&["helper", "main"])),
            Ok(String::new())
        );
    }

    #[test]
    fn print_writes_the_value_on_its_own_line() {
        assert_eq!(
            run_to_string(&main_with_body(vec![Statement::Print(42)])),
            Ok("42\n".to_string())
        );
    }

    #[test]
    fn statements_run_in_order() {
        let body = vec![
            Statement::Print(1),
            Statement::Print(i64::MAX),
            Statement::Print(0),
        ];
        assert_eq!(
            run_to_string(&main_with_body(body)),
            Ok("1\n9223372036854775807\n0\n".to_string())
        );
    }

    #[test]
    fn missing_main_is_an_error() {
        assert_eq!(
            run_to_string(&program(&["helper"])),
            Err(RunError::MissingMain)
        );
    }

    #[test]
    fn duplicate_function_is_an_error() {
        assert_eq!(
            run_to_string(&program(&["main", "main"])),
            Err(RunError::DuplicateFunction("main".to_string()))
        );
    }
}
