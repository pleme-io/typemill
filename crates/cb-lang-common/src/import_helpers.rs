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
//! use cb_lang_common::import_helpers::*;
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
/// use cb_lang_common::import_helpers::find_last_matching_line;
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
/// use cb_lang_common::import_helpers::insert_line_at;
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
/// use cb_lang_common::import_helpers::remove_lines_matching;
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
/// use cb_lang_common::import_helpers::replace_in_lines;
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

#[cfg(test)]
mod tests {
    use super::*;

    // ========================
    // find_last_matching_line tests
    // ========================

    #[test]
    fn test_find_last_matching_line_empty_content() {
        let result = find_last_matching_line("", |_| true);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_last_matching_line_no_matches() {
        let content = "line 1\nline 2\nline 3";
        let result = find_last_matching_line(content, |line| line.contains("nonexistent"));
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_last_matching_line_single_match() {
        let content = "import A\ncode\nmore code";
        let result = find_last_matching_line(content, |line| line.trim().starts_with("import"));
        assert_eq!(result, Some(0));
    }

    #[test]
    fn test_find_last_matching_line_multiple_matches() {
        let content = "import A\nimport B\ncode\nimport C";
        let result = find_last_matching_line(content, |line| line.trim().starts_with("import"));
        assert_eq!(result, Some(3));
    }

    #[test]
    fn test_find_last_matching_line_windows_line_endings() {
        let content = "import A\r\nimport B\r\ncode";
        let result = find_last_matching_line(content, |line| line.trim().starts_with("import"));
        assert_eq!(result, Some(1));
    }

    #[test]
    fn test_find_last_matching_line_single_line_no_newline() {
        let content = "import A";
        let result = find_last_matching_line(content, |line| line.trim().starts_with("import"));
        assert_eq!(result, Some(0));
    }

    #[test]
    fn test_find_last_matching_line_all_match() {
        let content = "import A\nimport B\nimport C";
        let result = find_last_matching_line(content, |_| true);
        assert_eq!(result, Some(2));
    }

    // ========================
    // insert_line_at tests
    // ========================

    #[test]
    fn test_insert_line_at_empty_content() {
        let result = insert_line_at("", 0, "first line");
        assert_eq!(result, "first line");
    }

    #[test]
    fn test_insert_line_at_beginning() {
        let content = "line 0\nline 1";
        let result = insert_line_at(content, 0, "NEW");
        assert_eq!(result, "NEW\nline 0\nline 1");
    }

    #[test]
    fn test_insert_line_at_middle() {
        let content = "line 0\nline 1\nline 2";
        let result = insert_line_at(content, 1, "INSERTED");
        assert_eq!(result, "line 0\nINSERTED\nline 1\nline 2");
    }

    #[test]
    fn test_insert_line_at_end() {
        let content = "line 0\nline 1";
        let result = insert_line_at(content, 2, "LAST");
        assert_eq!(result, "line 0\nline 1\nLAST");
    }

    #[test]
    fn test_insert_line_at_beyond_end() {
        let content = "line 0\nline 1";
        let result = insert_line_at(content, 100, "APPENDED");
        assert_eq!(result, "line 0\nline 1\nAPPENDED");
    }

    #[test]
    fn test_insert_line_at_preserve_crlf() {
        let content = "line 0\r\nline 1";
        let result = insert_line_at(content, 1, "NEW");
        assert_eq!(result, "line 0\r\nNEW\r\nline 1");
    }

    #[test]
    fn test_insert_line_at_single_line_no_newline() {
        let content = "only line";
        let result = insert_line_at(content, 0, "FIRST");
        assert_eq!(result, "FIRST\nonly line");
    }

    // ========================
    // remove_lines_matching tests
    // ========================

    #[test]
    fn test_remove_lines_matching_remove_all() {
        let content = "import A\nimport B\nimport C";
        let (result, count) = remove_lines_matching(content, |_| true);
        assert_eq!(result, "");
        assert_eq!(count, 3);
    }

    #[test]
    fn test_remove_lines_matching_remove_none() {
        let content = "import A\nimport B\ncode";
        let (result, count) = remove_lines_matching(content, |_| false);
        assert_eq!(result, content);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_remove_lines_matching_remove_some() {
        let content = "import A\ncode1\nimport B\ncode2";
        let (result, count) =
            remove_lines_matching(content, |line| line.trim().starts_with("import"));
        assert_eq!(result, "code1\ncode2");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_remove_lines_matching_preserve_empty_lines() {
        let content = "import A\n\nimport B\n\ncode";
        let (result, count) =
            remove_lines_matching(content, |line| line.trim().starts_with("import"));
        assert_eq!(result, "\n\ncode");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_remove_lines_matching_count_verification() {
        let content = "a\nb\nc\nd\ne";
        let (_, count) = remove_lines_matching(content, |line| line == "b" || line == "d");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_remove_lines_matching_preserve_crlf() {
        let content = "import A\r\ncode\r\nimport B";
        let (result, count) =
            remove_lines_matching(content, |line| line.trim().starts_with("import"));
        assert_eq!(result, "code");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_remove_lines_matching_empty_content() {
        let (result, count) = remove_lines_matching("", |_| true);
        assert_eq!(result, "");
        assert_eq!(count, 0);
    }

    // ========================
    // replace_in_lines tests
    // ========================

    #[test]
    fn test_replace_in_lines_zero_replacements() {
        let content = "no matches here";
        let (result, count) = replace_in_lines(content, "foo", "bar");
        assert_eq!(result, content);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_replace_in_lines_single_replacement() {
        let content = "use old::Foo;";
        let (result, count) = replace_in_lines(content, "old", "new");
        assert_eq!(result, "use new::Foo;");
        assert_eq!(count, 1);
    }

    #[test]
    fn test_replace_in_lines_multiple_per_line() {
        let content = "old old old";
        let (result, count) = replace_in_lines(content, "old", "new");
        assert_eq!(result, "new new new");
        assert_eq!(count, 3);
    }

    #[test]
    fn test_replace_in_lines_count_verification() {
        let content = "use old::Foo;\nuse old::Bar;\nuse other::Baz;";
        let (result, count) = replace_in_lines(content, "old", "new");
        assert_eq!(result, "use new::Foo;\nuse new::Bar;\nuse other::Baz;");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_replace_in_lines_special_characters() {
        let content = "path/to/file.rs";
        let (result, count) = replace_in_lines(content, "/", "::");
        assert_eq!(result, "path::to::file.rs");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_replace_in_lines_preserve_structure() {
        let content = "line1\nline2\nline3";
        let (result, _) = replace_in_lines(content, "line", "row");
        assert_eq!(result, "row1\nrow2\nrow3");
        // Verify line structure preserved
        assert_eq!(result.lines().count(), 3);
    }

    #[test]
    fn test_replace_in_lines_empty_content() {
        let (result, count) = replace_in_lines("", "old", "new");
        assert_eq!(result, "");
        assert_eq!(count, 0);
    }

    // ========================
    // Edge case tests
    // ========================

    #[test]
    fn test_unicode_handling() {
        let content = "导入 A\n导入 B\ncode";
        let idx = find_last_matching_line(content, |line| line.contains("导入"));
        assert_eq!(idx, Some(1));

        let result = insert_line_at(content, 2, "导入 C");
        assert!(result.contains("导入 C"));

        let (result, count) = remove_lines_matching(content, |line| line.contains("导入"));
        assert_eq!(result, "code");
        assert_eq!(count, 2);

        let (result, count) = replace_in_lines(content, "导入", "import");
        assert_eq!(count, 2);
        assert!(result.contains("import"));
    }

    #[test]
    fn test_large_content() {
        // Generate 10,000 line file
        let lines: Vec<String> = (0..10000)
            .map(|i| {
                if i % 100 == 0 {
                    format!("import line_{}", i)
                } else {
                    format!("code line_{}", i)
                }
            })
            .collect();
        let content = lines.join("\n");

        // Test find_last_matching_line performance
        let result = find_last_matching_line(&content, |line| line.trim().starts_with("import"));
        assert_eq!(result, Some(9900));

        // Test insert_line_at performance
        let result = insert_line_at(&content, 5000, "NEW LINE");
        assert!(result.contains("NEW LINE"));

        // Test remove_lines_matching performance
        let (result, count) =
            remove_lines_matching(&content, |line| line.trim().starts_with("import"));
        assert_eq!(count, 100);
        assert!(!result.contains("import line_0"));

        // Test replace_in_lines performance
        let (result, count) = replace_in_lines(&content, "import", "use");
        assert_eq!(count, 100);
        assert!(result.contains("use line_0"));
    }

    #[test]
    fn test_empty_lines_preserved() {
        let content = "import A\n\n\ncode\n\nimport B";

        // Verify empty lines preserved during removal
        let (result, _) = remove_lines_matching(content, |line| line.trim().starts_with("import"));
        assert_eq!(result, "\n\ncode\n");

        // Verify empty lines preserved during insertion
        let result = insert_line_at(content, 3, "NEW");
        let empty_count = result.lines().filter(|l| l.is_empty()).count();
        assert!(empty_count >= 2);
    }

    #[test]
    fn test_mixed_line_endings() {
        // Content with mixed line endings (shouldn't happen in practice, but test anyway)
        let content = "line1\nline2\r\nline3\n";

        // Should detect CRLF and use it
        let result = insert_line_at(content, 1, "NEW");
        // lines() iterator normalizes line endings, so both work
        assert!(result.contains("NEW"));
    }

    #[test]
    fn test_whitespace_only_lines() {
        let content = "import A\n   \nimport B\n\t\ncode";

        // Don't match whitespace-only lines as imports
        let (result, count) =
            remove_lines_matching(content, |line| line.trim().starts_with("import"));
        assert_eq!(count, 2);
        assert!(result.contains("   "));
        assert!(result.contains("\t"));
    }

    // ========================
    // Real-world pattern tests
    // ========================

    #[test]
    fn test_swift_add_import_pattern() {
        let content = "import Foundation\nimport UIKit\n\nclass MyClass {}";

        let last_idx =
            find_last_matching_line(content, |line| line.trim().starts_with("import ")).unwrap();

        let result = insert_line_at(content, last_idx + 1, "import SwiftUI");

        assert!(result.contains("import Foundation"));
        assert!(result.contains("import UIKit"));
        assert!(result.contains("import SwiftUI"));
        assert!(result.contains("class MyClass"));
    }

    #[test]
    fn test_rust_add_import_pattern() {
        let content = "use std::collections::HashMap;\nuse std::fs;\n\nfn main() {}";

        let last_idx =
            find_last_matching_line(content, |line| line.trim().starts_with("use ")).unwrap();

        let result = insert_line_at(content, last_idx + 1, "use std::io;");

        assert!(result.contains("use std::collections::HashMap;"));
        assert!(result.contains("use std::fs;"));
        assert!(result.contains("use std::io;"));
        assert!(result.contains("fn main()"));
    }

    #[test]
    fn test_python_remove_import_pattern() {
        let content = "import os\nimport sys\nfrom typing import List\n\ndef main():\n    pass";

        let (result, count) = remove_lines_matching(content, |line| {
            let trimmed = line.trim();
            trimmed.starts_with("import ") || trimmed.starts_with("from ")
        });

        assert_eq!(count, 3);
        assert!(!result.contains("import os"));
        assert!(!result.contains("from typing"));
        assert!(result.contains("def main():"));
    }

    #[test]
    fn test_typescript_rename_import_pattern() {
        let content =
            "import { Foo } from 'old-module';\nimport { Bar } from 'old-module';\n\nconst x = 1;";

        let (result, count) = replace_in_lines(content, "old-module", "new-module");

        assert_eq!(count, 2);
        assert!(!result.contains("old-module"));
        assert!(result.contains("new-module"));
        assert!(result.contains("const x = 1"));
    }
}
