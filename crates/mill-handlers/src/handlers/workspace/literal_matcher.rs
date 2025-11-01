//! Literal String Matching Engine for workspace.find_replace
//!
//! This module provides efficient literal string matching with optional word boundary detection
//! for the find_replace tool. It handles UTF-8 correctly and converts byte offsets to line/column
//! positions for editor integration.

use std::cmp::Ordering;

/// A single match result from literal string matching
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    /// Byte offset from start of content
    pub start_byte: usize,
    /// Byte offset of end of match (exclusive)
    pub end_byte: usize,
    /// The matched text
    pub matched_text: String,
    /// Line number (1-indexed)
    pub line: u32,
    /// Column number (1-indexed, character-based not byte-based)
    pub column: u32,
}

impl PartialOrd for Match {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Match {
    fn cmp(&self, other: &Self) -> Ordering {
        self.start_byte.cmp(&other.start_byte)
    }
}

/// Check if a character is a word boundary (non-alphanumeric and not underscore)
#[inline]
fn is_word_boundary_char(ch: char) -> bool {
    !ch.is_alphanumeric() && ch != '_'
}

/// Check if there's a word boundary at the given byte position
///
/// This checks if the character immediately BEFORE byte_pos is a word boundary character.
/// For checking after a match, pass the position after the match end.
#[allow(dead_code)] // Reserved for future word-boundary matching features
fn has_word_boundary_at(content: &str, byte_pos: usize) -> bool {
    // At start/end of content is always a word boundary
    if byte_pos == 0 || byte_pos >= content.len() {
        return true;
    }

    // Get the character immediately BEFORE this position (handle multi-byte UTF-8)
    // We need to find the last character before byte_pos
    content[..byte_pos]
        .chars()
        .next_back()
        .map(is_word_boundary_char)
        .unwrap_or(true)
}

/// Convert byte offset to line and column (both 1-indexed)
///
/// This handles UTF-8 correctly by counting characters, not bytes.
/// Line breaks are detected by '\n' (supports Unix, Windows CRLF is counted correctly).
fn byte_offset_to_line_column(content: &str, byte_offset: usize) -> (u32, u32) {
    let mut line = 1u32;
    let mut column = 1u32;

    for (byte_idx, ch) in content.char_indices() {
        if byte_idx >= byte_offset {
            break;
        }

        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    (line, column)
}

/// Find all literal matches of a pattern in content
///
/// # Arguments
/// * `content` - The text to search in
/// * `pattern` - The exact string to search for (case-sensitive)
/// * `whole_word` - If true, only match when pattern is surrounded by word boundaries
///
/// # Returns
/// A vector of matches, sorted by start position, with overlaps deduplicated
///
/// # Examples
/// ```
/// use mill_handlers::handlers::workspace::literal_matcher::find_literal_matches;
///
/// let content = "user is not username";
/// let matches = find_literal_matches(content, "user", true);
/// assert_eq!(matches.len(), 1); // Only "user", not "username"
/// assert_eq!(matches[0].matched_text, "user");
/// assert_eq!(matches[0].line, 1);
/// assert_eq!(matches[0].column, 1);
/// ```
pub(crate) fn find_literal_matches(content: &str, pattern: &str, whole_word: bool) -> Vec<Match> {
    // Empty pattern returns no matches
    if pattern.is_empty() {
        return Vec::new();
    }

    let mut matches = Vec::new();

    // Use efficient string search (std::str::match_indices uses Boyer-Moore-like algorithm)
    for (byte_offset, matched_str) in content.match_indices(pattern) {
        // If whole_word is enabled, check word boundaries
        if whole_word {
            // Check if there's a word boundary before the match start
            let before_ok = if byte_offset == 0 {
                true
            } else {
                content[..byte_offset]
                    .chars()
                    .next_back()
                    .map(is_word_boundary_char)
                    .unwrap_or(true)
            };

            // Check if there's a word boundary after the match end
            let after_ok = if byte_offset + pattern.len() >= content.len() {
                true
            } else {
                content[byte_offset + pattern.len()..]
                    .chars()
                    .next()
                    .map(is_word_boundary_char)
                    .unwrap_or(true)
            };

            if !before_ok || !after_ok {
                continue; // Not a whole word match
            }
        }

        let (line, column) = byte_offset_to_line_column(content, byte_offset);

        matches.push(Match {
            start_byte: byte_offset,
            end_byte: byte_offset + pattern.len(),
            matched_text: matched_str.to_string(),
            line,
            column,
        });
    }

    // Sort by start position (should already be sorted, but ensure it)
    matches.sort();

    // Deduplicate overlapping matches (shouldn't happen with literal matching, but be safe)
    matches.dedup_by(|a, b| a.start_byte == b.start_byte);

    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_literal_matching() {
        let content = "hello world hello";
        let matches = find_literal_matches(content, "hello", false);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].matched_text, "hello");
        assert_eq!(matches[0].start_byte, 0);
        assert_eq!(matches[0].line, 1);
        assert_eq!(matches[0].column, 1);

        assert_eq!(matches[1].matched_text, "hello");
        assert_eq!(matches[1].start_byte, 12);
        assert_eq!(matches[1].line, 1);
        assert_eq!(matches[1].column, 13);
    }

    #[test]
    fn test_whole_word_matching() {
        let content = "user is not username or user_id";
        let matches = find_literal_matches(content, "user", true);

        // Should only match standalone "user", not "username" or "user_id"
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].matched_text, "user");
        assert_eq!(matches[0].start_byte, 0);
    }

    #[test]
    fn test_whole_word_with_punctuation() {
        let content = "user.name, user!test, user;";
        let matches = find_literal_matches(content, "user", true);

        // Should match all three (punctuation is word boundary)
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn test_substring_matching() {
        let content = "user is not username or user_id";
        let matches = find_literal_matches(content, "user", false);

        // Should match all occurrences including substrings
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn test_empty_pattern() {
        let content = "hello world";
        let matches = find_literal_matches(content, "", false);

        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_pattern_not_found() {
        let content = "hello world";
        let matches = find_literal_matches(content, "goodbye", false);

        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_pattern_at_start() {
        let content = "hello world";
        let matches = find_literal_matches(content, "hello", true);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].start_byte, 0);
    }

    #[test]
    fn test_pattern_at_end() {
        let content = "hello world";
        let matches = find_literal_matches(content, "world", true);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].end_byte, content.len());
    }

    #[test]
    fn test_multiline_content() {
        let content = "line1 user\nline2 user\nline3 username";
        let matches = find_literal_matches(content, "user", true);

        assert_eq!(matches.len(), 2);

        assert_eq!(matches[0].line, 1);
        assert_eq!(matches[0].column, 7);

        assert_eq!(matches[1].line, 2);
        assert_eq!(matches[1].column, 7);
    }

    #[test]
    fn test_utf8_characters() {
        let content = "Hello 世界 Hello 🌍";
        let matches = find_literal_matches(content, "Hello", false);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].column, 1);

        // Second "Hello" starts after "Hello 世界 " (6 chars + 2 chars + 1 space)
        assert_eq!(matches[1].line, 1);
        assert_eq!(matches[1].column, 10); // "Hello 世界 H"
    }

    #[test]
    fn test_utf8_emoji() {
        let content = "user 👤 user";
        let matches = find_literal_matches(content, "user", true);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].column, 1);
        assert_eq!(matches[1].column, 8); // After emoji
    }

    #[test]
    fn test_line_column_calculation() {
        let content = "abc\ndefgh\nijkl";
        // Position of 'i' is at byte 10, line 3, column 1
        let (line, column) = byte_offset_to_line_column(content, 10);
        assert_eq!(line, 3);
        assert_eq!(column, 1);

        // Position of 'e' is at byte 5, line 2, column 2
        let (line, column) = byte_offset_to_line_column(content, 5);
        assert_eq!(line, 2);
        assert_eq!(column, 2);
    }

    #[test]
    fn test_windows_line_endings() {
        let content = "line1\r\nline2\r\nline3";
        let matches = find_literal_matches(content, "line2", false);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].line, 2);
        assert_eq!(matches[0].column, 1);
    }

    #[test]
    fn test_case_sensitive() {
        let content = "User user USER";
        let matches = find_literal_matches(content, "user", true);

        // Should only match lowercase "user"
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].column, 6);
    }

    #[test]
    fn test_overlapping_prevention() {
        // Rust's match_indices doesn't return overlapping matches
        // "aa" in "aaa" only matches at position 0 (not at 1)
        let content = "aaa";
        let matches = find_literal_matches(content, "aa", false);

        // Only one match (at position 0), not two
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].start_byte, 0);
    }

    #[test]
    fn test_whole_word_underscore_not_boundary() {
        let content = "my_user_name user my_user";
        let matches = find_literal_matches(content, "user", true);

        // Should only match standalone "user", underscore is NOT a boundary
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].column, 14);
    }

    #[test]
    fn test_word_boundary_helpers() {
        assert!(is_word_boundary_char(' '));
        assert!(is_word_boundary_char('.'));
        assert!(is_word_boundary_char(','));
        assert!(is_word_boundary_char('\n'));
        assert!(is_word_boundary_char('!'));

        assert!(!is_word_boundary_char('a'));
        assert!(!is_word_boundary_char('Z'));
        assert!(!is_word_boundary_char('0'));
        assert!(!is_word_boundary_char('_'));
    }

    #[test]
    fn test_multiline_pattern() {
        let content = "hello\nworld\nhello\nworld";
        let matches = find_literal_matches(content, "hello\nworld", false);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line, 1);
        assert_eq!(matches[1].line, 3);
    }

    #[test]
    fn test_unicode_boundaries() {
        // Test that Unicode word characters work correctly
        let content = "café user café";
        let matches = find_literal_matches(content, "user", true);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].column, 6);
    }

    #[test]
    fn test_cjk_characters() {
        let content = "用户 user 用户名";
        let matches = find_literal_matches(content, "user", true);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].column, 4); // After "用户 "
    }

    #[test]
    fn test_match_ordering() {
        let content = "z a b c";
        let matches = find_literal_matches(content, "a", true);

        // Should be sorted by position
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].start_byte, 2);
    }

    #[test]
    fn test_empty_content() {
        let content = "";
        let matches = find_literal_matches(content, "test", false);

        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_pattern_longer_than_content() {
        let content = "hi";
        let matches = find_literal_matches(content, "hello", false);

        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_exact_match() {
        let content = "user";
        let matches = find_literal_matches(content, "user", true);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].start_byte, 0);
        assert_eq!(matches[0].end_byte, 4);
    }
}
