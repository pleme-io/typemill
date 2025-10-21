//! Source location utilities
//!
//! Provides builders and helpers for creating SourceLocation instances
//! used in import tracking and error reporting.

use codebuddy_foundation::protocol::SourceLocation;

/// Builder for constructing SourceLocation instances
///
/// # Example
///
/// ```rust
/// use cb_lang_common::location::LocationBuilder;
///
/// let loc = LocationBuilder::at_line(42)
///     .with_columns(10, 25)
///     .build();
/// ```
pub struct LocationBuilder {
    start_line: u32,
    start_column: u32,
    end_line: u32,
    end_column: u32,
}

impl LocationBuilder {
    /// Create a location at a specific line (0-based indexing)
    ///
    /// By default, the location spans the entire line (column 0 to u32::MAX)
    pub fn at_line(line: u32) -> Self {
        Self {
            start_line: line,
            start_column: 0,
            end_line: line,
            end_column: u32::MAX,
        }
    }

    /// Create a location spanning multiple lines
    pub fn range(start_line: u32, end_line: u32) -> Self {
        Self {
            start_line,
            start_column: 0,
            end_line,
            end_column: u32::MAX,
        }
    }

    /// Create a location from a line number and text
    ///
    /// Calculates end_column from text length
    pub fn from_line_text(line: u32, text: &str) -> Self {
        Self {
            start_line: line,
            start_column: 0,
            end_line: line,
            end_column: text.len() as u32,
        }
    }

    /// Create a single-point location
    pub fn point(line: u32, column: u32) -> Self {
        Self {
            start_line: line,
            start_column: column,
            end_line: line,
            end_column: column,
        }
    }

    /// Set the starting column
    pub fn with_start_column(mut self, column: u32) -> Self {
        self.start_column = column;
        self
    }

    /// Set the ending column
    pub fn with_end_column(mut self, column: u32) -> Self {
        self.end_column = column;
        self
    }

    /// Set both start and end columns
    pub fn with_columns(mut self, start: u32, end: u32) -> Self {
        self.start_column = start;
        self.end_column = end;
        self
    }

    /// Set the ending line
    pub fn with_end_line(mut self, line: u32) -> Self {
        self.end_line = line;
        self
    }

    /// Build the final SourceLocation
    pub fn build(self) -> SourceLocation {
        SourceLocation {
            start_line: self.start_line,
            start_column: self.start_column,
            end_line: self.end_line,
            end_column: self.end_column,
        }
    }
}

/// Find the line and column of a byte offset in source text
///
/// Returns (line, column) both 0-based
pub fn offset_to_position(source: &str, offset: usize) -> (u32, u32) {
    let mut line = 0;
    let mut col = 0;
    let mut current_offset = 0;

    for ch in source.chars() {
        if current_offset >= offset {
            break;
        }

        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }

        current_offset += ch.len_utf8();
    }

    (line, col)
}

/// Find the byte offset of a line and column position
///
/// Returns None if the position is out of bounds
pub fn position_to_offset(source: &str, line: u32, column: u32) -> Option<usize> {
    let mut current_line = 0;
    let mut current_col = 0;
    let mut offset = 0;

    for ch in source.chars() {
        if current_line == line && current_col == column {
            return Some(offset);
        }

        if ch == '\n' {
            current_line += 1;
            current_col = 0;
        } else {
            current_col += 1;
        }

        offset += ch.len_utf8();
    }

    // Check if we're at the exact end position
    if current_line == line && current_col == column {
        Some(offset)
    } else {
        None
    }
}

/// Get the text at a specific location
pub fn extract_text_at_location(source: &str, location: &SourceLocation) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();

    let start_line = location.start_line as usize;
    let end_line = location.end_line as usize;

    if start_line >= lines.len() || end_line >= lines.len() {
        return None;
    }

    if start_line == end_line {
        // Single line
        let line = lines[start_line];
        let start_col = location.start_column as usize;
        let end_col = (location.end_column as usize).min(line.len());

        if start_col > line.len() {
            return None;
        }

        Some(line[start_col..end_col].to_string())
    } else {
        // Multiple lines
        let mut result = String::new();

        // First line
        let first_line = lines[start_line];
        let start_col = (location.start_column as usize).min(first_line.len());
        result.push_str(&first_line[start_col..]);
        result.push('\n');

        // Middle lines
        for line in lines.iter().take(end_line).skip(start_line + 1) {
            result.push_str(line);
            result.push('\n');
        }

        // Last line
        let last_line = lines[end_line];
        let end_col = (location.end_column as usize).min(last_line.len());
        result.push_str(&last_line[..end_col]);

        Some(result)
    }
}

/// Calculate the length of a location in characters
pub fn location_length(location: &SourceLocation) -> u32 {
    if location.start_line == location.end_line {
        location.end_column.saturating_sub(location.start_column)
    } else {
        // Multi-line, just return a large number
        u32::MAX
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_builder_single_line() {
        let loc = LocationBuilder::at_line(42).with_columns(10, 25).build();

        assert_eq!(loc.start_line, 42);
        assert_eq!(loc.start_column, 10);
        assert_eq!(loc.end_line, 42);
        assert_eq!(loc.end_column, 25);
    }

    #[test]
    fn test_location_builder_range() {
        let loc = LocationBuilder::range(10, 20).build();

        assert_eq!(loc.start_line, 10);
        assert_eq!(loc.end_line, 20);
    }

    #[test]
    fn test_location_builder_from_text() {
        let loc = LocationBuilder::from_line_text(5, "hello world").build();

        assert_eq!(loc.start_line, 5);
        assert_eq!(loc.start_column, 0);
        assert_eq!(loc.end_line, 5);
        assert_eq!(loc.end_column, 11);
    }

    #[test]
    fn test_offset_to_position() {
        let source = "line 0\nline 1\nline 2";

        assert_eq!(offset_to_position(source, 0), (0, 0));
        assert_eq!(offset_to_position(source, 7), (1, 0)); // Start of line 1
        assert_eq!(offset_to_position(source, 14), (2, 0)); // Start of line 2
    }

    #[test]
    fn test_position_to_offset() {
        let source = "line 0\nline 1\nline 2";

        assert_eq!(position_to_offset(source, 0, 0), Some(0));
        assert_eq!(position_to_offset(source, 1, 0), Some(7));
        assert_eq!(position_to_offset(source, 2, 0), Some(14));
        assert_eq!(position_to_offset(source, 10, 0), None); // Out of bounds
    }

    #[test]
    fn test_extract_text_single_line() {
        let source = "hello world\nfoo bar\nbaz qux";

        let loc = LocationBuilder::at_line(0).with_columns(6, 11).build();

        let text = extract_text_at_location(source, &loc);
        assert_eq!(text, Some("world".to_string()));
    }

    #[test]
    fn test_extract_text_multiple_lines() {
        let source = "line 0\nline 1\nline 2\nline 3";

        let loc = LocationBuilder::range(1, 2).build();

        let text = extract_text_at_location(source, &loc);
        assert!(text.is_some());
        assert!(text.unwrap().contains("line 1"));
    }

    #[test]
    fn test_location_length() {
        let loc = LocationBuilder::at_line(0).with_columns(5, 10).build();
        assert_eq!(location_length(&loc), 5);

        let multi = LocationBuilder::range(0, 5).build();
        assert_eq!(location_length(&multi), u32::MAX);
    }
}
