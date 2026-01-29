//! Generic flag-to-JSON parser for tools
//!
//! This module provides a unified parser that converts CLI flags into JSON
//! arguments. With the Magnificent Seven API, all tools require JSON arguments.
//!
//! # Architecture
//!
//! All Magnificent Seven tools require JSON arguments (no flag-based shortcuts).
//! Legacy tool names return helpful migration messages directing users to the
//! correct M7 tool.
//!
//! # Example
//!
//! ```rust
//! use std::collections::HashMap;
//! use flag_parser::parse_flags_to_json;
//!
//! // M7 tools require JSON - returns JsonOnly error with example
//! let result = parse_flags_to_json("rename_all", HashMap::new());
//! // Returns Err(FlagParseError::JsonOnly { ... })
//!
//! // Unknown tool names return UnknownTool error
//! let result = parse_flags_to_json("rename", HashMap::new());
//! // Returns Err(FlagParseError::UnknownTool { ... })
//! ```

use serde_json::Value;
use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during flag parsing
#[derive(Debug, Clone, PartialEq)]
pub enum FlagParseError {
    /// A required flag is missing
    MissingRequiredFlag(String),
    /// A flag has an invalid format
    #[allow(dead_code)]
    InvalidFormat { flag: String, expected: String },
    /// Multiple conflicting flags were provided
    #[allow(dead_code)]
    ConflictingFlags(Vec<String>),
    /// An unknown flag was provided
    UnknownFlag(String),
    /// Invalid value for a flag
    #[allow(dead_code)]
    InvalidValue {
        flag: String,
        value: String,
        reason: String,
    },
    /// Convention parsing error (from Agent 2's parsers)
    #[allow(dead_code)]
    ConventionError(String),
    /// Wrong tool for operation (with suggested correct tool)
    WrongTool {
        current_tool: String,
        suggested_tool: String,
        reason: String,
        example: String,
    },
    /// Tool requires JSON arguments, not flags
    JsonOnly { tool: String, example: String },
    /// Unknown tool name
    UnknownTool(String),
}

impl fmt::Display for FlagParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FlagParseError::MissingRequiredFlag(flag) => {
                write!(f, "Missing required flag: --{}", flag)
            }
            FlagParseError::InvalidFormat { flag, expected } => {
                write!(f, "Invalid format for --{}: expected {}", flag, expected)
            }
            FlagParseError::ConflictingFlags(flags) => {
                write!(f, "Conflicting flags: {}", flags.join(", "))
            }
            FlagParseError::UnknownFlag(flag) => {
                write!(f, "Unknown flag: --{}", flag)
            }
            FlagParseError::InvalidValue {
                flag,
                value,
                reason,
            } => {
                write!(f, "Invalid value '{}' for --{}: {}", value, flag, reason)
            }
            FlagParseError::ConventionError(msg) => {
                write!(f, "Convention parsing error: {}", msg)
            }
            FlagParseError::WrongTool {
                current_tool,
                suggested_tool,
                reason,
                example,
            } => {
                write!(
                    f,
                    "Wrong tool for this operation.\n\n\
                     Tool '{}' is for {}.\n\
                     For your operation, use '{}'.\n\n\
                     Example:\n  {}",
                    current_tool, reason, suggested_tool, example
                )
            }
            FlagParseError::JsonOnly { tool, example } => {
                write!(
                    f,
                    "Tool '{}' requires JSON arguments (not flags).\n\n\
                    Example usage:\n  {}\n\n\
                    For all tools: mill tools",
                    tool, example
                )
            }
            FlagParseError::UnknownTool(name) => {
                write!(
                    f,
                    "Unknown tool: '{}'\n\n\
                    Available tools (Magnificent Seven):\n\
                    - Navigation: inspect_code, search_code\n\
                    - Refactoring: rename_all, relocate, prune, refactor\n\
                    - Workspace: workspace\n\n\
                    List all tools: mill tools\n\
                    Tool help: mill docs tools/[category]",
                    name
                )
            }
        }
    }
}

impl std::error::Error for FlagParseError {}

// ============================================================================
// Main Entry Point
// ============================================================================

/// Parse flags into JSON for a given tool
///
/// This is the main entry point for the generic flag parser. It dispatches
/// to tool-specific parsers based on the tool name.
///
/// # Arguments
///
/// * `tool_name` - Name of the tool (e.g., "inspect_code")
/// * `flags` - HashMap of flag names to values
///
/// # Returns
///
/// JSON Value representing the tool's parameters, or a FlagParseError
///
/// # Note
///
/// All Magnificent Seven tools require JSON arguments.
pub fn parse_flags_to_json(
    tool_name: &str,
    _flags: HashMap<String, String>,
) -> Result<Value, FlagParseError> {
    match tool_name {
        // ===================================================================
        // MAGNIFICENT SEVEN - The Complete Public API
        // All M7 tools require JSON arguments (no flag-based shortcuts)
        // ===================================================================
        "inspect_code" | "search_code" | "rename_all" | "relocate" | "prune" | "refactor"
        | "workspace" => Err(FlagParseError::JsonOnly {
            tool: tool_name.to_string(),
            example: get_example_for_tool(tool_name),
        }),

        // Unknown tool
        _ => Err(FlagParseError::UnknownTool(tool_name.to_string())),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get example usage for a tool that requires JSON
fn get_example_for_tool(tool: &str) -> String {
    match tool {
        // Magnificent Seven - Public API
        "inspect_code" =>
            "mill tool inspect_code '{\"filePath\":\"src/app.rs\",\"line\":9,\"character\":5,\"include\":[\"definition\",\"typeInfo\"]}'".to_string(),
        "search_code" =>
            "mill tool search_code '{\"query\":\"MyClass\",\"limit\":10}'".to_string(),
        "rename_all" =>
            "mill tool rename_all '{\"target\":{\"kind\":\"symbol\",\"filePath\":\"src/app.rs\",\"line\":9,\"character\":5},\"newName\":\"newFunctionName\"}'".to_string(),
        "relocate" =>
            "mill tool relocate '{\"target\":{\"kind\":\"symbol\",\"filePath\":\"src/utils.rs\",\"line\":14,\"character\":0},\"destination\":{\"filePath\":\"src/helpers.rs\"}}'".to_string(),
        "prune" =>
            "mill tool prune '{\"target\":{\"kind\":\"symbol\",\"filePath\":\"src/app.rs\",\"line\":9,\"character\":5}}'".to_string(),
        "refactor" =>
            "mill tool refactor '{\"action\":\"extract\",\"params\":{\"kind\":\"function\",\"filePath\":\"src/app.rs\",\"range\":{\"startLine\":14,\"startCharacter\":0,\"endLine\":20,\"endCharacter\":0},\"name\":\"extractedFn\"}}'".to_string(),
        "workspace" =>
            "mill tool workspace '{\"action\":\"find_replace\",\"params\":{\"pattern\":\"oldName\",\"replacement\":\"newName\",\"mode\":\"literal\"}}'".to_string(),
        _ => format!("mill tool {} '<JSON arguments>'", tool),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Magnificent Seven tests - All require JSON
    // ========================================================================

    #[test]
    fn test_m7_tools_require_json() {
        let m7_tools = vec![
            "inspect_code",
            "search_code",
            "rename_all",
            "relocate",
            "prune",
            "refactor",
            "workspace",
        ];

        for tool in m7_tools {
            let result = parse_flags_to_json(tool, HashMap::new());
            assert!(result.is_err(), "Tool {} should require JSON", tool);
            match result.unwrap_err() {
                FlagParseError::JsonOnly { tool: t, example } => {
                    assert_eq!(t, tool);
                    assert!(!example.is_empty(), "Example should not be empty for {}", tool);
                }
                _ => panic!("Expected JsonOnly error for {}", tool),
            }
        }
    }

    // ========================================================================
    // Unknown tool tests
    // ========================================================================

    #[test]
    fn test_unknown_tool() {
        let result = parse_flags_to_json("nonexistent_tool", HashMap::new());
        assert!(result.is_err());
        match result.unwrap_err() {
            FlagParseError::UnknownTool(name) => {
                assert_eq!(name, "nonexistent_tool");
            }
            _ => panic!("Expected UnknownTool error"),
        }
    }

    // ========================================================================
    // Error display tests
    // ========================================================================

    #[test]
    fn test_error_display_json_only() {
        let err = FlagParseError::JsonOnly {
            tool: "inspect_code".to_string(),
            example: "mill tool inspect_code '{}'".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("requires JSON"));
        assert!(msg.contains("inspect_code"));
    }

    #[test]
    fn test_error_display_unknown_tool() {
        let err = FlagParseError::UnknownTool("foo".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Unknown tool"));
        assert!(msg.contains("foo"));
        assert!(msg.contains("Magnificent Seven"));
    }
}
