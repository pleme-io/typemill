//! Regex matching engine for workspace.find_replace tool
//!
//! Provides regex pattern matching with capture group extraction and template expansion.

use regex::{Captures, Regex};
use std::fmt;

/// A single regex match with position and replacement information
#[derive(Debug, Clone, PartialEq)]
pub struct RegexMatch {
    /// Start byte offset in the content
    pub start_byte: usize,
    /// End byte offset in the content
    pub end_byte: usize,
    /// The matched text
    pub matched_text: String,
    /// The replacement text with capture groups expanded
    pub replacement_text: String,
    /// Capture groups (index 0 is the full match)
    pub capture_groups: Vec<String>,
    /// Line number (1-based)
    pub line: u32,
    /// Column number (0-based)
    pub column: u32,
}

/// Errors that can occur during regex matching
#[derive(Debug, Clone)]
pub enum RegexError {
    /// Invalid regex pattern syntax
    InvalidPattern(String),
    /// Invalid replacement template
    InvalidReplacement(String),
}

impl fmt::Display for RegexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegexError::InvalidPattern(msg) => write!(f, "Invalid regex pattern: {}", msg),
            RegexError::InvalidReplacement(msg) => {
                write!(f, "Invalid replacement template: {}", msg)
            }
        }
    }
}

impl std::error::Error for RegexError {}

/// Find all regex matches in content with expanded replacement text
///
/// # Arguments
///
/// * `content` - The text to search in
/// * `pattern` - The regex pattern to match
/// * `replacement_template` - Template with $1, $2, ${name} placeholders
///
/// # Returns
///
/// Vector of non-overlapping matches with line/column positions and expanded replacements
///
/// # Errors
///
/// Returns `RegexError::InvalidPattern` if the regex syntax is invalid.
/// Returns `RegexError::InvalidReplacement` if the replacement template is malformed.
pub(crate) fn find_regex_matches(
    content: &str,
    pattern: &str,
    replacement_template: &str,
) -> Result<Vec<RegexMatch>, RegexError> {
    // Compile the regex pattern
    let regex = Regex::new(pattern).map_err(|e| RegexError::InvalidPattern(e.to_string()))?;

    // Validate replacement template early
    validate_replacement_template(replacement_template)?;

    let mut matches = Vec::new();

    // Find all matches (regex crate automatically handles non-overlapping)
    for capture in regex.captures_iter(content) {
        // Get the full match (capture group 0)
        let full_match = match capture.get(0) {
            Some(m) => m,
            None => continue, // Skip if no match (shouldn't happen)
        };

        let start_byte = full_match.start();
        let end_byte = full_match.end();
        let matched_text = full_match.as_str().to_string();

        // Extract all capture groups
        let capture_groups: Vec<String> = capture
            .iter()
            .map(|opt_match| {
                opt_match
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default()
            })
            .collect();

        // Expand replacement template
        let replacement_text = expand_replacement(replacement_template, &capture)
            .map_err(RegexError::InvalidReplacement)?;

        // Calculate line and column
        let (line, column) = byte_offset_to_position(content, start_byte);

        matches.push(RegexMatch {
            start_byte,
            end_byte,
            matched_text,
            replacement_text,
            capture_groups,
            line,
            column,
        });
    }

    Ok(matches)
}

/// Expand replacement template with capture groups
///
/// Supports:
/// - `$0` - Full match
/// - `$1`, `$2`, etc. - Numbered captures
/// - `${name}` - Named captures
/// - `$$` - Literal `$`
fn expand_replacement(template: &str, captures: &Captures) -> Result<String, String> {
    let mut result = String::new();
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            match chars.peek() {
                Some('$') => {
                    // Escaped dollar sign
                    chars.next();
                    result.push('$');
                }
                Some('{') => {
                    // Named capture: ${name}
                    chars.next(); // consume '{'
                    let mut name = String::new();
                    let mut found_close = false;

                    while let Some(&c) = chars.peek() {
                        if c == '}' {
                            chars.next();
                            found_close = true;
                            break;
                        }
                        name.push(c);
                        chars.next();
                    }

                    if !found_close {
                        return Err(format!("Unclosed named capture: ${{{}...", name));
                    }

                    if name.is_empty() {
                        return Err("Empty named capture: ${}".to_string());
                    }

                    // Get named capture
                    match captures.name(&name) {
                        Some(m) => result.push_str(m.as_str()),
                        None => {
                            return Err(format!("Named capture '{}' not found in pattern", name));
                        }
                    }
                }
                Some(c) if c.is_ascii_digit() => {
                    // Numbered capture: $1, $2, etc.
                    let mut num_str = String::new();
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_digit() {
                            num_str.push(c);
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    let group_num: usize = num_str
                        .parse()
                        .map_err(|_| format!("Invalid capture group number: {}", num_str))?;

                    // Get numbered capture
                    match captures.get(group_num) {
                        Some(m) => result.push_str(m.as_str()),
                        None => {
                            return Err(format!(
                                "Capture group ${} not found (pattern has {} groups)",
                                group_num,
                                captures.len() - 1
                            ));
                        }
                    }
                }
                _ => {
                    // Invalid escape sequence
                    return Err(format!("Invalid replacement syntax: ${:?}", chars.peek()));
                }
            }
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

/// Validate replacement template syntax without expanding
fn validate_replacement_template(template: &str) -> Result<(), RegexError> {
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            match chars.peek() {
                Some('$') => {
                    chars.next();
                }
                Some('{') => {
                    chars.next();
                    let mut found_close = false;
                    while let Some(&c) = chars.peek() {
                        if c == '}' {
                            chars.next();
                            found_close = true;
                            break;
                        }
                        chars.next();
                    }
                    if !found_close {
                        return Err(RegexError::InvalidReplacement(
                            "Unclosed named capture in template".to_string(),
                        ));
                    }
                }
                Some(c) if c.is_ascii_digit() => {
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_digit() {
                            chars.next();
                        } else {
                            break;
                        }
                    }
                }
                Some(_) => {
                    // Allow other characters after $ without erroring
                    // The actual expansion will validate capture group existence
                }
                None => {
                    return Err(RegexError::InvalidReplacement(
                        "Trailing $ in template".to_string(),
                    ));
                }
            }
        }
    }

    Ok(())
}

/// Convert byte offset to (line, column) position
///
/// Line numbers are 1-based, column numbers are 0-based
fn byte_offset_to_position(content: &str, byte_offset: usize) -> (u32, u32) {
    let mut line = 1;
    let mut column = 0;

    for (idx, ch) in content.char_indices() {
        if idx >= byte_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 0;
        } else {
            column += 1;
        }
    }

    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_regex_matching() {
        let content = "TYPEMILL_ENABLE_LOGS and TYPEMILL_DEBUG_MODE";
        let pattern = r"TYPEMILL_([A-Z_]+)";
        let replacement = "TYPEMILL_$1";

        let matches = find_regex_matches(content, pattern, replacement).unwrap();

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].matched_text, "TYPEMILL_ENABLE_LOGS");
        assert_eq!(matches[0].replacement_text, "TYPEMILL_ENABLE_LOGS");
        assert_eq!(matches[1].matched_text, "TYPEMILL_DEBUG_MODE");
        assert_eq!(matches[1].replacement_text, "TYPEMILL_DEBUG_MODE");
    }

    #[test]
    fn test_capture_group_extraction() {
        let content = "user_name and item_count";
        let pattern = r"(\w+)_(\w+)";
        let replacement = "$2_$1";

        let matches = find_regex_matches(content, pattern, replacement).unwrap();

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].capture_groups, vec!["user_name", "user", "name"]);
        assert_eq!(matches[0].replacement_text, "name_user");
        assert_eq!(
            matches[1].capture_groups,
            vec!["item_count", "item", "count"]
        );
        assert_eq!(matches[1].replacement_text, "count_item");
    }

    #[test]
    fn test_named_captures() {
        let content = "hello_world and foo_bar";
        let pattern = r"(?P<first>\w+)_(?P<second>\w+)";
        let replacement = "${second}_${first}";

        let matches = find_regex_matches(content, pattern, replacement).unwrap();

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].replacement_text, "world_hello");
        assert_eq!(matches[1].replacement_text, "bar_foo");
    }

    #[test]
    fn test_full_match_capture() {
        let content = "test123";
        let pattern = r"(\w+)(\d+)";
        let replacement = "Matched: $0, Word: $1, Digits: $2";

        let matches = find_regex_matches(content, pattern, replacement).unwrap();

        assert_eq!(matches.len(), 1);
        // \w includes digits, so \w+ matches "test123", leaving only last digit for \d+
        assert_eq!(
            matches[0].replacement_text,
            "Matched: test123, Word: test12, Digits: 3"
        );
    }

    #[test]
    fn test_escaped_dollar_sign() {
        let content = "price100";
        let pattern = r"(\w+)(\d+)";
        let replacement = "$$$$1-$2";

        let matches = find_regex_matches(content, pattern, replacement).unwrap();

        assert_eq!(matches.len(), 1);
        // Same as above: \w+ matches "price10", \d+ matches "0"
        assert_eq!(matches[0].replacement_text, "$$1-0");
    }

    #[test]
    fn test_invalid_pattern() {
        let content = "test";
        let pattern = r"[invalid("; // Unclosed bracket
        let replacement = "$1";

        let result = find_regex_matches(content, pattern, replacement);
        assert!(matches!(result, Err(RegexError::InvalidPattern(_))));
    }

    #[test]
    fn test_invalid_capture_group_reference() {
        let content = "test123";
        let pattern = r"(\w+)(\d+)";
        let replacement = "$99"; // Only 2 capture groups exist

        let result = find_regex_matches(content, pattern, replacement);
        assert!(matches!(result, Err(RegexError::InvalidReplacement(_))));
    }

    #[test]
    fn test_invalid_named_capture() {
        let content = "hello_world";
        let pattern = r"(\w+)_(\w+)";
        let replacement = "${nonexistent}";

        let result = find_regex_matches(content, pattern, replacement);
        assert!(matches!(result, Err(RegexError::InvalidReplacement(_))));
    }

    #[test]
    fn test_malformed_replacement_syntax() {
        let content = "test";
        let pattern = r"(\w+)";
        let replacement = "${unclosed";

        let result = find_regex_matches(content, pattern, replacement);
        assert!(matches!(result, Err(RegexError::InvalidReplacement(_))));
    }

    #[test]
    fn test_line_and_column_positions() {
        let content = "Line 1\nLine 2 has TYPEMILL_TEST\nLine 3";
        let pattern = r"TYPEMILL_(\w+)";
        let replacement = "TYPEMILL_$1";

        let matches = find_regex_matches(content, pattern, replacement).unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].line, 2);
        assert_eq!(matches[0].column, 11); // "Line 2 has " = 11 chars
    }

    #[test]
    fn test_non_overlapping_matches() {
        let content = "aaa";
        let pattern = r"aa"; // Could match twice overlapping
        let replacement = "XX";

        let matches = find_regex_matches(content, pattern, replacement).unwrap();

        // Regex crate returns non-overlapping matches by default
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].matched_text, "aa");
        assert_eq!(matches[0].start_byte, 0);
        assert_eq!(matches[0].end_byte, 2);
    }

    #[test]
    fn test_empty_capture_groups() {
        let content = "test_";
        let pattern = r"(\w+)_(\w*)"; // Second group might be empty
        let replacement = "$1-$2";

        let matches = find_regex_matches(content, pattern, replacement).unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].capture_groups, vec!["test_", "test", ""]);
        assert_eq!(matches[0].replacement_text, "test-");
    }

    #[test]
    fn test_zero_width_assertions() {
        let content = "word1 word2";
        let pattern = r"\b\w+"; // Word boundary (zero-width)
        let replacement = "[$0]";

        let matches = find_regex_matches(content, pattern, replacement).unwrap();

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].matched_text, "word1");
        assert_eq!(matches[0].replacement_text, "[word1]");
        assert_eq!(matches[1].matched_text, "word2");
        assert_eq!(matches[1].replacement_text, "[word2]");
    }

    #[test]
    fn test_multiline_content() {
        let content = "TYPEMILL_A\nTYPEMILL_B\nTYPEMILL_C";
        let pattern = r"TYPEMILL_([A-Z])";
        let replacement = "TYPEMILL_$1";

        let matches = find_regex_matches(content, pattern, replacement).unwrap();

        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].line, 1);
        assert_eq!(matches[1].line, 2);
        assert_eq!(matches[2].line, 3);
    }

    #[test]
    fn test_complex_replacement_template() {
        let content = "foo_bar_baz";
        let pattern = r"(?P<a>\w+)_(?P<b>\w+)_(?P<c>\w+)";
        let replacement = "${c}.${b}.${a} ($0)";

        let matches = find_regex_matches(content, pattern, replacement).unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].replacement_text, "baz.bar.foo (foo_bar_baz)");
    }

    #[test]
    fn test_no_matches() {
        let content = "no matches here";
        let pattern = r"TYPEMILL_\w+";
        let replacement = "TYPEMILL_$1";

        let matches = find_regex_matches(content, pattern, replacement).unwrap();

        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_byte_offset_to_position_basic() {
        let content = "abc\ndefg\nhij";

        assert_eq!(byte_offset_to_position(content, 0), (1, 0)); // 'a'
        assert_eq!(byte_offset_to_position(content, 2), (1, 2)); // 'c'
        assert_eq!(byte_offset_to_position(content, 4), (2, 0)); // 'd'
        assert_eq!(byte_offset_to_position(content, 7), (2, 3)); // 'g'
        assert_eq!(byte_offset_to_position(content, 9), (3, 0)); // 'h'
    }

    #[test]
    fn test_unicode_handling() {
        let content = "Hello 世界 TYPEMILL_TEST";
        let pattern = r"TYPEMILL_(\w+)";
        let replacement = "TYPEMILL_$1";

        let matches = find_regex_matches(content, pattern, replacement).unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].matched_text, "TYPEMILL_TEST");
        assert_eq!(matches[0].replacement_text, "TYPEMILL_TEST");
    }

    #[test]
    fn test_trailing_dollar_error() {
        let content = "test";
        let pattern = r"(\w+)";
        let replacement = "prefix$";

        let result = find_regex_matches(content, pattern, replacement);
        assert!(matches!(result, Err(RegexError::InvalidReplacement(_))));
    }

    #[test]
    fn test_empty_named_capture_error() {
        let content = "test";
        let pattern = r"(\w+)";
        let replacement = "${}";

        let result = find_regex_matches(content, pattern, replacement);
        assert!(matches!(result, Err(RegexError::InvalidReplacement(_))));
    }
}
