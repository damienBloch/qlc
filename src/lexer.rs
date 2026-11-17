//! The lexer is the first stage. It turns source text into a list of tokens.

use std::fmt;
use std::num::IntErrorKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// The `fn` keyword.
    Fn,
    /// The `print` keyword.
    Print,
    /// The `let` keyword.
    Let,
    Identifier(String),
    /// A non-negative integer literal, such as `42`.
    /// Negative numbers will be written with a unary minus, which is not supported yet.
    Integer(i64),
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    Semicolon,
    Equals,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Fn => write!(f, "`fn`"),
            Token::Print => write!(f, "`print`"),
            Token::Let => write!(f, "`let`"),
            Token::Identifier(name) => write!(f, "identifier `{name}`"),
            Token::Integer(value) => write!(f, "integer `{value}`"),
            Token::OpenParen => write!(f, "`(`"),
            Token::CloseParen => write!(f, "`)`"),
            Token::OpenBrace => write!(f, "`{{`"),
            Token::CloseBrace => write!(f, "`}}`"),
            Token::Semicolon => write!(f, "`;`"),
            Token::Equals => write!(f, "`=`"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum LexError {
    UnexpectedCharacter(char),
    /// The integer literal does not fit in an `i64`. It contains the digits as written.
    IntegerTooLarge(String),
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexError::UnexpectedCharacter(c) => write!(f, "unexpected character `{c}`"),
            LexError::IntegerTooLarge(digits) => {
                write!(f, "integer `{digits}` is too large")
            }
        }
    }
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, LexError> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c.is_ascii_whitespace() {
            chars.next();
        } else if is_identifier_start(c) {
            let mut word = String::new();
            while let Some(&c) = chars.peek() {
                if !is_identifier_continue(c) {
                    break;
                }
                word.push(c);
                chars.next();
            }
            tokens.push(keyword_or_identifier(word));
        } else if c.is_ascii_digit() {
            let mut digits = String::new();
            while let Some(&c) = chars.peek() {
                if !c.is_ascii_digit() {
                    break;
                }
                digits.push(c);
                chars.next();
            }
            tokens.push(integer_token(digits)?);
        } else {
            let token = match c {
                '(' => Token::OpenParen,
                ')' => Token::CloseParen,
                '{' => Token::OpenBrace,
                '}' => Token::CloseBrace,
                ';' => Token::Semicolon,
                '=' => Token::Equals,
                _ => return Err(LexError::UnexpectedCharacter(c)),
            };
            tokens.push(token);
            chars.next();
        }
    }

    Ok(tokens)
}

fn is_identifier_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_identifier_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn keyword_or_identifier(word: String) -> Token {
    match word.as_str() {
        "fn" => Token::Fn,
        "print" => Token::Print,
        "let" => Token::Let,
        _ => Token::Identifier(word),
    }
}

/// Converts the digits of an integer literal to a token.
///
/// `digits` must be a non-empty string of ASCII digits.
fn integer_token(digits: String) -> Result<Token, LexError> {
    match digits.parse() {
        Ok(value) => Ok(Token::Integer(value)),
        Err(error) if *error.kind() == IntErrorKind::PosOverflow => {
            Err(LexError::IntegerTooLarge(digits))
        }
        Err(error) => {
            panic!("integer literal `{digits}` should contain only ASCII digits: {error}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identifier(name: &str) -> Token {
        Token::Identifier(name.to_string())
    }

    #[test]
    fn empty_source_gives_no_tokens() {
        assert_eq!(tokenize(""), Ok(vec![]));
    }

    #[test]
    fn empty_main_function() {
        assert_eq!(
            tokenize("fn main() {}"),
            Ok(vec![
                Token::Fn,
                identifier("main"),
                Token::OpenParen,
                Token::CloseParen,
                Token::OpenBrace,
                Token::CloseBrace,
            ])
        );
    }

    #[test]
    fn whitespace_is_skipped_including_windows_line_endings() {
        assert_eq!(tokenize(" \t\r\n fn \r\n"), Ok(vec![Token::Fn]));
    }

    #[test]
    fn print_keyword() {
        assert_eq!(tokenize("print"), Ok(vec![Token::Print]));
    }

    #[test]
    fn let_statement() {
        assert_eq!(
            tokenize("let x = 1;"),
            Ok(vec![
                Token::Let,
                identifier("x"),
                Token::Equals,
                Token::Integer(1),
                Token::Semicolon,
            ])
        );
    }

    #[test]
    fn keyword_prefix_is_an_identifier() {
        assert_eq!(tokenize("fnord"), Ok(vec![identifier("fnord")]));
        assert_eq!(tokenize("printer"), Ok(vec![identifier("printer")]));
        assert_eq!(tokenize("letter"), Ok(vec![identifier("letter")]));
    }

    #[test]
    fn identifier_with_underscore_and_digits() {
        assert_eq!(tokenize("_foo_1"), Ok(vec![identifier("_foo_1")]));
    }

    #[test]
    fn integer() {
        assert_eq!(tokenize("42"), Ok(vec![Token::Integer(42)]));
    }

    #[test]
    fn integer_with_leading_zeros() {
        assert_eq!(tokenize("007"), Ok(vec![Token::Integer(7)]));
    }

    #[test]
    fn largest_integer() {
        assert_eq!(
            tokenize("9223372036854775807"),
            Ok(vec![Token::Integer(i64::MAX)])
        );
    }

    #[test]
    fn integer_too_large_is_an_error() {
        assert_eq!(
            tokenize("9223372036854775808"),
            Err(LexError::IntegerTooLarge("9223372036854775808".to_string()))
        );
    }

    #[test]
    fn digits_followed_by_letters_are_two_tokens() {
        // The parser rejects this, because an integer is never directly followed by an identifier.
        assert_eq!(
            tokenize("12ab"),
            Ok(vec![Token::Integer(12), identifier("ab")])
        );
    }

    #[test]
    #[should_panic(expected = "should contain only ASCII digits")]
    fn integer_token_panics_on_non_digit() {
        let _ = integer_token("1a".to_string());
    }

    #[test]
    #[should_panic(expected = "should contain only ASCII digits")]
    fn integer_token_panics_on_empty_digits() {
        let _ = integer_token(String::new());
    }

    #[test]
    fn semicolon() {
        assert_eq!(tokenize("x;"), Ok(vec![identifier("x"), Token::Semicolon]));
    }

    #[test]
    fn unknown_character_is_an_error() {
        assert_eq!(tokenize("fn $"), Err(LexError::UnexpectedCharacter('$')));
    }
}
