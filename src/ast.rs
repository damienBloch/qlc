//! The abstract syntax tree (AST). The parser produces it and the interpreter consumes it.

#[derive(Debug, PartialEq, Eq)]
pub struct Program {
    pub functions: Vec<Function>,
}

/// A function without parameters or return type.
/// These will be added when the language supports them.
#[derive(Debug, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub body: Vec<Statement>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Statement {
    /// Prints an integer on its own line.
    Print(i64),
}
