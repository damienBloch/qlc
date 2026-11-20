//! The abstract syntax tree (AST). The parser produces it and the resolver consumes it.

#[derive(Debug, PartialEq, Eq)]
pub struct Program {
    pub functions: Vec<Function>,
}

/// A function without parameters or return type.
/// These will be added when the language supports them.
#[derive(Debug, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    /// The names of the place parameters, without the `@`.
    ///
    /// - `fn f[@a, @b]@(a, b)() {}` gives `["a", "b"]`.
    /// - `fn f() {}` gives `[]`.
    pub place_parameters: Vec<String>,
    /// The names of the places the function runs on, as written after the place parameters.
    ///
    /// - `fn f[@a]@a() {}` gives `Some(["a"])`.
    /// - `fn f[@a, @b]@(a, b)() {}` gives `Some(["a", "b"])`.
    /// - `fn f() {}` gives `None`.
    pub places: Option<Vec<String>>,
    pub body: Vec<Statement>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Statement {
    /// Prints the value of an expression on its own line.
    Print(Expression),
    /// Deletes the device that is the value of an expression.
    Delete(Expression),
    /// Defines a variable named `name` and sets it to the value of `value`.
    Let { name: String, value: Expression },
}

#[derive(Debug, PartialEq, Eq)]
pub enum Expression {
    /// An integer literal, such as `42`.
    Integer(i64),
    /// The value of the variable named by the identifier.
    Variable(String),
    /// Creates a simulated device.
    NewSimDevice,
}
