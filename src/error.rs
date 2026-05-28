use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum DefError {
    Lex(String),
    Parse(String),
    Runtime(String),
    Request(String),
    LoopBreak, // internal signal: break() was called inside a loop body
    LoopNext,  // internal signal: next() was called inside a loop body
}

impl fmt::Display for DefError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DefError::Lex(message) => write!(f, "lexer error: {message}"),
            DefError::Parse(message) => write!(f, "parser error: {message}"),
            DefError::Runtime(message) => write!(f, "runtime error: {message}"),
            DefError::Request(message) => write!(f, "request error: {message}"),
            DefError::LoopBreak => write!(f, "runtime error: break() called outside of a loop"),
            DefError::LoopNext => write!(f, "runtime error: next() called outside of a loop"),
        }
    }
}

impl std::error::Error for DefError {}

impl DefError {
    pub fn in_file(self, file: &str) -> DefError {
        match self {
            DefError::Lex(msg) => DefError::Lex(format!("{msg} in '{file}'")),
            DefError::Parse(msg) => DefError::Parse(format!("{msg} in '{file}'")),
            DefError::Runtime(msg) => DefError::Runtime(format!("{msg} in '{file}'")),
            DefError::Request(msg) => DefError::Request(format!("{msg} in '{file}'")),
            other => other,
        }
    }

    pub fn at_location(self, line: usize, file: &str) -> DefError {
        match self {
            DefError::Runtime(msg) if !msg.contains(" in '") => {
                DefError::Runtime(format!("{msg} at line {line} in '{file}'"))
            }
            other => other,
        }
    }
}

pub type DefResult<T> = Result<T, DefError>;
