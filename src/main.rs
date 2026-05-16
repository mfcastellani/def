mod ast;
mod error;
mod interpreter;
mod lexer;
mod parser;
mod value;

use std::{env, fs, path::Path, process};

use error::{DefError, DefResult};
use interpreter::Interpreter;
use lexer::Lexer;
use parser::Parser;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> DefResult<()> {
    let path = env::args()
        .nth(1)
        .ok_or_else(|| DefError::Runtime("usage: def <file.def>".to_string()))?;

    let source = fs::read_to_string(&path)
        .map_err(|error| DefError::Runtime(format!("failed to read '{path}': {error}")))?;

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize()?;
    let program = Parser::new(tokens).parse_program()?;
    let base_dir = Path::new(&path).parent().unwrap_or(Path::new("."));
    let value = Interpreter::with_base_dir(base_dir).interpret(&program)?;

    println!("{value:?}");
    Ok(())
}
