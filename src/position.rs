use line_col::LineColLookup;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    pub start: Location,
    pub end: Location,
}

pub fn byte_to_location(source: &str, byte_index: usize) -> Location {
    let lookup = LineColLookup::new(source);
    let safe_index = byte_index.min(source.len());
    let (line, column) = lookup.get(safe_index);

    Location { line, column }
}

pub fn span_to_location(source: &str, start: usize, end: usize) -> SourceLocation {
    SourceLocation {
        start: byte_to_location(source, start),
        end: byte_to_location(source, end),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_byte_index_to_line_and_column() {
        let source = "abc\ndef";

        assert_eq!(byte_to_location(source, 0), Location { line: 1, column: 1 });
        assert_eq!(byte_to_location(source, 2), Location { line: 1, column: 3 });
        assert_eq!(byte_to_location(source, 4), Location { line: 2, column: 1 });
        assert_eq!(byte_to_location(source, 6), Location { line: 2, column: 3 });
    }

    #[test]
    fn clamps_index_after_file_end() {
        let source = "abc";

        assert_eq!(
            byte_to_location(source, 100),
            Location { line: 1, column: 4 }
        );
    }
}
