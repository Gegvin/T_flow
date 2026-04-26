use std::fmt::Debug;
use std::ops::Range;

use logos::Logos;

use crate::lexer_error::LexError;
use crate::position::{SourceLocation, span_to_location};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocatedToken<T> {
    pub kind: T,
    pub lexeme: String,
    pub byte_span: Range<usize>,
    pub location: SourceLocation,
}

pub fn lex_source<'source, T>(source: &'source str) -> Result<Vec<LocatedToken<T>>, LexError>
where
    T: Logos<'source, Source = str> + Debug,
    T::Error: Debug,
    T::Extras: Default,
{
    let mut result = Vec::new();

    for (token_result, span) in T::lexer(source).spanned() {
        match token_result {
            Ok(kind) => {
                let lexeme = source.get(span.clone()).unwrap_or("").to_string();

                result.push(LocatedToken {
                    kind,
                    lexeme,
                    byte_span: span.clone(),
                    location: span_to_location(source, span.start, span.end),
                });
            }
            Err(_) => {
                return Err(LexError::unexpected_character(source, span));
            }
        }
    }

    Ok(result)
}

pub fn format_tokens<T: Debug>(tokens: &[LocatedToken<T>]) -> String {
    let mut output = String::new();

    for token in tokens {
        let escaped_lexeme =
            serde_json::to_string(&token.lexeme).unwrap_or_else(|_| format!("{:?}", token.lexeme));

        output.push_str(&format!(
            "{:?}\t{}\t{}:{}-{}:{}\n",
            token.kind,
            escaped_lexeme,
            token.location.start.line,
            token.location.start.column,
            token.location.end.line,
            token.location.end.column,
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use logos::Logos;

    use super::*;

    #[derive(Debug, PartialEq, Logos)]
    #[logos(skip r"[ \t\r\n]+")]
    enum TestToken {
        #[token("let")]
        Let,

        #[token("=")]
        Eq,

        #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
        Ident,

        #[regex("[0-9]+")]
        Int,
    }

    #[test]
    fn returns_tokens_with_coordinates() {
        let source = "let x = 10";

        let tokens = lex_source::<TestToken>(source).unwrap();

        assert_eq!(tokens[0].location.start.line, 1);
        assert_eq!(tokens[0].location.start.column, 1);
        assert_eq!(tokens[0].location.end.column, 4);

        assert_eq!(tokens[1].location.start.column, 5);
        assert_eq!(tokens[2].location.start.column, 7);
        assert_eq!(tokens[3].location.start.column, 9);
    }

    #[test]
    fn returns_error_with_coordinates() {
        let source = "let x\n@";

        let error = lex_source::<TestToken>(source).unwrap_err();

        assert_eq!(error.location.line, 2);
        assert_eq!(error.location.column, 1);
        assert_eq!(error.message, "Error at 2:1: Unexpected character `@`");
    }

    #[test]
    fn formats_tokens_for_out_file() {
        let source = "let x";

        let tokens = lex_source::<TestToken>(source).unwrap();
        let output = format_tokens(&tokens);

        assert_eq!(output, "Let\t\"let\"\t1:1-1:4\nIdent\t\"x\"\t1:5-1:6\n");
    }
}
