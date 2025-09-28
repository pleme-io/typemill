use serde_json::{json, Value};

pub fn get_tool_definitions() -> Vec<Value> {
    vec![
        // Navigation Tools
        json!({
            "name": "find_definition",
            "description": "Find the definition of a symbol by name and kind in a file. Returns definitions for all matching symbols.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "The path to the file" },
                    "symbol_name": { "type": "string", "description": "The name of the symbol" },
                    "symbol_kind": { "type": "string", "description": "The kind of symbol (function, class, variable, method, etc.)" }
                },
                "required": ["file_path", "symbol_name"]
            }
        }),
        json!({
            "name": "find_references",
            "description": "Find all references to a symbol by name and kind in a file. Returns references for all matching symbols.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "The path to the file" },
                    "symbol_name": { "type": "string", "description": "The name of the symbol" },
                    "symbol_kind": { "type": "string", "description": "The kind of symbol (function, class, variable, method, etc.)" },
                    "include_declaration": { "type": "boolean", "description": "Whether to include the declaration", "default": true }
                },
                "required": ["file_path", "symbol_name"]
            }
        }),
        json!({
            "name": "search_workspace_symbols",
            "description": "Search for symbols (functions, classes, variables, etc.) across the entire workspace. Useful for finding symbols by name across multiple files.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query for symbol names (supports partial matching)" },
                    "workspace_path": { "type": "string", "description": "Optional workspace path to search within (defaults to current working directory)" }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "get_document_symbols",
            "description": "Get all symbols (functions, classes, variables, etc.) defined in a specific file. Returns a hierarchical structure of symbols with their locations and types.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "The path to the file" }
                },
                "required": ["file_path"]
            }
        }),

        // Refactoring Tools
        json!({
            "name": "rename_symbol",
            "description": "Rename a symbol by name and kind in a file. If multiple symbols match, returns candidate positions and suggests using rename_symbol_strict. By default, this will apply the rename to the files. Use dry_run to preview changes without applying them.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "The path to the file" },
                    "symbol_name": { "type": "string", "description": "The name of the symbol" },
                    "new_name": { "type": "string", "description": "The new name for the symbol" },
                    "symbol_kind": { "type": "string", "description": "The kind of symbol (function, class, variable, method, etc.)" },
                    "dry_run": { "type": "boolean", "description": "If true, only preview the changes without applying them (default: false)" }
                },
                "required": ["file_path", "symbol_name", "new_name"]
            }
        }),
        json!({
            "name": "rename_symbol_strict",
            "description": "Rename a symbol at a specific position in a file. Use this when rename_symbol returns multiple candidates. By default, this will apply the rename to the files. Use dry_run to preview changes without applying them.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "The path to the file" },
                    "line": { "type": "number", "description": "The line number (1-indexed)" },
                    "character": { "type": "number", "description": "The character position in the line (1-indexed)" },
                    "new_name": { "type": "string", "description": "The new name for the symbol" },
                    "dry_run": { "type": "boolean", "description": "If true, only preview the changes without applying them (default: false)" }
                },
                "required": ["file_path", "line", "character", "new_name"]
            }
        }),

        // Editing Tools
        json!({
            "name": "organize_imports",
            "description": "Automatically organizes and sorts import statements in a file according to the language-specific conventions. It removes unused imports, groups them, and sorts them.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "The absolute path to the file to organize imports for." }
                },
                "required": ["file_path"]
            }
        }),
        json!({
            "name": "get_code_actions",
            "description": "Get available code actions (quick fixes, refactors, organize imports) for a file or specific range. Can apply auto-fixes like removing unused imports, adding missing imports, and organizing imports.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "The path to the file" },
                    "range": {
                        "type": "object",
                        "description": "Optional range to get code actions for. If not provided, gets actions for entire file.",
                        "properties": {
                            "start": {
                                "type": "object",
                                "properties": {
                                    "line": { "type": "number", "description": "Start line (0-indexed)" },
                                    "character": { "type": "number", "description": "Start character (0-indexed)" }
                                },
                                "required": ["line", "character"]
                            },
                            "end": {
                                "type": "object",
                                "properties": {
                                    "line": { "type": "number", "description": "End line (0-indexed)" },
                                    "character": { "type": "number", "description": "End character (0-indexed)" }
                                },
                                "required": ["line", "character"]
                            }
                        },
                        "required": ["start", "end"]
                    }
                },
                "required": ["file_path"]
            }
        }),
        json!({
            "name": "format_document",
            "description": "Format a document using the language server's formatter. Applies consistent code style and formatting rules.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "The path to the file to format" },
                    "options": {
                        "type": "object",
                        "description": "Formatting options",
                        "properties": {
                            "tab_size": { "type": "number", "description": "Size of tabs (default: 2)" },
                            "insert_spaces": { "type": "boolean", "description": "Use spaces instead of tabs (default: true)" },
                            "trim_trailing_whitespace": { "type": "boolean", "description": "Trim trailing whitespace" },
                            "insert_final_newline": { "type": "boolean", "description": "Insert final newline" },
                            "trim_final_newlines": { "type": "boolean", "description": "Trim final newlines" }
                        }
                    }
                },
                "required": ["file_path"]
            }
        }),

        // Intelligence Tools
        json!({
            "name": "get_hover",
            "description": "Get hover information (documentation, types, signatures) for a symbol at a specific position. Provides rich context about project-specific APIs and functions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "The path to the file" },
                    "line": { "type": "number", "description": "The line number (1-indexed)" },
                    "character": { "type": "number", "description": "The character position in the line (0-indexed)" }
                },
                "required": ["file_path", "line", "character"]
            }
        }),
        json!({
            "name": "get_completions",
            "description": "Get intelligent code completions for a specific position. Returns project-aware suggestions including imports, methods, properties, and context-specific completions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "The path to the file" },
                    "line": { "type": "number", "description": "The line number (1-indexed)" },
                    "character": { "type": "number", "description": "The character position in the line (0-indexed)" },
                    "trigger_character": { "type": "string", "description": "Optional trigger character (e.g., \".\", \":\", \">\") that caused the completion request" }
                },
                "required": ["file_path", "line", "character"]
            }
        }),
        json!({
            "name": "get_signature_help",
            "description": "Get function signature help at a specific position. Shows function signatures, parameter information, and documentation for the function being called. Critical for AI agents when generating function calls with correct parameters.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "The path to the file" },
                    "line": { "type": "number", "description": "The line number (1-indexed)" },
                    "character": { "type": "number", "description": "The character position in the line (0-indexed)" },
                    "trigger_character": { "type": "string", "description": "Optional trigger character that invoked signature help (e.g., \"(\", \",\")" }
                },
                "required": ["file_path", "line", "character"]
            }
        }),

        // Diagnostics Tools
        json!({
            "name": "get_diagnostics",
            "description": "Get language diagnostics (errors, warnings, hints) for a file. Uses LSP textDocument/diagnostic to pull current diagnostics.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "The path to the file to get diagnostics for" }
                },
                "required": ["file_path"]
            }
        }),

        // File Management Tools
        json!({
            "name": "rename_file",
            "description": "Rename or move a file and automatically update all import statements that reference it. Works with TypeScript, JavaScript, JSX, and TSX files.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "old_path": { "type": "string", "description": "Current path to the file" },
                    "new_path": { "type": "string", "description": "New path for the file (can be in a different directory)" },
                    "dry_run": { "type": "boolean", "description": "Preview changes without applying them (default: false)" }
                },
                "required": ["old_path", "new_path"]
            }
        }),
        json!({
            "name": "create_file",
            "description": "Create a new file with optional content and notify relevant LSP servers. Ensures proper LSP workspace synchronization for newly created files.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "The path where the new file should be created" },
                    "content": { "type": "string", "description": "Initial content for the file (default: empty string)" },
                    "overwrite": { "type": "boolean", "description": "Whether to overwrite existing file if it exists (default: false)" }
                },
                "required": ["file_path"]
            }
        }),
        json!({
            "name": "delete_file",
            "description": "Delete a file and notify relevant LSP servers. Ensures proper LSP workspace synchronization and cleanup for deleted files.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "The path to the file to delete" },
                    "force": { "type": "boolean", "description": "Force deletion even if file has uncommitted changes (default: false)" }
                },
                "required": ["file_path"]
            }
        }),

        // Call Hierarchy Tools
        json!({
            "name": "prepare_call_hierarchy",
            "description": "Prepare for a call hierarchy request.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "The path to the file" },
                    "line": { "type": "number", "description": "The line number (1-indexed)" },
                    "character": { "type": "number", "description": "The character position in the line (0-indexed)" }
                },
                "required": ["file_path", "line", "character"]
            }
        }),
        json!({
            "name": "get_call_hierarchy_incoming_calls",
            "description": "Get incoming calls for a call hierarchy item.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "item": {
                        "type": "object",
                        "description": "The call hierarchy item"
                    }
                },
                "required": ["item"]
            }
        }),
        json!({
            "name": "get_call_hierarchy_outgoing_calls",
            "description": "Get outgoing calls for a call hierarchy item.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "item": {
                        "type": "object",
                        "description": "The call hierarchy item"
                    }
                },
                "required": ["item"]
            }
        }),

        // System Tools
        json!({
            "name": "list_files",
            "description": "List files and directories in a given path.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to list (defaults to current directory)" },
                    "recursive": { "type": "boolean", "description": "Whether to recursively list subdirectories" },
                    "include_hidden": { "type": "boolean", "description": "Whether to include hidden files" },
                    "pattern": { "type": "string", "description": "Optional pattern to filter files" }
                }
            }
        }),
        json!({
            "name": "analyze_imports",
            "description": "Analyze import statements in a file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Path to the file to analyze" }
                },
                "required": ["file_path"]
            }
        }),
        json!({
            "name": "find_dead_code",
            "description": "Find potentially unused code in a workspace.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "workspace_path": { "type": "string", "description": "Path to the workspace to analyze" }
                },
                "required": ["workspace_path"]
            }
        }),
        json!({
            "name": "update_dependencies",
            "description": "Update project dependencies using the appropriate package manager.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_path": { "type": "string", "description": "Path to the project (defaults to current directory)" },
                    "package_manager": { "type": "string", "description": "Package manager to use (auto, npm, yarn, pnpm, cargo, pip)" },
                    "update_type": { "type": "string", "description": "Type of update (minor, major, patch)" },
                    "dry_run": { "type": "boolean", "description": "Preview changes without applying them" }
                }
            }
        }),
        json!({
            "name": "rename_directory",
            "description": "Rename a directory and optionally update import statements.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "old_path": { "type": "string", "description": "Current directory path" },
                    "new_path": { "type": "string", "description": "New directory path" },
                    "update_imports": { "type": "boolean", "description": "Whether to update import statements" },
                    "dry_run": { "type": "boolean", "description": "Preview changes without applying them" }
                },
                "required": ["old_path", "new_path"]
            }
        }),
        json!({
            "name": "extract_function",
            "description": "Extract a block of code into a new function.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Path to the file" },
                    "start_line": { "type": "number", "description": "Start line of code to extract" },
                    "end_line": { "type": "number", "description": "End line of code to extract" },
                    "function_name": { "type": "string", "description": "Name for the new function" },
                    "dry_run": { "type": "boolean", "description": "Preview changes without applying them" }
                },
                "required": ["file_path", "start_line", "end_line", "function_name"]
            }
        }),
        json!({
            "name": "inline_variable",
            "description": "Inline a variable's value.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Path to the file" },
                    "variable_name": { "type": "string", "description": "Name of the variable to inline" },
                    "line": { "type": "number", "description": "Line number where the variable is declared" },
                    "dry_run": { "type": "boolean", "description": "Preview changes without applying them" }
                },
                "required": ["file_path", "variable_name", "line"]
            }
        }),
        json!({
            "name": "extract_variable",
            "description": "Extract an expression into a new variable.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Path to the file" },
                    "start_line": { "type": "number", "description": "Start line of expression" },
                    "start_character": { "type": "number", "description": "Start character of expression" },
                    "end_line": { "type": "number", "description": "End line of expression" },
                    "end_character": { "type": "number", "description": "End character of expression" },
                    "variable_name": { "type": "string", "description": "Name for the new variable" },
                    "dry_run": { "type": "boolean", "description": "Preview changes without applying them" }
                },
                "required": ["file_path", "start_line", "start_character", "end_line", "end_character", "variable_name"]
            }
        }),
        json!({
            "name": "fix_imports",
            "description": "Fix import statements by removing unused imports and organizing them.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Path to the file" },
                    "dry_run": { "type": "boolean", "description": "Preview changes without applying them" }
                },
                "required": ["file_path"]
            }
        }),

        // Advanced Tools
        json!({
            "name": "apply_workspace_edit",
            "description": "Apply a workspace edit (multi-file text changes) atomically. This is the most powerful editing tool for AI agents, allowing safe modification of multiple files in a single atomic operation with rollback capability. Essential for large refactoring operations.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "changes": {
                        "type": "object",
                        "description": "Map of file URIs/paths to arrays of text edits",
                        "additionalProperties": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "range": {
                                        "type": "object",
                                        "properties": {
                                            "start": {
                                                "type": "object",
                                                "properties": {
                                                    "line": { "type": "number", "description": "Start line (0-indexed)" },
                                                    "character": { "type": "number", "description": "Start character (0-indexed)" }
                                                },
                                                "required": ["line", "character"]
                                            },
                                            "end": {
                                                "type": "object",
                                                "properties": {
                                                    "line": { "type": "number", "description": "End line (0-indexed)" },
                                                    "character": { "type": "number", "description": "End character (0-indexed)" }
                                                },
                                                "required": ["line", "character"]
                                            }
                                        },
                                        "required": ["start", "end"]
                                    },
                                    "newText": { "type": "string", "description": "The new text to replace the range" }
                                },
                                "required": ["range", "newText"]
                            }
                        }
                    },
                    "validate_before_apply": { "type": "boolean", "description": "Whether to validate edit positions before applying (default: true)" }
                },
                "required": ["changes"]
            }
        }),

        // System Tools
        json!({
            "name": "restart_server",
            "description": "Manually restart LSP servers. Can restart servers for specific file extensions or all running servers.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "extensions": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Array of file extensions to restart servers for (e.g., [\"ts\", \"tsx\"]). If not provided, all servers will be restarted."
                    }
                }
            }
        }),
        json!({
            "name": "health_check",
            "description": "Get health status of the LSP servers and system resources. Returns information about active servers, resource usage, and system health.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "include_details": { "type": "boolean", "description": "Include detailed server information (default: false)" }
                }
            }
        }),

        // System Tools (workspace-level operations)
        json!({
            "name": "list_files",
            "description": "List files and directories in a given path.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to list files from (defaults to current directory)" },
                    "recursive": { "type": "boolean", "description": "Whether to list files recursively (default: false)" },
                    "include_hidden": { "type": "boolean", "description": "Include hidden files (default: false)" },
                    "pattern": { "type": "string", "description": "Optional glob pattern to filter files" }
                }
            }
        }),
        json!({
            "name": "analyze_imports",
            "description": "Analyze import statements and dependencies in a source file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Path to the file to analyze" }
                },
                "required": ["file_path"]
            }
        }),
        json!({
            "name": "find_dead_code",
            "description": "Find potentially unused code in a workspace.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "workspace_path": { "type": "string", "description": "Path to the workspace to analyze" }
                },
                "required": ["workspace_path"]
            }
        })
    ]
}