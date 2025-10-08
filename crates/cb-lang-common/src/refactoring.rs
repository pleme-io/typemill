//! Common refactoring primitives and utilities
//!
//! This module provides shared data structures and helper functions for
//! implementing refactoring operations across different language plugins.

use cb_protocol::EditLocation;
use serde::{Deserialize, Serialize};

/// Code range for refactoring operations
///
/// Represents a rectangular region in source code with line and column positions.
/// All positions are 0-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeRange {
    /// Starting line (0-based)
    pub start_line: u32,
    /// Starting column (0-based)
    pub start_col: u32,
    /// Ending line (0-based)
    pub end_line: u32,
    /// Ending column (0-based)
    pub end_col: u32,
}

impl CodeRange {
    /// Create a new code range
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self {
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }

    /// Create a range spanning entire lines (column 0 to end of line)
    pub fn from_lines(start_line: u32, end_line: u32) -> Self {
        Self {
            start_line,
            start_col: 0,
            end_line,
            end_col: u32::MAX, // Will be clamped to actual line length
        }
    }

    /// Create a single-line range
    pub fn single_line(line: u32, start_col: u32, end_col: u32) -> Self {
        Self {
            start_line: line,
            start_col,
            end_line: line,
            end_col,
        }
    }

    /// Check if this range contains a given position
    pub fn contains(&self, line: u32, col: u32) -> bool {
        if line < self.start_line || line > self.end_line {
            return false;
        }

        if line == self.start_line && col < self.start_col {
            return false;
        }

        if line == self.end_line && col > self.end_col {
            return false;
        }

        true
    }

    /// Check if this range is a single line
    pub fn is_single_line(&self) -> bool {
        self.start_line == self.end_line
    }

    /// Get the number of lines spanned by this range
    pub fn line_count(&self) -> u32 {
        self.end_line - self.start_line + 1
    }
}

impl From<CodeRange> for EditLocation {
    fn from(range: CodeRange) -> Self {
        EditLocation {
            start_line: range.start_line,
            start_column: range.start_col,
            end_line: range.end_line,
            end_column: range.end_col,
        }
    }
}

impl From<EditLocation> for CodeRange {
    fn from(loc: EditLocation) -> Self {
        CodeRange {
            start_line: loc.start_line,
            start_col: loc.start_column,
            end_line: loc.end_line,
            end_col: loc.end_column,
        }
    }
}

/// Helper utilities for working with source code lines
pub struct LineExtractor;

impl LineExtractor {
    /// Extract lines from source code within a given range
    ///
    /// # Arguments
    ///
    /// * `source` - The source code as a string
    /// * `range` - The range of lines to extract
    ///
    /// # Returns
    ///
    /// The extracted lines as a single string with newlines preserved
    pub fn extract_lines(source: &str, range: CodeRange) -> String {
        let lines: Vec<&str> = source.lines().collect();

        let start = range.start_line as usize;
        let end = range.end_line as usize;

        if start >= lines.len() || end >= lines.len() {
            return String::new();
        }

        lines[start..=end].join("\n")
    }

    /// Extract a specific line from source code
    pub fn extract_line(source: &str, line: u32) -> Option<String> {
        source.lines().nth(line as usize).map(|s| s.to_string())
    }

    /// Get the indentation level (number of leading spaces) of a line
    ///
    /// # Arguments
    ///
    /// * `source` - The source code as a string
    /// * `line` - The line number (0-based)
    ///
    /// # Returns
    ///
    /// Number of leading spaces, or 0 if line doesn't exist
    pub fn get_indentation(source: &str, line: u32) -> usize {
        if let Some(line_text) = source.lines().nth(line as usize) {
            line_text.len() - line_text.trim_start().len()
        } else {
            0
        }
    }

    /// Get indentation as a string (spaces)
    pub fn get_indentation_str(source: &str, line: u32) -> String {
        let count = Self::get_indentation(source, line);
        " ".repeat(count)
    }

    /// Count the number of lines in source code
    pub fn line_count(source: &str) -> usize {
        source.lines().count()
    }

    /// Check if a line number is valid for the given source
    pub fn is_valid_line(source: &str, line: u32) -> bool {
        (line as usize) < source.lines().count()
    }

    /// Get the character length of a specific line
    pub fn line_length(source: &str, line: u32) -> usize {
        source
            .lines()
            .nth(line as usize)
            .map(|s| s.len())
            .unwrap_or(0)
    }

    /// Insert text at a specific line
    ///
    /// Returns the modified source code
    pub fn insert_at_line(source: &str, line: u32, text: &str) -> String {
        let mut lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
        let insert_pos = line as usize;

        if insert_pos > lines.len() {
            return source.to_string();
        }

        lines.insert(insert_pos, text.to_string());
        lines.join("\n")
    }

    /// Replace a range of lines with new text
    ///
    /// Returns the modified source code
    pub fn replace_range(source: &str, range: CodeRange, new_text: &str) -> String {
        let mut lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();

        let start = range.start_line as usize;
        let end = range.end_line as usize;

        if start >= lines.len() || end >= lines.len() {
            return source.to_string();
        }

        // Remove old lines
        lines.drain(start..=end);

        // Insert new text (split into lines if necessary)
        for (i, new_line) in new_text.lines().enumerate() {
            lines.insert(start + i, new_line.to_string());
        }

        lines.join("\n")
    }

    /// Delete a range of lines
    pub fn delete_range(source: &str, range: CodeRange) -> String {
        let mut lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();

        let start = range.start_line as usize;
        let end = range.end_line as usize;

        if start >= lines.len() || end >= lines.len() {
            return source.to_string();
        }

        lines.drain(start..=end);
        lines.join("\n")
    }
}

/// Helper for detecting common indentation patterns
pub struct IndentationDetector;

impl IndentationDetector {
    /// Detect the indentation style used in source code
    ///
    /// Returns (indent_char, indent_size) where:
    /// - indent_char is ' ' for spaces or '\t' for tabs
    /// - indent_size is the number of spaces per indent level (or 1 for tabs)
    pub fn detect(source: &str) -> (char, usize) {
        let mut space_counts: Vec<usize> = Vec::new();
        let mut has_tabs = false;

        for line in source.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let leading_spaces = line.len() - line.trim_start_matches(' ').len();
            let leading_tabs = line.len() - line.trim_start_matches('\t').len();

            if leading_tabs > 0 {
                has_tabs = true;
            } else if leading_spaces > 0 {
                space_counts.push(leading_spaces);
            }
        }

        if has_tabs {
            return ('\t', 1);
        }

        // Find GCD of space counts to determine indent size
        if space_counts.is_empty() {
            return (' ', 4); // Default to 4 spaces
        }

        let gcd = space_counts.iter().copied().reduce(gcd).unwrap_or(4);

        (' ', gcd.max(1))
    }

    /// Create an indentation string for a given level
    pub fn indent_string(level: usize, indent_char: char, indent_size: usize) -> String {
        let count = level * indent_size;
        std::iter::repeat_n(indent_char, count).collect()
    }
}

/// Calculate the greatest common divisor
fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_range_contains() {
        let range = CodeRange::new(5, 10, 8, 20);

        assert!(range.contains(5, 10));
        assert!(range.contains(6, 15));
        assert!(range.contains(8, 20));

        assert!(!range.contains(4, 10));
        assert!(!range.contains(9, 10));
        assert!(!range.contains(5, 9));
        assert!(!range.contains(8, 21));
    }

    #[test]
    fn test_code_range_single_line() {
        let range = CodeRange::single_line(10, 5, 15);

        assert!(range.is_single_line());
        assert_eq!(range.line_count(), 1);
    }

    #[test]
    fn test_line_extractor() {
        let source = "line 0\nline 1\nline 2\nline 3";

        let range = CodeRange::from_lines(1, 2);
        let extracted = LineExtractor::extract_lines(source, range);
        assert_eq!(extracted, "line 1\nline 2");
    }

    #[test]
    fn test_indentation_detection() {
        let source_spaces = "def foo():\n    pass\n    return";
        let (char, size) = IndentationDetector::detect(source_spaces);
        assert_eq!(char, ' ');
        assert_eq!(size, 4);

        let source_tabs = "def foo():\n\tpass\n\treturn";
        let (char, size) = IndentationDetector::detect(source_tabs);
        assert_eq!(char, '\t');
        assert_eq!(size, 1);
    }

    #[test]
    fn test_line_extractor_indentation() {
        let source = "    indented line\nno indent\n  two spaces";

        assert_eq!(LineExtractor::get_indentation(source, 0), 4);
        assert_eq!(LineExtractor::get_indentation(source, 1), 0);
        assert_eq!(LineExtractor::get_indentation(source, 2), 2);
    }
}
