//! Common validation utilities for language plugins
//!
//! This module provides validation functions that are shared across all language plugins
//! to eliminate code duplication and ensure consistent behavior.

/// Validates that a constant name follows the SCREAMING_SNAKE_CASE convention.
///
/// SCREAMING_SNAKE_CASE is a standard naming convention for constants across many languages.
/// It improves code readability by making constants easily distinguishable from variables.
///
/// # Requirements
/// - Only uppercase letters (A-Z), digits (0-9), and underscores (_) are allowed
/// - Must contain at least one uppercase letter (prevents pure numeric names like "123")
/// - Cannot start with underscore (reserved for private/internal conventions)
/// - Cannot end with underscore (conventionally implies trailing metadata)
///
/// # Valid Examples
/// - `TAX_RATE` - simple constant
/// - `MAX_USERS` - multi-word constant
/// - `API_KEY_V2` - constant with version number
/// - `DB_TIMEOUT_MS` - constant with unit suffix
/// - `A` - single-letter constants are valid
/// - `PI` - mathematical constants
///
/// # Invalid Examples
/// - `tax_rate` - lowercase
/// - `TaxRate` - camelCase
/// - `_TAX_RATE` - starts with underscore
/// - `TAX_RATE_` - ends with underscore
/// - `TAX-RATE` - uses hyphen instead of underscore
/// - `123` - no uppercase letter
/// - `` (empty string)
///
/// # Example
/// ```
/// use mill_lang_common::validation::is_screaming_snake_case;
///
/// assert!(is_screaming_snake_case("TAX_RATE"));
/// assert!(is_screaming_snake_case("MAX_VALUE"));
/// assert!(!is_screaming_snake_case("tax_rate"));
/// assert!(!is_screaming_snake_case("_PRIVATE"));
/// ```
pub fn is_screaming_snake_case(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // Must not start or end with underscore to maintain style consistency
    if name.starts_with('_') || name.ends_with('_') {
        return false;
    }

    // Check each character - only uppercase, digits, and underscores allowed
    for ch in name.chars() {
        match ch {
            'A'..='Z' | '0'..='9' | '_' => continue,
            _ => return false,
        }
    }

    // Must have at least one uppercase letter to ensure it's not purely numeric
    name.chars().any(|c| c.is_ascii_uppercase())
}

/// Checks if a character at a given position is escaped.
///
/// A character is escaped if it's preceded by an odd number of consecutive backslashes.
/// This is critical for correctly parsing string literals in source code.
///
/// # Algorithm
/// Counts consecutive backslashes immediately before the position:
/// - 0 backslashes: not escaped
/// - 1 backslash: escaped
/// - 2 backslashes: not escaped (the backslashes form a literal `\\`)
/// - 3 backslashes: escaped (2 form `\\`, 1 escapes the char)
/// - etc.
///
/// # Arguments
/// * `text` - The text to check
/// * `pos` - The position of the character to check (0-indexed by character, not byte)
///
/// # Returns
/// `true` if the character is escaped, `false` otherwise
///
/// # Examples
/// ```
/// use mill_lang_common::validation::is_escaped;
///
/// // No escaping
/// assert!(!is_escaped("hello", 0));
/// assert!(!is_escaped("hello", 4));
///
/// // Single backslash escapes the quote
/// let text = r#"He said \"hi\""#;
/// assert!(is_escaped(text, 9)); // The quote is escaped
///
/// // Double backslash does not escape the following character
/// let text = r#"path\\to\\file"#;
/// assert!(!is_escaped(text, 6)); // 't' is not escaped (preceded by \\)
///
/// // Triple backslash escapes the following character
/// let text = r#"path\\\to"#;
/// assert!(is_escaped(text, 7)); // 't' is escaped
/// ```
///
/// # Note on backslash counting
/// Backslashes work in pairs:
/// - `\\` produces one literal backslash
/// - `\n` produces a newline
/// - `\\n` produces a literal backslash followed by 'n'
/// - `\\\n` produces a literal backslash followed by newline
pub fn is_escaped(text: &str, pos: usize) -> bool {
    if pos == 0 {
        return false;
    }

    let chars: Vec<char> = text.chars().collect();
    let mut backslash_count = 0;
    let mut check_pos = pos;

    // Count consecutive backslashes IMMEDIATELY before the position
    while check_pos > 0 {
        check_pos -= 1;
        if check_pos < chars.len() && chars[check_pos] == '\\' {
            backslash_count += 1;
        } else {
            break;
        }
    }

    // If odd number of backslashes, the character is escaped
    backslash_count % 2 == 1
}

/// Counts unescaped occurrences of a quote character in text.
///
/// This function is essential for determining whether a position in code is inside
/// a string literal. By counting unescaped quotes, we can determine if we're in an
/// odd or even quote context.
///
/// # Algorithm
/// Iterates through the text character by character:
/// 1. When encountering the target quote character, check if it's escaped
/// 2. If not escaped, increment the count
/// 3. Return the total count of unescaped quotes
///
/// # Arguments
/// * `text` - The text to scan
/// * `quote_char` - The quote character to count (e.g., `'`, `"`, `` ` ``)
///
/// # Returns
/// The number of unescaped occurrences of the quote character
///
/// # Examples
/// ```
/// use mill_lang_common::validation::count_unescaped_quotes;
///
/// // No quotes
/// assert_eq!(count_unescaped_quotes("hello", '"'), 0);
///
/// // Regular string
/// assert_eq!(count_unescaped_quotes("\"hello\"", '"'), 2);
///
/// // Escaped quotes don't count
/// let text = r#"He said \"hi\""#;
/// assert_eq!(count_unescaped_quotes(text, '"'), 0);
///
/// // Mixed escaped and unescaped
/// let text = r#"say "hello \"world\"""#;
/// assert_eq!(count_unescaped_quotes(text, '"'), 2); // outer quotes only
///
/// // Double backslash doesn't escape the quote
/// let text = r#""path\\to\\file""#;
/// assert_eq!(count_unescaped_quotes(text, '"'), 2);
/// ```
///
/// # Usage Pattern
/// Check if a position is inside a string by counting quotes before it:
/// ```
/// use mill_lang_common::validation::count_unescaped_quotes;
///
/// let line = r#"const x = "hello"; // "comment""#;
/// let before_literal = &line[..10]; // "const x = "
/// let quotes = count_unescaped_quotes(before_literal, '"');
/// let inside_string = quotes % 2 == 1; // Odd = inside string
/// assert!(!inside_string); // We're not inside a string
/// ```
pub fn count_unescaped_quotes(text: &str, quote_char: char) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut count = 0;

    for i in 0..chars.len() {
        if chars[i] == quote_char && !is_escaped(text, i) {
            count += 1;
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // is_screaming_snake_case tests
    // ========================================================================

    #[test]
    fn test_is_screaming_snake_case_valid() {
        assert!(is_screaming_snake_case("TAX_RATE"));
        assert!(is_screaming_snake_case("MAX_VALUE"));
        assert!(is_screaming_snake_case("A"));
        assert!(is_screaming_snake_case("PI"));
        assert!(is_screaming_snake_case("API_KEY"));
        assert!(is_screaming_snake_case("DB_TIMEOUT_MS"));
        assert!(is_screaming_snake_case("MAX_USERS_V2"));
    }

    #[test]
    fn test_is_screaming_snake_case_invalid() {
        // Empty string
        assert!(!is_screaming_snake_case(""));

        // Starts with underscore
        assert!(!is_screaming_snake_case("_TAX_RATE"));
        assert!(!is_screaming_snake_case("_PRIVATE"));

        // Ends with underscore
        assert!(!is_screaming_snake_case("TAX_RATE_"));
        assert!(!is_screaming_snake_case("VALUE_"));

        // Lowercase
        assert!(!is_screaming_snake_case("tax_rate"));
        assert!(!is_screaming_snake_case("max_value"));

        // Mixed case
        assert!(!is_screaming_snake_case("TaxRate"));
        assert!(!is_screaming_snake_case("Tax_Rate"));

        // Kebab-case
        assert!(!is_screaming_snake_case("tax-rate"));
        assert!(!is_screaming_snake_case("TAX-RATE"));

        // No uppercase letter
        assert!(!is_screaming_snake_case("123"));
        assert!(!is_screaming_snake_case("_"));
    }

    // ========================================================================
    // is_escaped tests
    // ========================================================================

    #[test]
    fn test_is_escaped_basic() {
        // First character cannot be escaped
        assert!(!is_escaped("hello", 0));

        // Regular characters are not escaped
        assert!(!is_escaped("hello", 1));
        assert!(!is_escaped("hello", 4));
    }

    #[test]
    fn test_is_escaped_single_backslash() {
        // Single backslash escapes the next character
        let text = r#"a\"b"#;
        assert!(is_escaped(text, 2)); // The quote is escaped

        let text = r#"a\nb"#;
        assert!(is_escaped(text, 2)); // The 'n' is escaped
    }

    #[test]
    fn test_is_escaped_double_backslash() {
        // Double backslash = literal backslash, doesn't escape next char
        let text = r#"a\\"#;
        assert!(is_escaped(text, 2)); // Second backslash IS escaped by first

        let text = r#"a\\b"#;
        assert!(!is_escaped(text, 3)); // 'b' is NOT escaped (preceded by \\)
    }

    #[test]
    fn test_is_escaped_triple_backslash() {
        // Triple backslash = \\ + \x (escaped char)
        let text = r#"a\\\"#;
        assert!(!is_escaped(text, 3)); // Third backslash preceded by 2 (even)

        let text = r#"a\\\b"#;
        assert!(is_escaped(text, 4)); // 'b' is escaped (preceded by 3 backslashes)
    }

    #[test]
    fn test_is_escaped_complex() {
        // Test the example from docs
        let text = r#"He said \"hi\""#;
        assert!(is_escaped(text, 9)); // First quote is escaped
        assert!(is_escaped(text, 13)); // Second quote is escaped

        // Path example
        let text = r#"path\\to\\file"#;
        assert!(!is_escaped(text, 6)); // 't' not escaped (preceded by \\)
        assert!(!is_escaped(text, 10)); // 'f' not escaped (preceded by \\)
    }

    // ========================================================================
    // count_unescaped_quotes tests
    // ========================================================================

    #[test]
    fn test_count_unescaped_quotes_empty() {
        assert_eq!(count_unescaped_quotes("", '"'), 0);
        assert_eq!(count_unescaped_quotes("", '\''), 0);
        assert_eq!(count_unescaped_quotes("", '`'), 0);
    }

    #[test]
    fn test_count_unescaped_quotes_no_quotes() {
        assert_eq!(count_unescaped_quotes("hello world", '"'), 0);
        assert_eq!(count_unescaped_quotes("const x = 42", '\''), 0);
    }

    #[test]
    fn test_count_unescaped_quotes_regular() {
        // Regular strings
        assert_eq!(count_unescaped_quotes(r#""hello""#, '"'), 2);
        assert_eq!(count_unescaped_quotes("'hello'", '\''), 2);
        assert_eq!(count_unescaped_quotes("`hello`", '`'), 2);

        // In context
        assert_eq!(count_unescaped_quotes(r#"x = "hello""#, '"'), 2);
        assert_eq!(count_unescaped_quotes("x = 'hello'", '\''), 2);
    }

    #[test]
    fn test_count_unescaped_quotes_all_escaped() {
        // All quotes are escaped - should count 0
        assert_eq!(count_unescaped_quotes(r#"\"hello\""#, '"'), 0);
        assert_eq!(count_unescaped_quotes(r#"\'hello\'"#, '\''), 0);
        assert_eq!(count_unescaped_quotes(r#"\`hello\`"#, '`'), 0);
    }

    #[test]
    fn test_count_unescaped_quotes_mixed() {
        // Mixed escaped and unescaped quotes
        let text = r#"say "hello \"world\"""#;
        assert_eq!(count_unescaped_quotes(text, '"'), 2); // outer quotes only

        let text = r#"\"quote\" in middle "real""#;
        assert_eq!(count_unescaped_quotes(text, '"'), 2); // only the "real" string quotes
    }

    #[test]
    fn test_count_unescaped_quotes_escaped_backslash() {
        // Double backslash doesn't escape the following quote
        let text = r#""path\\to\\file""#;
        assert_eq!(count_unescaped_quotes(text, '"'), 2);

        // Triple backslash escapes the following quote
        let text = r#""test\\\""#;
        assert_eq!(count_unescaped_quotes(text, '"'), 1); // only opening quote
    }

    #[test]
    fn test_count_unescaped_quotes_real_world_examples() {
        // Python-style string
        assert_eq!(count_unescaped_quotes(r#""He said \"hi\"""#, '"'), 2);
        assert_eq!(count_unescaped_quotes("'It\\'s fine'", '\''), 2);

        // Go-style backticks (Go doesn't escape backticks in raw strings)
        assert_eq!(count_unescaped_quotes("hello `world`", '`'), 2);

        // TypeScript template literal
        assert_eq!(count_unescaped_quotes("`template ${var}`", '`'), 2);
    }

    #[test]
    fn test_count_unescaped_quotes_multiple_different_types() {
        // Should only count the requested quote type
        let text = r#""It's fine""#;
        assert_eq!(count_unescaped_quotes(text, '"'), 2); // double quotes
        assert_eq!(count_unescaped_quotes(text, '\''), 1); // single quote
    }
}
