//! The qlc compiler and interpreter.
//!
//! The work is split in stages. Each stage consumes the output of the previous one:
//! source text -> [`lexer`] -> tokens -> [`parser`] -> AST -> [`interpreter`].
//! See `docs/architecture.md` for details.

pub mod ast;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod targets;

use std::fmt;
use std::io::Write;

/// An error found in the user's program, by any stage.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Lex(lexer::LexError),
    Parse(parser::ParseError),
    Run(interpreter::RunError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Lex(error) => write!(f, "{error}"),
            Error::Parse(error) => write!(f, "{error}"),
            Error::Run(error) => write!(f, "{error}"),
        }
    }
}

/// Runs all stages on `source`. What the program prints is written to `output`.
pub fn run_source(source: &str, output: &mut impl Write) -> Result<(), Error> {
    let tokens = lexer::tokenize(source).map_err(Error::Lex)?;
    let program = parser::parse(&tokens).map_err(Error::Parse)?;
    interpreter::run(&program, output).map_err(Error::Run)
}
