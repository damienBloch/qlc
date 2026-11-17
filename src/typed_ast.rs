//! The typed AST. The type checker produces it and the interpreter runs it.
//!
//! It is the resolved AST of a program whose types have been checked.
//! Each variable has a type.

pub use crate::resolved_ast::{FunctionId, VariableId};
use crate::types::Type;

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

    /// The function `id`. Panics if the program has no such function.
    pub fn function(&self, id: FunctionId) -> &Function {
        self.functions
            .get(id.0)
            .expect("a function id should refer to a function of the program")
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub body: Vec<Statement>,
    /// The type of each variable of the function, indexed by its [`VariableId`].
    pub variable_types: Vec<Type>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Statement {
    /// Prints the value of an expression on its own line.
    Print(Expression),
    /// Sets `variable` to the value of `value`.
    Let {
        variable: VariableId,
        value: Expression,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum Expression {
    /// An integer literal, such as `42`.
    Integer(i64),
    /// The value of a variable.
    Variable(VariableId),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "the `main` function should be one of the functions of the program")]
    fn program_with_an_invalid_main_id_panics() {
        Program::new(vec![], FunctionId(0));
    }

    #[test]
    #[should_panic(expected = "a function id should refer to a function of the program")]
    fn function_with_an_invalid_id_panics() {
        let main = Function {
            name: "main".to_string(),
            body: vec![],
            variable_types: vec![],
        };
        Program::new(vec![main], FunctionId(0)).function(FunctionId(1));
    }
}
