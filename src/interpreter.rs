//! The interpreter is the last stage. It executes a program from its AST.

use std::collections::HashSet;
use std::fmt;

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

/// Runs the `main` function of `program`.
pub fn run(program: &Program) -> Result<(), RunError> {
    check_no_duplicate_functions(program)?;
    let main = program
        .functions
        .iter()
        .find(|function| function.name == "main")
        .ok_or(RunError::MissingMain)?;
    call(main);
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
fn call(function: &Function) {
    for statement in &function.body {
        match statement {
            Statement::Print(_) => todo!("the interpreter does not support `print` yet"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(run(&program(&["main"])), Ok(()));
    }

    #[test]
    fn runs_program_with_main_and_other_functions() {
        assert_eq!(run(&program(&["helper", "main"])), Ok(()));
    }

    #[test]
    fn missing_main_is_an_error() {
        assert_eq!(run(&program(&["helper"])), Err(RunError::MissingMain));
    }

    #[test]
    fn duplicate_function_is_an_error() {
        assert_eq!(
            run(&program(&["main", "main"])),
            Err(RunError::DuplicateFunction("main".to_string()))
        );
    }
}
