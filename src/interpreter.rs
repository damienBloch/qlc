//! The interpreter is the last stage. It executes a program from its AST.

use std::collections::HashSet;
use std::fmt;
use std::io::Write;

use crate::ast::{Function, Program, Statement};
use crate::targets::Registry;

#[derive(Debug, PartialEq, Eq)]
pub enum RunError {
    MissingMain,
    DuplicateFunction(String),
    /// An `on` block uses a target that is not in the registry.
    UnknownTarget(String),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::MissingMain => write!(f, "no `main` function found"),
            RunError::DuplicateFunction(name) => {
                write!(f, "function `{name}` is defined more than once")
            }
            RunError::UnknownTarget(name) => write!(f, "unknown target `{name}`"),
        }
    }
}

/// Runs the `main` function of `program`. What the program prints is written to `output`.
///
/// `on` blocks run on the targets of `targets`.
/// The program is checked before it runs, so an error is returned before anything is printed.
pub fn run(program: &Program, targets: &Registry, output: &mut impl Write) -> Result<(), RunError> {
    check_no_duplicate_functions(program)?;
    check_targets_exist(program, targets)?;
    let main = program
        .functions
        .iter()
        .find(|function| function.name == "main")
        .ok_or(RunError::MissingMain)?;
    call(main, targets, output);
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

/// Checks that the `on` blocks of all functions use targets of `targets`.
///
/// Only the `on` blocks run by the host are checked.
/// An `on` block nested in another one is run by a device, which is not supported yet.
fn check_targets_exist(program: &Program, targets: &Registry) -> Result<(), RunError> {
    for function in &program.functions {
        for statement in &function.body {
            if let Statement::On { target, .. } = statement
                && targets.get(target).is_none()
            {
                return Err(RunError::UnknownTarget(target.clone()));
            }
        }
    }
    Ok(())
}

/// Calls `function`.
fn call(function: &Function, targets: &Registry, output: &mut impl Write) {
    for statement in &function.body {
        match statement {
            Statement::Print(value) => {
                writeln!(output, "{value}").expect("writing the program output should succeed")
            }
            Statement::On { target, body } => targets
                .get(target)
                .expect("targets should be checked before the program runs")
                .run(body, output),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::targets::Target;

    /// A target that writes the statements it receives, such as `fake: [Print(1)]`.
    /// It lets the tests check what the interpreter sends to a target.
    struct FakeTarget;

    impl Target for FakeTarget {
        fn run(&self, body: &[Statement], output: &mut dyn Write) {
            writeln!(output, "fake: {body:?}").expect("writing to a Vec should succeed");
        }
    }

    /// Runs `program` with one target, `fake`, and returns what it printed.
    fn run_to_string(program: &Program) -> Result<String, RunError> {
        let mut targets = Registry::empty();
        targets.add("fake", Box::new(FakeTarget));
        let mut output = Vec::new();
        run(program, &targets, &mut output)?;
        Ok(String::from_utf8(output).expect("the output should be UTF-8"))
    }

    /// An `on` block that runs `body` on `target`.
    fn on(target: &str, body: Vec<Statement>) -> Statement {
        Statement::On {
            target: target.to_string(),
            body,
        }
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
    fn on_block_is_sent_to_its_target() {
        let body = vec![
            Statement::Print(1),
            on("fake", vec![Statement::Print(2)]),
            Statement::Print(3),
        ];
        assert_eq!(
            run_to_string(&main_with_body(body)),
            Ok("1
fake: [Print(2)]
3
"
            .to_string())
        );
    }

    #[test]
    fn unknown_target_is_an_error_before_anything_runs() {
        let body = vec![Statement::Print(1), on("unknown", vec![])];
        assert_eq!(
            run_to_string(&main_with_body(body)),
            Err(RunError::UnknownTarget("unknown".to_string()))
        );
    }

    #[test]
    fn unknown_target_in_a_function_that_is_not_called_is_an_error() {
        let program = Program {
            functions: vec![
                Function {
                    name: "main".to_string(),
                    body: vec![],
                },
                Function {
                    name: "helper".to_string(),
                    body: vec![on("unknown", vec![])],
                },
            ],
        };
        assert_eq!(
            run_to_string(&program),
            Err(RunError::UnknownTarget("unknown".to_string()))
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
