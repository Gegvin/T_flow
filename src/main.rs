use clap::Parser;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;

mod lexer_error;
mod lexer_runner;
mod parser;
mod position;
mod token;

use crate::lexer_runner::{format_tokens, lex_source};
use crate::parser::Parser as TFlowParser;
use crate::token::Token;

#[derive(Parser)]
struct Args {
    input: Option<PathBuf>,

    #[arg(long)]
    check: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();

    if args.check {
        return run_check();
    }

    match args.input {
        Some(path) => match run(path) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{message}");
                ExitCode::FAILURE
            }
        },
        None => {
            eprintln!("Provide input file or use --check");
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
        Ok(t) => t,
    };

    let errors = TFlowParser::new(&tokens).parse();

    let json_items: Vec<String> = errors
        .iter()
        .map(|e| {
            format!(
                r#"{{"message":{},"line":{},"column":{}}}"#,
                serde_json::to_string(&e.message).unwrap(),
                e.line,
                e.column,
            )
        })
        .collect();

    println!("[{}]", json_items.join(","));
    ExitCode::SUCCESS
}

fn run(input_path: PathBuf) -> Result<(), String> {
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
