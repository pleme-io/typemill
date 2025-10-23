//! Case-preserving string replacement utilities
//!
//! This module provides utilities for detecting and preserving naming conventions
//! (case styles) when performing find/replace operations. This is particularly useful
//! for refactoring operations where you want to rename identifiers while maintaining
//! the casing convention used in each occurrence.
//!
//! # Examples
//!
//! ```
//! use mill_handlers::handlers::workspace::case_preserving::{replace_preserving_case, CaseStyle};
//!
//! // Snake case preserved
//! assert_eq!(replace_preserving_case("user_name", "account_id"), "account_id");
//!
//! // Camel case preserved
//! assert_eq!(replace_preserving_case("userName", "accountId"), "accountId");
//!
//! // Pascal case preserved
//! assert_eq!(replace_preserving_case("UserName", "AccountId"), "AccountId");
//! ```
//!
//! # Known Limitations
//!
//! - **Acronyms**: Consecutive uppercase letters like "HTTPServer" are treated as a single
//!   word "HTTP" followed by "Server". This may not match all style guides.
//! - **Mixed styles**: Identifiers using multiple conventions (e.g., "XMLHttpRequest")
//!   are detected as `Mixed` and conversion attempts best-effort matching.
//! - **Numbers**: Numbers stay attached to the preceding word segment (e.g., "user2Name" →
//!   ["user2", "Name"]), which may not match all conventions.
//! - **Non-ASCII**: Unicode identifiers (emoji, CJK characters) are preserved as-is but
//!   may not convert correctly across case styles.
//! - **Single characters**: Single letters are converted to lowercase for safety.

use dashmap::DashMap;
use std::sync::OnceLock;

/// Represents the detected case style of an identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaseStyle {
    /// All lowercase: "username"
    Lower,
    /// All uppercase: "USERNAME"
    Upper,
    /// Snake case: "user_name"
    Snake,
    /// Camel case: "userName"
    Camel,
    /// Pascal case: "UserName"
    Pascal,
    /// Screaming snake case: "USER_NAME"
    ScreamingSnake,
    /// Kebab case: "user-name"
    Kebab,
    /// Multiple styles mixed: "XMLHttpRequest"
    Mixed,
    /// Cannot determine: single char, empty, or complex pattern
    Unknown,
}

impl CaseStyle {
    /// Returns true if this case style is considered "risky" to auto-convert
    /// (i.e., may produce unexpected results)
    pub fn is_risky(&self) -> bool {
        matches!(self, CaseStyle::Mixed | CaseStyle::Unknown)
    }
}

// Global cache instance using DashMap for concurrent access without locks
fn case_style_cache() -> &'static DashMap<String, CaseStyle> {
    static CACHE: OnceLock<DashMap<String, CaseStyle>> = OnceLock::new();
    CACHE.get_or_init(DashMap::new)
}

/// Detect the case style used in a string
///
/// # Algorithm
///
/// Detection follows this priority order:
/// 1. Empty string → Unknown
/// 2. Single character → Lower (safest default)
/// 3. Contains underscore → ScreamingSnake (all caps) or Snake (mixed case)
/// 4. Contains hyphen → Kebab
/// 5. All uppercase → Upper
/// 6. All lowercase → Lower
/// 7. Starts with uppercase + has more uppercase → Pascal or Mixed
/// 8. Starts with lowercase + has uppercase → Camel
/// 9. Otherwise → Unknown
///
/// # Examples
///
/// ```
/// use mill_handlers::handlers::workspace::case_preserving::{detect_case_style, CaseStyle};
///
/// assert_eq!(detect_case_style("user_name"), CaseStyle::Snake);
/// assert_eq!(detect_case_style("userName"), CaseStyle::Camel);
/// assert_eq!(detect_case_style("UserName"), CaseStyle::Pascal);
/// assert_eq!(detect_case_style("USER_NAME"), CaseStyle::ScreamingSnake);
/// assert_eq!(detect_case_style("user-name"), CaseStyle::Kebab);
/// assert_eq!(detect_case_style("XMLHttpRequest"), CaseStyle::Mixed);
/// ```
pub fn detect_case_style(text: &str) -> CaseStyle {
    // Check cache first
    if let Some(cached) = case_style_cache().get(text) {
        return *cached;
    }

    let style = detect_case_style_impl(text);

    // Cache the result
    case_style_cache().insert(text.to_string(), style);

    style
}

fn detect_case_style_impl(text: &str) -> CaseStyle {
    // Empty string
    if text.is_empty() {
        return CaseStyle::Unknown;
    }

    // Single character - treat as lowercase for safety
    if text.len() == 1 {
        return CaseStyle::Lower;
    }

    let has_underscore = text.contains('_');
    let has_hyphen = text.contains('-');
    let has_uppercase = text.chars().any(|c| c.is_uppercase());
    let has_lowercase = text.chars().any(|c| c.is_lowercase());
    let all_uppercase = text.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase());
    let all_lowercase = text.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_lowercase());

    // Rule 1: Contains underscore
    if has_underscore {
        return if all_uppercase {
            CaseStyle::ScreamingSnake
        } else {
            CaseStyle::Snake
        };
    }

    // Rule 2: Contains hyphen
    if has_hyphen {
        return CaseStyle::Kebab;
    }

    // Rule 3: All uppercase (no delimiters)
    if all_uppercase && !has_lowercase {
        return CaseStyle::Upper;
    }

    // Rule 4: All lowercase (no delimiters)
    if all_lowercase && !has_uppercase {
        return CaseStyle::Lower;
    }

    // Rule 5: Mixed case - need to analyze further
    if has_uppercase && has_lowercase {
        let first_char = text.chars().next().unwrap();

        // Count uppercase letters (excluding first character)
        let uppercase_count = text.chars().skip(1).filter(|c| c.is_uppercase()).count();

        if first_char.is_uppercase() {
            // Starts with uppercase
            if uppercase_count == 0 {
                // Only first letter is uppercase: "Username"
                CaseStyle::Pascal
            } else if is_clean_pascal_or_camel(text) {
                // Clean word boundaries: "UserName", "HTTPServer"
                CaseStyle::Pascal
            } else {
                // Complex pattern: "XMLHttpRequest", "getHTTPStatus"
                CaseStyle::Mixed
            }
        } else {
            // Starts with lowercase
            if uppercase_count == 0 {
                // No uppercase letters: shouldn't happen (has_uppercase is true)
                CaseStyle::Lower
            } else if is_clean_pascal_or_camel(text) {
                // Clean word boundaries: "userName", "getHTTPStatus"
                CaseStyle::Camel
            } else {
                // Complex pattern
                CaseStyle::Mixed
            }
        }
    } else {
        // No clear pattern
        CaseStyle::Unknown
    }
}

/// Check if a mixed-case string has clean word boundaries (PascalCase/camelCase)
/// vs complex patterns (XMLHttpRequest)
fn is_clean_pascal_or_camel(text: &str) -> bool {
    let words = split_on_uppercase_boundaries(text);

    // If we get very short segments (1 char) mixed with longer ones, it's likely Mixed
    // e.g., "XMLHttpRequest" → ["X", "M", "L", "Http", "Request"]
    let has_single_char_words = words.iter().any(|w| w.len() == 1);
    let has_multi_char_words = words.iter().any(|w| w.len() > 1);

    // Mixed if we have both single-char and multi-char segments
    // (indicates acronyms mixed with normal words)
    if has_single_char_words && has_multi_char_words && words.len() > 2 {
        // Exception: Two-word combos like "XMLParser" are acceptable as Pascal
        // But three+ with single chars like "XMLHttpRequest" are Mixed
        return words.len() <= 2;
    }

    true
}

/// Split on uppercase letter boundaries (helper for detection)
fn split_on_uppercase_boundaries(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        if ch.is_uppercase() && !current.is_empty() {
            words.push(current.clone());
            current.clear();
        }
        current.push(ch);
    }

    if !current.is_empty() {
        words.push(current);
    }

    words
}

/// Convert a string to match a specific case style
///
/// # Examples
///
/// ```
/// use mill_handlers::handlers::workspace::case_preserving::{apply_case_style, CaseStyle};
///
/// assert_eq!(apply_case_style("account_id", CaseStyle::Snake), "account_id");
/// assert_eq!(apply_case_style("account_id", CaseStyle::Camel), "accountId");
/// assert_eq!(apply_case_style("account_id", CaseStyle::Pascal), "AccountId");
/// assert_eq!(apply_case_style("account_id", CaseStyle::ScreamingSnake), "ACCOUNT_ID");
/// assert_eq!(apply_case_style("account_id", CaseStyle::Kebab), "account-id");
/// ```
pub fn apply_case_style(text: &str, style: CaseStyle) -> String {
    if text.is_empty() {
        return String::new();
    }

    // Single character special case
    if text.len() == 1 {
        return match style {
            CaseStyle::Upper | CaseStyle::ScreamingSnake => text.to_uppercase(),
            _ => text.to_lowercase(),
        };
    }

    // Split into words
    let words = split_into_words(text);

    // Apply the target case style
    match style {
        CaseStyle::Lower => words.join("").to_lowercase(),
        CaseStyle::Upper => words.join("").to_uppercase(),
        CaseStyle::Snake => to_snake_case(&words),
        CaseStyle::Camel => to_camel_case(&words),
        CaseStyle::Pascal => to_pascal_case(&words),
        CaseStyle::ScreamingSnake => to_screaming_snake_case(&words),
        CaseStyle::Kebab => to_kebab_case(&words),
        CaseStyle::Mixed | CaseStyle::Unknown => {
            // Best effort: try Pascal case for Mixed/Unknown
            to_pascal_case(&words)
        }
    }
}

/// Split a string into word segments based on detected delimiters
///
/// Handles:
/// - Snake case: split on '_'
/// - Kebab case: split on '-'
/// - Camel/Pascal case: split on uppercase boundaries
/// - Numbers: keep attached to preceding word segment
///
/// # Examples
///
/// ```
/// use mill_handlers::handlers::workspace::case_preserving::split_into_words;
///
/// assert_eq!(split_into_words("user_name"), vec!["user", "name"]);
/// assert_eq!(split_into_words("userName"), vec!["user", "name"]);
/// assert_eq!(split_into_words("user-name"), vec!["user", "name"]);
/// assert_eq!(split_into_words("user2Name"), vec!["user2", "name"]);
/// ```
pub fn split_into_words(text: &str) -> Vec<String> {
    if text.is_empty() {
        return vec![];
    }

    // Check for explicit delimiters first
    if text.contains('_') {
        return text.split('_').filter(|s| !s.is_empty()).map(|s| s.to_lowercase()).collect();
    }

    if text.contains('-') {
        return text.split('-').filter(|s| !s.is_empty()).map(|s| s.to_lowercase()).collect();
    }

    // No explicit delimiters - split on case boundaries
    split_on_case_boundaries(text)
}

/// Split on case boundaries for camelCase/PascalCase
///
/// Handles special cases:
/// - Consecutive uppercase: "HTTPServer" → ["HTTP", "Server"]
/// - Numbers: "user2Name" → ["user2", "Name"]
fn split_on_case_boundaries(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch.is_uppercase() {
            // Check if we're starting a new word
            if !current.is_empty() {
                // Check if this is part of an acronym (next char is also uppercase)
                if let Some(&next_ch) = chars.peek() {
                    if next_ch.is_uppercase() {
                        // Part of acronym - continue building current word
                        current.push(ch);
                    } else {
                        // Start of new word after lowercase section
                        words.push(current.to_lowercase());
                        current = ch.to_string();
                    }
                } else {
                    // Last character
                    current.push(ch);
                }
            } else {
                // First character or continuing an acronym
                current.push(ch);
            }
        } else if ch.is_lowercase() {
            // Lowercase letter
            if !current.is_empty() && current.chars().all(|c| c.is_uppercase()) && current.len() > 1 {
                // We were building an acronym, need to split
                // e.g., "HTTPServer" at 's': current = "HTTPS", need to split to "HTTP" + "Server"
                let last_char = current.pop().unwrap();
                words.push(current.to_lowercase());
                current = format!("{}{}", last_char, ch);
            } else {
                current.push(ch);
            }
        } else if ch.is_numeric() {
            // Numbers stay with current word
            current.push(ch);
        } else {
            // Other characters (rare in identifiers) - treat as word boundary
            if !current.is_empty() {
                words.push(current.to_lowercase());
                current = String::new();
            }
        }
    }

    if !current.is_empty() {
        words.push(current.to_lowercase());
    }

    words
}

/// Convert words to snake_case
fn to_snake_case(words: &[String]) -> String {
    words.join("_")
}

/// Convert words to camelCase
fn to_camel_case(words: &[String]) -> String {
    if words.is_empty() {
        return String::new();
    }

    let mut result = words[0].to_lowercase();
    for word in &words[1..] {
        result.push_str(&capitalize_first(word));
    }
    result
}

/// Convert words to PascalCase
fn to_pascal_case(words: &[String]) -> String {
    words.iter().map(|w| capitalize_first(w)).collect()
}

/// Convert words to kebab-case
fn to_kebab_case(words: &[String]) -> String {
    words.join("-")
}

/// Convert words to SCREAMING_SNAKE_CASE
fn to_screaming_snake_case(words: &[String]) -> String {
    words.join("_").to_uppercase()
}

/// Capitalize the first character of a string
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

/// High-level function: replace a matched string with a replacement while preserving case
///
/// # Examples
///
/// ```
/// use mill_handlers::handlers::workspace::case_preserving::replace_preserving_case;
///
/// // Different case styles preserved
/// assert_eq!(replace_preserving_case("user_name", "account_id"), "account_id");
/// assert_eq!(replace_preserving_case("userName", "accountId"), "accountId");
/// assert_eq!(replace_preserving_case("UserName", "AccountId"), "AccountId");
/// assert_eq!(replace_preserving_case("USER_NAME", "ACCOUNT_ID"), "ACCOUNT_ID");
/// ```
pub fn replace_preserving_case(matched: &str, replacement: &str) -> String {
    let style = detect_case_style(matched);
    apply_case_style(replacement, style)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Detection Tests =====

    #[test]
    fn test_detect_snake_case() {
        assert_eq!(detect_case_style("user_name"), CaseStyle::Snake);
        assert_eq!(detect_case_style("get_user_by_id"), CaseStyle::Snake);
        assert_eq!(detect_case_style("a_b_c"), CaseStyle::Snake);
    }

    #[test]
    fn test_detect_camel_case() {
        assert_eq!(detect_case_style("userName"), CaseStyle::Camel);
        assert_eq!(detect_case_style("getUserById"), CaseStyle::Camel);
        assert_eq!(detect_case_style("a"), CaseStyle::Lower); // Single char
    }

    #[test]
    fn test_detect_pascal_case() {
        assert_eq!(detect_case_style("UserName"), CaseStyle::Pascal);
        assert_eq!(detect_case_style("GetUserById"), CaseStyle::Pascal);
        assert_eq!(detect_case_style("HttpClient"), CaseStyle::Pascal);
    }

    #[test]
    fn test_detect_screaming_snake() {
        assert_eq!(detect_case_style("USER_NAME"), CaseStyle::ScreamingSnake);
        assert_eq!(detect_case_style("MAX_BUFFER_SIZE"), CaseStyle::ScreamingSnake);
        assert_eq!(detect_case_style("A_B_C"), CaseStyle::ScreamingSnake);
    }

    #[test]
    fn test_detect_kebab_case() {
        assert_eq!(detect_case_style("user-name"), CaseStyle::Kebab);
        assert_eq!(detect_case_style("get-user-by-id"), CaseStyle::Kebab);
        assert_eq!(detect_case_style("a-b-c"), CaseStyle::Kebab);
    }

    #[test]
    fn test_detect_upper() {
        assert_eq!(detect_case_style("USERNAME"), CaseStyle::Upper);
        assert_eq!(detect_case_style("HTTP"), CaseStyle::Upper);
    }

    #[test]
    fn test_detect_lower() {
        assert_eq!(detect_case_style("username"), CaseStyle::Lower);
        assert_eq!(detect_case_style("http"), CaseStyle::Lower);
    }

    #[test]
    fn test_detect_mixed_case() {
        // XMLHttpRequest has single-char words mixed with multi-char
        assert_eq!(detect_case_style("XMLHttpRequest"), CaseStyle::Mixed);
        // But simple two-word acronym combos are Pascal
        assert_eq!(detect_case_style("XMLParser"), CaseStyle::Pascal);
    }

    #[test]
    fn test_detect_unknown() {
        assert_eq!(detect_case_style(""), CaseStyle::Unknown);
    }

    // ===== Conversion Tests =====

    #[test]
    fn test_preserve_case_snake_to_camel() {
        let matched = "user_name";
        let replacement = "account_id";
        assert_eq!(replace_preserving_case(matched, replacement), "account_id");
    }

    #[test]
    fn test_preserve_case_camel_to_snake() {
        let matched = "userName";
        let replacement = "account_id";
        // Detect camel, apply camel to "account_id"
        assert_eq!(replace_preserving_case(matched, replacement), "accountId");
    }

    #[test]
    fn test_preserve_case_screaming() {
        let matched = "USER_NAME";
        let replacement = "account_id";
        assert_eq!(replace_preserving_case(matched, replacement), "ACCOUNT_ID");
    }

    #[test]
    fn test_preserve_case_pascal() {
        let matched = "UserName";
        let replacement = "account_id";
        assert_eq!(replace_preserving_case(matched, replacement), "AccountId");
    }

    #[test]
    fn test_preserve_case_kebab() {
        let matched = "user-name";
        let replacement = "account_id";
        assert_eq!(replace_preserving_case(matched, replacement), "account-id");
    }

    // ===== Real-world Test Cases =====

    #[test]
    fn test_real_world_snake_to_all() {
        let original = "user_name";
        let new_val = "account_id";

        // snake_case → snake_case
        assert_eq!(apply_case_style(new_val, detect_case_style("user_name")), "account_id");
        // snake_case → camelCase
        assert_eq!(apply_case_style(new_val, detect_case_style("userName")), "accountId");
        // snake_case → PascalCase
        assert_eq!(apply_case_style(new_val, detect_case_style("UserName")), "AccountId");
        // snake_case → SCREAMING_SNAKE
        assert_eq!(apply_case_style(new_val, detect_case_style("USER_NAME")), "ACCOUNT_ID");
        // snake_case → kebab-case
        assert_eq!(apply_case_style(new_val, detect_case_style("user-name")), "account-id");
    }

    #[test]
    fn test_real_world_all_to_snake() {
        let inputs = vec![
            ("user_name", "account_id"),
            ("userName", "accountId"),
            ("UserName", "AccountId"),
            ("USER_NAME", "ACCOUNT_ID"),
            ("user-name", "account-id"),
        ];

        for (matched, expected) in inputs {
            assert_eq!(replace_preserving_case(matched, "account_id"), expected);
        }
    }

    // ===== Edge Cases =====

    #[test]
    fn test_acronyms() {
        // HTTPServer: "HTTP" + "Server"
        let words = split_into_words("HTTPServer");
        assert_eq!(words, vec!["http", "server"]);

        // Convert back to camelCase
        assert_eq!(to_camel_case(&words), "httpServer");

        // Convert back to PascalCase
        assert_eq!(to_pascal_case(&words), "HttpServer");
    }

    #[test]
    fn test_numbers() {
        // user2Name → ["user2", "name"]
        let words = split_into_words("user2Name");
        assert_eq!(words, vec!["user2", "name"]);

        // Convert to snake_case
        assert_eq!(to_snake_case(&words), "user2_name");

        // Convert to camelCase
        assert_eq!(to_camel_case(&words), "user2Name");
    }

    #[test]
    fn test_single_char() {
        assert_eq!(detect_case_style("a"), CaseStyle::Lower);
        assert_eq!(apply_case_style("a", CaseStyle::Upper), "A");
        assert_eq!(apply_case_style("A", CaseStyle::Lower), "a");
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(detect_case_style(""), CaseStyle::Unknown);
        assert_eq!(apply_case_style("", CaseStyle::Snake), "");
        assert_eq!(replace_preserving_case("", "replacement"), "Replacement");
    }

    // ===== Word Splitting Tests =====

    #[test]
    fn test_split_snake_case() {
        assert_eq!(split_into_words("user_name"), vec!["user", "name"]);
        assert_eq!(split_into_words("get_user_by_id"), vec!["get", "user", "by", "id"]);
    }

    #[test]
    fn test_split_kebab_case() {
        assert_eq!(split_into_words("user-name"), vec!["user", "name"]);
        assert_eq!(split_into_words("get-user-by-id"), vec!["get", "user", "by", "id"]);
    }

    #[test]
    fn test_split_camel_case() {
        assert_eq!(split_into_words("userName"), vec!["user", "name"]);
        assert_eq!(split_into_words("getUserById"), vec!["get", "user", "by", "id"]);
    }

    #[test]
    fn test_split_pascal_case() {
        assert_eq!(split_into_words("UserName"), vec!["user", "name"]);
        assert_eq!(split_into_words("GetUserById"), vec!["get", "user", "by", "id"]);
    }

    #[test]
    fn test_split_with_acronym() {
        assert_eq!(split_into_words("HTTPServer"), vec!["http", "server"]);
        assert_eq!(split_into_words("XMLParser"), vec!["xml", "parser"]);
    }

    #[test]
    fn test_split_with_numbers() {
        assert_eq!(split_into_words("user2Name"), vec!["user2", "name"]);
        assert_eq!(split_into_words("base64Encode"), vec!["base64", "encode"]);
    }

    // ===== Conversion Function Tests =====

    #[test]
    fn test_to_snake_case() {
        let words = vec!["user".to_string(), "name".to_string()];
        assert_eq!(to_snake_case(&words), "user_name");
        let words = vec!["get".to_string(), "user".to_string(), "by".to_string(), "id".to_string()];
        assert_eq!(to_snake_case(&words), "get_user_by_id");
    }

    #[test]
    fn test_to_camel_case() {
        let words = vec!["user".to_string(), "name".to_string()];
        assert_eq!(to_camel_case(&words), "userName");
        let words = vec!["get".to_string(), "user".to_string(), "by".to_string(), "id".to_string()];
        assert_eq!(to_camel_case(&words), "getUserById");
        assert_eq!(to_camel_case(&[]), "");
    }

    #[test]
    fn test_to_pascal_case() {
        let words = vec!["user".to_string(), "name".to_string()];
        assert_eq!(to_pascal_case(&words), "UserName");
        let words = vec!["get".to_string(), "user".to_string(), "by".to_string(), "id".to_string()];
        assert_eq!(to_pascal_case(&words), "GetUserById");
    }

    #[test]
    fn test_to_kebab_case() {
        let words = vec!["user".to_string(), "name".to_string()];
        assert_eq!(to_kebab_case(&words), "user-name");
        let words = vec!["get".to_string(), "user".to_string(), "by".to_string(), "id".to_string()];
        assert_eq!(to_kebab_case(&words), "get-user-by-id");
    }

    #[test]
    fn test_to_screaming_snake_case() {
        let words = vec!["user".to_string(), "name".to_string()];
        assert_eq!(to_screaming_snake_case(&words), "USER_NAME");
        let words = vec!["max".to_string(), "buffer".to_string(), "size".to_string()];
        assert_eq!(to_screaming_snake_case(&words), "MAX_BUFFER_SIZE");
    }

    // ===== Cache Tests =====

    #[test]
    fn test_cache_works() {
        // First call should cache
        let style1 = detect_case_style("userName");
        // Second call should hit cache
        let style2 = detect_case_style("userName");
        assert_eq!(style1, style2);
        assert_eq!(style1, CaseStyle::Camel);
    }

    // ===== Apply Case Style Tests =====

    #[test]
    fn test_apply_all_styles() {
        let words = "account_id";

        assert_eq!(apply_case_style(words, CaseStyle::Lower), "accountid");
        assert_eq!(apply_case_style(words, CaseStyle::Upper), "ACCOUNTID");
        assert_eq!(apply_case_style(words, CaseStyle::Snake), "account_id");
        assert_eq!(apply_case_style(words, CaseStyle::Camel), "accountId");
        assert_eq!(apply_case_style(words, CaseStyle::Pascal), "AccountId");
        assert_eq!(apply_case_style(words, CaseStyle::ScreamingSnake), "ACCOUNT_ID");
        assert_eq!(apply_case_style(words, CaseStyle::Kebab), "account-id");
    }
}
