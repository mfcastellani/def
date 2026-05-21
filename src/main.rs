mod ast;
mod error;
mod help;
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
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(String::as_str) {
        None => {
            help::print_usage();
        }
        Some("--version") => {
            help::print_version();
        }
        Some("--help") => match args.get(2).map(String::as_str) {
            None => help::print_help(),
            Some(topic) => help::print_topic(topic),
        },
        Some(path) => {
            let check_mode = args.get(2).map(String::as_str) == Some("--check");
            let result = if check_mode { check(path) } else { run(path) };
            if let Err(error) = result {
                eprintln!("{error}");
                process::exit(1);
            }
        }
    }
}

fn run(path: &str) -> DefResult<()> {
    let source = fs::read_to_string(path)
        .map_err(|error| DefError::Runtime(format!("failed to read '{path}': {error}")))?;

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize().map_err(|e| e.in_file(path))?;
    let program = Parser::new(tokens)
        .parse_program()
        .map_err(|e| e.in_file(path))?;
    let base_dir = Path::new(path).parent().unwrap_or(Path::new("."));
    let value = Interpreter::with_base_dir(base_dir)
        .with_source_file(path)
        .interpret(&program)?;

    println!("{value:?}");
    Ok(())
}

fn check(path: &str) -> DefResult<()> {
    let source = fs::read_to_string(path)
        .map_err(|error| DefError::Runtime(format!("failed to read '{path}': {error}")))?;

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize().map_err(|e| e.in_file(path))?;
    let program = Parser::new(tokens)
        .parse_program()
        .map_err(|e| e.in_file(path))?;
    let base_dir = Path::new(path).parent().unwrap_or(Path::new("."));
    Interpreter::with_base_dir(base_dir)
        .with_source_file(path)
        .with_dry_run(true)
        .interpret(&program)?;

    println!("{path}: syntax ok");
    Ok(())
}
