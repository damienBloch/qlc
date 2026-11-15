//! The parser is the second stage. It turns a list of tokens into an AST.
//!
//! Grammar:
//!
//! ```text
//! program  = function*
//! function = "fn" identifier "(" ")" "{" "}"
//! ```

use std::fmt;

use crate::ast::{Function, Program};
use crate::lexer::Token;

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    UnexpectedToken { expected: String, found: Token },
    UnexpectedEndOfInput { expected: String },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedToken { expected, found } => {
                write!(f, "expected {expected}, found {found}")
            }
            ParseError::UnexpectedEndOfInput { expected } => {
                write!(f, "expected {expected}, found end of input")
            }
        }
    }
}

pub fn parse(tokens: &[Token]) -> Result<Program, ParseError> {
    let mut parser = Parser {
        tokens,
        position: 0,
    };
    parser.parse_program()
}

struct Parser<'a> {
    tokens: &'a [Token],
    position: usize,
}

impl Parser<'_> {
    fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut functions = Vec::new();
        while self.position < self.tokens.len() {
            functions.push(self.parse_function()?);
        }
        Ok(Program { functions })
    }

    fn parse_function(&mut self) -> Result<Function, ParseError> {
        self.expect(&Token::Fn)?;
        let name = self.expect_identifier()?;
        self.expect(&Token::OpenParen)?;
        self.expect(&Token::CloseParen)?;
        self.expect(&Token::OpenBrace)?;
        self.expect(&Token::CloseBrace)?;
        Ok(Function { name })
    }

    /// Consumes the next token if it is equal to `expected`. Returns an error otherwise.
    fn expect(&mut self, expected: &Token) -> Result<(), ParseError> {
        let found = self.next_token(&expected.to_string())?;
        if found == expected {
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: expected.to_string(),
                found: found.clone(),
            })
        }
    }

    /// Consumes the next token if it is an identifier and returns its name.
    fn expect_identifier(&mut self) -> Result<String, ParseError> {
        let expected = "identifier";
        match self.next_token(expected)? {
            Token::Identifier(name) => Ok(name.clone()),
            found => Err(ParseError::UnexpectedToken {
                expected: expected.to_string(),
                found: found.clone(),
            }),
        }
    }

    /// Consumes and returns the next token. `expected` is only used for the error message.
    fn next_token(&mut self, expected: &str) -> Result<&Token, ParseError> {
        let token =
            self.tokens
                .get(self.position)
                .ok_or_else(|| ParseError::UnexpectedEndOfInput {
                    expected: expected.to_string(),
                })?;
        self.position += 1;
        Ok(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    /// Tokenizes and parses `source`. The lexer is used to keep the tests readable.
    fn parse_source(source: &str) -> Result<Program, ParseError> {
        let tokens = tokenize(source).expect("test source should be valid for the lexer");
        parse(&tokens)
    }

    fn function(name: &str) -> Function {
        Function {
            name: name.to_string(),
        }
    }

    #[test]
    fn empty_program() {
        assert_eq!(parse_source(""), Ok(Program { functions: vec![] }));
    }

    #[test]
    fn empty_main_function() {
        assert_eq!(
            parse_source("fn main() {}"),
            Ok(Program {
                functions: vec![function("main")]
            })
        );
    }

    #[test]
    fn several_functions() {
        assert_eq!(
            parse_source("fn a() {} fn b() {}"),
            Ok(Program {
                functions: vec![function("a"), function("b")]
            })
        );
    }

    #[test]
    fn missing_function_name() {
        assert_eq!(
            parse_source("fn () {}"),
            Err(ParseError::UnexpectedToken {
                expected: "identifier".to_string(),
                found: Token::OpenParen,
            })
        );
    }

    #[test]
    fn missing_closing_brace() {
        assert_eq!(
            parse_source("fn main() {"),
            Err(ParseError::UnexpectedEndOfInput {
                expected: "`}`".to_string(),
            })
        );
    }

    #[test]
    fn parameters_are_not_supported() {
        assert_eq!(
            parse_source("fn main(x) {}"),
            Err(ParseError::UnexpectedToken {
                expected: "`)`".to_string(),
                found: Token::Identifier("x".to_string()),
            })
        );
    }

    #[test]
    fn statements_are_not_supported() {
        assert_eq!(
            parse_source("fn main() { x }"),
            Err(ParseError::UnexpectedToken {
                expected: "`}`".to_string(),
                found: Token::Identifier("x".to_string()),
            })
        );
    }

    #[test]
    fn tokens_after_a_function_are_an_error() {
        assert_eq!(
            parse_source("fn main() {} }"),
            Err(ParseError::UnexpectedToken {
                expected: "`fn`".to_string(),
                found: Token::CloseBrace,
            })
        );
    }
}
