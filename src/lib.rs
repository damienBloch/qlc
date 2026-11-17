//! The qlc compiler and interpreter.
//!
//! The work is split in stages. Each stage consumes the output of the previous one:
//! source text -> [`lexer`] -> tokens -> [`parser`] -> AST -> [`resolver`] -> resolved AST
//! -> [`typechecker`] -> typed AST -> [`linearity`] check -> [`interpreter`].
//! See `docs/architecture.md` for details.

pub mod ast;
pub mod interpreter;
pub mod lexer;
pub mod linearity;
pub mod parser;
pub mod resolved_ast;
pub mod resolver;
pub mod typechecker;
pub mod typed_ast;
pub mod types;

use std::fmt;
use std::io::Write;

/// An error found in the user's program, by any stage.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Lex(lexer::LexError),
    Parse(parser::ParseError),
    Resolve(resolver::ResolveError),
    Type(typechecker::TypeError),
    Linearity(linearity::LinearityError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Lex(error) => write!(f, "{error}"),
            Error::Parse(error) => write!(f, "{error}"),
            Error::Resolve(error) => write!(f, "{error}"),
            Error::Type(error) => write!(f, "{error}"),
            Error::Linearity(error) => write!(f, "{error}"),
        }
    }
}

/// Runs all stages on `source`. What the program prints is written to `output`.
pub fn run_source(source: &str, output: &mut impl Write) -> Result<(), Error> {
    let tokens = lexer::tokenize(source).map_err(Error::Lex)?;
    let program = parser::parse(&tokens).map_err(Error::Parse)?;
    let program = resolver::resolve(program).map_err(Error::Resolve)?;
    let program = typechecker::check(program).map_err(Error::Type)?;
    linearity::check(&program).map_err(Error::Linearity)?;
    interpreter::run(&program, output);
    Ok(())
}
