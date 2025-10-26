//! Property-based tests for import helper primitives
//!
//! These tests use `proptest` to generate random inputs and verify
//! universal properties that must hold for all valid inputs.
//!
//! # Coverage
//!
//! - `find_last_matching_line` - 7 properties
//! - `insert_line_at` - 6 properties
//! - `remove_lines_matching` - 5 properties
//! - `replace_in_lines` - 4 properties
//!
//! Total: 22 property-based tests (100+ test cases per property)

use mill_lang_common::import_helpers::*;
use proptest::prelude::*;

// ============================================================================
// Property Tests: find_last_matching_line
// ============================================================================

proptest! {
    /// Property: If result is Some(idx), idx must be a valid line number
    #[test]
    fn prop_find_last_returns_valid_index(content in ".*") {
        if let Some(idx) = find_last_matching_line(&content, |_| true) {
            let line_count = content.lines().count();
            prop_assert!(idx < line_count, "Index {} must be < line count {}", idx, line_count);
        }
    }

    /// Property: Predicate that never matches returns None
    #[test]
    fn prop_find_last_no_match_returns_none(content in ".*") {
        let result = find_last_matching_line(&content, |_| false);
        prop_assert_eq!(result, None);
    }

    /// Property: If all lines match, returns last line index
    #[test]
    fn prop_find_last_all_match_returns_last(content in ".+") {
        let line_count = content.lines().count();
        if line_count > 0 {
            let result = find_last_matching_line(&content, |_| true);
            let expected = line_count - 1;
            prop_assert_eq!(result, Some(expected), "Expected last line index {}", expected);
        }
    }

    /// Property: Empty content always returns None
    #[test]
    fn prop_find_last_empty_returns_none(_any in any::<u32>()) {
        let result = find_last_matching_line("", |_| true);
        prop_assert_eq!(result, None);
    }

    /// Property: Result is deterministic (same input = same output)
    #[test]
    fn prop_find_last_deterministic(content in ".*") {
        let predicate = |line: &str| line.len().is_multiple_of(2);
        let result1 = find_last_matching_line(&content, predicate);
        let result2 = find_last_matching_line(&content, predicate);
        prop_assert_eq!(result1, result2);
    }

    /// Property: Finding with line.contains() should work correctly
    #[test]
    fn prop_find_last_contains(
        lines in prop::collection::vec("[a-z]+", 1..20),
        pattern in "[a-z]",
    ) {
        let content = lines.join("\n");
        let result = find_last_matching_line(&content, |line| line.contains(&pattern));

        if let Some(idx) = result {
            // Verify the found line actually contains the pattern
            let found_line = content.lines().nth(idx).unwrap();
            prop_assert!(found_line.contains(&pattern));

            // Verify no later line contains the pattern
            for (i, line) in content.lines().enumerate() {
                if i > idx {
                    prop_assert!(!line.contains(&pattern));
                }
            }
        }
    }

    /// Property: Result index, if Some, points to a line that matches predicate
    #[test]
    fn prop_find_last_result_matches_predicate(
        content in ".*",
        char_to_find in prop::sample::select(vec!['a', 'e', 'i', 'o', 'u']),
    ) {
        let result = find_last_matching_line(&content, |line| line.contains(char_to_find));

        if let Some(idx) = result {
            let line = content.lines().nth(idx).unwrap();
            prop_assert!(line.contains(char_to_find), "Line at index {} should contain '{}'", idx, char_to_find);
        }
    }
}

// ============================================================================
// Property Tests: insert_line_at
// ============================================================================

proptest! {
    /// Property: Inserting a non-empty line increases line count by exactly 1
    #[test]
    fn prop_insert_increases_line_count(
        content in ".*",
        idx in 0usize..100,
        line in ".+",  // Non-empty line
    ) {
        let original_count = if content.is_empty() { 0 } else { content.lines().count() };
        let result = insert_line_at(&content, idx, &line);
        let new_count = if result.is_empty() { 0 } else { result.lines().count() };

        prop_assert_eq!(
            new_count,
            original_count + 1,
            "Line count should increase by 1: {} -> {}",
            original_count,
            new_count
        );
    }

    /// Property: CRLF content stays CRLF, LF stays LF
    #[test]
    fn prop_insert_preserves_line_endings(
        lines in prop::collection::vec("[a-zA-Z0-9]+", 1..10),
        idx in 0usize..10,
        new_line in "[a-zA-Z0-9]+",
    ) {
        let crlf_content = lines.join("\r\n");
        let lf_content = lines.join("\n");

        let crlf_result = insert_line_at(&crlf_content, idx, &new_line);
        let lf_result = insert_line_at(&lf_content, idx, &new_line);

        if crlf_content.contains("\r\n") && !crlf_content.is_empty() {
            prop_assert!(crlf_result.contains("\r\n"), "Must preserve CRLF line endings");
        }

        // LF result should not have CRLF unless the new_line itself contains it
        if !new_line.contains("\r\n") {
            prop_assert!(!lf_result.contains("\r\n"), "Must preserve LF line endings");
        }
    }

    /// Property: Inserted line appears in the result
    #[test]
    fn prop_insert_line_appears_in_result(
        content in ".*",
        idx in 0usize..50,
        line in "[A-Z]+",
    ) {
        let result = insert_line_at(&content, idx, &line);
        prop_assert!(result.contains(&line), "Inserted line '{}' should appear in result", line);
    }

    /// Property: Inserting at index 0 makes it the first line
    #[test]
    fn prop_insert_at_zero_is_first(
        content in ".*",
        line in "[A-Z]+",
    ) {
        let result = insert_line_at(&content, 0, &line);
        let first_line = result.lines().next().unwrap_or("");
        prop_assert_eq!(first_line, line, "Inserted line should be first");
    }

    /// Property: Inserting beyond end appends at the end
    #[test]
    fn prop_insert_beyond_end_appends(
        content in prop::collection::vec("[a-z]+", 1..10).prop_map(|v| v.join("\n")),
        line in "[A-Z]+",
    ) {
        let _line_count = content.lines().count();
        let result = insert_line_at(&content, 1000, &line);
        let last_line = result.lines().last().unwrap_or("");
        prop_assert_eq!(last_line, line, "Inserted line should be last when inserted beyond end");
    }

    /// Property: Empty content + insert = single line
    #[test]
    fn prop_insert_empty_content(_any in any::<u32>(), line in ".*") {
        let result = insert_line_at("", 0, &line);
        prop_assert_eq!(result, line);
    }
}

// ============================================================================
// Property Tests: remove_lines_matching
// ============================================================================

proptest! {
    /// Property: Removal count equals actual lines removed
    #[test]
    fn prop_remove_count_accurate(content in ".*") {
        let original_count = if content.is_empty() { 0 } else { content.lines().count() };
        let (result, removed_count) = remove_lines_matching(&content, |_| true);

        if content.is_empty() {
            prop_assert_eq!(removed_count, 0);
            prop_assert_eq!(result, "");
        } else {
            prop_assert_eq!(removed_count, original_count);
            prop_assert_eq!(result.trim(), "");
        }
    }

    /// Property: Removing nothing returns original content
    #[test]
    fn prop_remove_nothing_unchanged(content in ".*") {
        let (result, count) = remove_lines_matching(&content, |_| false);
        prop_assert_eq!(result, content);
        prop_assert_eq!(count, 0);
    }

    /// Property: Count + remaining lines = original lines
    #[test]
    fn prop_remove_count_plus_remaining(
        content in ".*",
        remove_char in prop::sample::select(vec!['a', 'b', 'c']),
    ) {
        let original_count = if content.is_empty() { 0 } else { content.lines().count() };
        let (result, removed_count) = remove_lines_matching(&content, |line| line.contains(remove_char));
        let remaining_count = if result.is_empty() { 0 } else { result.lines().count() };

        prop_assert_eq!(
            removed_count + remaining_count,
            original_count,
            "Removed ({}) + remaining ({}) should equal original ({})",
            removed_count,
            remaining_count,
            original_count
        );
    }

    /// Property: Removed lines don't appear in result
    #[test]
    fn prop_remove_lines_dont_appear(
        lines in prop::collection::vec("[a-z]+", 1..20),
        marker in "[A-Z]",
    ) {
        // Create content with some lines containing marker
        let content_lines: Vec<String> = lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                if i % 2 == 0 {
                    format!("{}{}", marker, line)
                } else {
                    line.to_string()
                }
            })
            .collect();
        let content = content_lines.join("\n");

        let (result, _) = remove_lines_matching(&content, |line| line.starts_with(&marker));

        for line in result.lines() {
            prop_assert!(!line.starts_with(&marker), "Removed lines should not appear in result");
        }
    }

    /// Property: Line ending style preserved for remaining lines
    #[test]
    fn prop_remove_preserves_line_endings(
        lines in prop::collection::vec("[a-z]+", 2..10),
    ) {
        let crlf_content = lines.join("\r\n");
        let (result, _) = remove_lines_matching(&crlf_content, |line| line.len() > 5);

        if !result.is_empty() && crlf_content.contains("\r\n") {
            // If original had CRLF and result is not empty, should preserve CRLF
            // (Note: single line won't have line ending)
            if result.lines().count() > 1 {
                prop_assert!(result.contains("\r\n"), "Should preserve CRLF");
            }
        }
    }
}

// ============================================================================
// Property Tests: replace_in_lines
// ============================================================================

proptest! {
    /// Property: Replacement count equals actual replacements
    #[test]
    fn prop_replace_count_accurate(
        content in ".*",
        pattern in "[a-z]{2,5}",
        replacement in "[A-Z]{2,5}",
    ) {
        let expected_count = content.matches(&pattern).count();
        let (result, actual_count) = replace_in_lines(&content, &pattern, &replacement);

        prop_assert_eq!(
            actual_count,
            expected_count,
            "Count should match actual occurrences: expected {}, got {}",
            expected_count,
            actual_count
        );

        // Verify pattern no longer appears in result
        if expected_count > 0 {
            prop_assert!(!result.contains(&pattern), "Pattern should not appear in result");
        }
    }

    /// Property: Replacement doesn't change line count
    #[test]
    fn prop_replace_preserves_line_count(
        content in ".*",
        old in "[a-z]+",
        new in "[A-Z]+",
    ) {
        let original_count = if content.is_empty() { 0 } else { content.lines().count() };
        let (result, _) = replace_in_lines(&content, &old, &new);
        let new_count = if result.is_empty() { 0 } else { result.lines().count() };

        prop_assert_eq!(
            new_count,
            original_count,
            "Line count should not change: {} -> {}",
            original_count,
            new_count
        );
    }

    /// Property: No matches means unchanged content
    #[test]
    fn prop_replace_no_match_unchanged(
        content in ".*",
        old in "[0-9]+",
    ) {
        // Use content that won't contain digits
        let alpha_content: String = content.chars().filter(|c| c.is_alphabetic() || c.is_whitespace()).collect();
        let (result, count) = replace_in_lines(&alpha_content, &old, "REPLACED");

        prop_assert_eq!(count, 0);
        prop_assert_eq!(result, alpha_content);
    }

    /// Property: Replacing empty string or with itself returns original
    #[test]
    fn prop_replace_identity(content in ".*", pattern in "[a-z]+") {
        // Replace pattern with itself
        let (result, count) = replace_in_lines(&content, &pattern, &pattern);
        let expected_count = content.matches(&pattern).count();

        prop_assert_eq!(result, content, "Replacing with same value should not change content");
        prop_assert_eq!(count, expected_count, "Count should still be accurate");
    }
}

// ============================================================================
// Integration Property Tests (combining multiple functions)
// ============================================================================

proptest! {
    /// Property: Insert then remove should restore original (if predicate matches inserted line)
    #[test]
    fn prop_insert_remove_roundtrip(
        content in prop::collection::vec("[a-z]+", 1..10).prop_map(|v| v.join("\n")),
        idx in 0usize..10,
        marker in "[A-Z]{5}",
    ) {
        // Insert line with unique marker
        let after_insert = insert_line_at(&content, idx, &marker);

        // Remove lines with marker
        let (after_remove, count) = remove_lines_matching(&after_insert, |line| line == marker);

        prop_assert_eq!(count, 1, "Should remove exactly 1 line");
        prop_assert_eq!(after_remove, content, "Should restore original content");
    }

    /// Property: Find + Insert should place line at correct position
    #[test]
    fn prop_find_insert_combination(
        import_lines in prop::collection::vec("import [a-z]+", 1..5),
        code_lines in prop::collection::vec("code [a-z]+", 1..5),
    ) {
        // Build content with imports followed by code
        let mut all_lines = import_lines.clone();
        all_lines.extend(code_lines);
        let content = all_lines.join("\n");

        // Find last import
        let last_import = find_last_matching_line(&content, |line| line.starts_with("import"));

        if let Some(idx) = last_import {
            let new_import = "import NEWMODULE";
            let result = insert_line_at(&content, idx + 1, new_import);

            // Verify new import appears after the last original import
            let result_lines: Vec<&str> = result.lines().collect();
            prop_assert!(result_lines[idx + 1] == new_import, "New import should be at correct position");
        }
    }

    /// Property: Replace then replace back should restore original
    #[test]
    fn prop_replace_roundtrip(
        content in prop::collection::vec("old_[a-z]+", 1..10).prop_map(|v| v.join("\n")),
    ) {
        let (after_replace, count1) = replace_in_lines(&content, "old_", "new_");
        let (back_to_original, count2) = replace_in_lines(&after_replace, "new_", "old_");

        prop_assert_eq!(count1, count2, "Replacement counts should match");
        prop_assert_eq!(back_to_original, content, "Should restore original content");
    }
}

// ============================================================================
// Edge Case Properties
// ============================================================================

proptest! {
    /// Property: Unicode handling works correctly
    #[test]
    fn prop_unicode_safe(
        lines in prop::collection::vec("[\\p{L}]{3,10}", 1..10),
        marker in "ABC+",
    ) {
        let content = lines.join("\n");

        // Test insert with unicode
        let result = insert_line_at(&content, 0, &marker);
        prop_assert!(result.contains(&marker));

        // Test find with unicode
        let idx = find_last_matching_line(&result, |line| line.contains(&marker));
        prop_assert_eq!(idx, Some(0));

        // Test remove with unicode
        let (_removed, count) = remove_lines_matching(&result, |line| line.contains(&marker));
        prop_assert_eq!(count, 1);

        // Test replace with unicode
        let (_replaced, count) = replace_in_lines(&content, &lines[0], &marker);
        if !lines[0].is_empty() {
            prop_assert!(count > 0);
        }
    }

    /// Property: Very long lines don't break anything
    #[test]
    fn prop_long_lines_safe(
        line_length in 1000usize..10000,
        num_lines in 1usize..10,
    ) {
        let long_line = "a".repeat(line_length);
        let lines = vec![long_line.clone(); num_lines];
        let content = lines.join("\n");

        // All operations should handle long lines
        let idx = find_last_matching_line(&content, |line| line.len() > 500);
        prop_assert!(idx.is_some());

        let result = insert_line_at(&content, 0, "SHORT");
        prop_assert!(result.contains("SHORT"));

        let (_result, count) = remove_lines_matching(&content, |line| line.len() > 500);
        prop_assert_eq!(count, num_lines);

        let (_result, count) = replace_in_lines(&content, "a", "b");
        prop_assert_eq!(count, line_length * num_lines);
    }
}
