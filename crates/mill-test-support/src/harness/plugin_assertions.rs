//! Custom Assertions for Plugin Testing
//!
//! Provides specialized assertions for language plugin tests that make
//! test intent clearer and failure messages more helpful.

use std::time::Duration;

/// Assert that a plugin operation completed within a performance threshold
///
/// # Arguments
///
/// * `duration` - The actual duration of the operation
/// * `max_seconds` - Maximum allowed duration in seconds
///
/// # Panics
///
/// If `duration` exceeds `max_seconds`
///
/// # Example
///
/// ```no_run
/// use mill_plugin_test_utils::assert_performance;
/// use std::time::Instant;
///
/// let start = Instant::now();
/// // ... do some work ...
/// assert_performance(start.elapsed(), 5);
/// ```
pub fn assert_performance(duration: Duration, max_seconds: u64) {
    assert!(
        duration.as_secs() < max_seconds,
        "Performance test exceeded {} seconds: {:?}",
        max_seconds,
        duration
    );
}

/// Assert that a plugin operation completed within a performance threshold (milliseconds)
///
/// Useful for more precise performance assertions.
///
/// # Arguments
///
/// * `duration` - The actual duration of the operation
/// * `max_millis` - Maximum allowed duration in milliseconds
///
/// # Panics
///
/// If `duration` exceeds `max_millis`
pub fn assert_performance_millis(duration: Duration, max_millis: u128) {
    assert!(
        duration.as_millis() < max_millis,
        "Performance test exceeded {} milliseconds: {:?}",
        max_millis,
        duration
    );
}

/// Assert that a plugin extracted the expected number of symbols
///
/// # Arguments
///
/// * `actual` - Number of symbols found
/// * `expected` - Number of symbols expected
/// * `context` - Contextual information for the assertion (e.g., file name)
///
/// # Panics
///
/// If symbol count doesn't match
///
/// # Example
///
/// ```no_run
/// use mill_plugin_test_utils::assert_symbol_count;
///
/// let symbols = vec!["foo", "bar", "baz"];
/// assert_symbol_count(symbols.len(), 3, "my_module.rs");
/// ```
pub fn assert_symbol_count(actual: usize, expected: usize, context: &str) {
    assert_eq!(
        actual, expected,
        "{}: expected {} symbols, found {}",
        context, expected, actual
    );
}

/// Assert that a reference was found at the expected location
///
/// # Arguments
///
/// * `refs` - Vector of references (as tuples of module and line number)
/// * `module` - Expected module name
/// * `line` - Expected line number (1-indexed)
///
/// # Panics
///
/// If reference is not found at expected location
///
/// # Example
///
/// ```no_run
/// use mill_plugin_test_utils::assert_reference_at_line;
///
/// let refs = vec![("fmt", 3), ("io", 5)];
/// assert_reference_at_line(&refs, "fmt", 3);
/// ```
pub fn assert_reference_at_line(refs: &[(&str, usize)], module: &str, line: usize) {
    let found = refs.iter().any(|(m, l)| *m == module && *l == line);
    assert!(
        found,
        "Reference to '{}' not found at line {}",
        module, line
    );
}

/// Assert that no references to a module exist in the result
///
/// # Arguments
///
/// * `refs` - Vector of references (as module names)
/// * `module` - Module name that should not be found
/// * `context` - Contextual information for the assertion
///
/// # Panics
///
/// If reference to the module is found
pub fn assert_no_references_to(refs: &[&str], module: &str, context: &str) {
    let found = refs.iter().any(|r| *r == module);
    assert!(
        !found,
        "{}: Reference to '{}' should not exist",
        context, module
    );
}

/// Assert that file content contains expected patterns
///
/// # Arguments
///
/// * `content` - File content to check
/// * `patterns` - Patterns that should all be present
/// * `context` - Contextual information for the assertion
///
/// # Panics
///
/// If any pattern is not found
pub fn assert_contains_all(content: &str, patterns: &[&str], context: &str) {
    for pattern in patterns {
        assert!(
            content.contains(pattern),
            "{}: Expected pattern '{}' not found in content",
            context, pattern
        );
    }
}

/// Assert that file content does not contain any of the patterns
///
/// # Arguments
///
/// * `content` - File content to check
/// * `patterns` - Patterns that should all be absent
/// * `context` - Contextual information for the assertion
///
/// # Panics
///
/// If any pattern is found
pub fn assert_contains_none(content: &str, patterns: &[&str], context: &str) {
    for pattern in patterns {
        assert!(
            !content.contains(pattern),
            "{}: Unexpected pattern '{}' found in content",
            context, pattern
        );
    }
}

/// Assert that exactly one of the patterns exists
///
/// # Arguments
///
/// * `content` - File content to check
/// * `patterns` - Patterns, exactly one should be present
/// * `context` - Contextual information for the assertion
///
/// # Panics
///
/// If zero or more than one pattern is found
pub fn assert_contains_one_of(content: &str, patterns: &[&str], context: &str) {
    let matches = patterns.iter().filter(|p| content.contains(*p)).count();
    assert_eq!(
        matches, 1,
        "{}: Expected exactly one of patterns {:?}, found {}",
        context, patterns, matches
    );
}

/// Assert that a refactoring operation produced the expected number of edits
///
/// # Arguments
///
/// * `edit_count` - Number of edits produced
/// * `expected_count` - Expected number of edits
/// * `operation_name` - Name of the refactoring operation
///
/// # Panics
///
/// If edit count doesn't match
///
/// # Example
///
/// ```no_run
/// use mill_plugin_test_utils::assert_edit_count;
///
/// // After getting a refactoring plan from a plugin:
/// let edit_count = 3;  // Number of edits in the plan
/// assert_edit_count(edit_count, 3, "extract_function");
/// ```
pub fn assert_edit_count(edit_count: usize, expected_count: usize, operation_name: &str) {
    assert_eq!(
        edit_count, expected_count,
        "{}: expected {} edits, got {}",
        operation_name, expected_count, edit_count
    );
}

/// Assert that edits touch the expected files
///
/// # Arguments
///
/// * `file_paths` - File paths that were edited
/// * `expected_files` - Expected file paths
/// * `context` - Contextual information
///
/// # Panics
///
/// If edited files don't match expected
pub fn assert_files_edited(file_paths: &[&str], expected_files: &[&str], context: &str) {
    assert_eq!(
        file_paths.len(),
        expected_files.len(),
        "{}: expected {} files to be edited, got {}",
        context,
        expected_files.len(),
        file_paths.len()
    );

    for expected in expected_files {
        let found = file_paths.iter().any(|f| *f == *expected);
        assert!(
            found,
            "{}: Expected file '{}' not found in edits",
            context, expected
        );
    }
}

/// Assert that manifest dependency count matches expectation
///
/// # Arguments
///
/// * `actual_count` - Actual number of dependencies
/// * `expected_count` - Expected number of dependencies
/// * `manifest_type` - Type of manifest (e.g., "Cargo.toml")
///
/// # Panics
///
/// If counts don't match
pub fn assert_dependency_count(
    actual_count: usize,
    expected_count: usize,
    manifest_type: &str,
) {
    assert_eq!(
        actual_count, expected_count,
        "{}: expected {} dependencies, found {}",
        manifest_type, expected_count, actual_count
    );
}

/// Assert that a symbol has expected metadata
///
/// # Arguments
///
/// * `symbol_name` - Name of the symbol
/// * `symbols` - Vector of symbol names found
/// * `context` - Context information
///
/// # Panics
///
/// If symbol is not found
pub fn assert_symbol_exists(symbol_name: &str, symbols: &[&str], context: &str) {
    let found = symbols.iter().any(|s| *s == symbol_name);
    assert!(
        found,
        "{}: Symbol '{}' not found",
        context, symbol_name
    );
}

/// Assert that circular dependency detection works
///
/// # Arguments
///
/// * `circular_deps_found` - Whether circular dependency was detected
/// * `should_have_circular` - Whether we expect a circular dependency
/// * `context` - Context information
///
/// # Panics
///
/// If expectation doesn't match
pub fn assert_circular_dependency_detection(
    circular_deps_found: bool,
    should_have_circular: bool,
    context: &str,
) {
    assert_eq!(
        circular_deps_found, should_have_circular,
        "{}: circular dependency detection mismatch",
        context
    );
}

/// Assert that imports were correctly updated
///
/// # Arguments
///
/// * `old_import` - Old import statement/pattern
/// * `new_import` - New import statement/pattern
/// * `content` - File content after update
/// * `context` - Context information
///
/// # Panics
///
/// If content doesn't reflect the expected changes
pub fn assert_import_updated(
    old_import: &str,
    new_import: &str,
    content: &str,
    context: &str,
) {
    assert!(
        !content.contains(old_import),
        "{}: Old import '{}' still exists in content",
        context, old_import
    );
    assert!(
        content.contains(new_import),
        "{}: New import '{}' not found in content",
        context, new_import
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_performance_passes() {
        assert_performance(Duration::from_secs(2), 5);
    }

    #[test]
    #[should_panic]
    fn test_assert_performance_fails() {
        assert_performance(Duration::from_secs(10), 5);
    }

    #[test]
    fn test_assert_performance_millis() {
        assert_performance_millis(Duration::from_millis(100), 500);
    }

    #[test]
    fn test_assert_symbol_count() {
        assert_symbol_count(3, 3, "test.rs");
    }

    #[test]
    #[should_panic]
    fn test_assert_symbol_count_fails() {
        assert_symbol_count(2, 3, "test.rs");
    }

    #[test]
    fn test_assert_contains_all() {
        let content = "use foo; use bar; use baz;";
        assert_contains_all(content, &["foo", "bar"], "test");
    }

    #[test]
    #[should_panic]
    fn test_assert_contains_all_fails() {
        let content = "use foo;";
        assert_contains_all(content, &["foo", "bar"], "test");
    }

    #[test]
    fn test_assert_contains_none() {
        let content = "use foo;";
        assert_contains_none(content, &["bar", "baz"], "test");
    }

    #[test]
    #[should_panic]
    fn test_assert_contains_none_fails() {
        let content = "use foo; use bar;";
        assert_contains_none(content, &["bar", "baz"], "test");
    }

    #[test]
    fn test_assert_edit_count() {
        assert_edit_count(3, 3, "extract_function");
    }

    #[test]
    fn test_assert_symbol_exists() {
        let symbols = vec!["foo", "bar", "baz"];
        assert_symbol_exists("bar", &symbols, "test");
    }

    #[test]
    #[should_panic]
    fn test_assert_symbol_exists_fails() {
        let symbols = vec!["foo", "bar"];
        assert_symbol_exists("baz", &symbols, "test");
    }

    #[test]
    fn test_assert_import_updated() {
        let content = "use new_module; use bar;";
        assert_import_updated("use old_module", "use new_module", content, "test");
    }
}
