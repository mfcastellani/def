use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum DefError {
    Lex(String),
    Parse(String),
    Runtime(String),
}

impl fmt::Display for DefError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DefError::Lex(message) => write!(f, "lexer error: {message}"),
            DefError::Parse(message) => write!(f, "parser error: {message}"),
            DefError::Runtime(message) => write!(f, "runtime error: {message}"),
        }
    }
}

impl std::error::Error for DefError {}

pub type DefResult<T> = Result<T, DefError>;
