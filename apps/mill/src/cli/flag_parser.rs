//! Generic flag-to-JSON parser for refactoring tools
//!
//! This module provides a unified parser that converts CLI flags into JSON
//! arguments for all refactoring tools (rename, extract, inline, move, etc.).
//!
//! # Architecture
//!
//! The parser uses a two-phase approach:
//! 1. Tool-specific parsing: Validates required flags and builds JSON structure
//! 2. Convention parsing: Converts shorthand notation to full JSON (handled by Agent 2)
//!
//! # Example
//!
//! ```rust
//! use std::collections::HashMap;
//! use flag_parser::parse_flags_to_json;
//!
//! let mut flags = HashMap::new();
//! flags.insert("target".to_string(), "file:src/utils.rs".to_string());
//! flags.insert("new_name".to_string(), "src/helpers.rs".to_string());
//!
//! let json = parse_flags_to_json("rename", flags)?;
//! // Returns: {"target": {"kind": "file", "path": "src/utils.rs"}, "newName": "src/helpers.rs"}
//! ```

use serde_json::{json, Value};
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
    InvalidValue {
        flag: String,
        value: String,
        reason: String,
    },
    /// Convention parsing error (from Agent 2's parsers)
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
                    Common tools:\n\
                    - Refactoring: rename, extract, inline, move, delete\n\
                    - Navigation: find_definition, find_references, search_symbols\n\n\
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
/// * `tool_name` - Name of the refactoring tool (e.g., "rename")
/// * `flags` - HashMap of flag names to values
///
/// # Returns
///
/// JSON Value representing the tool's parameters, or a FlagParseError
///
/// # Example
///
/// ```rust
/// let mut flags = HashMap::new();
/// flags.insert("target".to_string(), "file:src/app.rs".to_string());
/// flags.insert("new_name".to_string(), "src/main.rs".to_string());
///
/// let json = parse_flags_to_json("rename", flags)?;
/// ```
pub fn parse_flags_to_json(
    tool_name: &str,
    flags: HashMap<String, String>,
) -> Result<Value, FlagParseError> {
    match tool_name {
        // Existing refactoring tools that support flags
        "rename" => parse_rename_flags(flags),
        "extract" => parse_extract_flags(flags),
        "move" => parse_move_flags(flags),
        "inline" => parse_inline_flags(flags),
        "reorder" => parse_reorder_flags(flags),
        "transform" => parse_transform_flags(flags),
        "delete" => parse_delete_flags(flags),

        // Navigation tools - require JSON
        "find_definition"
        | "find_references"
        | "find_implementations"
        | "find_type_definition"
        | "get_symbol_info" => Err(FlagParseError::JsonOnly {
            tool: tool_name.to_string(),
            example: get_example_for_tool(tool_name),
        }),

        // Navigation tools - require JSON
        "search_symbols" | "get_diagnostics" | "get_call_hierarchy" => {
            Err(FlagParseError::JsonOnly {
                tool: tool_name.to_string(),
                example: get_example_for_tool(tool_name),
            })
        }

        // Health check - takes empty object
        "health_check" => {
            if !flags.is_empty() {
                Err(FlagParseError::UnknownFlag(
                    "health_check takes no arguments. Use: mill tool health_check '{}'".to_string(),
                ))
            } else {
                Ok(json!({}))
            }
        }

        // Unknown tool
        _ => Err(FlagParseError::UnknownTool(tool_name.to_string())),
    }
}

// ============================================================================
// Tool-Specific Parsers
// ============================================================================

/// Parse flags for rename
///
/// Required flags: target, new_name
/// Optional flags: scope, exclude_patterns, strict, validate_scope, update_imports, consolidate
///
/// # JSON Schema
///
/// ```json
/// {
///   "target": {"kind": "file|directory|symbol", "path": "...", "selector": {...}},
///   "newName": "...",
///   "options": {
///     "scope": "code|standard|comments|everything|custom",
///     "custom_scope": {...},
///     "exclude_patterns": [...],
///     "strict": bool,
///     "validate_scope": bool,
///     "update_imports": bool,
///     "consolidate": bool
///   }
/// }
/// ```
fn parse_rename_flags(flags: HashMap<String, String>) -> Result<Value, FlagParseError> {
    // Validate allowed flags
    validate_flags(
        &flags,
        &[
            "target",
            "new_name",
            "scope",
            "exclude_patterns",
            "strict",
            "validate_scope",
            "consolidate",
            "update_code",
            "update_docs",
            "update_configs",
            "update_comments",
            "update_markdown_prose",
            "update_all",
            "update_string_literals",
            "update_exact_matches",
        ],
    )?;

    // Required flags
    let target = flags
        .get("target")
        .ok_or_else(|| FlagParseError::MissingRequiredFlag("target".to_string()))?;
    let new_name = flags
        .get("new_name")
        .ok_or_else(|| FlagParseError::MissingRequiredFlag("new_name".to_string()))?;

    // Parse target using convention (Agent 2 will implement)
    let target_json = parse_target_convention(target)?;

    let mut result = json!({
        "target": target_json,
        "newName": new_name,
    });

    // Build options object if any optional flags are present
    let mut options = json!({});
    let mut has_options = false;

    // Check if any update flags are present (opt-in flags only)
    // If so, we need to create a custom scope even if --scope wasn't explicitly set
    let has_update_flags = flags.keys().any(|k| {
        matches!(
            k.as_str(),
            "update_code"
                | "update_docs"
                | "update_configs"
                | "update_comments"
                | "update_markdown_prose"
                | "update_all"
                | "update_string_literals"
                | "update_exact_matches"
        )
    }) || flags.contains_key("exclude_patterns");

    // Scope configuration
    let scope = flags.get("scope").map(|s| s.as_str());

    // Auto-upgrade to custom scope if update flags are present
    // Accept both new names (code) and deprecated aliases (code-only)
    let effective_scope = if has_update_flags && scope != Some("code") && scope != Some("code-only")
    {
        "custom"
    } else {
        scope.unwrap_or("standard")
    };

    // Only set scope in options if it was explicitly provided or auto-upgraded
    if scope.is_some() || has_update_flags {
        validate_scope_value(effective_scope)?;
        options["scope"] = json!(effective_scope);
        has_options = true;
    }

    // Build custom_scope object if needed
    if effective_scope == "custom" && has_update_flags {
        let mut custom_scope = json!({});

        // Pass through all update flags to custom_scope
        for (key, value) in &flags {
            match key.as_str() {
                "update_code"
                | "update_docs"
                | "update_configs"
                | "update_comments"
                | "update_markdown_prose"
                | "update_all"
                | "update_string_literals"
                | "update_exact_matches" => {
                    custom_scope[key] = json!(parse_bool(value)?);
                }
                _ => {}
            }
        }

        if let Some(patterns) = flags.get("exclude_patterns") {
            custom_scope["exclude_patterns"] = parse_string_array(patterns)?;
        }

        options["custom_scope"] = custom_scope;
    }

    // Other options
    if let Some(strict) = flags.get("strict") {
        options["strict"] = json!(parse_bool(strict)?);
        has_options = true;
    }

    if let Some(validate_scope) = flags.get("validate_scope") {
        options["validate_scope"] = json!(parse_bool(validate_scope)?);
        has_options = true;
    }

    if let Some(update_imports) = flags.get("update_imports") {
        options["update_imports"] = json!(parse_bool(update_imports)?);
        has_options = true;
    }

    if let Some(consolidate) = flags.get("consolidate") {
        options["consolidate"] = json!(parse_bool(consolidate)?);
        has_options = true;
    }

    if has_options {
        result["options"] = options;
    }

    Ok(result)
}

/// Parse flags for extract
///
/// Required flags: kind, source, name
/// Optional flags: visibility
///
/// # JSON Schema
///
/// ```json
/// {
///   "kind": "function|variable|constant",
///   "source": {
///     "file_path": "...",
///     "range": {"start": {"line": N, "character": N}, "end": {...}}
///   },
///   "name": "...",
///   "visibility": "public|private|protected"
/// }
/// ```
fn parse_extract_flags(flags: HashMap<String, String>) -> Result<Value, FlagParseError> {
    // Validate allowed flags
    validate_flags(&flags, &["kind", "source", "name", "visibility"])?;

    // Required flags
    let kind = flags
        .get("kind")
        .ok_or_else(|| FlagParseError::MissingRequiredFlag("kind".to_string()))?;
    let source = flags
        .get("source")
        .ok_or_else(|| FlagParseError::MissingRequiredFlag("source".to_string()))?;
    let name = flags
        .get("name")
        .ok_or_else(|| FlagParseError::MissingRequiredFlag("name".to_string()))?;

    // Validate kind
    validate_extract_kind(kind)?;

    // Parse source using convention (Agent 2 will implement)
    let source_json = parse_source_convention(source)?;

    let mut result = json!({
        "kind": kind,
        "source": source_json,
        "name": name,
    });

    // Optional visibility
    if let Some(visibility) = flags.get("visibility") {
        validate_visibility(visibility)?;
        result["visibility"] = json!(visibility);
    }

    Ok(result)
}

/// Parse flags for move
///
/// Required flags: source, destination
/// Optional flags: kind, update_imports
///
/// # JSON Schema
///
/// ```json
/// {
///   "kind": "symbol|to_module",
///   "source": {
///     "file_path": "...",
///     "position": {"line": N, "character": N}
///   },
///   "destination": {
///     "file_path": "..."
///   },
///   "options": {
///     "update_imports": bool
///   }
/// }
/// ```
fn parse_move_flags(flags: HashMap<String, String>) -> Result<Value, FlagParseError> {
    // DETECTION: Check for common mistake - using 'move' for file operations
    // The 'move' tool is for moving CODE SYMBOLS (functions, classes) between files.
    // The 'rename' tool is for moving/renaming FILES and DIRECTORIES.

    // Check 1: If --target or --new-name are present, user wants 'rename'
    if flags.contains_key("target") || flags.contains_key("new_name") {
        let source_example = flags
            .get("target")
            .or_else(|| flags.get("source"))
            .cloned()
            .unwrap_or_else(|| "docs/old.md".to_string());
        let dest_example = flags
            .get("new_name")
            .or_else(|| flags.get("destination"))
            .cloned()
            .unwrap_or_else(|| "docs/new.md".to_string());

        // Ensure file: prefix is added only once
        let source_with_prefix =
            if source_example.starts_with("file:") || source_example.starts_with("directory:") {
                source_example
            } else {
                format!("file:{}", source_example)
            };

        return Err(FlagParseError::WrongTool {
            current_tool: "move".to_string(),
            suggested_tool: "rename".to_string(),
            reason: "moving CODE SYMBOLS (functions/classes) between files".to_string(),
            example: format!(
                "mill tool rename --target {} --new-name {}",
                source_with_prefix, dest_example
            ),
        });
    }

    // Check 2: If --source doesn't contain line:char, it's probably a file path
    if let Some(source) = flags.get("source") {
        // Source should be "path:line:char" for symbol moves
        // If it lacks ":" or has only one ":", it's likely a file path
        let colon_count = source.matches(':').count();
        if colon_count < 2 {
            let dest = flags
                .get("destination")
                .map(|s| s.as_str())
                .unwrap_or("destination/path");
            return Err(FlagParseError::WrongTool {
                current_tool: "move".to_string(),
                suggested_tool: "rename".to_string(),
                reason: "moving CODE SYMBOLS (functions/classes) between files".to_string(),
                example: format!(
                    "mill tool rename --target file:{} --new-name {}",
                    source, dest
                ),
            });
        }
    }

    // Validate allowed flags
    validate_flags(&flags, &["kind", "source", "destination", "update_imports"])?;

    // Required flags
    let source = flags
        .get("source")
        .ok_or_else(|| FlagParseError::MissingRequiredFlag("source".to_string()))?;
    let destination = flags
        .get("destination")
        .ok_or_else(|| FlagParseError::MissingRequiredFlag("destination".to_string()))?;

    // Parse source and destination using conventions (Agent 2 will implement)
    let source_json = parse_source_convention(source)?;
    let destination_json = parse_destination_convention(destination)?;

    let mut result = json!({
        "source": source_json,
        "destination": destination_json,
    });

    // Optional kind (defaults to "symbol" in backend)
    if let Some(kind) = flags.get("kind") {
        validate_move_kind(kind)?;
        result["kind"] = json!(kind);
    }

    // Optional update_imports
    if let Some(update_imports) = flags.get("update_imports") {
        let mut options = json!({});
        options["update_imports"] = json!(parse_bool(update_imports)?);
        result["options"] = options;
    }

    Ok(result)
}

/// Parse flags for inline
///
/// Required flags: target
/// Optional flags: kind, inline_all
///
/// # JSON Schema
///
/// ```json
/// {
///   "kind": "variable|function",
///   "target": {
///     "file_path": "...",
///     "position": {"line": N, "character": N}
///   },
///   "options": {
///     "inline_all": bool
///   }
/// }
/// ```
fn parse_inline_flags(flags: HashMap<String, String>) -> Result<Value, FlagParseError> {
    // Validate allowed flags
    validate_flags(&flags, &["kind", "target", "inline_all"])?;

    // Required flags
    let target = flags
        .get("target")
        .ok_or_else(|| FlagParseError::MissingRequiredFlag("target".to_string()))?;

    // Parse target using convention (Agent 2 will implement)
    let target_json = parse_source_convention(target)?;

    let mut result = json!({
        "target": target_json,
    });

    // Optional kind
    if let Some(kind) = flags.get("kind") {
        validate_inline_kind(kind)?;
        result["kind"] = json!(kind);
    }

    // Optional inline_all
    if let Some(inline_all) = flags.get("inline_all") {
        let mut options = json!({});
        options["inline_all"] = json!(parse_bool(inline_all)?);
        result["options"] = options;
    }

    Ok(result)
}

/// Parse flags for reorder
///
/// Required flags: kind, target
/// Optional flags: strategy, order
///
/// # JSON Schema
///
/// ```json
/// {
///   "kind": "parameters|imports",
///   "target": {
///     "file_path": "...",
///     "position": {"line": N, "character": N}  // For parameters
///     // OR just file_path for imports
///   },
///   "options": {
///     "strategy": "alphabetical|custom",
///     "order": [...]  // For custom strategy
///   }
/// }
/// ```
fn parse_reorder_flags(flags: HashMap<String, String>) -> Result<Value, FlagParseError> {
    // Validate allowed flags
    validate_flags(&flags, &["kind", "target", "strategy", "order"])?;

    // Required flags
    let kind = flags
        .get("kind")
        .ok_or_else(|| FlagParseError::MissingRequiredFlag("kind".to_string()))?;
    let target = flags
        .get("target")
        .ok_or_else(|| FlagParseError::MissingRequiredFlag("target".to_string()))?;

    // Validate kind
    validate_reorder_kind(kind)?;

    // Parse target using convention (Agent 2 will implement)
    let target_json = parse_source_convention(target)?;

    let mut result = json!({
        "kind": kind,
        "target": target_json,
    });

    // Optional strategy and order
    let mut options = json!({});
    let mut has_options = false;

    if let Some(strategy) = flags.get("strategy") {
        validate_reorder_strategy(strategy)?;
        options["strategy"] = json!(strategy);
        has_options = true;

        // If strategy is "custom", require order
        if strategy == "custom" {
            if let Some(order) = flags.get("order") {
                options["order"] = parse_string_array(order)?;
            } else {
                return Err(FlagParseError::MissingRequiredFlag(
                    "order (required with strategy=custom)".to_string(),
                ));
            }
        }
    }

    if has_options {
        result["options"] = options;
    }

    Ok(result)
}

/// Parse flags for transform
///
/// Required flags: kind, target
///
/// # JSON Schema
///
/// ```json
/// {
///   "kind": "to_async|loop_to_iterator",
///   "target": {
///     "file_path": "...",
///     "position": {"line": N, "character": N}
///     // OR range: {"start": {...}, "end": {...}}
///   }
/// }
/// ```
fn parse_transform_flags(flags: HashMap<String, String>) -> Result<Value, FlagParseError> {
    // Validate allowed flags
    validate_flags(&flags, &["kind", "target"])?;

    // Required flags
    let kind = flags
        .get("kind")
        .ok_or_else(|| FlagParseError::MissingRequiredFlag("kind".to_string()))?;
    let target = flags
        .get("target")
        .ok_or_else(|| FlagParseError::MissingRequiredFlag("target".to_string()))?;

    // Validate kind
    validate_transform_kind(kind)?;

    // Parse target using convention (Agent 2 will implement)
    let target_json = parse_source_convention(target)?;

    let result = json!({
        "kind": kind,
        "target": target_json,
    });

    Ok(result)
}

/// Parse flags for delete
///
/// Required flags: kind, target
///
/// # JSON Schema
///
/// ```json
/// {
///   "kind": "unused_imports",
///   "target": {
///     "scope": "file|workspace",
///     "path": "..."
///   }
/// }
/// ```
fn parse_delete_flags(flags: HashMap<String, String>) -> Result<Value, FlagParseError> {
    // Validate allowed flags
    validate_flags(&flags, &["kind", "target", "scope", "path"])?;

    // Required flags
    let kind = flags
        .get("kind")
        .ok_or_else(|| FlagParseError::MissingRequiredFlag("kind".to_string()))?;

    // Validate kind
    validate_delete_kind(kind)?;

    // For delete, target can be provided as a single flag or as scope+path
    let target_json = if let Some(target) = flags.get("target") {
        // Parse target convention (Agent 2 will implement)
        parse_delete_target_convention(target)?
    } else {
        // Build from scope and path
        let scope = flags
            .get("scope")
            .ok_or_else(|| FlagParseError::MissingRequiredFlag("scope or target".to_string()))?;
        let path = flags
            .get("path")
            .ok_or_else(|| FlagParseError::MissingRequiredFlag("path or target".to_string()))?;

        validate_delete_scope(scope)?;

        json!({
            "scope": scope,
            "path": path,
        })
    };

    let result = json!({
        "kind": kind,
        "target": target_json,
    });

    Ok(result)
}

// ============================================================================
// Convention Parsers (Integrated from Agent 2's conventions.rs)
// ============================================================================

/// Parse target convention using Agent 2's implementation
///
/// Examples:
/// - "file:src/utils.rs" -> {"kind": "file", "path": "src/utils.rs"}
/// - "directory:src/modules" -> {"kind": "directory", "path": "src/modules"}
/// - "symbol:src/app.rs:10:5" -> {"kind": "symbol", "path": "src/app.rs", "selector": {...}}
fn parse_target_convention(s: &str) -> Result<Value, FlagParseError> {
    use super::conventions;
    conventions::parse_target_convention(s)
        .map_err(|e| FlagParseError::ConventionError(e.to_string()))
}

/// Parse source convention using Agent 2's implementation
///
/// Examples:
/// - "src/app.rs:10:5" -> {"file_path": "src/app.rs", "line": 10, "character": 5}
fn parse_source_convention(s: &str) -> Result<Value, FlagParseError> {
    use super::conventions;
    conventions::parse_source_convention(s)
        .map_err(|e| FlagParseError::ConventionError(e.to_string()))
}

/// Parse destination convention using Agent 2's implementation
///
/// Examples:
/// - "src/new.rs" -> {"file_path": "src/new.rs"}
/// - "src/new.rs:10:0" -> {"file_path": "src/new.rs", "line": 10, "character": 0}
fn parse_destination_convention(s: &str) -> Result<Value, FlagParseError> {
    use super::conventions;
    conventions::parse_destination_convention(s)
        .map_err(|e| FlagParseError::ConventionError(e.to_string()))
}

/// Parse delete target convention
///
/// Examples:
/// - "file:src/app.rs" -> {"scope": "file", "path": "src/app.rs"}
/// - "workspace:." -> {"scope": "workspace", "path": "."}
fn parse_delete_target_convention(s: &str) -> Result<Value, FlagParseError> {
    // For delete operations, we support "scope:path" format
    // This is different from other target conventions
    if s.starts_with("file:") {
        Ok(json!({
            "scope": "file",
            "path": s.strip_prefix("file:").unwrap()
        }))
    } else if s.starts_with("workspace:") {
        Ok(json!({
            "scope": "workspace",
            "path": s.strip_prefix("workspace:").unwrap()
        }))
    } else {
        // Default to file scope
        Ok(json!({
            "scope": "file",
            "path": s
        }))
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get example usage for a tool that requires JSON
fn get_example_for_tool(tool: &str) -> String {
    match tool {
        "find_definition" =>
            "mill tool find_definition '{\"file_path\":\"src/app.rs\",\"line\":10,\"character\":5}'".to_string(),
        "find_references" =>
            "mill tool find_references '{\"file_path\":\"src/app.rs\",\"line\":10,\"character\":5}'".to_string(),
        "find_implementations" =>
            "mill tool find_implementations '{\"file_path\":\"src/app.rs\",\"line\":10,\"character\":5}'".to_string(),
        "find_type_definition" =>
            "mill tool find_type_definition '{\"file_path\":\"src/app.rs\",\"line\":10,\"character\":5}'".to_string(),
        "get_symbol_info" =>
            "mill tool get_symbol_info '{\"file_path\":\"src/app.rs\",\"line\":10,\"character\":5}'".to_string(),
        "search_symbols" =>
            "mill tool search_symbols '{\"query\":\"MyClass\",\"limit\":10}'".to_string(),
        "get_diagnostics" =>
            "mill tool get_diagnostics '{\"file_path\":\"src/app.rs\"}'".to_string(),
        "get_call_hierarchy" =>
            "mill tool get_call_hierarchy '{\"file_path\":\"src/app.rs\",\"line\":10,\"character\":5}'".to_string(),
        _ => format!("mill tool {} '<JSON arguments>'", tool),
    }
}

/// Validate that only expected flags are present
fn validate_flags(flags: &HashMap<String, String>, allowed: &[&str]) -> Result<(), FlagParseError> {
    for key in flags.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(FlagParseError::UnknownFlag(key.clone()));
        }
    }
    Ok(())
}

/// Parse a boolean value from a string
fn parse_bool(s: &str) -> Result<bool, FlagParseError> {
    match s.to_lowercase().as_str() {
        "true" | "t" | "yes" | "y" | "1" => Ok(true),
        "false" | "f" | "no" | "n" | "0" => Ok(false),
        _ => Err(FlagParseError::InvalidValue {
            flag: "boolean".to_string(),
            value: s.to_string(),
            reason: "expected true/false, yes/no, or 1/0".to_string(),
        }),
    }
}

/// Parse a comma-separated string into a JSON array
fn parse_string_array(s: &str) -> Result<Value, FlagParseError> {
    let items: Vec<&str> = s
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    Ok(json!(items))
}

// ============================================================================
// Validation Functions
// ============================================================================

fn validate_scope_value(scope: &str) -> Result<(), FlagParseError> {
    match scope {
        // New scope names (preferred)
        "code" | "standard" | "comments" | "everything" | "custom" => Ok(()),

        // Deprecated but still accepted
        "code-only" => {
            eprintln!("⚠️  Warning: 'code-only' is deprecated. Use 'code' instead.");
            Ok(())
        }
        "all" => {
            eprintln!("⚠️  Warning: 'all' is deprecated. Use 'standard' instead.");
            Ok(())
        }

        _ => Err(FlagParseError::InvalidValue {
            flag: "scope".to_string(),
            value: scope.to_string(),
            reason: "must be 'code', 'standard', 'comments', 'everything', or 'custom'".to_string(),
        }),
    }
}

fn validate_extract_kind(kind: &str) -> Result<(), FlagParseError> {
    match kind {
        "function" | "variable" | "constant" => Ok(()),
        _ => Err(FlagParseError::InvalidValue {
            flag: "kind".to_string(),
            value: kind.to_string(),
            reason: "must be 'function', 'variable', or 'constant'".to_string(),
        }),
    }
}

fn validate_visibility(visibility: &str) -> Result<(), FlagParseError> {
    match visibility {
        "public" | "private" | "protected" => Ok(()),
        _ => Err(FlagParseError::InvalidValue {
            flag: "visibility".to_string(),
            value: visibility.to_string(),
            reason: "must be 'public', 'private', or 'protected'".to_string(),
        }),
    }
}

fn validate_move_kind(kind: &str) -> Result<(), FlagParseError> {
    match kind {
        "symbol" | "to_module" => Ok(()),
        _ => Err(FlagParseError::InvalidValue {
            flag: "kind".to_string(),
            value: kind.to_string(),
            reason: "must be 'symbol' or 'to_module'".to_string(),
        }),
    }
}

fn validate_inline_kind(kind: &str) -> Result<(), FlagParseError> {
    match kind {
        "variable" | "function" => Ok(()),
        _ => Err(FlagParseError::InvalidValue {
            flag: "kind".to_string(),
            value: kind.to_string(),
            reason: "must be 'variable' or 'function'".to_string(),
        }),
    }
}

fn validate_reorder_kind(kind: &str) -> Result<(), FlagParseError> {
    match kind {
        "parameters" | "imports" => Ok(()),
        _ => Err(FlagParseError::InvalidValue {
            flag: "kind".to_string(),
            value: kind.to_string(),
            reason: "must be 'parameters' or 'imports'".to_string(),
        }),
    }
}

fn validate_reorder_strategy(strategy: &str) -> Result<(), FlagParseError> {
    match strategy {
        "alphabetical" | "custom" => Ok(()),
        _ => Err(FlagParseError::InvalidValue {
            flag: "strategy".to_string(),
            value: strategy.to_string(),
            reason: "must be 'alphabetical' or 'custom'".to_string(),
        }),
    }
}

fn validate_transform_kind(kind: &str) -> Result<(), FlagParseError> {
    match kind {
        "to_async" | "loop_to_iterator" => Ok(()),
        _ => Err(FlagParseError::InvalidValue {
            flag: "kind".to_string(),
            value: kind.to_string(),
            reason: "must be 'to_async' or 'loop_to_iterator'".to_string(),
        }),
    }
}

fn validate_delete_kind(kind: &str) -> Result<(), FlagParseError> {
    match kind {
        "unused_imports" => Ok(()),
        _ => Err(FlagParseError::InvalidValue {
            flag: "kind".to_string(),
            value: kind.to_string(),
            reason: "must be 'unused_imports'".to_string(),
        }),
    }
}

fn validate_delete_scope(scope: &str) -> Result<(), FlagParseError> {
    match scope {
        "file" | "workspace" => Ok(()),
        _ => Err(FlagParseError::InvalidValue {
            flag: "scope".to_string(),
            value: scope.to_string(),
            reason: "must be 'file' or 'workspace'".to_string(),
        }),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create flags HashMap
    fn flags(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    // ========================================================================
    // rename tests
    // ========================================================================

    #[test]
    fn test_rename_basic_file() {
        let result = parse_flags_to_json(
            "rename",
            flags(&[
                ("target", "file:src/utils.rs"),
                ("new_name", "src/helpers.rs"),
            ]),
        );
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["target"]["kind"], "file");
        assert_eq!(json["target"]["path"], "src/utils.rs");
        assert_eq!(json["newName"], "src/helpers.rs");
    }

    #[test]
    fn test_rename_directory() {
        let result = parse_flags_to_json(
            "rename",
            flags(&[("target", "directory:old-dir"), ("new_name", "new-dir")]),
        );
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["target"]["kind"], "directory");
        assert_eq!(json["target"]["path"], "old-dir");
    }

    #[test]
    fn test_rename_with_scope() {
        let result = parse_flags_to_json(
            "rename",
            flags(&[
                ("target", "file:src/app.rs"),
                ("new_name", "src/main.rs"),
                ("scope", "code"),
            ]),
        );
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["options"]["scope"], "code");
    }

    #[test]
    fn test_rename_with_custom_scope() {
        let result = parse_flags_to_json(
            "rename",
            flags(&[
                ("target", "file:src/app.rs"),
                ("new_name", "src/main.rs"),
                ("scope", "custom"),
                ("update_code", "true"),
                ("update_docs", "false"),
                ("exclude_patterns", "test_*,fixtures/**"),
            ]),
        );
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["options"]["scope"], "custom");
        assert_eq!(json["options"]["custom_scope"]["update_code"], true);
        assert_eq!(json["options"]["custom_scope"]["update_docs"], false);
        assert_eq!(
            json["options"]["custom_scope"]["exclude_patterns"][0],
            "test_*"
        );
    }

    #[test]
    fn test_rename_missing_required() {
        let result = parse_flags_to_json("rename", flags(&[("target", "file:src/app.rs")]));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FlagParseError::MissingRequiredFlag(_)
        ));
    }

    #[test]
    fn test_rename_unknown_flag() {
        let result = parse_flags_to_json(
            "rename",
            flags(&[
                ("target", "file:src/app.rs"),
                ("new_name", "src/main.rs"),
                ("invalid_flag", "value"),
            ]),
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FlagParseError::UnknownFlag(_)
        ));
    }

    #[test]
    fn test_rename_invalid_scope() {
        let result = parse_flags_to_json(
            "rename",
            flags(&[
                ("target", "file:src/app.rs"),
                ("new_name", "src/main.rs"),
                ("scope", "invalid"),
            ]),
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FlagParseError::InvalidValue { .. }
        ));
    }

    // ========================================================================
    // extract tests
    // ========================================================================

    #[test]
    fn test_extract_function() {
        let result = parse_flags_to_json(
            "extract",
            flags(&[
                ("kind", "function"),
                ("source", "src/app.rs:10:5"),
                ("name", "handleLogin"),
            ]),
        );
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["kind"], "function");
        assert_eq!(json["source"]["file_path"], "src/app.rs");
        assert_eq!(json["name"], "handleLogin");
    }

    #[test]
    fn test_extract_with_visibility() {
        let result = parse_flags_to_json(
            "extract",
            flags(&[
                ("kind", "function"),
                ("source", "src/app.rs:10:5"),
                ("name", "helper"),
                ("visibility", "private"),
            ]),
        );
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["visibility"], "private");
    }

    #[test]
    fn test_extract_missing_required() {
        let result = parse_flags_to_json(
            "extract",
            flags(&[("kind", "function"), ("source", "src/app.rs:10:5")]),
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FlagParseError::MissingRequiredFlag(_)
        ));
    }

    #[test]
    fn test_extract_invalid_kind() {
        let result = parse_flags_to_json(
            "extract",
            flags(&[
                ("kind", "invalid"),
                ("source", "src/app.rs:10:5"),
                ("name", "foo"),
            ]),
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FlagParseError::InvalidValue { .. }
        ));
    }

    // ========================================================================
    // move tests
    // ========================================================================

    #[test]
    fn test_move_symbol() {
        let result = parse_flags_to_json(
            "move",
            flags(&[
                ("source", "src/app.rs:10:5"),
                ("destination", "src/utils.rs"),
            ]),
        );
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["source"]["file_path"], "src/app.rs");
        assert_eq!(json["destination"]["file_path"], "src/utils.rs");
    }

    #[test]
    fn test_move_with_kind() {
        let result = parse_flags_to_json(
            "move",
            flags(&[
                ("kind", "to_module"),
                ("source", "src/app.rs:10:5"),
                ("destination", "src/utils.rs"),
            ]),
        );
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["kind"], "to_module");
    }

    #[test]
    fn test_move_missing_required() {
        let result = parse_flags_to_json("move", flags(&[("source", "src/app.rs:10:5")]));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FlagParseError::MissingRequiredFlag(_)
        ));
    }

    // ========================================================================
    // inline tests
    // ========================================================================

    #[test]
    fn test_inline_variable() {
        let result = parse_flags_to_json("inline", flags(&[("target", "src/app.rs:10:5")]));
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["target"]["file_path"], "src/app.rs");
    }

    #[test]
    fn test_inline_with_kind() {
        let result = parse_flags_to_json(
            "inline",
            flags(&[("kind", "function"), ("target", "src/app.rs:10:5")]),
        );
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["kind"], "function");
    }

    #[test]
    fn test_inline_with_inline_all() {
        let result = parse_flags_to_json(
            "inline",
            flags(&[("target", "src/app.rs:10:5"), ("inline_all", "true")]),
        );
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["options"]["inline_all"], true);
    }

    #[test]
    fn test_inline_missing_required() {
        let result = parse_flags_to_json("inline", flags(&[]));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FlagParseError::MissingRequiredFlag(_)
        ));
    }

    // ========================================================================
    // reorder tests
    // ========================================================================

    #[test]
    fn test_reorder_imports() {
        let result = parse_flags_to_json(
            "reorder",
            flags(&[
                ("kind", "imports"),
                ("target", "src/app.rs"),
                ("strategy", "alphabetical"),
            ]),
        );
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["kind"], "imports");
        assert_eq!(json["options"]["strategy"], "alphabetical");
    }

    #[test]
    fn test_reorder_parameters() {
        let result = parse_flags_to_json(
            "reorder",
            flags(&[("kind", "parameters"), ("target", "src/app.rs:10:5")]),
        );
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["kind"], "parameters");
    }

    #[test]
    fn test_reorder_custom_strategy() {
        let result = parse_flags_to_json(
            "reorder",
            flags(&[
                ("kind", "imports"),
                ("target", "src/app.rs"),
                ("strategy", "custom"),
                ("order", "std,external,internal"),
            ]),
        );
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["options"]["strategy"], "custom");
        assert_eq!(json["options"]["order"][0], "std");
    }

    #[test]
    fn test_reorder_custom_without_order() {
        let result = parse_flags_to_json(
            "reorder",
            flags(&[
                ("kind", "imports"),
                ("target", "src/app.rs"),
                ("strategy", "custom"),
            ]),
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FlagParseError::MissingRequiredFlag(_)
        ));
    }

    // ========================================================================
    // transform tests
    // ========================================================================

    #[test]
    fn test_transform_to_async() {
        let result = parse_flags_to_json(
            "transform",
            flags(&[("kind", "to_async"), ("target", "src/app.rs:10:5")]),
        );
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["kind"], "to_async");
    }

    #[test]
    fn test_transform_loop() {
        let result = parse_flags_to_json(
            "transform",
            flags(&[("kind", "loop_to_iterator"), ("target", "src/app.rs:10:5")]),
        );
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["kind"], "loop_to_iterator");
    }

    #[test]
    fn test_transform_invalid_kind() {
        let result = parse_flags_to_json(
            "transform",
            flags(&[("kind", "invalid"), ("target", "src/app.rs:10:5")]),
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FlagParseError::InvalidValue { .. }
        ));
    }

    // ========================================================================
    // delete tests
    // ========================================================================

    #[test]
    fn test_delete_unused_imports() {
        let result = parse_flags_to_json(
            "delete",
            flags(&[("kind", "unused_imports"), ("target", "file:src/app.rs")]),
        );
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["kind"], "unused_imports");
        assert_eq!(json["target"]["scope"], "file");
    }

    #[test]
    fn test_delete_with_scope_and_path() {
        let result = parse_flags_to_json(
            "delete",
            flags(&[
                ("kind", "unused_imports"),
                ("scope", "file"),
                ("path", "src/app.rs"),
            ]),
        );
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["target"]["scope"], "file");
        assert_eq!(json["target"]["path"], "src/app.rs");
    }

    #[test]
    fn test_delete_missing_target() {
        let result = parse_flags_to_json("delete", flags(&[("kind", "unused_imports")]));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FlagParseError::MissingRequiredFlag(_)
        ));
    }

    // ========================================================================
    // Helper function tests
    // ========================================================================

    #[test]
    fn test_parse_bool() {
        assert!(parse_bool("true").unwrap());
        assert!(parse_bool("TRUE").unwrap());
        assert!(parse_bool("yes").unwrap());
        assert!(parse_bool("1").unwrap());
        assert!(!parse_bool("false").unwrap());
        assert!(!parse_bool("no").unwrap());
        assert!(!parse_bool("0").unwrap());
        assert!(parse_bool("invalid").is_err());
    }

    #[test]
    fn test_parse_string_array() {
        let result = parse_string_array("a,b,c").unwrap();
        assert_eq!(result[0], "a");
        assert_eq!(result[1], "b");
        assert_eq!(result[2], "c");

        let result = parse_string_array("a, b , c").unwrap();
        assert_eq!(result[0], "a");
        assert_eq!(result[1], "b");
        assert_eq!(result[2], "c");

        let result = parse_string_array("").unwrap();
        assert_eq!(result.as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_validate_flags() {
        let flags = flags(&[("a", "1"), ("b", "2")]);
        assert!(validate_flags(&flags, &["a", "b", "c"]).is_ok());
        assert!(validate_flags(&flags, &["a"]).is_err());
    }

    // ========================================================================
    // Error display tests
    // ========================================================================

    #[test]
    fn test_error_display() {
        let err = FlagParseError::MissingRequiredFlag("target".to_string());
        assert_eq!(err.to_string(), "Missing required flag: --target");

        let err = FlagParseError::UnknownFlag("invalid".to_string());
        assert_eq!(err.to_string(), "Unknown flag: --invalid");

        let err = FlagParseError::InvalidValue {
            flag: "scope".to_string(),
            value: "bad".to_string(),
            reason: "must be 'code', 'standard', 'comments', 'everything', or 'custom'".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Invalid value 'bad' for --scope: must be 'code', 'standard', 'comments', 'everything', or 'custom'"
        );
    }

    // ========================================================================
    // Unknown tool tests
    // ========================================================================

    #[test]
    fn test_unknown_tool_old() {
        let result = parse_flags_to_json("unknown.plan", flags(&[]));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FlagParseError::UnknownTool(_)
        ));
    }

    // ========================================================================
    // Phase 3: JSON-only tools tests
    // ========================================================================

    #[test]
    fn test_json_only_tools() {
        let result = parse_flags_to_json(
            "find_definition",
            HashMap::from([("target".to_string(), "test".to_string())]),
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            FlagParseError::JsonOnly { tool, example } => {
                assert_eq!(tool, "find_definition");
                assert!(example.contains("file_path"));
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_health_check_empty() {
        let result = parse_flags_to_json("health_check", HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json!({}));
    }

    #[test]
    fn test_health_check_with_flags_error() {
        let result = parse_flags_to_json(
            "health_check",
            HashMap::from([("foo".to_string(), "bar".to_string())]),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_tool() {
        let result = parse_flags_to_json("nonexistent_tool", HashMap::new());
        assert!(result.is_err());
        match result.unwrap_err() {
            FlagParseError::UnknownTool(name) => {
                assert_eq!(name, "nonexistent_tool");
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_error_display_json_only() {
        let err = FlagParseError::JsonOnly {
            tool: "find_definition".to_string(),
            example: "mill tool find_definition '{}'".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("requires JSON"));
        assert!(msg.contains("find_definition"));
    }

    #[test]
    fn test_error_display_unknown_tool() {
        let err = FlagParseError::UnknownTool("foo".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Unknown tool"));
        assert!(msg.contains("foo"));
    }
}
