# CodeBuddy MCP Tools API Reference

**Version:** 1.0.0-rc1
**Last Updated:** 2025-10-04

Complete API documentation for all 44 MCP tools available in CodeBuddy.

---

## Table of Contents

- [Navigation & Intelligence](#navigation--intelligence) (13 tools)
- [Editing & Refactoring](#editing--refactoring) (10 tools)
- [File Operations](#file-operations) (6 tools)
- [Workspace Operations](#workspace-operations) (7 tools)
- [Advanced Operations](#advanced-operations) (2 tools)
- [LSP Lifecycle](#lsp-lifecycle) (3 tools)
- [System & Health](#system--health) (3 tools)
- [Common Patterns](#common-patterns)
- [Error Reference](#error-reference)

---

## Navigation & Intelligence

LSP-based navigation and code intelligence tools (13 tools). Language support depends on configured LSP servers.

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
codebuddy tool find_definition '{"file_path":"src/app.ts","line":15,"character":8}'
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
- Queries ALL active LSP servers concurrently
- Results are merged and deduplicated
- Maximum 10,000 symbols returned
- **Performance:** Fast for specific queries, slower for broad searches (e.g., single letter)

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

LSP-based editing and refactoring operations (10 tools).

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

### `fix_imports`

Convenience wrapper for organizing imports. Delegates to `organize_imports`.

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
- Alias for `organize_imports`
- Removes all unused import types
- Uses language server's organize imports functionality

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
  "file_path": "src/app.ts",    // Required: File path
  "workspace_id": "workspace1"  // Optional: Remote workspace ID
}
```

**Returns:**
```json
{
  "success": true,
  "file_path": "src/app.ts",
  "content": "import React from 'react';\n..."
}
```

**Notes:**
- Returns only file content (no size/lines metadata)
- Supports remote workspace execution via `workspace_id` parameter

---

### `write_file`

Write content to a file.

**Parameters:**
```json
{
  "file_path": "src/app.ts",              // Required: File path
  "content": "// New content",            // Required: Content to write
  "dry_run": false,                       // Optional: Preview operation (default: false)
  "workspace_id": "workspace1"            // Optional: Remote workspace ID
}
```

**Returns (dry_run: false):**
```json
{
  "operation": "write_file",
  "path": "src/app.ts",
  "written": true
}
```

**Returns (dry_run: true):**
```json
{
  "status": "preview",
  "operation": "write_file",
  "path": "src/app.ts",
  "content_size": 256,
  "exists": true
}
```

**Notes:**
- Overwrites existing content completely
- Creates parent directories automatically
- Invalidates AST cache after write
- Uses file locking for atomic safety
- Supports remote workspace execution via `workspace_id` parameter
- `dry_run: true` shows what would be written without making changes
- **Note:** `dry_run` not supported for remote workspace operations

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

**Returns (dry_run: false, success):**
```json
{
  "operation": "delete_file",
  "path": "src/old.ts",
  "deleted": true
}
```

**Returns (dry_run: true, success):**
```json
{
  "status": "preview",
  "operation": "delete_file",
  "path": "src/old.ts",
  "force": false,
  "affected_files": 0
}
```

**Error (file has imports, force: false):**
```json
{
  "error": {
    "code": "E1001",
    "message": "File is imported by 5 other files"
  }
}
```

**Returns (force: true, file doesn't exist):**
```json
{
  "operation": "delete_file",
  "path": "src/old.ts",
  "deleted": false,
  "reason": "not_exists"
}
```

**Notes:**
- **Safety check**: Scans entire workspace for files importing the target file
- Checks run during both `dry_run: true` and actual execution
- Use `force: true` to bypass import safety check
- Returns error (not warning) when file has imports and `force: false`
- Notifies LSP servers after successful deletion

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

List files in a directory with optional glob pattern filtering.

**Parameters:**
```json
{
  "directory": "src",               // Optional: Directory path (default: ".")
  "recursive": true,                // Optional: Recurse subdirectories (default: false)
  "pattern": "*.ts"                 // Optional: Glob pattern filter (e.g., "*.rs", "test_*.py")
}
```

**Returns:**
```json
{
  "success": true,
  "directory": "src",
  "pattern": "*.ts",               // Only present if pattern was specified
  "files": [
    "app.ts",
    "utils.ts",
    "index.ts"
  ]
}
```

**Notes:**
- Returns simple array of filenames/paths
- Pattern supports standard glob syntax: `*` (any characters), `?` (single char), `**` (recursive)
- Examples: `*.rs`, `test_*.py`, `**/*.tsx`

---

## Workspace Operations

Project-wide operations and analysis (7 tools).

### `rename_directory`

Rename a directory and automatically update all imports. Supports special **consolidation mode** for merging Rust crates.

**Parameters:**
```json
{
  "old_path": "src/components",    // Required: Current directory path
  "new_path": "src/ui",            // Required: New directory path
  "dry_run": false,                // Optional: Preview changes (default: false)
  "consolidate": false             // Optional: Rust crate consolidation mode (default: false)
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

**Consolidation Mode (`consolidate: true`):**

When enabled, performs Rust crate consolidation by merging one crate into another:

1. **Moves source code** from `old_path/src/*` to `new_path/*`
2. **Merges dependencies** from source `Cargo.toml` into target crate's `Cargo.toml`
3. **Removes old crate** from workspace members
4. **Updates imports** across workspace (e.g., `use old_crate::*` → `use target_crate::module::*`)
5. **Deletes old crate** directory

**Consolidation Example:**
```bash
# Preview consolidation
codebuddy tool rename_directory '{
  "old_path": "crates/cb-protocol",
  "new_path": "crates/cb-types/src/protocol",
  "consolidate": true,
  "dry_run": true
}'

# Execute consolidation
codebuddy tool rename_directory '{
  "old_path": "crates/cb-protocol",
  "new_path": "crates/cb-types/src/protocol",
  "consolidate": true
}'
```

**Returns (consolidate mode):**
```json
{
  "success": true,
  "operation": "consolidate_rust_package",
  "old_path": "crates/cb-protocol",
  "new_path": "crates/cb-types/src/protocol",
  "files_moved": 15,
  "import_updates": {
    "old_crate": "cb_protocol",
    "new_prefix": "cb_types::protocol",
    "imports_updated": 42,
    "files_modified": 8,
    "modified_files": ["crates/cb-server/src/main.rs", ...]
  },
  "next_steps": "📝 Next step: Add 'pub mod protocol;' to cb_types/src/lib.rs to make the consolidated module public",
  "note": "Consolidation complete! All imports have been automatically updated from 'cb_protocol' to 'cb_types::protocol'."
}
```

**Important: Manual Step Required After Consolidation**

After consolidation completes, you must manually add the module declaration to make it public:

```rust
// In target_crate/src/lib.rs
pub mod protocol;  // Add this line
```

This exposes the consolidated code at the new import path (`target_crate::protocol::*`).

**Notes:**
- **ALWAYS updates imports automatically** (no `update_imports` parameter)
- Processes ALL files in directory recursively
- Updates imports in ALL languages (TypeScript, Python, Go, Rust)
- Moves directory on filesystem first, then updates imports
- Safe for large refactorings
- **Performance:** Processing time scales with number of files in directory and workspace size
- For Cargo packages: Also updates workspace manifests and package dependencies
- **Consolidation is Rust-specific** - only use with Cargo projects

**Standard Rename Example:**
```bash
# Preview changes
codebuddy tool rename_directory '{"old_path":"crates","new_path":"lib","dry_run":true}'

# Apply changes
codebuddy tool rename_directory '{"old_path":"crates","new_path":"lib"}'
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
  "workspace_path": "/project"    // Optional: Workspace root path (default: ".")
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
- **Performance:** Can be slow on large workspaces (queries all LSP servers for symbols and references)

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

### `update_dependency`

Update a single dependency to a specific version.

**Parameters:**
```json
{
  "dependency_name": "react",        // Required: Name of the dependency
  "version": "18.3.0",               // Required: Target version
  "project_path": "/project",        // Optional: Project path (default: current dir)
  "package_manager": "auto"          // Optional: npm|yarn|pnpm|pip|cargo|go (default: auto)
}
```

**Returns:**
```json
{
  "success": true,
  "dependency": "react",
  "old_version": "18.2.0",
  "new_version": "18.3.0",
  "package_manager": "npm"
}
```

---

### `batch_update_dependencies`

Update multiple dependencies in a single operation.

**Parameters:**
```json
{
  "dependencies": [                   // Required: Array of dependency updates
    {
      "name": "react",
      "version": "18.3.0"
    },
    {
      "name": "typescript",
      "version": "5.3.0"
    }
  ],
  "project_path": "/project",        // Optional: Project path (default: current dir)
  "package_manager": "auto"          // Optional: npm|yarn|pnpm|pip|cargo|go (default: auto)
}
```

**Returns:**
```json
{
  "success": true,
  "package_manager": "npm",
  "updated": [
    {"name": "react", "old_version": "18.2.0", "new_version": "18.3.0"},
    {"name": "typescript", "old_version": "5.2.0", "new_version": "5.3.0"}
  ],
  "failed": []
}
```

---

### `extract_module_to_package`

Extract code from a module into a new package (Rust-specific).

**Parameters:**
```json
{
  "source_module": "src/utils.rs",     // Required: Source module path
  "target_package": "my-utils",        // Required: New package name
  "symbols": ["helper_fn", "MyStruct"] // Required: Symbols to extract
}
```

**Returns:**
```json
{
  "success": true,
  "package_created": "my-utils",
  "symbols_moved": 2,
  "files_updated": 5
}
```

**Notes:**
- Rust-specific refactoring operation
- Creates new Cargo package with extracted code
- Updates all references and imports automatically

---

## Advanced Operations

High-level operations combining multiple tools.

> **See also:** [docs/features/WORKFLOWS.md](docs/features/WORKFLOWS.md) - Complete guide to the Intent-Based Workflow Engine, including workflow execution, parameter templating, and built-in workflow recipes.

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

### `batch_execute`

Execute multiple file operations in a single batch with atomic guarantees.

**Parameters:**
```json
{
  "operations": [
    {
      "type": "create_file",
      "path": "src/utils/helper.ts",
      "content": "export const helper = () => {};"
    },
    {
      "type": "rename_file",
      "old_path": "src/old.ts",
      "new_path": "src/new.ts"
    },
    {
      "type": "write_file",
      "path": "src/config.json",
      "content": "{\"version\": \"1.0.0\"}"
    },
    {
      "type": "delete_file",
      "path": "src/deprecated.ts"
    }
  ]
}
```

**Supported Operation Types:**

| Type | Fields | Description |
|------|--------|-------------|
| `create_file` | `path`, `content` (optional) | Create new file with optional content |
| `rename_file` | `old_path`, `new_path` | Rename/move file with import updates |
| `write_file` | `path`, `content` | Write content to file (creates if not exists) |
| `delete_file` | `path` | Delete file |

**Returns:**
```json
{
  "success": true,
  "batch_id": "batch_abc123",
  "operations_queued": 4,
  "results": [
    {
      "operation": "create_file",
      "path": "src/utils/helper.ts",
      "success": true
    },
    {
      "operation": "rename_file",
      "old_path": "src/old.ts",
      "new_path": "src/new.ts",
      "success": true,
      "imports_updated": 3
    },
    {
      "operation": "write_file",
      "path": "src/config.json",
      "success": true
    },
    {
      "operation": "delete_file",
      "path": "src/deprecated.ts",
      "success": true
    }
  ]
}
```

**Error Handling:**
```json
{
  "success": false,
  "batch_id": "batch_abc123",
  "operations_queued": 4,
  "failed_at": 2,
  "error": "File already exists: src/config.json",
  "results": [
    {"operation": "create_file", "success": true},
    {"operation": "rename_file", "success": true},
    {"operation": "write_file", "success": false, "error": "File already exists"}
  ]
}
```

**Example:**
```bash
# Create directory structure and move files
codebuddy tool batch_execute '{
  "operations": [
    {
      "type": "create_file",
      "path": "docs/project/.gitkeep",
      "content": ""
    },
    {
      "type": "rename_file",
      "old_path": "CLAUDE.md",
      "new_path": "docs/project/CLAUDE.md"
    },
    {
      "type": "rename_file",
      "old_path": "MCP_API.md",
      "new_path": "docs/project/MCP_API.md"
    }
  ]
}'
```

**Features:**
- Atomic batch execution (all succeed or all rollback)
- Automatic import updates for `rename_file` operations
- Operations queued and executed sequentially
- Rollback on first failure
- Lock management for concurrent operations

**Limitations:**
- Does not support `rename_directory` (use individual MCP call)
- Maximum 100 operations per batch
- Each operation uses same working directory

**Use Cases:**
- Repository restructuring (move multiple files)
- Build artifact generation (create multiple files)
- Cleanup operations (delete multiple deprecated files)
- Project scaffolding (create directory structure)

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
codebuddy tool achieve_intent '{"intent":"refactor.renameSymbolWithImports","arguments":{...}}'
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

System health monitoring and web fetching (3 tools).

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

### `system_status`

Get basic system operational status.

**Parameters:**
```json
{}  // No parameters required
```

**Returns:**
```json
{
  "status": "ok",
  "uptime_seconds": 3600,
  "message": "System is operational"
}
```

**Notes:**
- Lightweight status check without detailed metrics
- Use `health_check` for comprehensive diagnostics

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
codebuddy tool rename_directory '{"old_path":"src","new_path":"lib","dry_run":true}'

# Apply
codebuddy tool rename_directory '{"old_path":"src","new_path":"lib"}'
```

**Tools with dry_run support:**
- File operations: `create_file`, `write_file`, `delete_file`, `rename_file`
- Directory operations: `rename_directory`
- Refactoring: `rename_symbol`, `rename_symbol_strict`, `extract_function`, `inline_variable`, `extract_variable`

### Safety Parameters

**`force` parameter:**
- Available on: `delete_file`
- Purpose: Bypass safety checks (import dependency analysis)
- Default: `false`
- When `false`: Operations are rejected if they would break imports/references
- When `true`: Operations proceed regardless of dependencies

### Position Indexing

- **Lines**: 1-indexed in user-facing APIs, 0-indexed in LSP protocol
- **Characters**: 0-indexed (always)

### File Paths

- Absolute paths recommended
- Relative paths resolved against workspace root
- Use forward slashes (Unix-style) on all platforms

---

## Error Reference

CodeBuddy uses a standardized error response format across all MCP tools for consistent error handling and programmatic parsing.

### Error Response Structure

When an error occurs, the MCP response will contain an `error` field with the following structure:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -1,
    "message": "Human-readable error message",
    "data": {
      "code": "E1001",
      "message": "Human-readable error message",
      "details": {
        // Optional context-specific information
      }
    }
  }
}
```

### Error Fields

- **`error.code`**: JSON-RPC error code (always `-1` for application errors)
- **`error.message`**: Human-readable error summary
- **`error.data`**: Structured error details
  - **`code`**: Machine-readable error code (e.g., "E1000", "E1001")
  - **`message`**: Detailed error message
  - **`details`**: Optional object with additional context (file paths, line numbers, etc.)

### Standard Error Codes

| Code | Category | Description | HTTP Equivalent |
|------|----------|-------------|-----------------|
| `E1000` | INTERNAL_SERVER_ERROR | Internal server error, unexpected failures | 500 |
| `E1001` | INVALID_REQUEST | Invalid request parameters or malformed input | 400 |
| `E1002` | FILE_NOT_FOUND | File or resource not found | 404 |
| `E1003` | LSP_ERROR | Language Server Protocol error | 500 |
| `E1004` | TIMEOUT | Operation timeout | 408 |
| `E1005` | PERMISSION_DENIED | Permission denied or authentication failure | 403 |
| `E1006` | RESOURCE_NOT_FOUND | Generic resource not found | 404 |
| `E1007` | NOT_SUPPORTED | Operation not supported | 501 |
| `E1008` | INVALID_DATA | Invalid data format or serialization error | 400 |

### Error Examples

#### Invalid Request (E1001)

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -1,
    "message": "Missing required parameter",
    "data": {
      "code": "E1001",
      "message": "Missing 'file_path' parameter",
      "details": {
        "parameter": "file_path",
        "tool": "find_definition"
      }
    }
  }
}
```

#### File Not Found (E1002)

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "error": {
    "code": -1,
    "message": "File does not exist",
    "data": {
      "code": "E1002",
      "message": "File does not exist",
      "details": {
        "path": "/workspace/src/missing.ts"
      }
    }
  }
}
```

#### LSP Server Error (E1003)

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "error": {
    "code": -1,
    "message": "Failed to communicate with language server",
    "data": {
      "code": "E1003",
      "message": "LSP server timeout",
      "details": {
        "extension": "ts",
        "method": "textDocument/definition"
      }
    }
  }
}
```

### Error Handling Best Practices

1. **Check for `error` field**: Always check if the response contains an `error` field
2. **Use error codes**: Parse the `error.data.code` field for programmatic error handling
3. **Display messages**: Use `error.data.message` for user-facing error messages
4. **Log details**: Include `error.data.details` in debug logs for troubleshooting
5. **Retry logic**: Implement exponential backoff for `E1000` and `E1003` errors
6. **Validation**: Check for `E1001` and `E1008` to improve request validation

### Example Client Code

```typescript
async function callMcpTool(toolName: string, args: any) {
  const response = await sendMcpRequest({
    method: "tools/call",
    params: {
      name: toolName,
      arguments: args
    }
  });

  if (response.error) {
    const errorData = response.error.data;
    const errorCode = errorData?.code || "UNKNOWN";

    switch (errorCode) {
      case "E1001":
      case "E1008":
        // Client-side error - fix the request
        throw new ValidationError(errorData.message, errorData.details);

      case "E1002":
      case "E1006":
        // Resource not found - handle gracefully
        return null;

      case "E1003":
      case "E1004":
        // Retry-able server error
        return await retryWithBackoff(() => callMcpTool(toolName, args));

      case "E1000":
      default:
        // Server error - log and notify user
        console.error("Server error:", errorData);
        throw new ServerError(errorData.message);
    }
  }

  return response.result;
}
```

---

## See Also

- [SUPPORT_MATRIX.md](./SUPPORT_MATRIX.md) - Language support matrix
- [docs/architecture/ARCHITECTURE.md](./docs/architecture/ARCHITECTURE.md) - Implementation architecture
- [docs/deployment/USAGE.md](./docs/deployment/USAGE.md) - CLI usage guide
- [.codebuddy/workflows.json](./.codebuddy/workflows.json) - Workflow definitions
