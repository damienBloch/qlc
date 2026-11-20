//! The parser is the second stage. It turns a list of tokens into an AST.
//!
//! Grammar:
//!
//! ```text
//! program          = function*
//! function         = "fn" identifier [ place_parameters ] [ places ] "(" ")" block
//! place_parameters = "[" place { "," place } "]"
//! places           = place | "@" "(" identifier { "," identifier } ")"
//! place            = "@" identifier
//! block            = "{" statement* "}"
//! statement        = print | delete | let
//! print            = "print" "(" expression ")" ";"
//! delete           = "delete" "(" expression ")" ";"
//! let              = "let" identifier "=" expression ";"
//! expression       = integer | identifier | sim_device
//! sim_device       = "sim_device" "(" ")"
//! ```

use std::fmt;

use crate::ast::{Expression, Function, Program, Statement};
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

impl<'a> Parser<'a> {
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
        let place_parameters = if self.peek() == Some(&Token::OpenBracket) {
            self.parse_place_parameters()?
        } else {
            vec![]
        };
        let places = if self.peek() == Some(&Token::At) {
            Some(self.parse_places()?)
        } else {
            None
        };
        self.expect(&Token::OpenParen)?;
        self.expect(&Token::CloseParen)?;
        let body = self.parse_block()?;
        Ok(Function {
            name,
            place_parameters,
            places,
            body,
        })
    }

    /// Parses `[@a, @b]` and returns the names of the places.
    fn parse_place_parameters(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect(&Token::OpenBracket)?;
        let names = self.parse_comma_separated(Self::parse_place)?;
        self.expect(&Token::CloseBracket)?;
        Ok(names)
    }

    /// Parses `@a` or `@(a, b)` and returns the names of the places.
    fn parse_places(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect(&Token::At)?;
        if self.peek() == Some(&Token::OpenParen) {
            self.expect(&Token::OpenParen)?;
            let names = self.parse_comma_separated(Self::expect_identifier)?;
            self.expect(&Token::CloseParen)?;
            Ok(names)
        } else {
            Ok(vec![self.expect_identifier()?])
        }
    }

    /// Parses `@a` and returns the name of the place.
    fn parse_place(&mut self) -> Result<String, ParseError> {
        self.expect(&Token::At)?;
        self.expect_identifier()
    }

    /// Parses one or more items with `parse_item`, separated by commas.
    fn parse_comma_separated<T>(
        &mut self,
        parse_item: fn(&mut Self) -> Result<T, ParseError>,
    ) -> Result<Vec<T>, ParseError> {
        let mut items = vec![parse_item(self)?];
        while self.peek() == Some(&Token::Comma) {
            self.expect(&Token::Comma)?;
            items.push(parse_item(self)?);
        }
        Ok(items)
    }

    /// Parses a list of statements between braces.
    fn parse_block(&mut self) -> Result<Vec<Statement>, ParseError> {
        self.expect(&Token::OpenBrace)?;
        let mut statements = Vec::new();
        while self.peek().is_some_and(|token| *token != Token::CloseBrace) {
            statements.push(self.parse_statement()?);
        }
        self.expect(&Token::CloseBrace)?;
        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match self.peek() {
            Some(Token::Print) => self.parse_print(),
            Some(Token::Delete) => self.parse_delete(),
            Some(Token::Let) => self.parse_let(),
            found => Err(unexpected("statement", found)),
        }
    }

    fn parse_print(&mut self) -> Result<Statement, ParseError> {
        self.expect(&Token::Print)?;
        self.expect(&Token::OpenParen)?;
        let value = self.parse_expression()?;
        self.expect(&Token::CloseParen)?;
        self.expect(&Token::Semicolon)?;
        Ok(Statement::Print(value))
    }

    fn parse_delete(&mut self) -> Result<Statement, ParseError> {
        self.expect(&Token::Delete)?;
        self.expect(&Token::OpenParen)?;
        let value = self.parse_expression()?;
        self.expect(&Token::CloseParen)?;
        self.expect(&Token::Semicolon)?;
        Ok(Statement::Delete(value))
    }

    fn parse_let(&mut self) -> Result<Statement, ParseError> {
        self.expect(&Token::Let)?;
        let name = self.expect_identifier()?;
        self.expect(&Token::Equals)?;
        let value = self.parse_expression()?;
        self.expect(&Token::Semicolon)?;
        Ok(Statement::Let { name, value })
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        match self.next_token() {
            Some(Token::Integer(value)) => Ok(Expression::Integer(*value)),
            Some(Token::Identifier(name)) => Ok(Expression::Variable(name.clone())),
            Some(Token::SimDevice) => {
                self.expect(&Token::OpenParen)?;
                self.expect(&Token::CloseParen)?;
                Ok(Expression::NewSimDevice)
            }
            found => Err(unexpected("expression", found)),
        }
    }

    /// Consumes the next token if it is equal to `expected`. Returns an error otherwise.
    fn expect(&mut self, expected: &Token) -> Result<(), ParseError> {
        match self.next_token() {
            Some(found) if found == expected => Ok(()),
            found => Err(unexpected(&expected.to_string(), found)),
        }
    }

    /// Consumes the next token if it is an identifier and returns its name.
    fn expect_identifier(&mut self) -> Result<String, ParseError> {
        match self.next_token() {
            Some(Token::Identifier(name)) => Ok(name.clone()),
            found => Err(unexpected("identifier", found)),
        }
    }

    /// Returns the next token without consuming it, or `None` at the end of the input.
    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.position)
    }

    /// Consumes and returns the next token, or returns `None` at the end of the input.
    fn next_token(&mut self) -> Option<&'a Token> {
        let token = self.tokens.get(self.position)?;
        self.position += 1;
        Some(token)
    }
}

/// The error for finding `found` where `expected` was expected.
/// `found` is `None` at the end of the input.
fn unexpected(expected: &str, found: Option<&Token>) -> ParseError {
    match found {
        Some(token) => ParseError::UnexpectedToken {
            expected: expected.to_string(),
            found: token.clone(),
        },
        None => ParseError::UnexpectedEndOfInput {
            expected: expected.to_string(),
        },
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

    /// A function without places.
    fn function(name: &str, body: Vec<Statement>) -> Function {
        Function {
            name: name.to_string(),
            place_parameters: vec![],
            places: None,
            body,
        }
    }

    /// Parses `source`, which has one function with an empty body,
    /// and returns its place parameters and its places.
    fn parse_header_places(source: &str) -> Result<(Vec<String>, Option<Vec<String>>), ParseError> {
        let mut program = parse_source(source)?;
        assert_eq!(
            program.functions.len(),
            1,
            "test source should have one function"
        );
        let function = program.functions.remove(0);
        Ok((function.place_parameters, function.places))
    }

    /// Converts names to `String`s.
    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    /// A statement that prints the integer `value`.
    fn print(value: i64) -> Statement {
        Statement::Print(Expression::Integer(value))
    }

    /// A statement that defines the variable `name` with the value `value`.
    fn let_statement(name: &str, value: Expression) -> Statement {
        Statement::Let {
            name: name.to_string(),
            value,
        }
    }

    fn variable(name: &str) -> Expression {
        Expression::Variable(name.to_string())
    }

    /// Parses `source` and returns the body of its only function.
    fn parse_body(source: &str) -> Result<Vec<Statement>, ParseError> {
        let mut program = parse_source(source)?;
        assert_eq!(
            program.functions.len(),
            1,
            "test source should have one function"
        );
        Ok(program.functions.remove(0).body)
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
                functions: vec![function("main", vec![])]
            })
        );
    }

    #[test]
    fn several_functions() {
        assert_eq!(
            parse_source("fn a() {} fn b() {}"),
            Ok(Program {
                functions: vec![function("a", vec![]), function("b", vec![])]
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
    fn place_parameters() {
        assert_eq!(
            parse_header_places("fn f[@a, @b]() {}"),
            Ok((names(&["a", "b"]), None))
        );
    }

    #[test]
    fn one_place() {
        assert_eq!(
            parse_header_places("fn f@a() {}"),
            Ok((vec![], Some(names(&["a"]))))
        );
    }

    #[test]
    fn list_of_places() {
        assert_eq!(
            parse_header_places("fn f@(a, b)() {}"),
            Ok((vec![], Some(names(&["a", "b"]))))
        );
    }

    #[test]
    fn list_of_one_place() {
        assert_eq!(
            parse_header_places("fn f@(a)() {}"),
            Ok((vec![], Some(names(&["a"]))))
        );
    }

    #[test]
    fn place_parameters_and_places() {
        assert_eq!(
            parse_header_places("fn f[@a, @b]@(b, a)() {}"),
            Ok((names(&["a", "b"]), Some(names(&["b", "a"]))))
        );
    }

    #[test]
    fn empty_place_parameters_are_an_error() {
        assert_eq!(
            parse_source("fn f[]() {}"),
            Err(ParseError::UnexpectedToken {
                expected: "`@`".to_string(),
                found: Token::CloseBracket,
            })
        );
    }

    #[test]
    fn place_parameter_without_at_is_an_error() {
        assert_eq!(
            parse_source("fn f[a]() {}"),
            Err(ParseError::UnexpectedToken {
                expected: "`@`".to_string(),
                found: Token::Identifier("a".to_string()),
            })
        );
    }

    #[test]
    fn empty_list_of_places_is_an_error() {
        assert_eq!(
            parse_source("fn f@() {}"),
            Err(ParseError::UnexpectedToken {
                expected: "identifier".to_string(),
                found: Token::CloseParen,
            })
        );
    }

    #[test]
    fn places_before_place_parameters_are_an_error() {
        assert_eq!(
            parse_source("fn f@a[@a]() {}"),
            Err(ParseError::UnexpectedToken {
                expected: "`(`".to_string(),
                found: Token::OpenBracket,
            })
        );
    }

    #[test]
    fn print_statement() {
        assert_eq!(parse_body("fn main() { print(42); }"), Ok(vec![print(42)]));
    }

    #[test]
    fn several_statements() {
        assert_eq!(
            parse_body("fn main() { print(1); print(2); }"),
            Ok(vec![print(1), print(2)])
        );
    }

    #[test]
    fn let_statement_with_integer() {
        assert_eq!(
            parse_body("fn main() { let x = 1; }"),
            Ok(vec![let_statement("x", Expression::Integer(1))])
        );
    }

    #[test]
    fn let_statement_with_variable() {
        assert_eq!(
            parse_body("fn main() { let y = x; }"),
            Ok(vec![let_statement("y", variable("x"))])
        );
    }

    #[test]
    fn print_variable() {
        assert_eq!(
            parse_body("fn main() { print(x); }"),
            Ok(vec![Statement::Print(variable("x"))])
        );
    }

    #[test]
    fn let_statement_with_sim_device() {
        assert_eq!(
            parse_body("fn main() { let d = sim_device(); }"),
            Ok(vec![let_statement("d", Expression::NewSimDevice)])
        );
    }

    #[test]
    fn sim_device_without_parentheses() {
        assert_eq!(
            parse_source("fn main() { let d = sim_device; }"),
            Err(ParseError::UnexpectedToken {
                expected: "`(`".to_string(),
                found: Token::Semicolon,
            })
        );
    }

    #[test]
    fn delete_statement() {
        assert_eq!(
            parse_body("fn main() { delete(d); }"),
            Ok(vec![Statement::Delete(variable("d"))])
        );
    }

    #[test]
    fn delete_without_semicolon() {
        assert_eq!(
            parse_source("fn main() { delete(d) }"),
            Err(ParseError::UnexpectedToken {
                expected: "`;`".to_string(),
                found: Token::CloseBrace,
            })
        );
    }

    #[test]
    fn let_without_equals() {
        assert_eq!(
            parse_source("fn main() { let x 1; }"),
            Err(ParseError::UnexpectedToken {
                expected: "`=`".to_string(),
                found: Token::Integer(1),
            })
        );
    }

    #[test]
    fn let_without_name() {
        assert_eq!(
            parse_source("fn main() { let = 1; }"),
            Err(ParseError::UnexpectedToken {
                expected: "identifier".to_string(),
                found: Token::Equals,
            })
        );
    }

    #[test]
    fn missing_closing_brace_after_statement() {
        assert_eq!(
            parse_source("fn main() { print(1);"),
            Err(ParseError::UnexpectedEndOfInput {
                expected: "`}`".to_string(),
            })
        );
    }

    #[test]
    fn unknown_statement() {
        assert_eq!(
            parse_source("fn main() { x }"),
            Err(ParseError::UnexpectedToken {
                expected: "statement".to_string(),
                found: Token::Identifier("x".to_string()),
            })
        );
    }

    #[test]
    fn print_without_expression() {
        assert_eq!(
            parse_source("fn main() { print(); }"),
            Err(ParseError::UnexpectedToken {
                expected: "expression".to_string(),
                found: Token::CloseParen,
            })
        );
    }

    #[test]
    fn print_without_semicolon() {
        assert_eq!(
            parse_source("fn main() { print(1) }"),
            Err(ParseError::UnexpectedToken {
                expected: "`;`".to_string(),
                found: Token::CloseBrace,
            })
        );
    }

    #[test]
    fn integer_followed_by_identifier() {
        assert_eq!(
            parse_source("fn main() { print(12ab); }"),
            Err(ParseError::UnexpectedToken {
                expected: "`)`".to_string(),
                found: Token::Identifier("ab".to_string()),
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
