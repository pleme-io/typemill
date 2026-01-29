//! Tool Definitions Module - Single Source of Truth
//!
//! This module defines the canonical schemas and public tool list for the
//! Magnificent Seven refactored API. All tool discovery and validation
//! should reference these definitions.
//!
//! ## Public Tools (7)
//!
//! 1. `inspect_code` - Aggregate code intelligence (definition, references, types, diagnostics)
//! 2. `search_code` - Workspace-wide symbol search
//! 3. `rename_all` - Rename symbols, files, or directories with reference updates
//! 4. `relocate` - Move symbols, files, or directories
//! 5. `prune` - Delete symbols, files, or directories with cleanup
//! 6. `refactor` - Extract, inline, and transform operations
//! 7. `workspace` - Package management, dependency extraction, find/replace

use serde_json::{json, Value};

/// List of all public tool names
pub const PUBLIC_TOOLS: &[&str] = &[
    "inspect_code",
    "search_code",
    "rename_all",
    "relocate",
    "prune",
    "refactor",
    "workspace",
];

/// Get all public tool definitions as JSON schemas
pub fn get_all_tool_definitions() -> Vec<Value> {
    vec![
        inspect_code_schema(),
        search_code_schema(),
        rename_all_schema(),
        relocate_schema(),
        prune_schema(),
        refactor_schema(),
        workspace_schema(),
    ]
}

/// Check if a tool name is in the public tool list
pub fn is_public_tool(name: &str) -> bool {
    PUBLIC_TOOLS.contains(&name)
}

// =============================================================================
// Tool Schema Definitions
// =============================================================================

/// Schema for `inspect_code` - aggregates code intelligence
pub fn inspect_code_schema() -> Value {
    json!({
        "name": "inspect_code",
        "description": "Aggregate code intelligence for a symbol or position. Returns definition, type info, references, implementations, call hierarchy, and diagnostics based on the 'include' parameter.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "filePath": {
                    "type": "string",
                    "description": "Path to the file containing the symbol"
                },
                "line": {
                    "type": "integer",
                    "description": "0-based line number of the symbol position"
                },
                "character": {
                    "type": "integer",
                    "description": "0-based character offset within the line"
                },
                "symbolName": {
                    "type": "string",
                    "description": "Alternative: symbol name to search for (if line/character not provided)"
                },
                "include": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": ["definition", "typeInfo", "references", "implementations", "callHierarchy", "diagnostics"]
                    },
                    "default": ["definition", "typeInfo"],
                    "description": "Which information to include in the response"
                },
                "detailLevel": {
                    "type": "string",
                    "enum": ["basic", "deep"],
                    "default": "basic",
                    "description": "Level of detail: 'basic' for quick lookups, 'deep' for comprehensive analysis"
                },
                "limit": {
                    "type": "integer",
                    "default": 50,
                    "description": "Maximum number of results for list fields (references, implementations)"
                },
                "offset": {
                    "type": "integer",
                    "default": 0,
                    "description": "Offset for pagination of list fields"
                }
            },
            "anyOf": [
                { "required": ["filePath", "line", "character"] },
                { "required": ["filePath", "symbolName"] }
            ]
        }
    })
}

/// Schema for `search_code` - workspace symbol search
pub fn search_code_schema() -> Value {
    json!({
        "name": "search_code",
        "description": "Search for symbols across the workspace. Supports fuzzy matching and filtering by kind.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query string (supports fuzzy matching)"
                },
                "kind": {
                    "type": "string",
                    "enum": ["function", "class", "interface", "variable", "constant", "module", "type", "enum", "struct", "trait", "method", "property"],
                    "description": "Filter results by symbol kind"
                },
                "workspacePath": {
                    "type": "string",
                    "description": "Root path to search within (defaults to project root)"
                },
                "limit": {
                    "type": "integer",
                    "default": 50,
                    "description": "Maximum number of results to return"
                },
                "offset": {
                    "type": "integer",
                    "default": 0,
                    "description": "Offset for pagination"
                }
            },
            "required": ["query"]
        }
    })
}

/// Schema for `rename_all` - unified rename operation
pub fn rename_all_schema() -> Value {
    json!({
        "name": "rename_all",
        "description": "Rename a symbol, file, or directory with automatic reference updates across the codebase.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "target": {
                    "type": "object",
                    "description": "The target to rename",
                    "properties": {
                        "kind": {
                            "type": "string",
                            "enum": ["symbol", "file", "directory"],
                            "description": "Type of target to rename"
                        },
                        "filePath": {
                            "type": "string",
                            "description": "Path to the file or directory, or file containing the symbol"
                        },
                        "line": {
                            "type": "integer",
                            "description": "0-based line number (required for symbol rename)"
                        },
                        "character": {
                            "type": "integer",
                            "description": "0-based character offset (required for symbol rename)"
                        }
                    },
                    "required": ["kind", "filePath"]
                },
                "newName": {
                    "type": "string",
                    "description": "New name for the target (for files/directories, can be a path)"
                },
                "options": {
                    "type": "object",
                    "properties": {
                        "dryRun": {
                            "type": "boolean",
                            "default": true,
                            "description": "Preview changes without applying (default: true for safety)"
                        },
                        "scope": {
                            "type": "string",
                            "enum": ["code", "standard", "comments", "everything"],
                            "default": "standard",
                            "description": "What to update: code (imports only), standard (code+docs+configs), comments (standard+code comments), everything (all text)"
                        }
                    }
                }
            },
            "required": ["target", "newName"]
        }
    })
}

/// Schema for `relocate` - move symbols, files, or directories
pub fn relocate_schema() -> Value {
    json!({
        "name": "relocate",
        "description": "Move a symbol, file, or directory to a new location with automatic import updates.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "target": {
                    "type": "object",
                    "description": "The target to move",
                    "properties": {
                        "kind": {
                            "type": "string",
                            "enum": ["symbol", "file", "directory"],
                            "description": "Type of target to move"
                        },
                        "filePath": {
                            "type": "string",
                            "description": "Path to the file or directory, or file containing the symbol"
                        },
                        "line": {
                            "type": "integer",
                            "description": "0-based line number (required for symbol move)"
                        },
                        "character": {
                            "type": "integer",
                            "description": "0-based character offset (required for symbol move)"
                        }
                    },
                    "required": ["kind", "filePath"]
                },
                "destination": {
                    "type": "string",
                    "description": "Destination path (file path for symbols, directory path for files/directories)"
                },
                "options": {
                    "type": "object",
                    "properties": {
                        "dryRun": {
                            "type": "boolean",
                            "default": true,
                            "description": "Preview changes without applying (default: true for safety)"
                        }
                    }
                }
            },
            "required": ["target", "destination"]
        }
    })
}

/// Schema for `prune` - delete with cleanup
pub fn prune_schema() -> Value {
    json!({
        "name": "prune",
        "description": "Delete a symbol, file, or directory with automatic cleanup of imports and references.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "target": {
                    "type": "object",
                    "description": "The target to delete",
                    "properties": {
                        "kind": {
                            "type": "string",
                            "enum": ["symbol", "file", "directory"],
                            "description": "Type of target to delete"
                        },
                        "filePath": {
                            "type": "string",
                            "description": "Path to the file or directory, or file containing the symbol"
                        },
                        "line": {
                            "type": "integer",
                            "description": "0-based line number (required for symbol delete)"
                        },
                        "character": {
                            "type": "integer",
                            "description": "0-based character offset (required for symbol delete)"
                        }
                    },
                    "required": ["kind", "filePath"]
                },
                "options": {
                    "type": "object",
                    "properties": {
                        "dryRun": {
                            "type": "boolean",
                            "default": true,
                            "description": "Preview changes without applying (default: true for safety)"
                        },
                        "cleanupImports": {
                            "type": "boolean",
                            "default": true,
                            "description": "Remove orphaned imports after deletion"
                        },
                        "force": {
                            "type": "boolean",
                            "default": false,
                            "description": "Force deletion even if target has dependents"
                        },
                        "removeTests": {
                            "type": "boolean",
                            "default": false,
                            "description": "Also remove associated test files/functions"
                        }
                    }
                }
            },
            "required": ["target"]
        }
    })
}

/// Schema for `refactor` - extract, inline, and transform operations
pub fn refactor_schema() -> Value {
    json!({
        "name": "refactor",
        "description": "Perform code refactoring operations: extract (function, variable, constant, module), inline (variable, function, constant), or transform.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["extract", "inline", "transform"],
                    "description": "The refactoring action to perform"
                },
                "params": {
                    "type": "object",
                    "description": "Action-specific parameters",
                    "properties": {
                        "kind": {
                            "type": "string",
                            "enum": ["function", "variable", "constant", "module"],
                            "description": "For extract/inline: the kind of code element"
                        },
                        "filePath": {
                            "type": "string",
                            "description": "Path to the source file"
                        },
                        "range": {
                            "type": "object",
                            "description": "Code range to extract (for extract action)",
                            "properties": {
                                "startLine": { "type": "integer", "description": "0-based start line" },
                                "startCharacter": { "type": "integer", "description": "0-based start character" },
                                "endLine": { "type": "integer", "description": "0-based end line" },
                                "endCharacter": { "type": "integer", "description": "0-based end character" }
                            },
                            "required": ["startLine", "startCharacter", "endLine", "endCharacter"]
                        },
                        "line": {
                            "type": "integer",
                            "description": "0-based line number (for inline action)"
                        },
                        "character": {
                            "type": "integer",
                            "description": "0-based character offset (for inline action)"
                        },
                        "name": {
                            "type": "string",
                            "description": "Name for the extracted element (for extract action)"
                        },
                        "destination": {
                            "type": "string",
                            "description": "Destination file path (for extract module)"
                        }
                    },
                    "required": ["kind", "filePath"]
                },
                "options": {
                    "type": "object",
                    "properties": {
                        "dryRun": {
                            "type": "boolean",
                            "default": true,
                            "description": "Preview changes without applying (default: true for safety)"
                        }
                    }
                }
            },
            "required": ["action", "params"]
        }
    })
}

/// Schema for `workspace` - package and workspace operations
pub fn workspace_schema() -> Value {
    json!({
        "name": "workspace",
        "description": "Workspace-level operations: create packages, extract dependencies, find/replace, update members, or verify project health.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["create_package", "extract_dependencies", "find_replace", "update_members", "verify_project"],
                    "description": "The workspace action to perform"
                },
                "params": {
                    "type": "object",
                    "description": "Action-specific parameters",
                    "properties": {
                        "language": {
                            "type": "string",
                            "enum": ["rust", "typescript", "python"],
                            "description": "For create_package: the package language"
                        },
                        "name": {
                            "type": "string",
                            "description": "For create_package: the package name"
                        },
                        "path": {
                            "type": "string",
                            "description": "For create_package: the package path"
                        },
                        "template": {
                            "type": "string",
                            "enum": ["lib", "bin", "hybrid"],
                            "description": "For create_package: package template type"
                        },
                        "filePath": {
                            "type": "string",
                            "description": "For extract_dependencies: the file to analyze"
                        },
                        "pattern": {
                            "type": "string",
                            "description": "For find_replace: the pattern to search for"
                        },
                        "replacement": {
                            "type": "string",
                            "description": "For find_replace: the replacement text"
                        },
                        "mode": {
                            "type": "string",
                            "enum": ["literal", "regex", "case_preserving"],
                            "default": "literal",
                            "description": "For find_replace: matching mode"
                        },
                        "glob": {
                            "type": "string",
                            "description": "For find_replace: glob pattern to filter files"
                        },
                        "action": {
                            "type": "string",
                            "enum": ["add", "remove", "list"],
                            "description": "For update_members: the operation to perform"
                        },
                        "workspaceManifest": {
                            "type": "string",
                            "description": "For update_members: path to workspace Cargo.toml"
                        },
                        "members": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "For update_members: member paths to add/remove"
                        }
                    }
                },
                "options": {
                    "type": "object",
                    "properties": {
                        "dryRun": {
                            "type": "boolean",
                            "default": true,
                            "description": "Preview changes without applying (default: true for safety)"
                        }
                    }
                }
            },
            "required": ["action"]
        }
    })
}

// =============================================================================
// Shared Response Envelope Types
// =============================================================================

/// Standard response envelope for write operations
/// Used by rename_all, relocate, prune, refactor, and workspace (write actions)
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteResponse {
    /// Operation status
    pub status: WriteStatus,
    /// Human-readable summary
    pub summary: String,
    /// List of files that were changed
    pub files_changed: Vec<String>,
    /// Diagnostic messages (warnings, errors)
    pub diagnostics: Vec<Diagnostic>,
    /// Optional structured changes (plan or result)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changes: Option<serde_json::Value>,
}

/// Status of a write operation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WriteStatus {
    Success,
    Error,
    Preview,
}

/// Diagnostic message
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

/// Diagnostic severity level
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

impl WriteResponse {
    /// Create a success response
    pub fn success(summary: impl Into<String>, files_changed: Vec<String>) -> Self {
        Self {
            status: WriteStatus::Success,
            summary: summary.into(),
            files_changed,
            diagnostics: Vec::new(),
            changes: None,
        }
    }

    /// Create a preview response (dryRun mode)
    pub fn preview(
        summary: impl Into<String>,
        files_changed: Vec<String>,
        changes: serde_json::Value,
    ) -> Self {
        Self {
            status: WriteStatus::Preview,
            summary: summary.into(),
            files_changed,
            diagnostics: Vec::new(),
            changes: Some(changes),
        }
    }

    /// Create an error response
    pub fn error(summary: impl Into<String>, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            status: WriteStatus::Error,
            summary: summary.into(),
            files_changed: Vec::new(),
            diagnostics,
            changes: None,
        }
    }

    /// Add a warning diagnostic
    pub fn with_warning(mut self, message: impl Into<String>) -> Self {
        self.diagnostics.push(Diagnostic {
            severity: DiagnosticSeverity::Warning,
            message: message.into(),
            file_path: None,
            line: None,
        });
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_tools_list() {
        assert_eq!(PUBLIC_TOOLS.len(), 7);
        assert!(PUBLIC_TOOLS.contains(&"inspect_code"));
        assert!(PUBLIC_TOOLS.contains(&"search_code"));
        assert!(PUBLIC_TOOLS.contains(&"rename_all"));
        assert!(PUBLIC_TOOLS.contains(&"relocate"));
        assert!(PUBLIC_TOOLS.contains(&"prune"));
        assert!(PUBLIC_TOOLS.contains(&"refactor"));
        assert!(PUBLIC_TOOLS.contains(&"workspace"));
    }

    #[test]
    fn test_is_public_tool() {
        assert!(is_public_tool("inspect_code"));
        assert!(is_public_tool("workspace"));
        assert!(!is_public_tool("find_definition")); // Legacy tool
        assert!(!is_public_tool("rename")); // Legacy tool
    }

    #[test]
    fn test_tool_definitions_count() {
        let definitions = get_all_tool_definitions();
        assert_eq!(definitions.len(), 7);
    }

    #[test]
    fn test_tool_schema_names() {
        let definitions = get_all_tool_definitions();
        let names: Vec<&str> = definitions
            .iter()
            .filter_map(|d| d.get("name").and_then(|n| n.as_str()))
            .collect();

        assert_eq!(names, PUBLIC_TOOLS);
    }

    #[test]
    fn test_inspect_code_schema_structure() {
        let schema = inspect_code_schema();
        assert_eq!(schema["name"], "inspect_code");
        assert!(schema["inputSchema"]["properties"]["filePath"].is_object());
        assert!(schema["inputSchema"]["properties"]["include"].is_object());
    }

    #[test]
    fn test_write_response_serialization() {
        let response =
            WriteResponse::success("Renamed 3 files", vec!["a.rs".into(), "b.rs".into()]);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"success\""));
        assert!(json.contains("\"filesChanged\""));
    }
}
