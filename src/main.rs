use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

mod lexer_error;
mod lexer_runner;
mod position;
mod token;

use crate::lexer_runner::{format_tokens, lex_source};
use crate::token::Token;

#[derive(Parser)]
struct Args {
    input: PathBuf,
}

fn main() -> ExitCode {
    let args = Args::parse();

    match run(args.input) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
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
