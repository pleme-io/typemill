//! Convention parsers for CLI flag support
//!
//! This module provides smart parsers that understand special flag conventions:
//! - Target convention: "kind:path" or "kind:path:line:char"
//! - Source convention: "path:line:char"
//! - Destination convention: "path" or "path:line:char"
//! - Naming conventions: kebab-case, snake_case, camelCase, PascalCase

use serde_json::{json, Value};
use std::fmt;
use std::path::Path;

/// Errors that can occur when parsing convention strings
#[derive(Debug)]
pub enum ConventionError {
    /// The input format doesn't match expected convention
    InvalidFormat { input: String, expected: String },
    /// A numeric field couldn't be parsed
    InvalidNumber { field: String, value: String },
}

impl fmt::Display for ConventionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConventionError::InvalidFormat { input, expected } => {
                write!(
                    f,
                    "Invalid format: '{}'. Expected: {}",
                    input, expected
                )
            }
            ConventionError::InvalidNumber { field, value } => {
                write!(
                    f,
                    "Invalid number for field '{}': '{}' is not a valid integer",
                    field, value
                )
            }
        }
    }
}

impl std::error::Error for ConventionError {}

/// Parse target convention: "kind:path" or "kind:path:line:char"
///
/// # Examples
///
/// Simple target:
/// ```
/// use codebuddy::cli::conventions::parse_target_convention;
/// let result = parse_target_convention("directory:crates/mill-client").unwrap();
/// assert_eq!(result["kind"], "directory");
/// assert_eq!(result["path"], "crates/mill-client");
/// ```
///
/// Target with position (symbol):
/// ```
/// use codebuddy::cli::conventions::parse_target_convention;
/// let result = parse_target_convention("symbol:src/app.rs:10:5").unwrap();
/// assert_eq!(result["kind"], "symbol");
/// assert_eq!(result["path"], "src/app.rs");
/// assert_eq!(result["selector"]["position"]["line"], 10);
/// assert_eq!(result["selector"]["position"]["character"], 5);
/// ```
pub fn parse_target_convention(s: &str) -> Result<Value, ConventionError> {
    let parts: Vec<&str> = s.split(':').collect();

    match parts.as_slice() {
        [kind, path] => {
            // Simple: directory:path or file:path
            // Normalize "dir" to "directory"
            let normalized_kind = if *kind == "dir" { "directory" } else { *kind };
            Ok(json!({
                "kind": normalized_kind,
                "path": path
            }))
        }
        [kind, path, line, char] => {
            // Symbol with position
            let line_num: u32 = line.parse().map_err(|_| ConventionError::InvalidNumber {
                field: "line".to_string(),
                value: line.to_string(),
            })?;
            let char_num: u32 = char.parse().map_err(|_| ConventionError::InvalidNumber {
                field: "character".to_string(),
                value: char.to_string(),
            })?;

            Ok(json!({
                "kind": kind,
                "path": path,
                "selector": {
                    "position": {
                        "line": line_num,
                        "character": char_num
                    }
                }
            }))
        }
        _ => Err(ConventionError::InvalidFormat {
            input: s.to_string(),
            expected: "kind:path or kind:path:line:char".to_string(),
        }),
    }
}

/// Parse source convention: "path:line:char" or just "path"
///
/// # Examples
///
/// ```
/// use codebuddy::cli::conventions::parse_source_convention;
/// let result = parse_source_convention("src/app.rs:45:8").unwrap();
/// assert_eq!(result["file_path"], "src/app.rs");
/// assert_eq!(result["line"], 45);
/// assert_eq!(result["character"], 8);
/// ```
///
/// Just path (for operations that don't need a position):
/// ```
/// use codebuddy::cli::conventions::parse_source_convention;
/// let result = parse_source_convention("src/app.rs").unwrap();
/// assert_eq!(result["file_path"], "src/app.rs");
/// ```
pub fn parse_source_convention(s: &str) -> Result<Value, ConventionError> {
    let parts: Vec<&str> = s.split(':').collect();

    match parts.as_slice() {
        [path] => {
            // Just a file path, no position (used for reorder imports, etc.)
            Ok(json!({
                "file_path": path
            }))
        }
        [path, line, char] => {
            let line_num: u32 = line.parse().map_err(|_| ConventionError::InvalidNumber {
                field: "line".to_string(),
                value: line.to_string(),
            })?;
            let char_num: u32 = char.parse().map_err(|_| ConventionError::InvalidNumber {
                field: "character".to_string(),
                value: char.to_string(),
            })?;

            Ok(json!({
                "file_path": path,
                "line": line_num,
                "character": char_num
            }))
        }
        _ => Err(ConventionError::InvalidFormat {
            input: s.to_string(),
            expected: "path or path:line:char".to_string(),
        }),
    }
}

/// Parse destination convention: "path" or "path:line:char"
///
/// # Examples
///
/// Simple path:
/// ```
/// use codebuddy::cli::conventions::parse_destination_convention;
/// let result = parse_destination_convention("src/utils.rs").unwrap();
/// assert_eq!(result["file_path"], "src/utils.rs");
/// ```
///
/// Path with position:
/// ```
/// use codebuddy::cli::conventions::parse_destination_convention;
/// let result = parse_destination_convention("src/utils.rs:10:0").unwrap();
/// assert_eq!(result["file_path"], "src/utils.rs");
/// assert_eq!(result["line"], 10);
/// assert_eq!(result["character"], 0);
/// ```
pub fn parse_destination_convention(s: &str) -> Result<Value, ConventionError> {
    let parts: Vec<&str> = s.split(':').collect();

    match parts.as_slice() {
        [path] => {
            // Simple path only
            Ok(json!({
                "file_path": path
            }))
        }
        [path, line, char] => {
            // Path with position
            let line_num: u32 = line.parse().map_err(|_| ConventionError::InvalidNumber {
                field: "line".to_string(),
                value: line.to_string(),
            })?;
            let char_num: u32 = char.parse().map_err(|_| ConventionError::InvalidNumber {
                field: "character".to_string(),
                value: char.to_string(),
            })?;

            Ok(json!({
                "file_path": path,
                "line": line_num,
                "character": char_num
            }))
        }
        _ => Err(ConventionError::InvalidFormat {
            input: s.to_string(),
            expected: "path or path:line:char".to_string(),
        }),
    }
}

/// Convert a filename from one naming convention to another
///
/// # Examples
///
/// ```
/// use codebuddy::cli::conventions::convert_filename;
/// assert_eq!(convert_filename("user-profile.js", "kebab-case", "camelCase"), Some("userProfile.js".to_string()));
/// assert_eq!(convert_filename("user_name.rs", "snake_case", "camelCase"), Some("userName.rs".to_string()));
/// assert_eq!(convert_filename("UserData.ts", "PascalCase", "kebab-case"), Some("user-data.ts".to_string()));
/// ```
pub fn convert_filename(filename: &str, from: &str, to: &str) -> Option<String> {
    let path = Path::new(filename);
    let stem = path.file_stem()?.to_str()?;
    let extension = path.extension().and_then(|e| e.to_str());

    // Convert the stem (filename without extension)
    let converted_stem = convert_string(stem, from, to)?;

    // Rebuild filename with extension
    if let Some(ext) = extension {
        Some(format!("{}.{}", converted_stem, ext))
    } else {
        Some(converted_stem)
    }
}

/// Convert a string from one naming convention to another
fn convert_string(s: &str, from: &str, to: &str) -> Option<String> {
    // First, split into words based on the source convention
    let words = split_by_convention(s, from)?;

    // Then, join words using the target convention
    Some(join_by_convention(&words, to))
}

/// Split a string into words based on naming convention
fn split_by_convention(s: &str, convention: &str) -> Option<Vec<String>> {
    match convention {
        "kebab-case" => {
            Some(s.split('-').map(|w| w.to_lowercase()).collect())
        }
        "snake_case" => {
            Some(s.split('_').map(|w| w.to_lowercase()).collect())
        }
        "camelCase" => {
            // Split on capital letters: userName -> ["user", "Name"]
            let mut words = Vec::new();
            let mut current_word = String::new();

            for (i, ch) in s.chars().enumerate() {
                if ch.is_uppercase() && i > 0 {
                    if !current_word.is_empty() {
                        words.push(current_word.to_lowercase());
                    }
                    current_word = ch.to_string();
                } else {
                    current_word.push(ch);
                }
            }
            if !current_word.is_empty() {
                words.push(current_word.to_lowercase());
            }
            Some(words)
        }
        "PascalCase" => {
            // Split on capital letters: UserName -> ["User", "Name"]
            let mut words = Vec::new();
            let mut current_word = String::new();

            for ch in s.chars() {
                if ch.is_uppercase() && !current_word.is_empty() {
                    words.push(current_word.to_lowercase());
                    current_word = ch.to_string();
                } else {
                    current_word.push(ch);
                }
            }
            if !current_word.is_empty() {
                words.push(current_word.to_lowercase());
            }
            Some(words)
        }
        _ => None, // Unsupported convention
    }
}

/// Join words using a naming convention
fn join_by_convention(words: &[String], convention: &str) -> String {
    match convention {
        "kebab-case" => words.join("-"),
        "snake_case" => words.join("_"),
        "camelCase" => {
            if words.is_empty() {
                String::new()
            } else {
                let first = words[0].to_lowercase();
                let rest: String = words[1..]
                    .iter()
                    .map(|w| capitalize_first(w))
                    .collect();
                format!("{}{}", first, rest)
            }
        }
        "PascalCase" => {
            words.iter().map(|w| capitalize_first(w)).collect()
        }
        _ => words.join(""),
    }
}

/// Capitalize the first character of a string
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_target_directory() {
        let result = parse_target_convention("directory:crates/mill-client").unwrap();
        assert_eq!(result["kind"], "directory");
        assert_eq!(result["path"], "crates/mill-client");
        assert!(result.get("selector").is_none());
    }

    #[test]
    fn test_parse_target_file() {
        let result = parse_target_convention("file:src/app.rs").unwrap();
        assert_eq!(result["kind"], "file");
        assert_eq!(result["path"], "src/app.rs");
        assert!(result.get("selector").is_none());
    }

    #[test]
    fn test_parse_target_symbol() {
        let result = parse_target_convention("symbol:src/app.rs:10:5").unwrap();
        assert_eq!(result["kind"], "symbol");
        assert_eq!(result["path"], "src/app.rs");
        assert_eq!(result["selector"]["position"]["line"], 10);
        assert_eq!(result["selector"]["position"]["character"], 5);
    }

    #[test]
    fn test_parse_target_invalid_format() {
        let result = parse_target_convention("invalid");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConventionError::InvalidFormat { input, expected } => {
                assert_eq!(input, "invalid");
                assert!(expected.contains("kind:path"));
            }
            _ => panic!("Expected InvalidFormat error"),
        }
    }

    #[test]
    fn test_parse_target_invalid_number() {
        let result = parse_target_convention("symbol:src/app.rs:abc:5");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConventionError::InvalidNumber { field, value } => {
                assert_eq!(field, "line");
                assert_eq!(value, "abc");
            }
            _ => panic!("Expected InvalidNumber error"),
        }
    }

    #[test]
    fn test_parse_source_valid() {
        let result = parse_source_convention("src/app.rs:45:8").unwrap();
        assert_eq!(result["file_path"], "src/app.rs");
        assert_eq!(result["line"], 45);
        assert_eq!(result["character"], 8);
    }

    #[test]
    fn test_parse_source_just_path() {
        // Now supports just a path for operations like reorder imports
        let result = parse_source_convention("src/app.rs");
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["file_path"], "src/app.rs");
    }

    #[test]
    fn test_parse_source_invalid_format() {
        // Only path:line (missing character) is invalid
        let result = parse_source_convention("src/app.rs:45");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConventionError::InvalidFormat { input, expected } => {
                assert_eq!(input, "src/app.rs:45");
                assert_eq!(expected, "path or path:line:char");
            }
            _ => panic!("Expected InvalidFormat error"),
        }
    }

    #[test]
    fn test_parse_source_invalid_number() {
        let result = parse_source_convention("src/app.rs:45:xyz");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConventionError::InvalidNumber { field, value } => {
                assert_eq!(field, "character");
                assert_eq!(value, "xyz");
            }
            _ => panic!("Expected InvalidNumber error"),
        }
    }

    #[test]
    fn test_parse_destination_path_only() {
        let result = parse_destination_convention("src/utils.rs").unwrap();
        assert_eq!(result["file_path"], "src/utils.rs");
        assert!(result.get("line").is_none());
        assert!(result.get("character").is_none());
    }

    #[test]
    fn test_parse_destination_with_position() {
        let result = parse_destination_convention("src/utils.rs:10:0").unwrap();
        assert_eq!(result["file_path"], "src/utils.rs");
        assert_eq!(result["line"], 10);
        assert_eq!(result["character"], 0);
    }

    #[test]
    fn test_parse_destination_invalid_format() {
        // Only path:line without character
        let result = parse_destination_convention("src/utils.rs:10");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConventionError::InvalidFormat { input, expected } => {
                assert_eq!(input, "src/utils.rs:10");
                assert_eq!(expected, "path or path:line:char");
            }
            _ => panic!("Expected InvalidFormat error"),
        }
    }

    #[test]
    fn test_complex_paths_with_colons() {
        // Windows-style path with drive letter (C:)
        // This will be parsed as "file" : "C" : "/path/to/file.rs"
        // which will fail because it doesn't match any expected format
        // For Windows paths, users should either:
        // 1. Use JSON arguments instead of flags
        // 2. Use relative paths without drive letters
        let result = parse_target_convention("file:C:/path/to/file.rs");
        assert!(result.is_err()); // Expected to fail with current format
    }

    #[test]
    fn test_paths_with_spaces() {
        let result = parse_target_convention("directory:path with spaces/subdir");
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["path"], "path with spaces/subdir");
    }

    #[test]
    fn test_error_display() {
        let err = ConventionError::InvalidFormat {
            input: "bad".to_string(),
            expected: "good".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("bad"));
        assert!(display.contains("good"));

        let err = ConventionError::InvalidNumber {
            field: "line".to_string(),
            value: "abc".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("line"));
        assert!(display.contains("abc"));
    }
}
