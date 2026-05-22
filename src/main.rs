mod ast;
mod error;
mod help;
mod interpreter;
mod lexer;
mod parser;
mod value;

use std::{fs, path::Path, process};

use clap::{Parser, Subcommand};
use error::{DefError, DefResult};
use interpreter::Interpreter;
use lexer::Lexer;
use parser::Parser as DefParser;

#[derive(Parser)]
#[command(
    name = "def",
    about = "DefLang — a scripting language for HTTP workflows",
    version,
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run a .def script
    Run {
        /// Path to the .def file to execute
        file: String,
    },
    /// Validate a .def script without making HTTP calls (dry-run)
    Check {
        /// Path to the .def file to validate
        file: String,
    },
    /// Format a .def script (not yet implemented)
    Fmt {
        /// Path to the .def file to format
        file: String,
    },
    /// Show DefLang language help and topic list
    Help {
        /// Topic to show details for (omit to list all topics)
        topic: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Run { file } => {
            if let Err(error) = run(&file) {
                eprintln!("{error}");
                process::exit(1);
            }
        }
        Command::Check { file } => {
            if let Err(error) = check(&file) {
                eprintln!("{error}");
                process::exit(1);
            }
        }
        Command::Fmt { file } => {
            eprintln!("fmt: not yet implemented (file: {file})");
            process::exit(1);
        }
        Command::Help { topic } => match topic {
            None => help::print_help(),
            Some(topic) => help::print_topic(&topic),
        },
    }
}

fn run(path: &str) -> DefResult<()> {
    let source = fs::read_to_string(path)
        .map_err(|error| DefError::Runtime(format!("failed to read '{path}': {error}")))?;

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize().map_err(|e| e.in_file(path))?;
    let program = DefParser::new(tokens)
        .parse_program()
        .map_err(|e| e.in_file(path))?;
    let base_dir = Path::new(path).parent().unwrap_or(Path::new("."));
    Interpreter::with_base_dir(base_dir)
        .with_source_file(path)
        .interpret(&program)?;

    Ok(())
}

fn check(path: &str) -> DefResult<()> {
    let source = fs::read_to_string(path)
        .map_err(|error| DefError::Runtime(format!("failed to read '{path}': {error}")))?;

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize().map_err(|e| e.in_file(path))?;
    let program = DefParser::new(tokens)
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
