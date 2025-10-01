# CodeBuddy MCP Tools API Reference

**Version:** 0.1.0
**Last Updated:** 2025-10-01

Complete API documentation for all 40 MCP tools available in CodeBuddy.

---

## Table of Contents

- [Navigation & Intelligence](#navigation--intelligence) (13 tools)
- [Editing & Refactoring](#editing--refactoring) (8 tools)
- [File Operations](#file-operations) (6 tools)
- [Workspace Operations](#workspace-operations) (4 tools)
- [Advanced Operations](#advanced-operations) (3 tools)
- [LSP Lifecycle](#lsp-lifecycle) (3 tools)
- [System & Health](#system--health) (1 tool)
- [Web/Network](#webnetwork) (1 tool)
- [Common Patterns](#common-patterns)

---

## Navigation & Intelligence

LSP-based navigation and code intelligence tools. Language support depends on configured LSP servers.

### `find_definition`

Find the definition of a symbol at a specific position.

**Parameters:**
```json
{
  "file_path": "src/index.ts",     // Required: Absolute or relative file path
  "line": 10,                      // Required: Line number (1-indexed)
  "character": 5,                  // Required: Character position (0-indexed)
  "symbol_kind": "function"        // Optional: Symbol type hint (function, class, variable, etc.)
}
```

**Returns:**
```json
{
  "definitions": [
    {
      "uri": "file:///path/to/file.ts",
      "range": {
        "start": {"line": 5, "character": 0},
        "end": {"line": 10, "character": 1}
      }
    }
  ]
}
```

**Example:**
```bash
codebuddy call find_definition '{"file_path":"src/app.ts","line":15,"character":8}'
```

---

### `find_references`

Find all references to a symbol.

**Parameters:**
```json
{
  "file_path": "src/utils.ts",          // Required: File path
  "line": 5,                            // Required: Line number (1-indexed)
  "character": 10,                      // Required: Character position (0-indexed)
  "symbol_name": "formatDate",          // Required: Symbol name
  "include_declaration": true,          // Optional: Include definition (default: true)
  "symbol_kind": "function"             // Optional: Symbol type hint
}
```

**Returns:**
```json
{
  "references": [
    {
      "uri": "file:///path/to/caller.ts",
      "range": {
        "start": {"line": 20, "character": 5},
        "end": {"line": 20, "character": 15}
      }
    }
  ],
  "total": 12
}
```

---

### `search_workspace_symbols`

Search for symbols across the entire workspace.

**Parameters:**
```json
{
  "query": "Button",              // Required: Search query (supports partial matching)
  "workspace_path": "/project"    // Optional: Workspace path (defaults to current directory)
}
```

**Returns:**
```json
{
  "symbols": [
    {
      "name": "Button",
      "kind": "Class",
      "location": {
        "uri": "file:///src/components/Button.tsx",
        "range": {"start": {"line": 5, "character": 0}, "end": {"line": 20, "character": 1}}
      },
      "containerName": "components"
    }
  ],
  "total": 5
}
```

**Notes:**
- Queries ALL active LSP servers
- Results are merged and deduplicated
- Maximum 10,000 symbols returned

---

### `get_document_symbols`

Get hierarchical symbol structure for a file.

**Parameters:**
```json
{
  "file_path": "src/app.ts"    // Required: File path
}
```

**Returns:**
```json
{
  "symbols": [
    {
      "name": "App",
      "kind": "Class",
      "range": {"start": {"line": 10, "character": 0}, "end": {"line": 50, "character": 1}},
      "children": [
        {
          "name": "render",
          "kind": "Method",
          "range": {"start": {"line": 20, "character": 2}, "end": {"line": 30, "character": 3}}
        }
      ]
    }
  ]
}
```

---

### `get_hover`

Get hover information (documentation, types, signatures) at a position.

**Parameters:**
```json
{
  "file_path": "src/utils.ts",    // Required: File path
  "line": 10,                     // Required: Line number (1-indexed)
  "character": 5                  // Required: Character position (0-indexed)
}
```

**Returns:**
```json
{
  "contents": {
    "kind": "markdown",
    "value": "```typescript\nfunction formatDate(date: Date): string\n```\nFormats a date as YYYY-MM-DD"
  },
  "range": {
    "start": {"line": 10, "character": 5},
    "end": {"line": 10, "character": 15}
  }
}
```

---

### `get_completions`

Get intelligent code completions at a specific position.

**Parameters:**
```json
{
  "file_path": "src/app.ts",       // Required: File path
  "line": 15,                      // Required: Line number (1-indexed)
  "character": 10,                 // Required: Character position (0-indexed)
  "trigger_character": "."         // Optional: Trigger character (., :, >)
}
```

**Returns:**
```json
{
  "items": [
    {
      "label": "toString",
      "kind": "Method",
      "detail": "(): string",
      "documentation": "Returns string representation"
    }
  ],
  "isIncomplete": false
}
```

---

### `get_signature_help`

Get function signature help at a position.

**Parameters:**
```json
{
  "file_path": "src/app.ts",       // Required: File path
  "line": 20,                      // Required: Line number (1-indexed)
  "character": 15,                 // Required: Character position (0-indexed)
  "trigger_character": "("         // Optional: Trigger character ((, ,)
}
```

**Returns:**
```json
{
  "signatures": [
    {
      "label": "formatDate(date: Date, format?: string): string",
      "documentation": "Formats a date with optional format string",
      "parameters": [
        {"label": "date", "documentation": "Date to format"},
        {"label": "format", "documentation": "Format string (optional)"}
      ]
    }
  ],
  "activeSignature": 0,
  "activeParameter": 0
}
```

---

### `get_diagnostics`

Get diagnostics (errors, warnings, hints) for a file.

**Parameters:**
```json
{
  "file_path": "src/app.ts"    // Required: File path
}
```

**Returns:**
```json
{
  "diagnostics": [
    {
      "range": {"start": {"line": 10, "character": 5}, "end": {"line": 10, "character": 10}},
      "severity": "Error",
      "message": "Cannot find name 'foo'",
      "code": 2304,
      "source": "typescript"
    }
  ],
  "total": 3
}
```

---

### `prepare_call_hierarchy`

Prepare call hierarchy for a symbol.

**Parameters:**
```json
{
  "file_path": "src/utils.ts",    // Required: File path
  "line": 10,                     // Required: Line number (1-indexed)
  "character": 5                  // Required: Character position (0-indexed)
}
```

**Returns:**
```json
{
  "item": {
    "name": "processData",
    "kind": "Function",
    "uri": "file:///src/utils.ts",
    "range": {"start": {"line": 10, "character": 0}, "end": {"line": 20, "character": 1}},
    "selectionRange": {"start": {"line": 10, "character": 9}, "end": {"line": 10, "character": 20}}
  }
}
```

---

### `get_call_hierarchy_incoming_calls`

Get incoming calls for a call hierarchy item.

**Parameters:**
```json
{
  "item": {
    // Call hierarchy item from prepare_call_hierarchy
    "name": "processData",
    "kind": "Function",
    "uri": "file:///src/utils.ts",
    // ... (full item object)
  }
}
```

**Returns:**
```json
{
  "calls": [
    {
      "from": {
        "name": "handleSubmit",
        "kind": "Function",
        "uri": "file:///src/app.ts"
      },
      "fromRanges": [
        {"start": {"line": 50, "character": 10}, "end": {"line": 50, "character": 21}}
      ]
    }
  ]
}
```

---

### `get_call_hierarchy_outgoing_calls`

Get outgoing calls from a call hierarchy item.

**Parameters:**
```json
{
  "item": {
    // Call hierarchy item from prepare_call_hierarchy
  }
}
```

**Returns:**
```json
{
  "calls": [
    {
      "to": {
        "name": "validateData",
        "kind": "Function",
        "uri": "file:///src/validation.ts"
      },
      "fromRanges": [
        {"start": {"line": 15, "character": 5}, "end": {"line": 15, "character": 17}}
      ]
    }
  ]
}
```

---

### `find_implementations`

Find implementations of an interface or abstract class.

**Parameters:**
```json
{
  "file_path": "src/interfaces.ts",    // Required: File path
  "line": 5,                           // Required: Line number (1-indexed)
  "character": 10                      // Required: Character position (0-indexed)
}
```

**Returns:**
```json
{
  "implementations": [
    {
      "uri": "file:///src/concrete.ts",
      "range": {"start": {"line": 10, "character": 0}, "end": {"line": 30, "character": 1}}
    }
  ]
}
```

---

### `find_type_definition`

Find underlying type definition.

**Parameters:**
```json
{
  "file_path": "src/app.ts",    // Required: File path
  "line": 15,                   // Required: Line number (1-indexed)
  "character": 8                // Required: Character position (0-indexed)
}
```

**Returns:**
```json
{
  "definitions": [
    {
      "uri": "file:///src/types.ts",
      "range": {"start": {"line": 5, "character": 0}, "end": {"line": 10, "character": 1}}
    }
  ]
}
```

---

## Editing & Refactoring

LSP-based editing and refactoring operations.

### `rename_symbol`

Rename a symbol across the project.

**Parameters:**
```json
{
  "file_path": "src/utils.ts",        // Required: File path
  "symbol_name": "oldName",           // Required: Current symbol name
  "new_name": "newName",              // Required: New symbol name
  "symbol_kind": "function",          // Optional: Symbol type (function, class, variable, method)
  "dry_run": false                    // Optional: Preview changes (default: false)
}
```

**Returns (success):**
```json
{
  "changes": {
    "file:///src/utils.ts": [
      {
        "range": {"start": {"line": 5, "character": 9}, "end": {"line": 5, "character": 16}},
        "newText": "newName"
      }
    ],
    "file:///src/app.ts": [
      {
        "range": {"start": {"line": 20, "character": 5}, "end": {"line": 20, "character": 12}},
        "newText": "newName"
      }
    ]
  },
  "files_modified": 2,
  "total_changes": 5
}
```

**Returns (multiple candidates):**
```json
{
  "status": "multiple_candidates",
  "message": "Multiple symbols named 'oldName' found. Use rename_symbol_strict.",
  "candidates": [
    {"line": 5, "character": 9, "kind": "function"},
    {"line": 50, "character": 15, "kind": "variable"}
  ]
}
```

**Notes:**
- May return multiple candidates if symbol name is ambiguous
- Use `rename_symbol_strict` for position-specific rename
- LSP servers automatically update imports

---

### `rename_symbol_strict`

Rename a symbol at a specific position.

**Parameters:**
```json
{
  "file_path": "src/utils.ts",    // Required: File path
  "line": 10,                     // Required: Line number (1-indexed)
  "character": 5,                 // Required: Character position (0-indexed)
  "new_name": "newName",          // Required: New symbol name
  "dry_run": false                // Optional: Preview changes (default: false)
}
```

**Returns:**
```json
{
  "changes": {
    // Same format as rename_symbol
  },
  "files_modified": 3,
  "total_changes": 8
}
```

---

### `organize_imports`

Organize and sort imports according to language conventions.

**Parameters:**
```json
{
  "file_path": "src/app.ts"    // Required: File path
}
```

**Returns:**
```json
{
  "changes": [
    {
      "range": {"start": {"line": 0, "character": 0}, "end": {"line": 5, "character": 0}},
      "newText": "import { Button } from './components/Button';\nimport React from 'react';\n"
    }
  ],
  "imports_removed": 2,
  "imports_sorted": true
}
```

**Notes:**
- Removes unused imports
- Groups imports by type (external, internal, etc.)
- Sorts alphabetically

---

### `get_code_actions`

Get available code actions (quick fixes, refactors) for a file or range.

**Parameters:**
```json
{
  "file_path": "src/app.ts",         // Required: File path
  "range": {                         // Optional: Specific range
    "start": {"line": 10, "character": 0},
    "end": {"line": 15, "character": 0}
  }
}
```

**Returns:**
```json
{
  "actions": [
    {
      "title": "Add missing import",
      "kind": "quickfix",
      "edit": {
        "changes": {
          "file:///src/app.ts": [
            {
              "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 0}},
              "newText": "import { useState } from 'react';\n"
            }
          ]
        }
      }
    },
    {
      "title": "Extract function",
      "kind": "refactor.extract.function"
    }
  ]
}
```

---

### `format_document`

Format a document using the language server's formatter.

**Parameters:**
```json
{
  "file_path": "src/app.ts",         // Required: File path
  "options": {                       // Optional: Formatting options
    "tab_size": 2,                   // Default: 2
    "insert_spaces": true,           // Default: true
    "trim_trailing_whitespace": true,
    "insert_final_newline": true,
    "trim_final_newlines": true
  }
}
```

**Returns:**
```json
{
  "changes": [
    {
      "range": {"start": {"line": 0, "character": 0}, "end": {"line": 100, "character": 0}},
      "newText": "// Formatted code\n..."
    }
  ],
  "formatted": true
}
```

---

### `extract_function`

Extract selected code into a new function.

**Implementation:** LSP-first with AST fallback

**Parameters:**
```json
{
  "file_path": "src/app.ts",      // Required: File path
  "start_line": 10,               // Required: Start line (0-indexed for LSP)
  "end_line": 15,                 // Required: End line (0-indexed for LSP)
  "function_name": "handleClick", // Required: New function name
  "dry_run": false                // Optional: Preview changes (default: false)
}
```

**Returns:**
```json
{
  "changes": {
    "file:///src/app.ts": [
      {
        "range": {"start": {"line": 10, "character": 0}, "end": {"line": 15, "character": 0}},
        "newText": "handleClick();"
      },
      {
        "range": {"start": {"line": 20, "character": 0}, "end": {"line": 20, "character": 0}},
        "newText": "function handleClick() {\n  // extracted code\n}\n"
      }
    ]
  },
  "success": true
}
```

---

### `inline_variable`

Inline a variable's value at all usage sites.

**Implementation:** LSP-first with AST fallback

**Parameters:**
```json
{
  "file_path": "src/app.ts",     // Required: File path
  "variable_name": "tempVar",    // Required: Variable name
  "line": 10,                    // Required: Declaration line (1-indexed)
  "dry_run": false               // Optional: Preview changes (default: false)
}
```

**Returns:**
```json
{
  "changes": {
    "file:///src/app.ts": [
      {
        "range": {"start": {"line": 10, "character": 0}, "end": {"line": 11, "character": 0}},
        "newText": ""
      },
      {
        "range": {"start": {"line": 15, "character": 10}, "end": {"line": 15, "character": 17}},
        "newText": "42"
      }
    ]
  },
  "success": true
}
```

---

### `extract_variable`

Extract an expression into a new variable.

**Implementation:** LSP-first with AST fallback

**Parameters:**
```json
{
  "file_path": "src/app.ts",       // Required: File path
  "start_line": 10,                // Required: Expression start line (0-indexed)
  "start_character": 5,            // Required: Expression start character (0-indexed)
  "end_line": 10,                  // Required: Expression end line (0-indexed)
  "end_character": 20,             // Required: Expression end character (0-indexed)
  "variable_name": "result",       // Required: New variable name
  "dry_run": false                 // Optional: Preview changes (default: false)
}
```

**Returns:**
```json
{
  "changes": {
    "file:///src/app.ts": [
      {
        "range": {"start": {"line": 9, "character": 0}, "end": {"line": 9, "character": 0}},
        "newText": "const result = someExpression();\n"
      },
      {
        "range": {"start": {"line": 10, "character": 5}, "end": {"line": 10, "character": 20}},
        "newText": "result"
      }
    ]
  },
  "success": true
}
```

---

## File Operations

File system operations with LSP awareness and import tracking.

### `create_file`

Create a new file with optional content.

**Parameters:**
```json
{
  "file_path": "src/components/Button.tsx",    // Required: File path
  "content": "export const Button = () => {};", // Optional: Initial content (default: "")
  "overwrite": false,                          // Optional: Overwrite if exists (default: false)
  "dry_run": false                             // Optional: Preview operation (default: false)
}
```

**Returns (dry_run: false):**
```json
{
  "success": true,
  "file_path": "src/components/Button.tsx",
  "created": true
}
```

**Returns (dry_run: true):**
```json
{
  "dry_run": true,
  "result": {
    "status": "preview",
    "operation": "create_file",
    "file_path": "src/components/Button.tsx",
    "would_create": true,
    "content_preview": "export const Button = () => {};"
  }
}
```

**Errors:**
```json
{
  "error": "File already exists",
  "file_path": "src/components/Button.tsx"
}
```

**Notes:**
- Creates parent directories automatically
- Notifies LSP servers of new file

---

### `read_file`

Read file contents.

**Parameters:**
```json
{
  "file_path": "src/app.ts"    // Required: File path
}
```

**Returns:**
```json
{
  "content": "import React from 'react';\n...",
  "file_path": "src/app.ts",
  "size": 1024,
  "lines": 50
}
```

---

### `write_file`

Write content to a file.

**Parameters:**
```json
{
  "file_path": "src/app.ts",              // Required: File path
  "content": "// New content",            // Required: Content to write
  "dry_run": false                        // Optional: Preview operation (default: false)
}
```

**Returns (dry_run: false):**
```json
{
  "success": true,
  "file_path": "src/app.ts",
  "bytes_written": 256
}
```

**Returns (dry_run: true):**
```json
{
  "dry_run": true,
  "result": {
    "status": "preview",
    "operation": "write_file",
    "file_path": "src/app.ts",
    "would_write": true,
    "content_preview": "// New content"
  }
}
```

**Notes:**
- Overwrites existing content
- Invalidates AST cache
- Uses file locking for safety
- `dry_run: true` shows what would be written without making changes

---

### `delete_file`

Delete a file.

**Parameters:**
```json
{
  "file_path": "src/old.ts",    // Required: File path
  "force": false,               // Optional: Force delete even if imported (default: false)
  "dry_run": false              // Optional: Preview operation (default: false)
}
```

**Returns (dry_run: false):**
```json
{
  "success": true,
  "file_path": "src/old.ts",
  "deleted": true
}
```

**Returns (dry_run: true):**
```json
{
  "dry_run": true,
  "result": {
    "status": "preview",
    "operation": "delete_file",
    "file_path": "src/old.ts",
    "would_delete": true,
    "warnings": []
  }
}
```

**Warnings:**
```json
{
  "warning": "File is imported by other files",
  "imported_by": ["src/app.ts", "src/utils.ts"],
  "deleted": false
}
```

**Notes:**
- Checks for imports before deletion unless `force: true`
- Notifies LSP servers
- `dry_run: true` shows what would be deleted without making changes

---

### `rename_file`

Rename a file and automatically update imports.

**Parameters:**
```json
{
  "old_path": "src/utils.ts",       // Required: Current file path
  "new_path": "src/helpers.ts",     // Required: New file path
  "dry_run": false                  // Optional: Preview changes (default: false)
}
```

**Returns (dry_run: true):**
```json
{
  "status": "preview",
  "operation": "rename_file",
  "old_path": "src/utils.ts",
  "new_path": "src/helpers.ts",
  "changes": {
    "file_renamed": true,
    "imports_updated": 5,
    "files_affected": ["src/app.ts", "src/components/Button.tsx"]
  }
}
```

**Returns (dry_run: false):**
```json
{
  "success": true,
  "old_path": "src/utils.ts",
  "new_path": "src/helpers.ts",
  "import_updates": {
    "files_updated": 5,
    "imports_updated": 8,
    "updated_paths": ["src/app.ts", "src/components/Button.tsx"],
    "errors": []
  }
}
```

**Notes:**
- **Automatically updates imports** in all affected files
- Supports TypeScript, JavaScript, Python, Go, Rust
- Creates parent directories if needed

---

### `list_files`

List files in a directory.

**Parameters:**
```json
{
  "path": "src",                    // Optional: Directory path (default: workspace root)
  "recursive": true,                // Optional: Recurse subdirectories (default: false)
  "pattern": "*.ts",                // Optional: Glob pattern filter
  "include_hidden": false           // Optional: Include hidden files (default: false)
}
```

**Returns:**
```json
{
  "files": [
    {
      "path": "src/app.ts",
      "name": "app.ts",
      "size": 1024,
      "is_dir": false,
      "modified": "2025-10-01T10:00:00Z"
    },
    {
      "path": "src/components",
      "name": "components",
      "is_dir": true
    }
  ],
  "total": 25
}
```

**Notes:**
- Respects `.gitignore` patterns
- Supports glob patterns (`*.ts`, `**/*.tsx`)

---

## Workspace Operations

Project-wide operations and analysis.

### `rename_directory`

Rename a directory and automatically update all imports.

**Parameters:**
```json
{
  "old_path": "src/components",    // Required: Current directory path
  "new_path": "src/ui",            // Required: New directory path
  "dry_run": false                 // Optional: Preview changes (default: false)
}
```

**Returns (dry_run: true):**
```json
{
  "status": "preview",
  "operation": "rename_directory",
  "old_path": "src/components",
  "new_path": "src/ui",
  "changes": {
    "files_to_move": 15,
    "imports_to_update": 42,
    "affected_files": ["src/app.ts", "src/pages/Home.tsx"]
  },
  "preview": {
    "files_updated": 12,
    "imports_updated": 42,
    "updated_paths": [...]
  }
}
```

**Returns (dry_run: false):**
```json
{
  "success": true,
  "old_path": "src/components",
  "new_path": "src/ui",
  "files_moved": 15,
  "import_updates": {
    "files_updated": 12,
    "imports_updated": 42,
    "failed_files": 0,
    "updated_paths": ["src/app.ts", "src/pages/Home.tsx"],
    "errors": []
  }
}
```

**Notes:**
- **ALWAYS updates imports automatically** (no `update_imports` parameter)
- Processes ALL files in directory recursively
- Updates imports in ALL languages (TypeScript, Python, Go, Rust)
- Moves directory on filesystem first, then updates imports
- Safe for large refactorings

**Example:**
```bash
# Preview changes
codebuddy call rename_directory '{"old_path":"rust/crates","new_path":"lib","dry_run":true}'

# Apply changes
codebuddy call rename_directory '{"old_path":"rust/crates","new_path":"lib"}'
```

---

### `analyze_imports`

Analyze import statements in a file.

**Implementation:** AST-based parsing (all languages)

**Parameters:**
```json
{
  "file_path": "src/app.ts"    // Required: File path
}
```

**Returns:**
```json
{
  "imports": [
    {
      "source": "react",
      "imported": ["useState", "useEffect"],
      "type": "named",
      "line": 1
    },
    {
      "source": "./components/Button",
      "imported": ["Button"],
      "type": "named",
      "line": 2,
      "is_relative": true
    }
  ],
  "total_imports": 2,
  "external_imports": 1,
  "internal_imports": 1
}
```

**Language Support:**
- TypeScript/JavaScript: SWC parser (Rust native)
- Python: Native AST parser
- Go: `go/parser` (subprocess)
- Rust: `syn` crate

---

### `find_dead_code`

Find potentially unused code in the workspace.

**Implementation:** LSP-based via `workspace/symbol` + `textDocument/references`

**Parameters:**
```json
{
  "workspace_path": "/project"    // Required: Workspace root path
}
```

**Returns:**
```json
{
  "dead_code": [
    {
      "symbol": "unusedFunction",
      "file": "src/utils.ts",
      "line": 42,
      "kind": "function",
      "references": 0
    }
  ],
  "total_symbols_analyzed": 250,
  "potentially_unused": 8
}
```

**Notes:**
- Only finds symbols with zero references
- Does not detect all dead code (e.g., unreachable code paths)
- Works across all LSP-enabled languages

---

### `update_dependencies`

Update project dependencies using package manager.

**Parameters:**
```json
{
  "project_path": "/project",           // Optional: Project path (default: current dir)
  "package_manager": "auto",            // Optional: npm|yarn|pnpm|pip|cargo|go (default: auto)
  "update_type": "minor",               // Optional: major|minor|patch (default: minor)
  "dry_run": false                      // Optional: Preview changes (default: false)
}
```

**Returns:**
```json
{
  "success": true,
  "package_manager": "npm",
  "updates": {
    "updated": ["react@18.2.0", "typescript@5.0.0"],
    "skipped": ["lodash@4.17.21"]
  },
  "stdout": "...",
  "stderr": ""
}
```

**Auto-detection:**
- `package.json` → npm/yarn/pnpm
- `requirements.txt` or `setup.py` → pip
- `Cargo.toml` → cargo
- `go.mod` → go mod

---

## Advanced Operations

High-level operations combining multiple tools.

### `apply_edits`

Apply atomic multi-file edits with rollback on failure.

**Parameters:**
```json
{
  "edit_plan": {
    "source_file": "src/app.ts",
    "edits": [
      {
        "edit_type": "rename",
        "location": {
          "start_line": 10,
          "start_column": 5,
          "end_line": 10,
          "end_column": 15
        },
        "original_text": "oldName",
        "new_text": "newName",
        "priority": 1,
        "description": "Rename variable"
      }
    ],
    "dependency_updates": [
      {
        "target_file": "src/utils.ts",
        "update_type": "import",
        "old_reference": "./app",
        "new_reference": "./application"
      }
    ],
    "validations": [],
    "metadata": {
      "intent_name": "refactor.rename",
      "intent_arguments": {},
      "created_at": "2025-10-01T10:00:00Z",
      "complexity": 5,
      "impact_areas": ["imports", "references"]
    }
  },
  "validate_before_apply": true    // Optional: Validate positions (default: true)
}
```

**Returns:**
```json
{
  "success": true,
  "files_modified": 3,
  "total_edits": 8,
  "rollback_available": false
}
```

**Error (with rollback):**
```json
{
  "success": false,
  "error": "Edit failed at line 10",
  "rollback_performed": true,
  "files_restored": 3
}
```

**Features:**
- Atomic: All edits succeed or all rollback
- File snapshots created before modifications
- AST cache invalidation
- File-level locking

---

### `rename_symbol_with_imports`

High-level workflow combining symbol rename with import updates.

**Implementation:** Workflow-based via `achieve_intent`

**Parameters:**
```json
{
  "intent": "refactor.renameSymbolWithImports",
  "arguments": {
    "file_path": "src/utils.ts",
    "old_name": "formatDate",
    "new_name": "formatDateTime"
  }
}
```

**Workflow Steps:**
1. Calls `rename_symbol` (LSP-based)
2. LSP server generates `WorkspaceEdit` with import updates
3. User confirmation prompt
4. Apply edits atomically

**Returns:**
```json
{
  "workflow_id": "wf_abc123",
  "status": "completed",
  "steps": [
    {"step": "rename_symbol", "status": "completed"},
    {"step": "confirm", "status": "completed"},
    {"step": "apply_edits", "status": "completed"}
  ],
  "result": {
    "files_modified": 5,
    "symbols_renamed": 12
  }
}
```

**Invocation:**
```bash
codebuddy call achieve_intent '{"intent":"refactor.renameSymbolWithImports","arguments":{...}}'
```

---

### `achieve_intent`

Execute high-level workflows via intent-based planning.

**Parameters:**
```json
{
  "intent": "refactor.renameSymbolWithImports",    // Required: Intent name
  "arguments": {                                   // Required: Intent-specific args
    "file_path": "src/app.ts",
    "old_name": "foo",
    "new_name": "bar"
  },
  "resume_workflow_id": null                       // Optional: Resume existing workflow
}
```

**Returns (planning):**
```json
{
  "workflow_id": "wf_abc123",
  "status": "planned",
  "steps": [
    {"step": "rename_symbol", "status": "pending"},
    {"step": "apply_edits", "status": "pending"}
  ],
  "requires_confirmation": true
}
```

**Returns (completed):**
```json
{
  "workflow_id": "wf_abc123",
  "status": "completed",
  "result": {
    "success": true,
    "files_modified": 3
  }
}
```

**Configuration:**
- Workflows defined in `.codebuddy/workflows.json`
- Custom workflows can be added without code changes

---

## LSP Lifecycle

Notify LSP servers of file lifecycle events.

### `notify_file_opened`

Notify LSP servers that a file was opened.

**Parameters:**
```json
{
  "file_path": "src/app.ts"    // Required: File path
}
```

**Returns:**
```json
{
  "success": true,
  "notified_servers": ["typescript"]
}
```

**Notes:**
- Triggers plugin hooks
- Enables proper LSP indexing

---

### `notify_file_saved`

Notify LSP servers that a file was saved.

**Parameters:**
```json
{
  "file_path": "src/app.ts"    // Required: File path
}
```

**Returns:**
```json
{
  "success": true,
  "notified_servers": ["typescript"]
}
```

---

### `notify_file_closed`

Notify LSP servers that a file was closed.

**Parameters:**
```json
{
  "file_path": "src/app.ts"    // Required: File path
}
```

**Returns:**
```json
{
  "success": true,
  "notified_servers": ["typescript"]
}
```

---

## System & Health

### `health_check`

Get server health status and statistics.

**Parameters:**
```json
{
  "include_details": true    // Optional: Include detailed info (default: false)
}
```

**Returns:**
```json
{
  "status": "healthy",
  "uptime_seconds": 3600,
  "version": "0.1.0",
  "timestamp": "2025-10-01T10:00:00Z",
  "lsp_servers": {
    "typescript": "running",
    "python": "running"
  },
  "plugins": {
    "total": 5,
    "active": 5
  },
  "memory_usage": {
    "rss": 45678912,
    "heap": 12345678
  }
}
```

---

## Web/Network

### `web_fetch`

Fetch content from a URL.

**Parameters:**
```json
{
  "url": "https://api.example.com/data",    // Required: URL to fetch
  "method": "GET",                          // Optional: HTTP method (default: GET)
  "headers": {                              // Optional: HTTP headers
    "Authorization": "Bearer token"
  }
}
```

**Returns:**
```json
{
  "content": "Response body text",
  "status_code": 200,
  "headers": {
    "content-type": "application/json"
  }
}
```

**Notes:**
- Returns plain text content
- HTTPS only for security

---

## Common Patterns

### Error Handling

All tools return errors in this format:

```json
{
  "error": "Error message",
  "code": "NOT_FOUND",
  "details": {
    "file_path": "src/missing.ts"
  }
}
```

### Dry-Run Pattern

Tools supporting dry-run preview changes:

```bash
# Preview
codebuddy call rename_directory '{"old_path":"src","new_path":"lib","dry_run":true}'

# Apply
codebuddy call rename_directory '{"old_path":"src","new_path":"lib"}'
```

### Position Indexing

- **Lines**: 1-indexed in user-facing APIs, 0-indexed in LSP protocol
- **Characters**: 0-indexed (always)

### File Paths

- Absolute paths recommended
- Relative paths resolved against workspace root
- Use forward slashes (Unix-style) on all platforms

---

## See Also

- [SUPPORT_MATRIX.md](./SUPPORT_MATRIX.md) - Language support matrix
- [rust/docs/ARCHITECTURE.md](./rust/docs/ARCHITECTURE.md) - Implementation architecture
- [rust/docs/USAGE.md](./rust/docs/USAGE.md) - CLI usage guide
- [.codebuddy/workflows.json](./.codebuddy/workflows.json) - Workflow definitions
