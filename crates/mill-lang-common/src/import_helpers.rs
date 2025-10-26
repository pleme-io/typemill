//! Import support helper utilities
//!
//! Provides low-level primitives for import manipulation across language plugins.
//! Designed to be language-agnostic - all language-specific logic stays in plugins.
//!
//! # Design Principles
//! - Simple, composable functions (not frameworks)
//! - Zero-cost abstractions
//! - 100% test coverage
//! - No language-specific logic
//!
//! # Functions
//!
//! - [`find_last_matching_line`] - Find the last line matching a predicate
//! - [`insert_line_at`] - Insert a line at a specific position
//! - [`remove_lines_matching`] - Remove all lines matching a predicate
//! - [`replace_in_lines`] - Replace all occurrences of a pattern
//!
//! # Examples
//!
//! ```rust
//! use mill_lang_common::import_helpers::*;
//!
//! let content = "import A\nimport B\ncode";
//! let idx = find_last_matching_line(content, |line| line.trim().starts_with("import"));
//! assert_eq!(idx, Some(1));
//!
//! let result = insert_line_at(content, 2, "import C");
//! assert!(result.contains("import C"));
//!
//! let (result, count) = remove_lines_matching(content, |line| {
//!     line.trim().starts_with("import")
//! });
//! assert_eq!(result, "code");
//! assert_eq!(count, 2);
//!
//! let (result, count) = replace_in_lines(content, "import", "use");
//! assert_eq!(count, 2);
//! ```

/// Find the index of the last line matching a predicate.
///
/// Returns the 0-based line index, or None if no match found.
/// Handles both Unix (LF) and Windows (CRLF) line endings.
///
/// # Complexity
/// O(n) - single pass through content
///
/// # Examples
/// ```
/// use mill_lang_common::import_helpers::find_last_matching_line;
///
/// let content = "import A\nimport B\ncode";
/// let idx = find_last_matching_line(content, |line| line.trim().starts_with("import"));
/// assert_eq!(idx, Some(1));
///
/// // No matches
/// let idx = find_last_matching_line(content, |line| line.trim().starts_with("export"));
/// assert_eq!(idx, None);
///
/// // Empty content
/// let idx = find_last_matching_line("", |_| true);
/// assert_eq!(idx, None);
/// ```
pub fn find_last_matching_line<F>(content: &str, predicate: F) -> Option<usize>
where
    F: Fn(&str) -> bool,
{
    content
        .lines()
        .enumerate()
        .filter(|(_, line)| predicate(line))
        .last()
        .map(|(idx, _)| idx)
}

/// Insert a line at the specified 0-based line index.
///
/// If index is beyond the end of content, appends to end.
/// Preserves existing line endings (LF or CRLF).
///
/// # Complexity
/// O(n) - split, insert, join operations
///
/// # Examples
/// ```
/// use mill_lang_common::import_helpers::insert_line_at;
///
/// let content = "line 0\nline 1";
/// let result = insert_line_at(content, 1, "NEW");
/// assert_eq!(result, "line 0\nNEW\nline 1");
///
/// // Insert at beginning
/// let result = insert_line_at(content, 0, "FIRST");
/// assert_eq!(result, "FIRST\nline 0\nline 1");
///
/// // Insert beyond end (append)
/// let result = insert_line_at(content, 100, "LAST");
/// assert!(result.ends_with("LAST"));
///
/// // Empty content
/// let result = insert_line_at("", 0, "ONLY");
/// assert_eq!(result, "ONLY");
/// ```
pub fn insert_line_at(content: &str, line_index: usize, new_line: &str) -> String {
    // Detect line ending style
    let line_ending = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };

    let mut lines: Vec<&str> = content.lines().collect();

    // Handle empty content
    if lines.is_empty() {
        return new_line.to_string();
    }

    // Handle position beyond end (append)
    if line_index >= lines.len() {
        lines.push(new_line);
    } else {
        lines.insert(line_index, new_line);
    }

    lines.join(line_ending)
}

/// Remove all lines matching a predicate.
///
/// Returns (new_content, count_removed).
/// Preserves line endings for remaining lines.
///
/// # Complexity
/// O(n) - single pass filter operation
///
/// # Examples
/// ```
/// use mill_lang_common::import_helpers::remove_lines_matching;
///
/// let content = "import A\nimport B\ncode";
/// let (result, count) = remove_lines_matching(content, |line| {
///     line.trim().starts_with("import")
/// });
/// assert_eq!(result, "code");
/// assert_eq!(count, 2);
///
/// // Remove nothing
/// let (result, count) = remove_lines_matching(content, |_| false);
/// assert_eq!(result, content);
/// assert_eq!(count, 0);
///
/// // Remove everything
/// let (result, count) = remove_lines_matching(content, |_| true);
/// assert_eq!(result, "");
/// assert_eq!(count, 3);
/// ```
pub fn remove_lines_matching<F>(content: &str, predicate: F) -> (String, usize)
where
    F: Fn(&str) -> bool,
{
    // Detect line ending style
    let line_ending = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };

    let mut removed_count = 0;
    let remaining: Vec<&str> = content
        .lines()
        .filter(|line| {
            if predicate(line) {
                removed_count += 1;
                false
            } else {
                true
            }
        })
        .collect();

    let result = remaining.join(line_ending);
    (result, removed_count)
}

/// Replace all occurrences of a pattern in content.
///
/// Returns (new_content, count_replaced).
/// Counts total replacements, not lines changed.
///
/// # Complexity
/// O(n * m) - where n is content length, m is pattern length
///
/// # Examples
/// ```
/// use mill_lang_common::import_helpers::replace_in_lines;
///
/// let content = "use old::Foo;\nuse old::Bar;";
/// let (result, count) = replace_in_lines(content, "old", "new");
/// assert_eq!(result, "use new::Foo;\nuse new::Bar;");
/// assert_eq!(count, 2);
///
/// // Multiple replacements per line
/// let content = "old old old";
/// let (result, count) = replace_in_lines(content, "old", "new");
/// assert_eq!(result, "new new new");
/// assert_eq!(count, 3);
///
/// // Zero replacements
/// let content = "no matches here";
/// let (result, count) = replace_in_lines(content, "foo", "bar");
/// assert_eq!(result, content);
/// assert_eq!(count, 0);
/// ```
pub fn replace_in_lines(content: &str, old: &str, new: &str) -> (String, usize) {
    // Count occurrences
    let count = content.matches(old).count();

    // Perform replacement
    let result = content.replace(old, new);

    (result, count)
}
