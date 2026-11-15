//! The lexer is the first stage. It turns source text into a list of tokens.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// The `fn` keyword.
    Fn,
    Identifier(String),
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Fn => write!(f, "`fn`"),
            Token::Identifier(name) => write!(f, "identifier `{name}`"),
            Token::OpenParen => write!(f, "`(`"),
            Token::CloseParen => write!(f, "`)`"),
            Token::OpenBrace => write!(f, "`{{`"),
            Token::CloseBrace => write!(f, "`}}`"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum LexError {
    UnexpectedCharacter(char),
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexError::UnexpectedCharacter(c) => write!(f, "unexpected character `{c}`"),
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
        } else {
            let token = match c {
                '(' => Token::OpenParen,
                ')' => Token::CloseParen,
                '{' => Token::OpenBrace,
                '}' => Token::CloseBrace,
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
        _ => Token::Identifier(word),
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
    fn keyword_prefix_is_an_identifier() {
        assert_eq!(tokenize("fnord"), Ok(vec![identifier("fnord")]));
    }

    #[test]
    fn identifier_with_underscore_and_digits() {
        assert_eq!(tokenize("_foo_1"), Ok(vec![identifier("_foo_1")]));
    }

    #[test]
    fn unknown_character_is_an_error() {
        assert_eq!(tokenize("fn $"), Err(LexError::UnexpectedCharacter('$')));
    }
}
