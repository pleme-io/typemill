//! Common refactoring primitives and utilities
//!
//! This module provides shared data structures and helper functions for
//! implementing refactoring operations across different language plugins.

pub mod edit_plan_builder;
pub mod extract_constant_builder;

use mill_foundation::protocol::EditLocation;
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

    /// Extracts the text content within this range from the source code.
    ///
    /// # Arguments
    /// * `source` - The source code string
    ///
    /// # Returns
    /// * `Ok(String)` - The extracted text
    /// * `Err(String)` - If the range is invalid
    pub fn extract_text(&self, source: &str) -> Result<String, String> {
        let lines: Vec<&str> = source.lines().collect();

        if self.start_line as usize >= lines.len() {
            return Err(format!("Start line {} out of bounds", self.start_line));
        }

        if self.start_line == self.end_line {
            // Single line extraction
            let line = lines[self.start_line as usize];
            if self.end_col as usize > line.len() {
                return Err(format!("End column {} out of bounds on line {}", self.end_col, self.start_line));
            }
            Ok(line[self.start_col as usize..self.end_col as usize].to_string())
        } else {
            // Multi-line extraction
            let mut result = String::new();

            // First line
            if let Some(first_line) = lines.get(self.start_line as usize) {
                result.push_str(&first_line[self.start_col as usize..]);
                result.push('\n');
            }

            // Middle lines
            for line_idx in (self.start_line + 1)..(self.end_line) {
                if let Some(line) = lines.get(line_idx as usize) {
                    result.push_str(line);
                    result.push('\n');
                }
            }

            // Last line
            if let Some(last_line) = lines.get(self.end_line as usize) {
                result.push_str(&last_line[..self.end_col as usize]);
            }

            Ok(result)
        }
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

/// Variable usage information for refactoring analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VariableUsage {
    pub name: String,
    pub declaration_location: Option<CodeRange>,
    pub usages: Vec<CodeRange>,
    pub scope_depth: u32,
    pub is_parameter: bool,
    pub is_declared_in_selection: bool,
    pub is_used_after_selection: bool,
}

/// Information about a function that can be extracted
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractableFunction {
    pub selected_range: CodeRange,
    pub required_parameters: Vec<String>,
    pub return_variables: Vec<String>,
    pub suggested_name: String,
    pub insertion_point: CodeRange,
    pub contains_return_statements: bool,
    pub complexity_score: u32,
}

/// Analysis result for inline variable refactoring
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InlineVariableAnalysis {
    pub variable_name: String,
    pub declaration_range: CodeRange,
    pub initializer_expression: String,
    pub usage_locations: Vec<CodeRange>,
    pub is_safe_to_inline: bool,
    pub blocking_reasons: Vec<String>,
}

/// Analysis result for extract variable refactoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractVariableAnalysis {
    pub expression: String,
    pub expression_range: CodeRange,
    pub can_extract: bool,
    pub suggested_name: String,
    pub insertion_point: CodeRange,
    pub blocking_reasons: Vec<String>,
    pub scope_type: String,
}

/// Analysis result for extract constant refactoring
///
/// This struct represents the analysis outcome when attempting to extract a literal value
/// into a named constant. It is used across all language plugins to provide consistent
/// analysis for the extract constant refactoring operation.
///
/// The analysis identifies:
/// - The literal value to be extracted (e.g., `42`, `"hello"`, `true`)
/// - All locations where this literal appears in the code
/// - Whether extraction is valid (considering context like strings, comments, etc.)
/// - Where the constant declaration should be inserted
/// - Any blocking issues that prevent extraction
///
/// # Example
/// ```rust,ignore
/// // For source code:
/// // let x = 42;
/// // let y = 42;
/// // let msg = "The answer is 42";
///
/// ExtractConstantAnalysis {
///     literal_value: "42".to_string(),
///     occurrence_ranges: vec![
///         CodeRange::new(0, 8, 0, 10),  // First 42
///         CodeRange::new(1, 8, 1, 10),  // Second 42
///         // Note: Third 42 in string is not included
///     ],
///     is_valid_literal: true,
///     blocking_reasons: vec![],
///     insertion_point: CodeRange::new(0, 0, 0, 0),
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ExtractConstantAnalysis {
    /// The literal value to extract (e.g., `42`, `"hello"`, `true`)
    pub literal_value: String,
    /// All locations where this same literal value appears in valid contexts
    /// (excludes occurrences in strings, comments, or other invalid locations)
    pub occurrence_ranges: Vec<CodeRange>,
    /// Whether this is a valid literal to extract
    /// (false if it's in a string, comment, or other invalid context)
    pub is_valid_literal: bool,
    /// Blocking reasons if extraction is not valid
    /// (e.g., "Literal is inside a string", "No valid occurrences found")
    pub blocking_reasons: Vec<String>,
    /// Where to insert the constant declaration
    /// (typically at the top of the current scope or file)
    pub insertion_point: CodeRange,
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

/// Finds all occurrences of a literal value in source code.
///
/// This is a generic implementation that works across all languages by accepting
/// a validation callback to determine whether each match is a valid literal location
/// (i.e., not inside strings, comments, or other invalid contexts).
///
/// # Algorithm
/// 1. Iterate through each line of source code
/// 2. Find all substring matches of the literal value
/// 3. For each match, call the validation function to check if it's a valid location
/// 4. Collect all valid matches as CodeRange structures
///
/// # Arguments
/// * `source` - The source code to search
/// * `literal_value` - The literal value to find (e.g., "42", "\"hello\"", "true")
/// * `is_valid_location` - Validation callback that takes (line_text, column, literal_length)
///   and returns true if the location is valid for replacement
///
/// # Returns
/// A vector of CodeRange structures representing all valid occurrences
///
/// # Performance Note
/// Currently advances by 1 character after each match to find overlapping occurrences.
/// This is O(n*m) where n is source length and m is number of matches.
/// Could be optimized to advance by literal_value.len() if overlapping matches are not needed.
///
/// # Example
/// ```
/// use mill_lang_common::refactoring::{find_literal_occurrences, CodeRange};
///
/// let source = "const x = 42; // The answer is 42";
/// let literal = "42";
///
/// // Define a simple validator (in practice, this would check for strings/comments)
/// let is_valid = |line: &str, col: usize, _len: usize| {
///     // Simplified: just check we're not in a comment
///     let before = &line[..col];
///     !before.contains("//")
/// };
///
/// let occurrences = find_literal_occurrences(source, literal, is_valid);
/// assert_eq!(occurrences.len(), 1); // Only the first 42, not the one in comment
/// assert_eq!(occurrences[0].start_col, 10);
/// ```
///
/// # Language-Specific Usage
/// Each language plugin should provide its own `is_valid_literal_location` function
/// that understands the language's syntax for strings, comments, and other contexts.
///
/// For example:
/// ```rust
/// fn is_valid_typescript_location(line: &str, pos: usize, len: usize) -> bool {
///     // Check for strings with ", ', `
///     // Check for comments with //, /* */
///     // Check for template literal expressions
///     // etc.
/// }
///
/// let occurrences = find_literal_occurrences(source, literal, is_valid_typescript_location);
/// ```
pub fn find_literal_occurrences<F>(
    source: &str,
    literal_value: &str,
    is_valid_location: F,
) -> Vec<CodeRange>
where
    F: Fn(&str, usize, usize) -> bool,
{
    let mut occurrences = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    // Iterate through each line to find all potential matches
    for (line_idx, line_text) in lines.iter().enumerate() {
        let mut start_pos = 0;
        while let Some(pos) = line_text[start_pos..].find(literal_value) {
            let col = start_pos + pos;

            // Validate that this match is not inside a string literal, comment, or other invalid context
            if is_valid_location(line_text, col, literal_value.len()) {
                occurrences.push(CodeRange {
                    start_line: line_idx as u32,
                    start_col: col as u32,
                    end_line: line_idx as u32,
                    end_col: (col + literal_value.len()) as u32,
                });
            }

            // Advance position to continue searching
            // Note: Currently advances by 1 to find overlapping matches
            // Could be optimized to advance by literal_value.len() if overlapping not needed
            start_pos = col + 1;
        }
    }

    occurrences
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

    #[test]
    fn test_find_literal_occurrences_basic() {
        let source = "const x = 42;\nlet y = 42;";
        let literal = "42";

        // Simple validator that accepts everything
        let always_valid = |_: &str, _: usize, _: usize| true;

        let occurrences = find_literal_occurrences(source, literal, always_valid);
        assert_eq!(occurrences.len(), 2);

        // First occurrence
        assert_eq!(occurrences[0].start_line, 0);
        assert_eq!(occurrences[0].start_col, 10);
        assert_eq!(occurrences[0].end_line, 0);
        assert_eq!(occurrences[0].end_col, 12);

        // Second occurrence
        assert_eq!(occurrences[1].start_line, 1);
        assert_eq!(occurrences[1].start_col, 8);
    }

    #[test]
    fn test_find_literal_occurrences_with_validation() {
        let source = r#"const x = 42; // The answer is 42"#;
        let literal = "42";

        // Validator that rejects anything after '//'
        let not_in_comment = |line: &str, col: usize, _len: usize| {
            let before = &line[..col];
            !before.contains("//")
        };

        let occurrences = find_literal_occurrences(source, literal, not_in_comment);
        assert_eq!(occurrences.len(), 1); // Only first occurrence, not the one in comment
        assert_eq!(occurrences[0].start_col, 10);
    }

    #[test]
    fn test_find_literal_occurrences_string_literal() {
        let source = r#"const msg = "The value is 42";"#;
        let literal = "42";

        // Validator that rejects anything inside double quotes
        let not_in_string = |line: &str, col: usize, _len: usize| {
            let before = &line[..col];
            let quote_count = before.chars().filter(|&c| c == '"').count();
            quote_count % 2 == 0 // Even number of quotes = outside string
        };

        let occurrences = find_literal_occurrences(source, literal, not_in_string);
        assert_eq!(occurrences.len(), 0); // Should find nothing (42 is inside string)
    }

    #[test]
    fn test_find_literal_occurrences_no_matches() {
        let source = "const x = 100;\nlet y = 200;";
        let literal = "42";

        let always_valid = |_: &str, _: usize, _: usize| true;

        let occurrences = find_literal_occurrences(source, literal, always_valid);
        assert_eq!(occurrences.len(), 0);
    }

    #[test]
    fn test_find_literal_occurrences_empty_source() {
        let source = "";
        let literal = "42";

        let always_valid = |_: &str, _: usize, _: usize| true;

        let occurrences = find_literal_occurrences(source, literal, always_valid);
        assert_eq!(occurrences.len(), 0);
    }

    #[test]
    fn test_find_literal_occurrences_multiple_per_line() {
        let source = "const x = 42, y = 42, z = 42;";
        let literal = "42";

        let always_valid = |_: &str, _: usize, _: usize| true;

        let occurrences = find_literal_occurrences(source, literal, always_valid);
        assert_eq!(occurrences.len(), 3);
        assert_eq!(occurrences[0].start_col, 10);
        assert_eq!(occurrences[1].start_col, 18);
        assert_eq!(occurrences[2].start_col, 26);
    }

    #[test]
    fn test_find_literal_occurrences_string_value() {
        let source = r#"const x = "hello"; const y = "hello";"#;
        let literal = r#""hello""#;

        let always_valid = |_: &str, _: usize, _: usize| true;

        let occurrences = find_literal_occurrences(source, literal, always_valid);
        assert_eq!(occurrences.len(), 2);
        assert_eq!(occurrences[0].start_col, 10);
        assert_eq!(occurrences[1].start_col, 29);
    }

    #[test]
    fn test_extract_text_single_line() {
        let source = "let x = 42;";
        let range = CodeRange::new(0, 8, 0, 10);
        assert_eq!(range.extract_text(source).unwrap(), "42");
    }

    #[test]
    fn test_extract_text_multi_line() {
        let source = "fn main() {\n    println!(\"hello\");\n}";
        let range = CodeRange::new(0, 11, 2, 1);
        let extracted = range.extract_text(source).unwrap();
        assert!(extracted.contains("println!"));
    }

    #[test]
    fn test_extract_text_out_of_bounds() {
        let source = "short";
        let range = CodeRange::new(10, 0, 10, 5);
        assert!(range.extract_text(source).is_err());
    }
}
