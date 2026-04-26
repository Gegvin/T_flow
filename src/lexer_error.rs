use std::ops::Range;

use crate::position::{Location, byte_to_location};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    pub location: Location,
    pub span: Range<usize>,
}

impl LexError {
    pub fn unexpected_character(source: &str, span: Range<usize>) -> Self {
        let location = byte_to_location(source, span.start);

        let found = source
            .get(span.clone())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                source
                    .get(span.start..)
                    .and_then(|tail| tail.chars().next().map(|ch| &tail[..ch.len_utf8()]))
            })
            .unwrap_or("<EOF>");

        Self {
            message: format!(
                "Error at {}:{}: Unexpected character `{}`",
                location.line,
                location.column,
                escape_for_message(found),
            ),
            location,
            span,
        }
    }
}

fn escape_for_message(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
        .replace('\r', "\\r")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_human_readable_error() {
        let source = "let x = 1;\n@";

        let error = LexError::unexpected_character(source, 11..12);

        assert_eq!(error.location.line, 2);
        assert_eq!(error.location.column, 1);
        assert_eq!(error.message, "Error at 2:1: Unexpected character `@`");
    }
}
