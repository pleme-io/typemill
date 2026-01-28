use serde_json::{json, Value};

pub(crate) fn tool_definitions() -> Vec<Value> {
    vec![
        // Navigation Tools
        json!({
            "name": "find_definition",
            "description": "Find the definition of a symbol by name and kind in a file. Returns definitions for all matching symbols.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filePath": { "type": "string", "description": "The path to the file" },
                    "symbol_name": { "type": "string", "description": "The name of the symbol" },
                    "symbol_kind": { "type": "string", "description": "The kind of symbol (function, class, variable, method, etc.)" }
                },
                "required": ["filePath", "symbol_name"]
            }
        }),
        json!({
            "name": "find_references",
            "description": "Find all references to a symbol by name and kind in a file. Returns references for all matching symbols.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filePath": { "type": "string", "description": "The path to the file" },
                    "symbol_name": { "type": "string", "description": "The name of the symbol" },
                    "symbol_kind": { "type": "string", "description": "The kind of symbol (function, class, variable, method, etc.)" },
                    "include_declaration": { "type": "boolean", "description": "Whether to include the declaration", "default": true }
                },
                "required": ["filePath", "symbol_name"]
            }
        }),
        json!({
            "name": "find_implementations",
            "description": "Find all implementations of an interface or abstract class. Useful for finding concrete classes that implement an interface.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filePath": { "type": "string", "description": "The path to the file" },
                    "symbol_name": { "type": "string", "description": "The name of the interface or abstract class" },
                    "symbol_kind": { "type": "string", "description": "The kind of symbol (interface, class, etc.)" }
                },
                "required": ["filePath", "symbol_name"]
            }
        }),
        json!({
            "name": "find_type_definition",
            "description": "Find the type definition of a symbol. For variables, this finds the type declaration rather than the variable declaration.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filePath": { "type": "string", "description": "The path to the file" },
                    "symbol_name": { "type": "string", "description": "The name of the symbol" },
                    "symbol_kind": { "type": "string", "description": "The kind of symbol (variable, property, etc.)" }
                },
                "required": ["filePath", "symbol_name"]
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
                    "filePath": { "type": "string", "description": "The path to the file" }
                },
                "required": ["filePath"]
            }
        }),
        // Editing Tools
        json!({
            "name": "organize_imports",
            "description": "Automatically organizes and sorts import statements in a file according to the language-specific conventions. It removes unused imports, groups them, and sorts them.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filePath": { "type": "string", "description": "The absolute path to the file to organize imports for." }
                },
                "required": ["filePath"]
            }
        }),
        json!({
            "name": "get_code_actions",
            "description": "Get available code actions (quick fixes, refactors, organize imports) for a file or specific range. Can apply auto-fixes like removing unused imports, adding missing imports, and organizing imports.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filePath": { "type": "string", "description": "The path to the file" },
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
                "required": ["filePath"]
            }
        }),
        json!({
            "name": "format_document",
            "description": "Format a document using the language server's formatter. Applies consistent code style and formatting rules.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filePath": { "type": "string", "description": "The path to the file to format" },
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
                "required": ["filePath"]
            }
        }),
        // Intelligence Tools
        json!({
            "name": "get_hover",
            "description": "Get hover information (documentation, types, signatures) for a symbol at a specific position. Provides rich context about project-specific APIs and functions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filePath": { "type": "string", "description": "The path to the file" },
                    "line": { "type": "number", "description": "The line number (1-indexed)" },
                    "character": { "type": "number", "description": "The character position in the line (0-indexed)" }
                },
                "required": ["filePath", "line", "character"]
            }
        }),
        json!({
            "name": "get_completions",
            "description": "Get intelligent code completions for a specific position. Returns project-aware suggestions including imports, methods, properties, and context-specific completions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filePath": { "type": "string", "description": "The path to the file" },
                    "line": { "type": "number", "description": "The line number (1-indexed)" },
                    "character": { "type": "number", "description": "The character position in the line (0-indexed)" },
                    "trigger_character": { "type": "string", "description": "Optional trigger character (e.g., \".\", \":\", \">\") that caused the completion request" }
                },
                "required": ["filePath", "line", "character"]
            }
        }),
        json!({
            "name": "get_signature_help",
            "description": "Get function signature help at a specific position. Shows function signatures, parameter information, and documentation for the function being called. Critical for AI agents when generating function calls with correct parameters.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filePath": { "type": "string", "description": "The path to the file" },
                    "line": { "type": "number", "description": "The line number (1-indexed)" },
                    "character": { "type": "number", "description": "The character position in the line (0-indexed)" },
                    "trigger_character": { "type": "string", "description": "Optional trigger character that invoked signature help (e.g., \"(\", \",\")" }
                },
                "required": ["filePath", "line", "character"]
            }
        }),
        // Diagnostics Tools
        json!({
            "name": "get_diagnostics",
            "description": "Get language diagnostics (errors, warnings, hints) for a file. Uses LSP textDocument/diagnostic to pull current diagnostics.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filePath": { "type": "string", "description": "The path to the file to get diagnostics for" }
                },
                "required": ["filePath"]
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
                    "dryRun": { "type": "boolean", "description": "Preview changes without applying them (default: true)" }
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
                    "filePath": { "type": "string", "description": "The path where the new file should be created" },
                    "content": { "type": "string", "description": "Initial content for the file (default: empty string)" },
                    "overwrite": { "type": "boolean", "description": "Whether to overwrite existing file if it exists (default: false)" },
                    "dryRun": { "type": "boolean", "description": "Preview changes without applying them (default: true)" }
                },
                "required": ["filePath"]
            }
        }),
        json!({
            "name": "delete_file",
            "description": "Delete a file and notify relevant LSP servers. Ensures proper LSP workspace synchronization and cleanup for deleted files.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filePath": { "type": "string", "description": "The path to the file to delete" },
                    "force": { "type": "boolean", "description": "Force deletion even if file has uncommitted changes (default: false)" },
                    "dryRun": { "type": "boolean", "description": "Preview changes without applying them (default: true)" }
                },
                "required": ["filePath"]
            }
        }),
        // Call Hierarchy Tools
        json!({
            "name": "prepare_call_hierarchy",
            "description": "Prepare for a call hierarchy request.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filePath": { "type": "string", "description": "The path to the file" },
                    "line": { "type": "number", "description": "The line number (1-indexed)" },
                    "character": { "type": "number", "description": "The character position in the line (0-indexed)" }
                },
                "required": ["filePath", "line", "character"]
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
        // LSP Notification Tools
        json!({
            "name": "notify_file_opened",
            "description": "Notify the LSP server that a file has been opened. This helps ensure the language server is aware of files for proper project indexing and symbol resolution.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filePath": { "type": "string", "description": "Path to the file that was opened" }
                },
                "required": ["filePath"]
            }
        }),
        json!({
            "name": "notify_file_saved",
            "description": "Notify the LSP server that a file has been saved. Triggers on_file_save hooks on all plugins that support the file extension.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filePath": { "type": "string", "description": "The path to the file that was saved" }
                },
                "required": ["filePath"]
            }
        }),
        json!({
            "name": "notify_file_closed",
            "description": "Notify the LSP server that a file has been closed. Triggers on_file_close hooks on all plugins that support the file extension.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filePath": { "type": "string", "description": "The path to the file that was closed" }
                },
                "required": ["filePath"]
            }
        }),
    ]
}
