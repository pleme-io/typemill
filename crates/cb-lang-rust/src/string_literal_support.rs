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
/// - Must contain a slash (/) or backslash (\) indicating a path separator, OR
/// - Must contain a period AND end with a known file extension
///
/// This avoids false positives on prose text that happens to mention directory names.
fn is_path_like(s: &str) -> bool {
    // Must contain a slash/backslash OR have a file extension
    s.contains('/') || s.contains('\\')
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
/// Uses regex to find both regular and raw string literals (to handle them in macros where AST parsing fails).
/// Supports raw strings: r"...", r#"..."#, r##"..."##, etc.
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
    let old_path_str = old_path.to_string_lossy();
    let new_path_str = new_path.to_string_lossy();

    // Extract just the filename/dirname for relative matching
    let old_name = old_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let mut modified_source = source.to_string();
    let mut change_count = 0;

    // Pattern 1: Regular strings "..." (but not raw strings r"...")
    // Use negative lookbehind alternative: check preceding character is not 'r' or '#'
    let string_regex = Regex::new(r#""([^"\\]*(\\.[^"\\]*)*)""#)?;
    for cap in string_regex.captures_iter(source) {
        let full_match = cap.get(0).unwrap().as_str();
        let match_start = source.find(full_match).unwrap();

        // Skip if this is part of a raw string (preceded by 'r' or '#')
        if match_start > 0 {
            let prev_char = source.chars().nth(match_start - 1);
            if let Some(ch) = prev_char {
                if ch == 'r' || ch == '#' {
                    continue; // This is a raw string, skip it
                }
            }
        }

        let string_content = cap.get(1).unwrap().as_str();

        if is_path_like(string_content) {
            // Try to match against multiple forms:
            // 1. Absolute path: /workspace/config
            // 2. Relative path starting with name: config/settings.toml
            // But NOT nested paths like: src/config/file.rs (unless it's part of absolute path)
            let matches = string_content.contains(old_path_str.as_ref())
                || (!old_name.is_empty() && (
                    string_content == old_name  // Exact match
                    || string_content.starts_with(&format!("{}/", old_name))  // Starts with dir/
                ));

            if matches {
                // Replace both absolute and relative forms
                let new_content = if string_content.contains(old_path_str.as_ref()) {
                    string_content.replace(old_path_str.as_ref(), new_path_str.as_ref())
                } else if !old_name.is_empty() {
                    // Extract new name for relative replacement
                    let new_name = new_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(new_path_str.as_ref());
                    string_content.replace(old_name, new_name)
                } else {
                    string_content.to_string()
                };

                let new_match = format!("\"{}\"", new_content);
                modified_source = modified_source.replace(full_match, &new_match);
                change_count += 1;
            }
        }
    }

    // Pattern 2: Raw strings r"...", r#"..."#, r##"..."##, etc.
    // We need separate regexes since the regex crate doesn't support backreferences
    // Cover common cases: r"...", r#"..."#, r##"..."##, r###"..."###, r####"..."####, r#####"..."#####
    // Note: For hashed raw strings (r#"..."#), the content can contain quotes
    let raw_patterns = vec![
        (Regex::new(r#"r"([^"]*)""#)?, 0),                        // r"..." (no quotes inside)
        (Regex::new(r##"r#"(.*?)"#"##)?, 1),                      // r#"..."# (can contain quotes)
        (Regex::new(r###"r##"(.*?)"##"###)?, 2),                  // r##"..."##
        (Regex::new(r####"r###"(.*?)"###"####)?, 3),              // r###"..."###
        (Regex::new(r#####"r####"(.*?)"####"#####)?, 4),          // r####"..."####
        (Regex::new(r######"r#####"(.*?)"#####"######)?, 5),      // r#####"..."#####
    ];

    for (raw_regex, hash_count) in raw_patterns {
        let hash_marks = "#".repeat(hash_count);
        for cap in raw_regex.captures_iter(source) {
            let full_match = cap.get(0).unwrap().as_str();
            let string_content = cap.get(1).unwrap().as_str();

            if is_path_like(string_content) {
                // Same matching logic as regular strings
                let matches = string_content.contains(old_path_str.as_ref())
                    || (!old_name.is_empty() && (
                        string_content == old_name  // Exact match
                        || string_content.starts_with(&format!("{}/", old_name))  // Starts with dir/
                    ));

                if matches {
                    // Replace both absolute and relative forms
                    let new_content = if string_content.contains(old_path_str.as_ref()) {
                        string_content.replace(old_path_str.as_ref(), new_path_str.as_ref())
                    } else if !old_name.is_empty() {
                        let new_name = new_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(new_path_str.as_ref());
                        string_content.replace(old_name, new_name)
                    } else {
                        string_content.to_string()
                    };

                    let new_match = format!("r{}\"{}\"{}",hash_marks, new_content, hash_marks);
                    modified_source = modified_source.replace(full_match, &new_match);
                    change_count += 1;
                }
            }
        }
    }

    Ok((modified_source, change_count))
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
        assert!(is_path_like(r"C:\Users\path\file.rs")); // Windows backslashes

        // Should NOT be considered paths
        assert!(!is_path_like("integration-tests"));
        assert!(!is_path_like("some prose text"));
        assert!(!is_path_like("version 1.0.0"));
        assert!(!is_path_like("test_function_name"));
        assert!(!is_path_like("error: file not found"));
    }

    #[test]
    fn test_raw_string_simple() {
        let source = r#"
fn main() {
    let path = r"old-dir\file.rs";
}
"#;
        let (result, count) = rewrite_string_literals(
            source,
            Path::new("old-dir"),
            Path::new("new-dir")
        ).unwrap();

        assert_eq!(count, 1);
        assert!(result.contains(r#"r"new-dir\file.rs""#));
    }

    #[test]
    fn test_raw_string_with_hashes() {
        let source = r##"
fn main() {
    let path = r#"old-dir/file"with"quotes.rs"#;
}
"##;
        let (result, count) = rewrite_string_literals(
            source,
            Path::new("old-dir"),
            Path::new("new-dir")
        ).unwrap();

        assert_eq!(count, 1);
        assert!(result.contains(r##"r#"new-dir/file"with"quotes.rs"#"##));
    }

    #[test]
    fn test_windows_paths_in_raw_strings() {
        let source = r#"
fn main() {
    let path = r"C:\Users\integration-tests\file.rs";
}
"#;
        let (result, count) = rewrite_string_literals(
            source,
            Path::new("integration-tests"),
            Path::new("tests")
        ).unwrap();

        assert_eq!(count, 1);
        assert!(result.contains(r"C:\Users\tests\file.rs"));
    }

    #[test]
    fn test_mixed_regular_and_raw_strings() {
        let source = r#"
fn main() {
    let regular = "old-dir/file1.rs";
    let raw = r"old-dir\file2.rs";
}
"#;
        let (result, count) = rewrite_string_literals(
            source,
            Path::new("old-dir"),
            Path::new("new-dir")
        ).unwrap();

        assert_eq!(count, 2);
        assert!(result.contains("\"new-dir/file1.rs\""));
        assert!(result.contains(r#"r"new-dir\file2.rs""#));
    }

    #[test]
    fn test_raw_string_with_multiple_hashes() {
        let source = r###"
fn main() {
    let path = r##"old-dir/file##with##hashes.rs"##;
}
"###;
        let (result, count) = rewrite_string_literals(
            source,
            Path::new("old-dir"),
            Path::new("new-dir")
        ).unwrap();

        assert_eq!(count, 1);
        assert!(result.contains(r###"r##"new-dir/file##with##hashes.rs"##"###));
    }

    #[test]
    fn test_relative_path_with_absolute_old_path() {
        // This tests the bug Codex identified: relative strings like "config/settings.toml"
        // should match when old_path is absolute like "/workspace/config"
        let source = r#"
fn main() {
    let path = "config/settings.toml";
    std::fs::read("config/data.json").unwrap();
}
"#;
        let old_path = Path::new("/workspace/config");
        let new_path = Path::new("/workspace/configuration");

        let (result, count) = rewrite_string_literals(source, old_path, new_path).unwrap();

        assert_eq!(count, 2, "Should update both relative paths");
        assert!(result.contains("\"configuration/settings.toml\""));
        assert!(result.contains("\"configuration/data.json\""));
    }

    #[test]
    fn test_relative_path_starting_with_directory_name() {
        let source = r#"
fn main() {
    let path = "integration-tests/fixtures/test.rs";
    let other = "src/integration-tests/file.rs";
}
"#;
        // Absolute path for directory
        let old_path = Path::new("/workspace/integration-tests");
        let new_path = Path::new("/workspace/tests");

        let (result, count) = rewrite_string_literals(source, old_path, new_path).unwrap();

        assert_eq!(count, 1, "Should only update path starting with directory name");
        assert!(result.contains("\"tests/fixtures/test.rs\""));
        assert!(result.contains("\"src/integration-tests/file.rs\""), "Should not update nested occurrence");
    }
}
