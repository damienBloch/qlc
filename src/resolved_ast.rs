//! The resolved AST. The resolver produces it and the outliner consumes it.
//!
//! It is the AST of a program whose names have been checked:
//! function names are unique, and the `main` function exists.

use crate::ast::Function;

pub struct Program {
    functions: Vec<Function>,
    main: FunctionId,
}

impl Program {
    /// Creates a program whose `main` function is `functions[main]`.
    pub fn new(functions: Vec<Function>, main: FunctionId) -> Self {
        assert!(
            main.0 < functions.len(),
            "the `main` function should be one of the functions of the program"
        );
        Program { functions, main }
    }

    pub fn functions(&self) -> &[Function] {
        &self.functions
    }

    /// The id of the `main` function.
    pub fn main(&self) -> FunctionId {
        self.main
    }
}

/// The index of a function in the functions of a program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionId(pub usize);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "the `main` function should be one of the functions of the program")]
    fn program_with_an_invalid_main_id_panics() {
        Program::new(vec![], FunctionId(0));
    }
}
