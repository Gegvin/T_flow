use clap::Parser;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;

mod ast;
mod ast_parser;
mod interpreter;
mod lexer_error;
mod lexer_runner;
mod parser;
mod pest_parser;
mod position;
mod token;

use crate::ast_parser::AstParser;
use crate::interpreter::Interpreter;
use crate::lexer_runner::{format_tokens, lex_source};
use crate::parser::Parser as TFlowParser;
use crate::pest_parser::parse_pest;
use crate::token::Token;

#[derive(Parser)]
struct Args {
    input: Option<PathBuf>,

    #[arg(long)]
    check: bool,

    #[arg(long)]
    bench_parsers: Option<PathBuf>,

    #[arg(long)]
    ast: bool,

    #[arg(long)]
    run: bool,

    #[arg(long)]
    pest: bool,

    #[arg(long, default_value_t = 1)]
    steps: usize,
}

fn main() -> ExitCode {
    let args = Args::parse();

    if args.check {
        return run_check();
    }

    if args.ast {
        return run_ast(args.input);
    }

    if args.run {
        return run_interpreter(args.input, args.steps);
    }

    if args.pest {
        return run_pest(args.input);
    }

    match args.input {
        Some(path) => match run_lexer(path) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{message}");
                ExitCode::FAILURE
            }
        },
        None => {
            eprintln!("Provide input file, or use --check, --ast, --run, --pest");
            ExitCode::FAILURE
        }
    }
}

fn run_check() -> ExitCode {
    let mut source = String::new();
    if io::stdin().read_to_string(&mut source).is_err() {
        eprintln!("Cannot read stdin");
        return ExitCode::FAILURE;
    }

    let tokens = match lex_source::<Token>(&source) {
        Err(lex_err) => {
            let json = format!(
                r#"[{{"message":{},"line":{},"column":{}}}]"#,
                serde_json::to_string(&lex_err.message).unwrap(),
                lex_err.location.line,
                lex_err.location.column,
            );
            println!("{json}");
            return ExitCode::SUCCESS;
        }
        Ok(tokens) => tokens,
    };

    let errors = TFlowParser::new(&tokens).parse();

    let json_items: Vec<String> = errors
        .iter()
        .map(|error| {
            format!(
                r#"{{"message":{},"line":{},"column":{}}}"#,
                serde_json::to_string(&error.message).unwrap(),
                error.line,
                error.column,
            )
        })
        .collect();

    println!("[{}]", json_items.join(","));
    ExitCode::SUCCESS
}
fn run_ast(input: Option<PathBuf>) -> ExitCode {
    let input_path = match input {
        Some(path) => path,
        None => {
            eprintln!("Provide input file for --ast");
            return ExitCode::FAILURE;
        }
    };

    let source = match fs::read_to_string(&input_path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("Cannot read input file: {error}");
            return ExitCode::FAILURE;
        }
    };

    let tokens = match lex_source::<Token>(&source) {
        Ok(tokens) => tokens,
        Err(error) => {
            eprintln!("{}", error.message);
            return ExitCode::FAILURE;
        }
    };

    match AstParser::new(&tokens).parse_program() {
        Ok(program) => {
            println!("{:#?}", program);
            ExitCode::SUCCESS
        }
        Err(errors) => {
            for error in errors {
                eprintln!("{}:{}: {}", error.line, error.column, error.message);
            }

            ExitCode::FAILURE
        }
    }
}

fn run_interpreter(input: Option<PathBuf>, steps: usize) -> ExitCode {
    let input_path = match input {
        Some(path) => path,
        None => {
            eprintln!("Provide input file for --run");
            return ExitCode::FAILURE;
        }
    };

    let source = match fs::read_to_string(&input_path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("Cannot read input file: {error}");
            return ExitCode::FAILURE;
        }
    };

    let tokens = match lex_source::<Token>(&source) {
        Ok(tokens) => tokens,
        Err(error) => {
            eprintln!("{}", error.message);
            return ExitCode::FAILURE;
        }
    };

    let program = match AstParser::new(&tokens).parse_program() {
        Ok(program) => program,
        Err(errors) => {
            for error in errors {
                eprintln!("{}:{}: {}", error.line, error.column, error.message);
            }

            return ExitCode::FAILURE;
        }
    };

    let mut interpreter = match Interpreter::new(program) {
        Ok(interpreter) => interpreter,
        Err(error) => {
            eprintln!("{}", error.message);
            return ExitCode::FAILURE;
        }
    };

    match interpreter.run_steps(steps) {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{}", error.message);
            ExitCode::FAILURE
        }
    }
}

fn run_pest(input: Option<PathBuf>) -> ExitCode {
    let input_path = match input {
        Some(path) => path,
        None => {
            eprintln!("Provide input file for --pest");
            return ExitCode::FAILURE;
        }
    };

    let source = match fs::read_to_string(&input_path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("Cannot read input file: {error}");
            return ExitCode::FAILURE;
        }
    };

    match parse_pest(&source) {
        Ok(()) => {
            println!("OK");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run_lexer(input_path: PathBuf) -> Result<(), String> {
    let source = fs::read_to_string(&input_path)
        .map_err(|error| format!("Cannot read input file: {error}"))?;
    let tokens = lex_source::<Token>(&source).map_err(|error| error.message)?;
    let output = format_tokens(&tokens);
    let output_path = input_path.with_extension(format!(
        "{}.out",
        input_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
    ));
    fs::write(&output_path, output)
        .map_err(|error| format!("Cannot write output file: {error}"))?;
    Ok(())
}
