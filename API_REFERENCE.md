# Codebuddy MCP Tools API Reference

**Version:** 1.0.0-rc4
**Last Updated:** 2025-10-08

Your complete guide to all MCP tools available in Codebuddy. Use this reference to understand parameters, return types, and see practical examples for each tool.

---

## Table of Contents

- [Language Support Matrix](#language-support-matrix) - Quick reference
- [Language Plugin Architecture](#language-plugin-architecture) - Capability-based plugin system
- [Navigation & Intelligence](#navigation--intelligence)
- [Editing & Refactoring](#editing--refactoring)
- [Unified Analysis API](#unified-analysis-api) - NEW: Consistent analyze commands
- [Code Analysis](#code-analysis)
- [File Operations](#file-operations)
- [Workspace Operations](#workspace-operations)
- [Advanced Operations](#advanced-operations)
- [System & Health](#system--health)
- [Internal Tools](#internal-tools)
- [Common Patterns](#common-patterns)
- [Error Reference](#error-reference)

---

## Authentication & Multi-Tenancy

Codebuddy now operates in a multi-tenant mode to ensure user isolation and security. All operations that interact with workspaces (e.g., `register_workspace`, `list_workspaces`, `execute_command` in a workspace) are scoped to the authenticated user.

### JWT `user_id` Claim (Required)

To support multi-tenancy, all API requests that affect a user's workspace must include a JSON Web Token (JWT) in the `Authorization` header. This JWT **must** contain a `user_id` claim.

**Example JWT Payload:**
```json
{
  "sub": "api_client",
  "exp": 1732992134,
  "iat": 1732988534,
  "iss": "codebuddy_server",
  "aud": "codebuddy_clients",
  "project_id": "project-123",
  "user_id": "user-abc-456"
}
```

- **`user_id`**: This new claim is **required** for all workspace operations. It uniquely identifies the user and ensures that they can only access their own registered workspaces.

Requests to endpoints like `/workspaces` or `/workspaces/{id}/execute` without a valid JWT containing a `user_id` will be rejected with a `401 Unauthorized` error.

---

## Language Support Matrix

**MCP Tools**

### Navigation & Intelligence (LSP-based)

| Tool | TypeScript/JS | Python | Go | Rust | Java | Swift | C# | Notes |
|------|---------------|--------|-----|------|------|-------|-----|-------|
| `find_definition` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | LSP-based, language server dependent |
| `find_references` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Supports `include_declaration` param |
| `find_implementations` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | For interfaces/abstract classes |
| `find_type_definition` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Find underlying type definitions |
| `search_workspace_symbols` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Queries ALL active LSP servers |
| `prepare_call_hierarchy` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Returns call hierarchy item |
| `get_call_hierarchy_incoming_calls` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Requires item from prepare step |
| `get_call_hierarchy_outgoing_calls` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Requires item from prepare step |
| `get_hover` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Documentation, types, signatures |
| `get_completions` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Project-aware suggestions |
| `get_signature_help` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Parameter information |
| `get_diagnostics` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Errors, warnings, hints |

**Internal tools (not in public API):**
- `get_document_symbols` - Hierarchical symbol structure (replaced by future `analyze.structure`)

### Editing & Refactoring (Unified API)

| Tool | TypeScript/JS | Python | Go | Rust | Java | Swift | C# | Notes |
|------|---------------|--------|-----|------|------|-------|-----|-------|
| `*.plan` / `workspace.apply_edit` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Unified `plan -> apply` API for all refactorings. See details below. |

### Code Analysis (AST-based)

| Tool | TypeScript/JS | Python | Go | Rust | Java | Swift | C# | Notes |
|------|---------------|--------|-----|------|------|-------|-----|-------|
| `suggest_refactoring` | ✅ AST | ✅ AST | ✅ AST | ✅ AST | ✅ AST | ✅ AST | ✅ AST | Pattern-based refactoring suggestions |
| `find_complexity_hotspots` | ✅ AST | ✅ AST | ✅ AST | ✅ AST | ✅ AST | ✅ AST | ✅ AST | Top N most complex functions/classes |

**Internal tools (not in public API):**
- `analyze_complexity` - Cyclomatic complexity metrics (replaced by `analyze.quality`)

**Note:** Legacy internal tools `analyze_project_complexity`, `analyze_imports`, and `find_dead_code` have been removed. Use the Unified Analysis API instead (`analyze.quality`, `analyze.dead_code`, `analyze.dependencies`).

### File Operations

| Tool | TypeScript/JS | Python | Go | Rust | Java | Swift | C# | Notes |
|------|---------------|--------|-----|------|------|-------|-----|-------|
| `create_file` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Notifies LSP servers |
| `read_file` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | With locking |
| `write_file` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Cache invalidation |
| `delete_file` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Checks for imports |
| `rename_file` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | **Auto-updates imports** |
| `list_files` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Respects .gitignore |

### Workspace Operations

| Tool | TypeScript/JS | Python | Go | Rust | Java | Swift | C# | Notes |
|------|---------------|--------|-----|------|------|-------|-----|-------|
| `update_dependencies` | ✅ npm/yarn | ✅ pip | ✅ go mod | ✅ cargo | ✅ mvn | ✅ swift | ✅ nuget | Executes package manager |
| `update_dependency` | ✅ npm/yarn | ✅ pip | ✅ go mod | ✅ cargo | ✅ mvn | ✅ swift | ✅ nuget | Executes package manager |
| `extract_module_to_package` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Multi-language support |

**Internal tools (not in public API):**
- `rename_directory` - Auto-updates imports, Rust crate consolidation

### Advanced & System

| Tool | TypeScript/JS | Python | Go | Rust | Java | Swift | C# | Notes |
|------|---------------|--------|-----|------|------|-------|-----|-------|
| `health_check` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Server status |
| `web_fetch` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | URL content fetching |
| `system_status` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Lightweight server status |

**Internal tools (not in public API):**
- `apply_edits` - Atomic multi-file edits (replaced by `workspace.apply_edit`)
- `batch_execute` - Batch operations (replaced by future `analyze.batch`)

**Note:** Language support depends on configured LSP servers in `.codebuddy/config.json`. LSP-first tools attempt LSP code actions, falling back to AST parsing if unsupported.

---

## Language Plugin Architecture

### Capability-Based Design

Language plugins use a capability-based architecture:

**Core Trait**: `LanguagePlugin` with 9 methods (6 required, 3 default)
- `metadata()` - Language metadata (name, extensions, etc.)
- `parse()` - AST parsing and symbol extraction
- `analyze_manifest()` - Manifest file analysis
- `capabilities()` - Feature flags for optional capabilities
- `import_support()` - Optional ImportSupport trait object
- `workspace_support()` - Optional WorkspaceSupport trait object

**Optional Capability Traits**:
- `ImportSupport` - 6 sync methods for import operations
- `WorkspaceSupport` - 5 sync methods for workspace operations

### Accessing Capabilities

```rust
// Check if plugin supports imports
if let Some(import_support) = plugin.import_support() {
    let imports = import_support.parse_imports(&content);  // Sync call!
}

// Check via capability flags
if plugin.capabilities().workspace {
    if let Some(ws) = plugin.workspace_support() {
        ws.add_workspace_member(&content, &member);  // Sync call!
    }
}
```

### Current Plugin Capabilities

| Plugin     | Import Support | Workspace Support |
|------------|---------------|-------------------|
| Rust       | ✅ Yes         | ✅ Yes            |
| TypeScript | ✅ Yes         | ❌ No             |
| Go         | ✅ Yes         | ❌ No             |
| Python     | ✅ Yes         | ❌ No             |
| Java       | ✅ Yes         | ✅ Yes            |
| Swift      | ✅ Yes         | ❌ No             |
| C#         | ✅ Yes         | ❌ No             |

### Metadata Access Pattern

```rust
// Access language metadata
let plugin = registry.find_by_extension("rs")?;
let name = plugin.metadata().name;           // "Rust"
let exts = plugin.metadata().extensions;     // &["rs"]
let manifest = plugin.metadata().manifest_filename; // "Cargo.toml"
```

### Downcasting for Plugin-Specific Methods

Some methods are implementation-specific and require downcasting:

```rust
use cb_lang_rust::RustPlugin;

if let Some(rust_plugin) = plugin.as_any().downcast_ref::<RustPlugin>() {
    // Access Rust-specific implementation methods
    let imports = rust_plugin.parse_imports(path).await?;
    let manifest = rust_plugin.generate_manifest("my-crate", &deps);
}
```

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
  "symbol_name": "formatDate",          // Optional: Symbol name (LSP can infer from position)
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

**Internal Tool** - Not visible in MCP tools/list. Replaced by Unified Analysis API (future `analyze.structure("symbols")`).

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

## Unified Refactoring API

All refactoring operations now follow a consistent, two-step `plan -> apply` pattern. This approach enhances safety by allowing you to preview changes before they are written to disk.

1.  **`<operation>.plan(...)`**: Generates a refactoring plan. This command is read-only and **never** modifies files. It returns a detailed plan object containing the proposed edits, a summary of changes, and any warnings.
2.  **`workspace.apply_edit(plan)`**: Executes a plan generated by a `.plan()` command. This is the only command that writes changes to the filesystem. It supports atomic application, checksum validation, and automatic rollback on failure.

### Core Plan Structure

All `.plan` commands return a structured plan object. Key fields include:
- `plan_type`: The type of plan (e.g., `RenamePlan`, `ExtractPlan`).
- `edits`: An array of LSP `WorkspaceEdit` objects describing the changes.
- `summary`: A summary of affected, created, and deleted files.
- `warnings`: Potential issues detected during planning (e.g., ambiguous symbols).
- `file_checksums`: SHA-256 hashes of files to be modified, used to prevent applying stale plans.

---

### 1. Rename Operations (`rename.plan`)

Rename a symbol, file, or directory.

**Parameters for `rename.plan`:**
```json
{
  "target": {
    "kind": "symbol" | "file" | "directory",
    "path": "src/lib.rs",
    "selector": {
      "position": { "line": 12, "character": 8 } // For kind: "symbol"
    }
  },
  "new_name": "newName"
}
```

**Returns:** A `RenamePlan` object.

**Example: Rename a symbol**
```bash
codebuddy tool rename.plan '{
  "target": {
    "kind": "symbol",
    "path": "src/app.ts",
    "selector": { "position": { "line": 15, "character": 8 } }
  },
  "new_name": "newUser"
}'
```

**Example: Rename a file**
```bash
codebuddy tool rename.plan '{
  "target": { "kind": "file", "path": "src/old.ts" },
  "new_name": "src/new.ts"
}'
```

---

### 2. Extract Operations (`extract.plan`)

Extract a block of code into a new function, variable, module, etc.

**Parameters for `extract.plan`:**
```json
{
  "kind": "function" | "variable" | "constant",
  "source": {
    "file_path": "src/app.rs",
    "range": { "start": {"line": 10, "character": 4}, "end": {"line": 12, "character": 5} },
    "name": "extracted_item"
  }
}
```

**Returns:** An `ExtractPlan` object.

**Example: Extract to function**
```bash
codebuddy tool extract.plan '{
  "kind": "function",
  "source": {
    "file_path": "src/app.ts",
    "range": { "start": {"line": 25, "character": 8}, "end": {"line": 30, "character": 9} },
    "name": "handleLogin"
  }
}'
```

---

### 3. Inline Operations (`inline.plan`)

Inline a variable, function, or constant.

**Parameters for `inline.plan`:**
```json
{
  "kind": "variable" | "function",
  "target": {
    "file_path": "src/app.rs",
    "position": { "line": 10, "character": 5 }
  },
  "options": {
    "inline_all": false // optional, defaults to false
  }
}
```

**Returns:** An `InlinePlan` object.

**Example: Inline a variable**
```bash
codebuddy tool inline.plan '{
  "kind": "variable",
  "target": {
    "file_path": "src/utils.ts",
    "position": { "line": 15, "character": 12 }
  }
}'
```

---

### 4. Move Operations (`move.plan`)

Move code between files or modules.

**Parameters for `move.plan`:**
```json
{
  "kind": "symbol" | "to_module",
  "source": {
    "file_path": "src/old.rs",
    "position": { "line": 10, "character": 5 }
  },
  "destination": {
    "file_path": "src/new.rs"
  }
}
```

**Returns:** A `MovePlan` object.

**Example: Move a function to another file**
```bash
codebuddy tool move.plan '{
  "kind": "symbol",
  "source": { "file_path": "src/app.ts", "position": { "line": 40, "character": 9 } },
  "destination": { "file_path": "src/utils.ts" }
}'
```

---

### 5. Reorder Operations (`reorder.plan`)

Reorder elements like function parameters or imports.

**Parameters for `reorder.plan`:**
```json
{
  "kind": "parameters" | "imports",
  "target": {
    "file_path": "src/app.rs",
    "position": { "line": 10, "character": 5 } // For parameters
  },
  "options": {
    "strategy": "alphabetical" // For imports
  }
}
```

**Returns:** A `ReorderPlan` object.

**Example: Sort imports alphabetically**
```bash
codebuddy tool reorder.plan '{
  "kind": "imports",
  "target": { "file_path": "src/app.ts" },
  "options": { "strategy": "alphabetical" }
}'
```

---

### 6. Transform Operations (`transform.plan`)

Apply automated transformations, like converting a function to be `async`.

**Parameters for `transform.plan`:**
```json
{
  "kind": "to_async" | "loop_to_iterator",
  "target": {
    "file_path": "src/app.ts",
    "range": { "start": {"line": 10, "character": 0}, "end": {"line": 15, "character": 1} }
  }
}
```

**Returns:** A `TransformPlan` object.

**Example: Convert function to async**
```bash
codebuddy tool transform.plan '{
  "kind": "to_async",
  "target": { "file_path": "src/api.js", "position": { "line": 20, "character": 9 } }
}'
```
---

### 7. Delete Operations (`delete.plan`)

Delete unused code, such as unused imports or dead functions.

**Parameters for `delete.plan`:**
```json
{
  "kind": "unused_imports" | "dead_code",
  "target": {
    "scope": "file" | "workspace",
    "path": "src/app.ts" // file path for file scope, directory for directory scope
  }
}
```

**Returns:** A `DeletePlan` object.

**Example: Remove unused imports from a file**
```bash
codebuddy tool delete.plan '{
  "kind": "unused_imports",
  "target": { "scope": "file", "path": "src/app.ts" }
}'
```

---

### Shared Apply Command (`workspace.apply_edit`)

Executes any refactoring plan.

**Parameters:**
```json
{
  "plan": { /* The full plan object from a *.plan() call */ },
  "options": {
    "validate_checksums": true,    // Optional: Fail if files changed since plan creation (default: true)
    "rollback_on_error": true,     // Optional: Automatically revert changes if an error occurs (default: true)
    "validation": {                // Optional: Run a command after applying and roll back if it fails
      "command": "cargo check --workspace",
      "timeout_seconds": 60
    }
  }
}
```

**Returns:**
```json
{
  "success": true,
  "applied_files": ["src/lib.rs", "src/app.rs"],
  "created_files": ["src/new.rs"],
  "deleted_files": [],
  "validation": { "passed": true, "exit_code": 0 } // If validation was run
}
```

**Example: Applying a rename plan**
```bash
# 1. Generate the plan
PLAN=$(codebuddy tool rename.plan '{...}')

# 2. Apply the plan
codebuddy tool workspace.apply_edit "{\"plan\": $PLAN}"
```

---

### Formatting Plans (Server-Side Utility)

The Rust server/client (`crates/cb-client`) provides a `format_plan` utility for generating human-readable descriptions of refactoring plans. This is a **server-side utility** used for CLI output, logging, and debugging.

**Function Signature:**
```rust
pub fn format_plan(plan: &RefactorPlan) -> String
```

**Usage:**
```rust
use cb_client::format_plan;
use cb_protocol::refactor_plan::RefactorPlan;

// After generating a plan
let plan: RefactorPlan = /* from rename.plan, extract.plan, etc. */;
let description = format_plan(&plan);
println!("{}", description);
// Output: "Renames function across 3 files"
```

**Example Outputs:**
- RenamePlan: `"Renames function across 3 files"`
- ExtractPlan: `"Extracts function into a new declaration in 2 files"`
- InlinePlan: `"Inlines constant in 2 files"`
- MovePlan: `"Moves symbol affecting 3 files"`
- ReorderPlan: `"Reorders parameters in 1 file"`
- TransformPlan: `"Transforms code (to_async) in 2 files"`
- DeletePlan: `"Deletes dead_code from 3 files (2 files removed)"`

**Features:**
- Handles all 7 plan types
- Proper pluralization (file vs. files)
- Reports file creation/deletion when applicable
- Lightweight utility for logging, debugging, and UI display

**Architecture Decision:**
- **Rust only** - No TypeScript/JavaScript implementation needed
- Client libraries in other languages can format plans using the structured data already present in plan objects
- Avoids duplication and maintains a single source of truth
- Plans already contain all necessary data: `summary.affected_files`, `metadata.kind`, etc.

---

## Unified Analysis API

**NEW:** Unified commands for code quality and analysis. These commands follow a consistent `analyze.<category>(kind, scope, options)` pattern and return standardized `AnalysisResult` structures.

**Benefits:**
- Consistent result format across all analysis types
- Actionable suggestions with safety metadata
- Configurable thresholds and filtering
- Integration with refactoring API

**Available Commands:**
- `analyze.quality` - Code quality analysis (complexity, smells, maintainability, readability) ✅ **AVAILABLE**
- `analyze.dead_code` - Unused code detection (imports, symbols, parameters, variables, types, unreachable) ✅ **AVAILABLE**
- `analyze.dependencies` - Dependency analysis (imports, graph, circular, coupling, cohesion, depth) ✅ **AVAILABLE**
- `analyze.structure` - Code structure analysis (symbols, hierarchy, interfaces, inheritance, modules) ✅ **AVAILABLE**
- `analyze.documentation` - Documentation quality (coverage, quality, style, examples, todos) ✅ **AVAILABLE**
- `analyze.tests` - Test analysis (coverage, quality, assertions, organization) ✅ **AVAILABLE**

**Language Support**: Currently supports Rust (.rs) and TypeScript/JavaScript (.ts, .tsx, .js, .jsx). Additional languages (Python, Go, Java, Swift, C#) available in git tag `pre-language-reduction`.

---

### `analyze.quality`

Analyze code quality metrics including complexity, code smells, maintainability, and readability.

**Parameters:**
```json
{
  "kind": "complexity",  // Required: "complexity" | "smells" | "maintainability" | "readability"
  "scope": {             // Optional: Defaults to file scope
    "type": "file",      // "workspace" | "directory" | "file" | "symbol"
    "path": "src/app.rs" // Required: File/directory path
  },
  "options": {           // Optional: Analysis configuration
    "thresholds": {
      "cyclomatic_complexity": 15,  // Default: 15
      "cognitive_complexity": 10,   // Default: 10
      "nesting_depth": 4,           // Default: 4
      "parameter_count": 5,         // Default: 5
      "function_length": 50         // Default: 50
    },
    "severity_filter": null,        // null (all) | "high" | "medium" | "low"
    "limit": 1000,                  // Max findings to return (default: 1000)
    "include_suggestions": true     // Include refactoring suggestions (default: true)
  }
}
```

**Returns:**
```json
{
  "findings": [
    {
      "id": "complexity-1",
      "kind": "complexity_hotspot",
      "severity": "high",
      "location": {
        "file_path": "src/app.rs",
        "range": {
          "start": {"line": 10, "character": 0},
          "end": {"line": 45, "character": 1}
        },
        "symbol": "process_order",
        "symbol_kind": "function"
      },
      "metrics": {
        "cyclomatic_complexity": 25,
        "cognitive_complexity": 18,
        "nesting_depth": 5,
        "parameter_count": 8,
        "line_count": 35
      },
      "message": "Function 'process_order' has high cyclomatic complexity (25)",
      "suggestions": [
        {
          "action": "extract_function",
          "description": "Extract nested conditional block to separate function",
          "estimated_impact": "reduces complexity by ~8 points",
          "safety": "requires_review",
          "confidence": 0.85,
          "reversible": true,
          "refactor_call": {
            "command": "extract.plan",
            "arguments": {
              "kind": "function",
              "source": {
                "file_path": "src/app.rs",
                "range": {
                  "start": {"line": 15, "character": 4},
                  "end": {"line": 23, "character": 5}
                },
                "name": "validate_order"
              }
            }
          }
        }
      ]
    }
  ],
  "summary": {
    "total_findings": 5,
    "returned_findings": 5,
    "has_more": false,
    "by_severity": {"high": 2, "medium": 2, "low": 1},
    "files_analyzed": 1,
    "symbols_analyzed": 12,
    "analysis_time_ms": 234
  },
  "metadata": {
    "category": "quality",
    "kind": "complexity",
    "scope": {"type": "file", "path": "src/app.rs"},
    "language": "rust",
    "timestamp": "2025-10-11T12:00:00Z",
    "thresholds": {
      "cyclomatic_complexity": 15,
      "cognitive_complexity": 10
    }
  }
}
```

**Example:**
```bash
# Analyze complexity in a file
codebuddy tool analyze.quality '{
  "kind": "complexity",
  "scope": {"type": "file", "path": "src/app.rs"}
}'

# With custom thresholds
codebuddy tool analyze.quality '{
  "kind": "complexity",
  "scope": {"type": "file", "path": "src/handlers.rs"},
  "options": {
    "thresholds": {"cyclomatic_complexity": 20},
    "severity_filter": "high"
  }
}'
```

**Supported Kinds (MVP):**
- `"complexity"` ✅ - Cyclomatic and cognitive complexity analysis

**Coming Soon:**
- `"smells"` - Code smell detection (long methods, god classes, magic numbers)
- `"maintainability"` - Overall maintainability metrics
- `"readability"` - Readability issues (nesting, parameter count, length)

**Notes:**
- Reuses proven complexity analysis from `cb_ast::complexity`
- Suggestions include safety metadata for automated refactoring
- All findings link to refactoring commands for closed-loop workflow
- Future kinds will use the same result structure

**Language Support:** All languages with AST support (Rust, TypeScript, Go, Python, Java, Swift, C#)

---

## Code Analysis

AST-based code analysis tools for detecting code smells and optimization opportunities.

### `analyze_complexity`

**Internal Tool** - Not visible in MCP tools/list. **Replaced by `analyze.quality("complexity")` ✅ (MVP available now)**. Previously named `analyze_code`.

Calculate comprehensive complexity and code quality metrics for functions in a file.

**Parameters:**
```json
{
  "file_path": "src/business-logic.ts"    // Required: File path
}
```

**Returns:**
```json
{
  "file_path": "src/business-logic.ts",
  "functions": [
    {
      "name": "processOrder",
      "line": 15,
      "cyclomatic": 8,
      "cognitive": 12,
      "max_nesting_depth": 3,
      "sloc": 28,
      "total_lines": 32,
      "comment_lines": 4,
      "comment_ratio": 0.14,
      "parameters": 4,
      "rating": "moderate",
      "issues": [],
      "recommendation": null
    },
    {
      "name": "calculateDiscount",
      "line": 50,
      "cyclomatic": 12,
      "cognitive": 18,
      "max_nesting_depth": 5,
      "sloc": 45,
      "total_lines": 52,
      "comment_lines": 3,
      "comment_ratio": 0.07,
      "parameters": 6,
      "rating": "complex",
      "issues": [
        "High cognitive complexity (18) due to nesting depth (5)",
        "Too many parameters (6 > 5 recommended)",
        "Low comment ratio (0.07) for 45 lines of code"
      ],
      "recommendation": "Consider refactoring to reduce complexity"
    },
    {
      "name": "validatePayment",
      "line": 120,
      "cyclomatic": 20,
      "cognitive": 32,
      "max_nesting_depth": 6,
      "sloc": 68,
      "total_lines": 75,
      "comment_lines": 2,
      "comment_ratio": 0.03,
      "parameters": 8,
      "rating": "verycomplex",
      "issues": [
        "High cognitive complexity (32) due to nesting depth (6)",
        "Too many parameters (8 > 5 recommended)",
        "Deep nesting (6 levels) reduces readability",
        "Low comment ratio (0.03) for 68 lines of code"
      ],
      "recommendation": "Strongly recommended to refactor into smaller functions"
    }
  ],
  "average_complexity": 13.3,
  "average_cognitive_complexity": 20.7,
  "max_complexity": 20,
  "max_cognitive_complexity": 32,
  "total_functions": 8,
  "total_sloc": 340,
  "average_sloc": 42.5,
  "total_issues": 7,
  "summary": "8 functions analyzed. 2 functions need attention (complexity > 10)."
}
```

**Metrics Explained:**

**Complexity Metrics:**
- **cyclomatic**: Traditional cyclomatic complexity (decision points + 1)
- **cognitive**: Cognitive complexity with nesting penalties (more accurate for readability)
- **max_nesting_depth**: Maximum nesting level (includes function braces)

**Code Quality Metrics:**
- **sloc**: Source Lines of Code (excluding blanks and comments)
- **total_lines**: Total lines including blanks and comments
- **comment_lines**: Number of comment lines
- **comment_ratio**: Comment density (comment_lines / sloc)
- **parameters**: Number of function parameters (excludes self/this)

**Complexity Ratings** (based on cognitive complexity):
- **Simple** (1-5): Low risk, easy to test
- **Moderate** (6-10): Manageable complexity
- **Complex** (11-20): Needs attention, harder to test - gets recommendation
- **Very Complex** (21+): High risk, should be refactored - gets strong recommendation

**Automatic Issue Detection:**
- High cognitive complexity (>15)
- Too many parameters (>5)
- Deep nesting (>4 levels)
- Low comment ratio (<0.1 for functions >20 SLOC)

**Algorithms:**

*Cyclomatic Complexity:* CC = decision points + 1
- Decision points: if, else if, for, while, match/case, catch, &&, ||, ? (ternary)

*Cognitive Complexity:* More accurate measure of comprehension difficulty
- Base increment for each decision point (+1)
- Nesting penalty (+1 per nesting level)
- Early returns don't increase complexity (guard clauses are good)
- Example: Nested if at level 3 = +1 (base) + 3 (nesting) = 4

**Comparison Example:**
```javascript
// Cyclomatic: 4, Cognitive: 10 (deeply nested)
function nested(a, b, c) {
    if (a) {           // +1 base, +1 nesting = 2
        if (b) {       // +1 base, +2 nesting = 3
            if (c) {   // +1 base, +3 nesting = 4
                return true;
            }
        }
    }
}

// Cyclomatic: 4, Cognitive: 3 (flat structure)
function flat(a, b, c) {
    if (!a) return false;  // +1 (early return, no penalty)
    if (!b) return false;  // +1
    if (!c) return false;  // +1
    return true;
}
```

**Language Support:** Rust, Go, Java, TypeScript, JavaScript, Python

---

### `suggest_refactoring`

Analyze code and suggest refactoring opportunities based on cognitive complexity metrics and code patterns.

**Parameters:**
```json
{
  "file_path": "src/legacy-code.ts"    // Required: File path
}
```

**Returns:**
```json
{
  "file_path": "src/legacy-code.ts",
  "language": "TypeScript",
  "suggestions": [
    {
      "kind": "reduce_complexity",
      "location": 50,
      "function_name": "processData",
      "description": "Function 'processData': High cognitive complexity (22) due to nesting depth (6)",
      "suggestion": "This function has very high cognitive complexity (22). Consider:\n- Breaking it into smaller functions (extract method pattern)\n- Using early returns to reduce nesting\n- Extracting complex conditional logic into named boolean functions\n- Simplifying nested if statements with guard clauses",
      "priority": "high"
    },
    {
      "kind": "reduce_nesting",
      "location": 50,
      "function_name": "processData",
      "description": "Function 'processData': Deep nesting (6 levels) reduces readability",
      "suggestion": "Reduce nesting depth from 6 to 2-3 levels using:\n- Early returns (guard clauses): if (!condition) return;\n- Extract nested blocks into separate functions\n- Invert conditions to flatten structure\n- Replace nested if-else with strategy pattern or lookup tables",
      "priority": "high"
    },
    {
      "kind": "consolidate_parameters",
      "location": 120,
      "function_name": "handleRequest",
      "description": "Function 'handleRequest': Too many parameters (8 > 5 recommended)",
      "suggestion": "Consolidate 8 parameters using:\n- Create a configuration object/struct grouping related parameters\n- Use the builder pattern for complex initialization\n- Consider if this function is doing too much (Single Responsibility Principle)",
      "priority": "medium"
    },
    {
      "kind": "extract",
      "location": 120,
      "function_name": "handleRequest",
      "description": "Function 'handleRequest' has 85 source lines of code (>50 SLOC recommended)",
      "suggestion": "Consider breaking this function into smaller, more focused functions. Extract logical blocks into separate functions with descriptive names.",
      "priority": "high"
    },
    {
      "kind": "improve_documentation",
      "location": 200,
      "function_name": "calculatePrice",
      "description": "Function 'calculatePrice': Low comment ratio (0.05) for 42 lines of code",
      "suggestion": "Add documentation (current comment ratio: 0.05):\n- Add function/method docstring describing purpose\n- Document parameters and return values\n- Include usage examples for complex functions\n- Explain non-obvious business logic",
      "priority": "low"
    },
    {
      "kind": "replace_magic_number",
      "location": 1,
      "function_name": null,
      "description": "Magic number '86400' appears 4 times",
      "suggestion": "Consider extracting '86400' to a named constant",
      "priority": "medium"
    }
  ],
  "total_suggestions": 6,
  "complexity_report": {
    "average_complexity": 10.5,
    "average_cognitive_complexity": 15.2,
    "max_complexity": 18,
    "max_cognitive_complexity": 22,
    "total_functions": 12,
    "total_sloc": 485,
    "average_sloc": 40.4,
    "total_issues": 15
  }
}
```

**Suggestion Types:**
- **reduce_complexity**: Function has high cognitive complexity (>15) - Provides specific refactoring techniques
- **reduce_nesting**: Function has deep nesting (>4 levels) - Suggests flattening strategies
- **consolidate_parameters**: Too many parameters (>5) - Recommends object/struct consolidation
- **improve_documentation**: Low comment ratio (<0.1) - Guides documentation improvements
- **extract**: Function too long (>50 SLOC) - Suggests using `extract.plan` to break into smaller functions
- **replace_magic_number**: Numeric literal appears multiple times - Recommends named constants
- **remove_duplication**: Duplicate code patterns detected (future enhancement)
- **extract_variable**: Complex expression should be named (future enhancement)

**Priority Levels:**
- **high**: Critical issues (cognitive >20, nesting >4, SLOC >100, parameters >7)
- **medium**: Should address (cognitive 15-20, parameters 5-7, SLOC 50-100)
- **low**: Nice to have (documentation, minor improvements)

**Actionable Suggestions:**
Each suggestion includes specific, actionable advice:
- **ReduceComplexity**: Extract method pattern, early returns, guard clauses, boolean functions
- **ReduceNesting**: Early returns, extract blocks, invert conditions, strategy pattern
- **ConsolidateParameters**: Config objects, builder pattern, SRP evaluation
- **ImproveDocumentation**: Docstrings, parameter docs, examples, business logic explanation

**Notes:**
- Uses **cognitive complexity** (more accurate than cyclomatic) for ratings
- Automatically detects issues from comprehensive code metrics
- Provides **multi-line, detailed suggestions** with bullet points
- Results sorted by priority (high → medium → low)
- Includes enhanced complexity report with SLOC and issue counts

---

### `find_complexity_hotspots`

Find the most complex functions and classes in a project (top N worst offenders). Useful for prioritizing refactoring efforts.

**Parameters:**
```json
{
  "directory_path": "src/",        // Required: Directory to scan
  "limit": 10,                      // Optional: Number of top items (default: 10)
  "metric": "cognitive"             // Optional: "cognitive" or "cyclomatic" (default: "cognitive")
}
```

**Returns:**
```json
{
  "directory": "src/",
  "metric": "cognitive",
  "top_functions": [
    {
      "name": "processComplexData",
      "file_path": "src/processor.ts",
      "line": 145,
      "complexity": 28,
      "cognitive_complexity": 35,
      "rating": "very_complex",
      "sloc": 234
    },
    {
      "name": "validatePayment",
      "file_path": "src/payment.ts",
      "line": 89,
      "complexity": 20,
      "cognitive_complexity": 32,
      "rating": "very_complex",
      "sloc": 156
    }
  ],
  "top_classes": [
    {
      "name": "PaymentProcessor",
      "file_path": "src/payment.ts",
      "line": 25,
      "function_count": 18,
      "total_complexity": 145,
      "total_cognitive_complexity": 178,
      "average_complexity": 8.1,
      "average_cognitive_complexity": 9.9,
      "max_complexity": 28,
      "max_cognitive_complexity": 35,
      "total_sloc": 890,
      "rating": "moderate",
      "issues": []
    }
  ],
  "summary": "Top 10 complexity hotspots identified. 3 very complex functions require immediate refactoring."
}
```

**Sorting:**
- **cognitive metric**: Sorts by cognitive complexity (recommended - better predicts maintenance difficulty)
- **cyclomatic metric**: Sorts by cyclomatic complexity (traditional approach)
- Returns top N functions and top N classes separately

**Hotspot Prioritization:**
- Focuses on "very_complex" functions (cognitive complexity > 20)
- Provides actionable summary of critical issues
- Includes file path and line number for quick navigation

**Use Cases:**
- Sprint planning for technical debt reduction
- Code review focus areas
- Onboarding documentation (avoid complex modules)
- Identifying candidates for extraction/refactoring

**Performance:**
- Sequential file processing (optimized for AST cache)
- Typical performance: 100-500 files in <10 seconds
- Lightweight - only extracts complexity metrics, no full analysis

**Language Support:** All languages with AST support (Rust, Go, Java, TypeScript, JavaScript, Python)

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
- Supports all languages with native plugins (Rust, TypeScript, Go, Python, Java)
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

Project-wide operations and analysis.

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
- Updates imports in ALL languages with native plugins (Rust, TypeScript, Go, Python, Java)
- Moves directory on filesystem first, then updates imports
- Safe for large refactorings
- **Performance:** Processing time scales with number of files in directory and workspace size
- For Cargo packages: Also updates workspace manifests and package dependencies
- For Maven projects: Updates workspace modules in parent pom.xml
- **Consolidation is Rust-specific** - only use with Cargo projects

**Standard Rename Example:**
```bash
# Preview changes
codebuddy tool rename_directory '{"old_path":"crates","new_path":"lib","dry_run":true}'

# Apply changes
codebuddy tool rename_directory '{"old_path":"crates","new_path":"lib"}'
```

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

**Internal Tool** - Not visible in MCP tools/list. Replaced by `workspace.apply_edit` in the Unified Refactoring API. Also known as `execute_edits`.

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

**Internal Tool** - Not visible in MCP tools/list. Will be replaced by Unified Analysis API (future `analyze.batch`). Also known as `execute_batch`.

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

## System & Health

System health monitoring and web fetching.

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

## Internal Tools

**Backend-only tools - Not exposed via MCP `tools/list`**

These 22 internal tools are used by the Codebuddy workflow system and LSP protocol interop. They are **not visible** to AI agents via MCP tool listings, but remain callable by the backend for orchestration and plugin lifecycle management.

**Why hidden:**
- AI agents use the unified refactoring API (e.g., `rename.plan` + `workspace.apply_edit` instead of `rename_symbol_with_imports`)
- Legacy analysis tools replaced by Unified Analysis API (future `analyze.*` commands)
- These tools are implementation details / workflow plumbing
- Simplifies the API surface for AI agent developers

**Internal Tool Categories:**
- **Lifecycle hooks (3)**: notify_file_opened, notify_file_saved, notify_file_closed
- **Legacy editing (1)**: rename_symbol_with_imports
- **Legacy workspace (1)**: apply_workspace_edit
- **Intelligence (2)**: get_completions, get_signature_help
- **Workspace tools (2)**: update_dependencies, update_dependency
- **File operations (4)**: create_file, delete_file, rename_file, rename_directory
- **File utilities (3)**: read_file, write_file, list_files
- **Structure analysis (1)**: get_document_symbols
- **Advanced plumbing (2)**: execute_edits (apply_edits), execute_batch (batch_execute)
- **Migration note**: Legacy internal tools (analyze_project_complexity, analyze_imports, find_dead_code) removed in favor of Unified Analysis API

### `notify_file_opened`

**Purpose:** Notify LSP servers that a file was opened (lifecycle hook)

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

**Usage:** Backend editors/IDEs notify LSP servers to enable proper indexing and plugin hooks

---

### `notify_file_saved`

**Purpose:** Notify LSP servers that a file was saved (lifecycle hook)

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

**Usage:** Backend editors/IDEs notify LSP servers after file saves

---

### `notify_file_closed`

**Purpose:** Notify LSP servers that a file was closed (lifecycle hook)

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

**Usage:** Backend editors/IDEs notify LSP servers to clean up resources

---

### `rename_symbol_with_imports`

**Purpose:** Internal workflow tool combining symbol rename with import updates

**Implementation:** Legacy workflow wrapper - replaced by unified refactoring API

**Parameters:**
```json
{
  "file_path": "src/utils.ts",
  "old_name": "formatDate",
  "new_name": "formatDateTime",
  "dry_run": false
}
```

**Returns:**
```json
{
  "success": true,
  "files_modified": 5,
  "symbols_renamed": 12
}
```

**Usage:** Called by workflow planner when orchestrating multi-step refactorings. AI agents should use the unified refactoring API instead: `rename.plan` followed by `workspace.apply_edit`.

---

### `apply_workspace_edit`

**Purpose:** Apply LSP workspace edits (multi-file refactoring operations)

**Implementation:** Converts LSP `WorkspaceEdit` format to Codebuddy `EditPlan` and applies atomically

**Parameters:**
```json
{
  "changes": {
    "file:///path/to/file1.ts": [
      {
        "range": {
          "start": {"line": 10, "character": 0},
          "end": {"line": 10, "character": 10}
        },
        "newText": "newSymbolName"
      }
    ]
  },
  "dry_run": false
}
```

**Returns:**
```json
{
  "applied": true,
  "files_modified": ["file1.ts", "file2.ts"]
}
```

**Usage:** Called internally when LSP servers return workspace edits for refactoring operations. AI agents should use the unified refactoring API: `*.plan` commands (e.g., `rename.plan`, `extract.plan`, `inline.plan`, `move.plan`) followed by `workspace.apply_edit`.

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

The unified refactoring API uses a two-step **plan-then-apply** pattern for safe refactoring:

```bash
# Step 1: Generate plan (always dry-run, never modifies files)
PLAN=$(codebuddy tool rename.plan '{
  "target": {
    "kind": "symbol",
    "path": "src/app.ts",
    "selector": { "position": { "line": 15, "character": 8 } }
  },
  "new_name": "newUser"
}')

# Step 2: Apply the plan
codebuddy tool workspace.apply_edit "{\"plan\": $PLAN}"

# Optional: Preview final application with dry_run
codebuddy tool workspace.apply_edit "{\"plan\": $PLAN, \"options\": {\"dry_run\": true}}"
```

**File operations with dry_run support:**
```bash
# Preview
codebuddy tool rename_directory '{"old_path":"src","new_path":"lib","dry_run":true}'

# Apply
codebuddy tool rename_directory '{"old_path":"src","new_path":"lib"}'
```

**Tools with dry_run support:**
- **Refactoring (Unified API)**: All `*.plan` commands are always dry-run. Use `workspace.apply_edit` to execute plans.
  - `rename.plan`, `extract.plan`, `inline.plan`, `move.plan`, `reorder.plan`, `transform.plan`, `delete.plan`
  - `workspace.apply_edit` accepts optional `dry_run: true` for final preview before execution
- **File operations**: `create_file`, `write_file`, `delete_file`, `rename_file`
- **Directory operations**: `rename_directory` (including consolidation mode)

### Safety Parameters

**`force` parameter:**
- Available on: `delete_file`
- Purpose: Bypass safety checks (import dependency analysis)
- Default: `false`
- When `false`: Operations are rejected if they would break imports/references
- When `true`: Operations proceed regardless of dependencies

### Position Indexing

All position parameters in this API follow a standard convention to align with common editor user interfaces:

- **Line numbers** (e.g., `line`, `start_line`) are **1-indexed**. The first line in a file is line 1.
- **Character/column positions** (e.g., `character`, `start_character`) are **0-indexed**. The first character on a line is at position 0.

The server handles any necessary conversion to the underlying LSP protocol format internally.

### File Paths

- Absolute paths recommended
- Relative paths resolved against workspace root
- Use forward slashes (Unix-style) on all platforms

---

## Error Reference

Codebuddy uses a standardized error response format across all MCP tools for consistent error handling and programmatic parsing.

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

- [Language Support Matrix](#language-support-matrix) - See above for language support table
- [docs/architecture/overview.md](./docs/architecture/overview.md) - Implementation architecture
- [docs/deployment/DOCKER_DEPLOYMENT.md](./docs/deployment/DOCKER_DEPLOYMENT.md) - Docker deployment guide
- [.codebuddy/workflows.json](./.codebuddy/workflows.json) - Workflow definitions
