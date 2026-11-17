//! The resolver is the third stage. It checks the names of the AST and returns the resolved AST.
//!
//! Today it checks the function names: they must be unique, and there must be a `main` function.

use std::collections::HashSet;
use std::fmt;

use crate::ast;
use crate::resolved_ast::{self, FunctionId};

#[derive(Debug, PartialEq, Eq)]
pub enum ResolveError {
    MissingMain,
    DuplicateFunction(String),
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolveError::MissingMain => write!(f, "no `main` function found"),
            ResolveError::DuplicateFunction(name) => {
                write!(f, "function `{name}` is defined more than once")
            }
        }
    }
}

/// Checks the names of `program` and returns its resolved AST.
pub fn resolve(program: ast::Program) -> Result<resolved_ast::Program, ResolveError> {
    check_no_duplicate_functions(&program)?;
    let main = program
        .functions
        .iter()
        .position(|function| function.name == "main")
        .ok_or(ResolveError::MissingMain)?;
    let functions = program
        .functions
        .into_iter()
        .map(resolve_function)
        .collect();
    Ok(resolved_ast::Program::new(functions, FunctionId(main)))
}

fn resolve_function(function: ast::Function) -> resolved_ast::Function {
    resolved_ast::Function {
        name: function.name,
        body: function.body.into_iter().map(resolve_statement).collect(),
    }
}

fn resolve_statement(statement: ast::Statement) -> resolved_ast::Statement {
    match statement {
        ast::Statement::Print(value) => resolved_ast::Statement::Print(resolve_expression(value)),
    }
}

fn resolve_expression(expression: ast::Expression) -> resolved_ast::Expression {
    match expression {
        ast::Expression::Integer(value) => resolved_ast::Expression::Integer(value),
    }
}

fn check_no_duplicate_functions(program: &ast::Program) -> Result<(), ResolveError> {
    let mut names = HashSet::new();
    for function in &program.functions {
        if !names.insert(function.name.as_str()) {
            return Err(ResolveError::DuplicateFunction(function.name.clone()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A program with empty functions named `function_names`.
    fn program(function_names: &[&str]) -> ast::Program {
        ast::Program {
            functions: function_names
                .iter()
                .map(|name| ast::Function {
                    name: name.to_string(),
                    body: vec![],
                })
                .collect(),
        }
    }

    /// Resolves `program` and returns the id of its `main` function.
    fn main_id(program: ast::Program) -> Result<FunctionId, ResolveError> {
        Ok(resolve(program)?.main())
    }

    #[test]
    fn main_is_found() {
        assert_eq!(main_id(program(&["main"])), Ok(FunctionId(0)));
    }

    #[test]
    fn main_is_found_among_other_functions() {
        assert_eq!(
            main_id(program(&["helper", "main", "other"])),
            Ok(FunctionId(1))
        );
    }

    #[test]
    fn missing_main_is_an_error() {
        assert_eq!(
            resolve(program(&["helper"])).err(),
            Some(ResolveError::MissingMain)
        );
    }

    #[test]
    fn empty_program_has_no_main() {
        assert_eq!(resolve(program(&[])).err(), Some(ResolveError::MissingMain));
    }

    #[test]
    fn function_bodies_are_resolved() {
        let program = ast::Program {
            functions: vec![ast::Function {
                name: "main".to_string(),
                body: vec![ast::Statement::Print(ast::Expression::Integer(42))],
            }],
        };
        let program = resolve(program).expect("the program should be valid");
        assert_eq!(
            program.function(program.main()),
            &resolved_ast::Function {
                name: "main".to_string(),
                body: vec![resolved_ast::Statement::Print(
                    resolved_ast::Expression::Integer(42)
                )],
            }
        );
    }

    #[test]
    fn duplicate_function_is_an_error() {
        assert_eq!(
            resolve(program(&["main", "helper", "helper"])).err(),
            Some(ResolveError::DuplicateFunction("helper".to_string()))
        );
    }

    #[test]
    fn duplicate_main_is_an_error() {
        assert_eq!(
            resolve(program(&["main", "main"])).err(),
            Some(ResolveError::DuplicateFunction("main".to_string()))
        );
    }
}
