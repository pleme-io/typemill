//! String Literal Path Rewriting Support
//!
//! This module provides functionality to detect and rewrite path-like string literals
//! in Rust source code during rename operations. This extends coverage for file/directory
//! renames by catching hardcoded paths that aren't part of the import system.

use regex::Regex;
use std::path::Path;

/// Check if a string literal looks like a path that should be updated
///
/// Conservative heuristic: only match strings that clearly look like file paths:
/// - Must contain a slash (/) indicating a path separator, OR
/// - Must contain a period AND end with a known file extension
///
/// This avoids false positives on prose text that happens to mention directory names.
fn is_path_like(s: &str) -> bool {
    // Must contain a slash OR have a file extension
    s.contains('/')
        || (s.contains('.') && {
            s.ends_with(".rs")
                || s.ends_with(".toml")
                || s.ends_with(".md")
                || s.ends_with(".yaml")
                || s.ends_with(".yml")
                || s.ends_with(".json")
                || s.ends_with(".txt")
                || s.ends_with(".conf")
                || s.ends_with(".config")
        })
}

/// Rewrite string literals in Rust source code that match path patterns
///
/// Uses regex to find string literals (to handle them in macros where AST parsing fails).
///
/// # Arguments
/// * `source` - The Rust source code to process
/// * `old_path` - The old path to search for in string literals
/// * `new_path` - The new path to replace with
///
/// # Returns
/// A tuple of (modified_source, change_count) where change_count is the number
/// of string literals that were updated.
///
/// # Example
/// ```ignore
/// let source = r#"let path = "integration-tests/fixtures/test.rs";"#;
/// let old_path = Path::new("integration-tests");
/// let new_path = Path::new("tests");
/// let (result, count) = rewrite_string_literals(source, old_path, new_path)?;
/// assert_eq!(count, 1);
/// assert!(result.contains("\"tests/fixtures/test.rs\""));
/// ```
pub fn rewrite_string_literals(
    source: &str,
    old_path: &Path,
    new_path: &Path,
) -> Result<(String, usize), Box<dyn std::error::Error>> {
    // Regex to match string literals (handles simple strings, not raw strings for now)
    // Matches: "..." but not r"..." or r#"..."#
    let string_literal_regex = Regex::new(r#""([^"\\]*(\\.[^"\\]*)*)""#)?;

    let old_path_str = old_path.to_string_lossy();
    let new_path_str = new_path.to_string_lossy();

    let mut change_count = 0;

    // Replace all string literals that contain the old path
    let result = string_literal_regex.replace_all(source, |caps: &regex::Captures| {
        let full_match = &caps[0]; // The full "..." string with quotes
        let content = &caps[1]; // The content inside quotes

        // Check if this looks like a path and contains the old path
        if is_path_like(content) && content.contains(old_path_str.as_ref()) {
            let new_content = content.replace(old_path_str.as_ref(), new_path_str.as_ref());
            change_count += 1;
            format!("\"{}\"", new_content)
        } else {
            full_match.to_string()
        }
    });

    Ok((result.to_string(), change_count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_updates_path_strings_with_slashes() {
        let source = r#"
fn main() {
    let path = "integration-tests/fixtures/test.rs";
    std::fs::read("integration-tests/test.toml").unwrap();
}
"#;
        let old_path = Path::new("integration-tests");
        let new_path = Path::new("tests");

        let (result, count) = rewrite_string_literals(source, old_path, new_path).unwrap();

        assert_eq!(count, 2);
        assert!(result.contains("\"tests/fixtures/test.rs\""));
        assert!(result.contains("\"tests/test.toml\""));
    }

    #[test]
    fn test_skips_prose_without_slashes() {
        let source = r#"
fn main() {
    let msg = "We use integration-tests as a pattern";
}
"#;
        let old_path = Path::new("integration-tests");
        let new_path = Path::new("tests");

        let (result, count) = rewrite_string_literals(source, old_path, new_path).unwrap();

        assert_eq!(count, 0);
        assert!(result.contains("\"We use integration-tests as a pattern\""));
    }

    #[test]
    fn test_updates_file_extension_paths() {
        let source = r#"
fn main() {
    let config = "config.toml";
    let readme = "README.md";
}
"#;
        let old_path = Path::new("config.toml");
        let new_path = Path::new("settings.toml");

        let (result, count) = rewrite_string_literals(source, old_path, new_path).unwrap();

        assert_eq!(count, 1);
        assert!(result.contains("\"settings.toml\""));
        assert!(result.contains("\"README.md\""));
    }

    #[test]
    fn test_handles_nested_paths() {
        let source = r#"
fn main() {
    let deep = "src/handlers/tools/navigation.rs";
    let shallow = "src/lib.rs";
}
"#;
        let old_path = Path::new("src/handlers");
        let new_path = Path::new("src/components");

        let (result, count) = rewrite_string_literals(source, old_path, new_path).unwrap();

        assert_eq!(count, 1);
        assert!(result.contains("\"src/components/tools/navigation.rs\""));
        assert!(result.contains("\"src/lib.rs\""));
    }

    #[test]
    fn test_preserves_non_path_strings() {
        let source = r#"
fn main() {
    let url = "https://example.com/path";  // URLs should be left alone
    let error = "File not found: integration-tests";  // Prose
    let name = "test_integration";  // No slash or extension
}
"#;
        let old_path = Path::new("integration-tests");
        let new_path = Path::new("tests");

        let (result, count) = rewrite_string_literals(source, old_path, new_path).unwrap();

        // Should not change any of these
        assert_eq!(count, 0);
        assert!(result.contains("\"https://example.com/path\""));
        assert!(result.contains("\"File not found: integration-tests\""));
        assert!(result.contains("\"test_integration\""));
    }

    #[test]
    fn test_multiple_occurrences_in_same_file() {
        let source = r#"
fn test_paths() {
    assert_eq!(read("old/path/file.rs"), expected);
    assert_eq!(read("old/path/other.rs"), expected);
    let third = "old/path/third.rs";
}
"#;
        let old_path = Path::new("old/path");
        let new_path = Path::new("new/path");

        let (result, count) = rewrite_string_literals(source, old_path, new_path).unwrap();

        assert_eq!(count, 3);
        assert!(result.contains("\"new/path/file.rs\""));
        assert!(result.contains("\"new/path/other.rs\""));
        assert!(result.contains("\"new/path/third.rs\""));
        assert!(!result.contains("\"old/path"));
    }

    #[test]
    fn test_is_path_like_heuristic() {
        // Should be considered paths
        assert!(is_path_like("integration-tests/fixtures/test.rs"));
        assert!(is_path_like("config.toml"));
        assert!(is_path_like("README.md"));
        assert!(is_path_like("src/lib.rs"));
        assert!(is_path_like("../relative/path.txt"));
        assert!(is_path_like("./current/dir.yml"));

        // Should NOT be considered paths
        assert!(!is_path_like("integration-tests"));
        assert!(!is_path_like("some prose text"));
        assert!(!is_path_like("version 1.0.0"));
        assert!(!is_path_like("test_function_name"));
        assert!(!is_path_like("error: file not found"));
    }
}
