pub use super::Interpreter;
pub use crate::error::DefError;
pub use crate::lexer::Lexer;
pub use crate::parser::Parser;
pub use crate::value::{MockValue, ResponseValue, Value};
pub use std::collections::HashMap;
pub use std::fs;
pub use std::path::PathBuf;

use std::sync::atomic::{AtomicUsize, Ordering};

static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn run(input: &str) -> Value {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    Interpreter::new().interpret(&program).unwrap()
}

pub fn run_with_base_dir(input: &str, base_dir: &str) -> Value {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    Interpreter::with_base_dir(base_dir)
        .interpret(&program)
        .unwrap()
}

pub fn interpreter_after(input: &str, base_dir: impl Into<PathBuf>) -> Interpreter {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut interpreter = Interpreter::with_base_dir(base_dir);
    interpreter.interpret(&program).unwrap();
    interpreter
}

pub fn interpret_error(input: &str, base_dir: impl Into<PathBuf>) -> DefError {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    Interpreter::with_base_dir(base_dir)
        .interpret(&program)
        .unwrap_err()
}

#[allow(dead_code)]
pub fn run_with_params(input: &str, params: &[(&str, &str)]) -> Value {
    let map: HashMap<String, String> = params
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    Interpreter::new().with_params(map).interpret(&program).unwrap()
}

pub fn run_with_params_error(input: &str, params: &[(&str, &str)]) -> DefError {
    let map: HashMap<String, String> = params
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    Interpreter::new()
        .with_params(map)
        .interpret(&program)
        .unwrap_err()
}

pub fn interp_with_params(input: &str, params: &[(&str, &str)]) -> Interpreter {
    let map: HashMap<String, String> = params
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut interp = Interpreter::new().with_params(map);
    interp.interpret(&program).unwrap();
    interp
}

pub fn temp_dir() -> PathBuf {
    let id = TEMP_COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!("def-headers-test-{id}"));
    if path.exists() {
        fs::remove_dir_all(&path).unwrap();
    }
    fs::create_dir_all(&path).unwrap();
    path
}

pub fn request_headers(interpreter: &Interpreter, name: &str) -> Vec<(String, String)> {
    match interpreter.variables.get(name) {
        Some(Value::Request(request)) => request.headers.clone(),
        value => panic!("expected request '{name}', got {value:?}"),
    }
}

pub fn request_query_strings(interpreter: &Interpreter, name: &str) -> Vec<(String, String)> {
    match interpreter.variables.get(name) {
        Some(Value::Request(request)) => request.query_strings.clone(),
        value => panic!("expected request '{name}', got {value:?}"),
    }
}

pub fn header_value(headers: &[(String, String)], name: &str) -> Option<String> {
    let name = name.to_ascii_lowercase();
    headers.iter().rev().find_map(|(header_name, value)| {
        (header_name.to_ascii_lowercase() == name).then(|| value.clone())
    })
}

pub fn query_string_value(query_strings: &[(String, String)], name: &str) -> Option<String> {
    query_strings
        .iter()
        .rev()
        .find_map(|(query_name, value)| (query_name == name).then(|| value.clone()))
}

pub fn json_response(body: &str) -> ResponseValue {
    ResponseValue {
        status: 200,
        body: body.to_string(),
        headers: Vec::new(),
        duration_ms: 0,
        method: String::new(),
        url: String::new(),
    }
}

mod builtins;
mod html;
mod collections;
mod control_flow;
mod datetime;
mod expressions;
mod imports;
mod json;
mod mocks;
mod numeric;
mod params;
mod query_string;
mod request;
mod response;
mod snapshot;
mod variables;
