//! Convention parsers for CLI flag support
//!
//! This module provides smart parsers that understand special flag conventions:
//! - Target convention: "kind:path" or "kind:path:line:char"
//! - Source convention: "path:line:char"
//! - Destination convention: "path" or "path:line:char"

use serde_json::{json, Value};
use std::fmt;

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
/// let result = parse_target_convention("directory:crates/cb-client").unwrap();
/// assert_eq!(result["kind"], "directory");
/// assert_eq!(result["path"], "crates/cb-client");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_target_directory() {
        let result = parse_target_convention("directory:crates/cb-client").unwrap();
        assert_eq!(result["kind"], "directory");
        assert_eq!(result["path"], "crates/cb-client");
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
