//! The abstract syntax tree (AST). The parser produces it and the interpreter consumes it.

#[derive(Debug, PartialEq, Eq)]
pub struct Program {
    pub functions: Vec<Function>,
}

/// A function without parameters, return type or statements.
/// These will be added when the language supports them.
#[derive(Debug, PartialEq, Eq)]
pub struct Function {
    pub name: String,
}
