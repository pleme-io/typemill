# Editing & Refactoring Tools

**7 tools with unified dryRun API for safe, reviewable refactoring operations**

The Unified Refactoring API provides a single tool per refactoring operation with an `options.dryRun` parameter:

- **Default behavior (`dryRun: true`):** Generate a plan of changes without modifying files (safe preview mode)
- **Execution mode (`dryRun: false`):** Apply changes immediately with validation and rollback support

All refactoring operations support checksum validation, rollback on error, and post-apply validation.

**Tool count:** 7 tools
**Related categories:** [Navigation](navigation.md), [Analysis](analysis.md), [Workspace](workspace.md)

## Table of Contents

- [Tools](#tools)
  - [rename](#rename)
  - [extract](#extract)
  - [inline](#inline)
  - [move](#move)
  - [reorder](#reorder)
  - [transform](#transform)
  - [delete](#delete)
- [Common Patterns](#common-patterns)
  - [Safe Preview Pattern (Recommended)](#safe-preview-pattern-recommended)
  - [Direct Execution Pattern](#direct-execution-pattern)
  - [Checksum Validation](#checksum-validation)
  - [Post-Apply Validation](#post-apply-validation)
  - [Batch Operations](#batch-operations)
  - [Rust-Specific: Crate Consolidation](#rust-specific-crate-consolidation)
  - [Rust-Specific: File Rename Updates](#rust-specific-file-rename-updates)
  - [Comprehensive Rename Coverage](#comprehensive-rename-coverage)

---

## Tools

### rename

**Purpose:** Rename symbols, files, or directories with automatic import/reference updates.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `target` | `object` | Yes* | Single target to rename (mutually exclusive with `targets`) |
| `targets` | `array` | Yes* | Array of targets for batch rename (mutually exclusive with `target`) |
| `target.kind` | `string` | **Yes** | Type of target: `"symbol"` \| `"file"` \| `"directory"` |
| `target.path` | `string` | **Yes** | Absolute path to file/directory, or file path for symbol |
| `target.line` | `number` | Conditional | Line number (0-indexed) - **required for symbol renames** |
| `target.character` | `number` | Conditional | Column position (0-indexed) - **required for symbol renames** |
| `newName` | `string` | Yes** | New name or path (required for single `target`, optional for `targets`) |
| `destination` | `string` | No | Alternative to `newName` for file/directory moves |
| `options` | `object` | No | Rename options (see options below) |
| `options.dryRun` | `boolean` | No | Preview mode - don't apply changes (**default: true**) |
| `options.consolidate` | `boolean` | No | Enable Rust crate consolidation mode (auto-detected if not specified) |
| `options.scope` | `string` | No | Update scope (**default: "standard"**) - see Scope Values below |

\* Either `target` or `targets` required, not both
\*\* `newName` required when using `target`, optional for batch `targets` (each target can have its own `newName`)

**Returns:**

**Preview mode (`dryRun: true`, default):** Returns a `RenamePlan` object containing:
- `plan_type`: `"RenamePlan"`
- `edits`: LSP `WorkspaceEdit` with all file changes
- `summary`: Counts of affected/created/deleted files
- `warnings`: Array of warnings (e.g., missing LSP server)
- `metadata`: Plan version, kind, language, estimated impact, timestamp
- `file_checksums`: SHA-256 checksums for validation
- `is_consolidation`: Boolean flag for crate consolidation (Rust-specific)

**Execution mode (`dryRun: false`):** Returns an `ExecutionResult` object containing:
- `success`: Boolean indicating operation success
- `applied_files`: Array of file paths that were modified
- `created_files`: Array of newly created file paths
- `deleted_files`: Array of deleted file paths
- `warnings`: Array of warning messages
- `validation`: Optional validation result object
- `rollback_available`: Boolean indicating if rollback is still possible

**Example - Preview mode (safe default):**

```json
// Request - Preview rename (dryRun defaults to true)
{
  "method": "tools/call",
  "params": {
    "name": "rename",
    "arguments": {
      "target": {
        "kind": "file",
        "path": "/workspace/src/utils.rs"
      },
      "newName": "/workspace/src/helpers.rs"
    }
  }
}

// Response - Plan preview (no files modified)
{
  "result": {
    "content": {
      "plan_type": "RenamePlan",
      "edits": {
        "changes": {
          "file:///workspace/src/lib.rs": [
            {
              "range": {
                "start": {"line": 2, "character": 0},
                "end": {"line": 2, "character": 15}
              },
              "newText": "pub mod helpers;"
            }
          ]
        }
      },
      "summary": {
        "affected_files": 3,
        "created_files": 0,
        "deleted_files": 0
      },
      "warnings": [],
      "metadata": {
        "plan_version": "1.0",
        "kind": "rename",
        "language": "rust",
        "estimated_impact": "low",
        "created_at": "2025-10-25T10:30:00Z"
      },
      "file_checksums": {
        "/workspace/src/utils.rs": "a1b2c3...",
        "/workspace/src/lib.rs": "d4e5f6..."
      },
      "is_consolidation": false
    }
  }
}
```text
**Example - Execution mode (explicit opt-in):**

```json
// Request - Execute rename immediately
{
  "method": "tools/call",
  "params": {
    "name": "rename",
    "arguments": {
      "target": {
        "kind": "file",
        "path": "/workspace/src/old.rs"
      },
      "newName": "/workspace/src/new.rs",
      "options": {
        "dryRun": false  // Explicit execution
      }
    }
  }
}

// Response - Execution result
{
  "result": {
    "content": {
      "success": true,
      "applied_files": ["/workspace/src/new.rs", "/workspace/src/lib.rs"],
      "created_files": [],
      "deleted_files": ["/workspace/src/old.rs"],
      "warnings": [],
      "validation": null,
      "rollback_available": false
    }
  }
}
```text
**Parameter Details:**

**Target Types (`target.kind`):**
- `"symbol"` - Rename variable, function, class, interface, type, etc.
  - **Requires:** `line` and `character` to identify symbol location
  - **Updates:** All references across workspace via LSP
- `"file"` - Rename a single file
  - **Requires:** Current file path in `path`, new path in `newName`
  - **Updates:** Imports, module declarations, string paths, docs, configs
- `"directory"` - Rename a directory
  - **Requires:** Current directory path in `path`, new path in `newName`
  - **Updates:** All references in code, docs, configs (comprehensive)

**Options - dryRun (Preview vs Execute):**
- `true` (default) - **Preview mode**: Generate plan, don't modify files
  - Returns `RenamePlan` with edits, summary, warnings, checksums
  - **Safe by default** - forces review before applying
  - No files modified on disk
- `false` - **Execution mode**: Apply changes immediately
  - Returns `ExecutionResult` with success status, modified files
  - Includes checksum validation to prevent applying stale plans
  - Supports automatic rollback on failure

**Common Pitfall:** Forgetting `dryRun: false`!
```json
// This does nothing (just returns a preview):
{
  "target": {"kind": "file", "path": "src/old.ts"},
  "newName": "src/new.ts"
  // options.dryRun defaults to true
}

// This actually applies the change:
{
  "target": {"kind": "file", "path": "src/old.ts"},
  "newName": "src/new.ts",
  "options": {"dryRun": false}  // Don't forget this!
}
```text
**Options - scope (Update Coverage):**

| Scope | Code | Docs | Configs | Comments | Prose | Use Case |
|-------|------|------|---------|----------|-------|----------|
| `"code"` | ✓ | ✗ | ✗ | ✗ | ✗ | Code-only rename |
| `"standard"` (default) | ✓ | ✓ | ✓ | ✗ | ✗ | Recommended for most renames |
| `"comments"` | ✓ | ✓ | ✓ | ✓ | ✗ | Include code comments |
| `"everything"` | ✓ | ✓ | ✓ | ✓ | ✓ | Include markdown prose text |
| `"custom"` | Custom | Custom | Custom | Custom | Custom | Fine-grained control with patterns |

**What each scope updates:**
- **Code**: Imports, module declarations, qualified paths, string literal paths
- **Docs**: Markdown links, inline code references in .md files
- **Configs**: Path values in .toml, .yaml, .json, Cargo.toml workspace members
- **Comments**: Code comments mentioning the old name
- **Prose**: Natural language text in markdown files

**Example - Scope control:**
```json
{
  "target": {"kind": "directory", "path": "old-module"},
  "newName": "new-module",
  "options": {
    "scope": "code",      // Only update code, skip docs/configs
    "dryRun": false
  }
}
```text
**Batch Rename:**
```json
{
  "targets": [
    {"kind": "file", "path": "src/a.rs", "newName": "src/a2.rs"},
    {"kind": "file", "path": "src/b.rs", "newName": "src/b2.rs"}
  ],
  "options": {"dryRun": false}
}
```text
**Notes:**
- **Rust file renames** automatically update module declarations (`pub mod utils;` → `pub mod helpers;`), import statements (`use utils::*`), and qualified paths (`utils::helper()`)
- **Directory renames** update all string literal paths, markdown links, config file paths, and Cargo.toml entries (100% coverage with `scope: "standard"`)
- **Crate consolidation mode** merges dependencies and removes source crate from workspace when renaming into another crate's `src/` directory
- **Checksum validation** prevents applying stale operations after file modifications
- **Safe default:** `dryRun: true` requires explicit `dryRun: false` for execution

---

### extract

**Purpose:** Extract code into functions, variables, constants, or modules.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `kind` | `string` | **Yes** | Extraction type: `"function"` \| `"variable"` \| `"constant"` \| `"module"` |
| `source` | `object` | **Yes** | Source code selection (see source structure) |
| `source.file_path` | `string` | **Yes** | Absolute path to source file |
| `source.range` | `object` | **Yes** | LSP range of code to extract |
| `source.range.start` | `object` | **Yes** | Start position: `{line: number, character: number}` (0-indexed) |
| `source.range.end` | `object` | **Yes** | End position: `{line: number, character: number}` (0-indexed) |
| `source.name` | `string` | **Yes** | Name for extracted element |
| `source.destination` | `string` | No | Destination file path (for module extraction) |
| `options` | `object` | No | Extract options (see options below) |
| `options.dryRun` | `boolean` | No | Preview mode - don't apply changes (**default: true**) |
| `options.visibility` | `string` | No | Visibility modifier: `"public"` \| `"private"` (default: `"private"`) |
| `options.destination_path` | `string` | No | Override destination path |

**Extraction Types:**
- `"function"` - Extract code block into a new function (most common)
- `"variable"` - Extract expression into a variable
- `"constant"` - Extract literal/expression into a constant
- `"module"` - Extract code into a separate module file

**Error Messages:**
- Missing `kind`: "Invalid request: Missing 'kind' parameter"
- Invalid `kind`: "Unsupported kind 'invalid'. Valid: function, variable, constant, module"
- Invalid range: "Range must have valid start and end positions"

**Returns:**

**Preview mode (`dryRun: true`, default):** Returns an `ExtractPlan` object.
**Execution mode (`dryRun: false`):** Returns an `ExecutionResult` object.

**Example - Preview extraction:**

```json
// Request - Preview function extraction
{
  "method": "tools/call",
  "params": {
    "name": "extract",
    "arguments": {
      "kind": "function",
      "source": {
        "file_path": "/workspace/src/calc.rs",
        "range": {
          "start": {"line": 1, "character": 4},
          "end": {"line": 2, "character": 26}
        },
        "name": "compute_sum_doubled"
      },
      "options": {
        "visibility": "private"
      }
    }
  }
}

// Response - Preview (dryRun: true by default)
{
  "result": {
    "content": {
      "plan_type": "ExtractPlan",
      "edits": {
        "changes": {
          "file:///workspace/src/calc.rs": [
            {
              "range": {
                "start": {"line": 1, "character": 4},
                "end": {"line": 2, "character": 26}
              },
              "newText": "compute_sum_doubled(x, y)"
            },
            {
              "range": {
                "start": {"line": 4, "character": 0},
                "end": {"line": 4, "character": 0}
              },
              "newText": "\nfn compute_sum_doubled(x: i32, y: i32) -> i32 {\n    let sum = x + y;\n    sum * 2\n}\n"
            }
          ]
        }
      },
      "summary": {
        "affected_files": 1,
        "created_files": 0,
        "deleted_files": 0
      },
      "warnings": [],
      "metadata": {
        "plan_version": "1.0",
        "kind": "extract",
        "language": "rust",
        "estimated_impact": "low",
        "created_at": "2025-10-25T10:35:00Z"
      },
      "file_checksums": {
        "/workspace/src/calc.rs": "f7g8h9..."
      }
    }
  }
}
```text
**Notes:**
- Uses AST-based refactoring (no LSP required)
- Automatically infers parameters and return types
- Module extraction requires language plugin support
- Preview mode (`dryRun: true`) is the safe default

---

### inline

**Purpose:** Inline variables, functions, or constants by replacing references with definitions.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `kind` | `string` | **Yes** | Inline type: `"variable"` \| `"function"` \| `"constant"` |
| `target` | `object` | **Yes** | Target to inline (see target structure) |
| `target.file_path` | `string` | **Yes** | Absolute path to file containing definition |
| `target.position` | `object` | **Yes** | Position of definition: `{line: number, character: number}` (0-indexed) |
| `options` | `object` | No | Inline options (see options below) |
| `options.dryRun` | `boolean` | No | Preview mode - don't apply changes (**default: true**) |
| `options.inline_all` | `boolean` | No | Inline all usages (true) or current only (**default: false**) |

**Inline Types:**
- `"variable"` - Inline a variable by replacing references with its value
- `"function"` - Inline a function by replacing calls with function body
- `"constant"` - Inline a constant by replacing references with its value

**Error Messages:**
- Missing `kind`: "Invalid request: Missing 'kind' parameter"
- Invalid `kind`: "Unsupported kind 'invalid'. Valid: variable, function, constant"
- Missing position: "Target must include 'position' with line and character"

**Returns:**

**Preview mode (`dryRun: true`, default):** Returns an `InlinePlan` object.
**Execution mode (`dryRun: false`):** Returns an `ExecutionResult` object.

**Example:**

```json
// Request - Preview variable inline
{
  "method": "tools/call",
  "params": {
    "name": "inline",
    "arguments": {
      "kind": "variable",
      "target": {
        "file_path": "/workspace/src/vars.rs",
        "position": {"line": 1, "character": 8}
      },
      "options": {
        "inline_all": true
      }
    }
  }
}

// Response - Plan preview (dryRun: true default)
{
  "result": {
    "content": {
      "plan_type": "InlinePlan",
      "edits": {
        "changes": {
          "file:///workspace/src/vars.rs": [
            {
              "range": {
                "start": {"line": 1, "character": 4},
                "end": {"line": 1, "character": 31}
              },
              "newText": ""
            },
            {
              "range": {
                "start": {"line": 2, "character": 4},
                "end": {"line": 2, "character": 10}
              },
              "newText": "(10 + 5) * 2"
            }
          ]
        }
      },
      "summary": {
        "affected_files": 1,
        "created_files": 0,
        "deleted_files": 0
      },
      "warnings": [],
      "metadata": {
        "plan_version": "1.0",
        "kind": "inline",
        "language": "rust",
        "estimated_impact": "low",
        "created_at": "2025-10-25T10:40:00Z"
      },
      "file_checksums": {
        "/workspace/src/vars.rs": "j1k2l3..."
      }
    }
  }
}
```text
**Notes:**
- Uses AST-based refactoring
- Removes definition after inlining
- Does not create or delete files
- Preview mode is the safe default

---

### move

**Purpose:** Move symbols, files, directories, or code blocks to different locations.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `target` | `object` | **Yes** | Target to move (see target structure) |
| `target.kind` | `string` | **Yes** | Move type: `"symbol"` \| `"file"` \| `"directory"` \| `"module"` |
| `target.path` | `string` | **Yes** | Absolute path to source |
| `target.selector` | `object` | Conditional | Symbol selector - **required for symbol moves** |
| `target.selector.position` | `object` | Conditional | Position: `{line: number, character: number}` (0-indexed) |
| `destination` | `string` | **Yes** | Destination file path or directory |
| `options` | `object` | No | Move options (see options below) |
| `options.dryRun` | `boolean` | No | Preview mode - don't apply changes (**default: true**) |
| `options.update_imports` | `boolean` | No | Update import statements (**default: true**) |
| `options.preserve_formatting` | `boolean` | No | Preserve code formatting (**default: true**) |

**Move Types:**
- `"symbol"` - Move function, class, variable, etc. to different file
  - **Requires:** `selector.position` to identify symbol
  - **Updates:** Imports and references automatically
- `"file"` - Move entire file to different location
  - Similar to rename but explicitly for moves
- `"directory"` - Move directory to different location
  - Updates all references comprehensively
- `"module"` - Move module with dependency updates

**Error Messages:**
- Missing `destination`: "Invalid request: Missing 'destination' parameter"
- Invalid `target.kind`: "Unsupported kind 'invalid'. Valid: symbol, file, directory, module"
- Symbol without position: "Symbol moves require 'selector.position'"

**Returns:**

**Preview mode (`dryRun: true`, default):** Returns a `MovePlan` object.
**Execution mode (`dryRun: false`):** Returns an `ExecutionResult` object.

**Example:**

```json
// Request - Preview function move
{
  "method": "tools/call",
  "params": {
    "name": "move",
    "arguments": {
      "target": {
        "kind": "symbol",
        "path": "/workspace/src/utils.rs",
        "selector": {
          "position": {"line": 5, "character": 4}
        }
      },
      "destination": "/workspace/src/helpers.rs"
    }
  }
}
```text
**Notes:**
- Automatically updates imports and references
- Handles cross-file symbol moves
- File/directory moves update all references
- Module moves require language plugin support

---

### reorder

**Purpose:** Reorder function parameters, struct fields, imports, or statements.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `target` | `object` | **Yes** | Target to reorder (see target structure) |
| `target.kind` | `string` | **Yes** | Reorder type: `"parameters"` \| `"fields"` \| `"imports"` \| `"statements"` |
| `target.file_path` | `string` | **Yes** | Absolute path to file |
| `target.position` | `object` | **Yes** | Position: `{line: number, character: number}` (0-indexed) |
| `new_order` | `array` | **Yes** | Array of strings specifying new order (parameter/field names) |
| `options` | `object` | No | Reorder options (see options below) |
| `options.dryRun` | `boolean` | No | Preview mode - don't apply changes (**default: true**) |
| `options.preserve_formatting` | `boolean` | No | Preserve code formatting (**default: true**) |
| `options.update_call_sites` | `boolean` | No | Update all call sites for parameter reordering (**default: true**) |

**Reorder Types:**
- `"parameters"` - Reorder function/method parameters
  - **Important:** Updates all call sites across workspace when `update_call_sites: true`
  - **Requires:** `new_order` array with parameter names in desired order
- `"fields"` - Reorder struct/class fields
  - Preserves field values, just changes declaration order
- `"imports"` - Reorder import statements
  - Can use LSP "organize imports" feature
- `"statements"` - Reorder statements within a block

**Example `new_order` for parameters:**
```json
{
  "new_order": ["endpoint", "method", "headers", "body"]
}
```text
**Error Messages:**
- Missing `new_order`: "Invalid request: Missing 'new_order' parameter"
- Invalid `target.kind`: "Unsupported kind 'invalid'. Valid: parameters, fields, imports, statements"
- Invalid order: "new_order must contain all existing parameter names"

**Returns:**

**Preview mode (`dryRun: true`, default):** Returns a `ReorderPlan` object.
**Execution mode (`dryRun: false`):** Returns an `ExecutionResult` object.

**Example:**

```json
// Request - Preview parameter reorder
{
  "method": "tools/call",
  "params": {
    "name": "reorder",
    "arguments": {
      "target": {
        "kind": "parameters",
        "file_path": "/workspace/src/api.rs",
        "position": {"line": 10, "character": 8}
      },
      "new_order": ["endpoint", "method", "headers", "body"],
      "options": {
        "update_call_sites": true
      }
    }
  }
}
```text
**Notes:**
- Parameter reordering updates all call sites across the codebase
- Requires LSP server support for best results
- Import reordering uses LSP organize imports feature

---

### transform

**Purpose:** Apply syntax transformations (if-to-match, add/remove async, etc.).

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `transformation` | `object` | **Yes** | Transformation specification (see structure below) |
| `transformation.kind` | `string` | **Yes** | Transformation type (see Transformation Types below) |
| `transformation.file_path` | `string` | **Yes** | Absolute path to file |
| `transformation.range` | `object` | **Yes** | LSP range to transform |
| `transformation.range.start` | `object` | **Yes** | Start position: `{line: number, character: number}` (0-indexed) |
| `transformation.range.end` | `object` | **Yes** | End position: `{line: number, character: number}` (0-indexed) |
| `options` | `object` | No | Transform options (see options below) |
| `options.dryRun` | `boolean` | No | Preview mode - don't apply changes (**default: true**) |
| `options.preserve_formatting` | `boolean` | No | Preserve code formatting (**default: true**) |
| `options.preserve_comments` | `boolean` | No | Preserve comments (**default: true**) |

**Transformation Types:**
- `"if_to_match"` - Convert if/else chain to match expression (Rust)
- `"match_to_if"` - Convert match expression to if/else chain
- `"add_async"` - Make function async
- `"remove_async"` - Remove async from function
- `"fn_to_closure"` - Convert function to closure
- `"closure_to_fn"` - Convert closure to function

**Language Support:**
- Rust: All transformation types
- TypeScript/JavaScript: `add_async`, `remove_async`
- Other languages: Limited support

**Error Messages:**
- Missing `transformation.kind`: "Invalid request: Missing 'kind' parameter"
- Invalid `kind`: "Unsupported transformation 'invalid'. Valid: if_to_match, match_to_if, add_async, remove_async, fn_to_closure, closure_to_fn"
- Invalid range: "Range must specify valid start and end positions"

**Returns:**

**Preview mode (`dryRun: true`, default):** Returns a `TransformPlan` object.
**Execution mode (`dryRun: false`):** Returns an `ExecutionResult` object.

**Example:**

```json
// Request - Preview if-to-match transformation
{
  "method": "tools/call",
  "params": {
    "name": "transform",
    "arguments": {
      "transformation": {
        "kind": "if_to_match",
        "file_path": "/workspace/src/logic.rs",
        "range": {
          "start": {"line": 15, "character": 4},
          "end": {"line": 25, "character": 5}
        }
      },
      "options": {
        "preserve_formatting": true,
        "preserve_comments": true
      }
    }
  }
}
```text
**Notes:**
- Uses LSP code actions when available
- Falls back to AST-based transformations
- Language-specific transformation support
- Preview mode is the safe default

---

### delete

**Purpose:** Delete symbols, files, directories, or dead code.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `target` | `object` | **Yes** | Target to delete (see target structure) |
| `target.kind` | `string` | **Yes** | Delete type: `"symbol"` \| `"file"` \| `"directory"` \| `"dead_code"` |
| `target.path` | `string` | **Yes** | Absolute path to file/directory, or file containing symbol |
| `target.selector` | `object` | Conditional | Symbol selector - **required for symbol deletes** |
| `target.selector.line` | `number` | Conditional | Line number (0-indexed) |
| `target.selector.character` | `number` | Conditional | Column position (0-indexed) |
| `target.selector.symbol_name` | `string` | No | Optional symbol name hint for validation |
| `options` | `object` | No | Delete options (see options below) |
| `options.dryRun` | `boolean` | No | Preview mode - don't apply changes (**default: true**) |
| `options.cleanup_imports` | `boolean` | No | Remove unused imports (**default: true**) |
| `options.remove_tests` | `boolean` | No | Also delete associated tests (**default: false**) |
| `options.force` | `boolean` | No | Force delete without safety checks (**default: false**) |

**Delete Types:**
- `"symbol"` - Delete function, class, variable, etc.
  - **Status:** Placeholder (requires AST support)
  - **Requires:** `selector` with line/character position
- `"file"` - Delete a file
  - **Status:** Fully implemented
  - **Updates:** Removes imports, updates references
- `"directory"` - Delete a directory recursively
  - **Status:** Fully implemented
  - **Warning:** Deletes all files in directory!
- `"dead_code"` - Automatically detect and delete unused code
  - **Status:** Placeholder (requires analysis integration)

**Safety Features:**
- `cleanup_imports: true` - Automatically removes imports after deletion
- `force: false` - Requires confirmation for destructive operations
- `dryRun: true` - Preview deletions before applying

**Error Messages:**
- Missing `target`: "Invalid request: Missing 'target' parameter"
- Invalid `target.kind`: "Unsupported kind 'invalid'. Valid: symbol, file, directory, dead_code"
- Symbol without selector: "Symbol deletes require 'selector' with line and character"

**Returns:**

**Preview mode (`dryRun: true`, default):** Returns a `DeletePlan` object containing:
- `plan_type`: `"DeletePlan"`
- `deletions`: Array of deletion targets with path and kind
- `summary`: File counts
- `warnings`: Array of warnings
- `metadata`: Plan metadata
- `file_checksums`: SHA-256 checksums

**Execution mode (`dryRun: false`):** Returns an `ExecutionResult` object.

**Example:**

```json
// Request - Preview file deletion
{
  "method": "tools/call",
  "params": {
    "name": "delete",
    "arguments": {
      "target": {
        "kind": "file",
        "path": "/workspace/src/deprecated.rs"
      },
      "options": {
        "cleanup_imports": true
      }
    }
  }
}

// Response - Plan preview
{
  "result": {
    "content": {
      "plan_type": "DeletePlan",
      "deletions": [
        {
          "path": "/workspace/src/deprecated.rs",
          "kind": "file"
        }
      ],
      "summary": {
        "affected_files": 0,
        "created_files": 0,
        "deleted_files": 1
      },
      "warnings": [],
      "metadata": {
        "plan_version": "1.0",
        "kind": "delete",
        "language": "rust",
        "estimated_impact": "low",
        "created_at": "2025-10-25T10:50:00Z"
      },
      "file_checksums": {
        "/workspace/src/deprecated.rs": "m4n5o6..."
      }
    }
  }
}
```text
**Notes:**
- File/directory deletes are fully implemented
- Symbol/dead_code deletes are placeholders (require AST support)
- Cleanup imports automatically when deleting files
- DeletePlan uses `deletions` field instead of LSP `WorkspaceEdit`
- Preview mode is the safe default

---

## CLI vs MCP Tool Interfaces

Refactoring tools are primarily accessed via the MCP tool interface, but some operations have CLI shortcuts:

**Via MCP tool** (primary interface - required JSON):
```bash
# Rename file
mill tool rename '{
  "target": {"kind": "file", "path": "src/old.rs"},
  "newName": "src/new.rs",
  "options": {"dryRun": false}
}'

# Extract function
mill tool extract '{
  "kind": "function",
  "source": {
    "file_path": "src/calc.rs",
    "range": {"start": {"line": 1, "character": 4}, "end": {"line": 2, "character": 26}},
    "name": "compute_sum"
  }
}'
```text
**Via CLI flags** (limited operations):
```bash
# Not available for most refactoring operations
# Use MCP tool interface instead
```text
**Why MCP tool interface?**
- Refactoring requires complex structured data (ranges, positions, options)
- JSON format prevents ambiguity and parsing errors
- Consistent with analysis tools and other MCP operations
- Enables programmatic/AI agent use

---

## Common Patterns

### Safe Preview Pattern (Recommended)

The safest refactoring approach uses the default `dryRun: true` behavior:

```json
// Step 1: Preview changes (dryRun defaults to true)
{
  "name": "rename",
  "arguments": {
    "target": {"kind": "file", "path": "/workspace/src/old.rs"},
    "newName": "/workspace/src/new.rs"
  }
}

// Step 2: Review the plan output (edits, summary, warnings, metadata)

// Step 3: Execute if satisfied (explicit dryRun: false)
{
  "name": "rename",
  "arguments": {
    "target": {"kind": "file", "path": "/workspace/src/old.rs"},
    "newName": "/workspace/src/new.rs",
    "options": {
      "dryRun": false
    }
  }
}
```text
**Benefits:**
- Default behavior prevents accidental execution
- Preview all changes before applying
- Verify estimated impact and affected files
- Catch issues early with warnings
- Explicit opt-in required for execution

### Direct Execution Pattern

For small, trusted operations, you can execute directly:

```json
{
  "name": "rename",
  "arguments": {
    "target": {"kind": "file", "path": "/workspace/src/old.rs"},
    "newName": "/workspace/src/new.rs",
    "options": {
      "dryRun": false  // Skip preview, execute immediately
    }
  }
}
```text
**Use cases:**
- Small, low-risk refactorings
- Operations you've previewed before
- Trusted automated workflows

**Caution:** Less safe than preview pattern - no review step before execution.

### Checksum Validation

Checksums prevent applying stale operations after file modifications:

```json
// T0: Preview rename
{
  "name": "rename",
  "arguments": {
    "target": {"kind": "file", "path": "/workspace/src/file.rs"},
    "newName": "/workspace/src/new.rs"
  }
}
// Returns plan with file_checksums: {"file.rs": "abc123..."}

// T1: File modified externally (plan now stale)

// T2: Try to execute - will detect checksum mismatch
{
  "name": "rename",
  "arguments": {
    "target": {"kind": "file", "path": "/workspace/src/file.rs"},
    "newName": "/workspace/src/new.rs",
    "options": {
      "dryRun": false
    }
  }
}
// Error: "Checksum mismatch for /workspace/src/file.rs - file modified since plan generation"
```text
**How it works:**
1. Preview mode captures SHA-256 checksums of all affected files
2. Execution mode validates checksums before applying
3. Mismatches abort the operation to prevent data loss

### Post-Apply Validation

Run tests or checks after applying changes with automatic rollback on failure:

```json
{
  "name": "rename",
  "arguments": {
    "target": {"kind": "directory", "path": "/workspace/src/module"},
    "newName": "/workspace/src/refactored",
    "options": {
      "dryRun": false,
      "validation": {
        "command": "cargo test --workspace",
        "timeout_seconds": 300
      }
    }
  }
}
```text
**Behavior:**
1. Applies changes to filesystem
2. Runs validation command (`cargo test --workspace`)
3. If validation **fails**: automatic rollback to pre-apply state
4. If validation **succeeds**: changes are permanent

**Note:** Validation options are only available in execution mode (`dryRun: false`).

### Batch Operations

Apply multiple refactorings sequentially:

```json
// Each operation is atomic and safe
await call("rename", {
  target: {kind: "file", path: "src/old1.rs"},
  newName: "src/new1.rs",
  options: {dryRun: false}
});

await call("rename", {
  target: {kind: "file", path: "src/old2.rs"},
  newName: "src/new2.rs",
  options: {dryRun: false}
});
```text
**Note:** Each operation is atomic, but multiple calls are not. For dependent refactorings, preview all operations first to ensure they won't conflict.

**Alternative - Batch rename:**
```json
{
  "name": "rename",
  "arguments": {
    "targets": [
      {"kind": "file", "path": "src/old1.rs", "newName": "src/new1.rs"},
      {"kind": "file", "path": "src/old2.rs", "newName": "src/new2.rs"}
    ],
    "options": {
      "dryRun": false
    }
  }
}
```text
### Rust-Specific: Crate Consolidation

Merge a Rust crate into another crate's module:

```json
{
  "name": "rename",
  "arguments": {
    "target": {
      "kind": "directory",
      "path": "/workspace/crates/source-crate"
    },
    "newName": "/workspace/crates/target-crate/src/module",
    "options": {
      "consolidate": true,  // Optional - auto-detected
      "dryRun": false
    }
  }
}
```text
**Consolidation automatically:**
1. Moves `source-crate/src/*` into `target-crate/src/module/*`
2. Merges dependencies from `source-crate/Cargo.toml`
3. Updates all imports: `use source_crate::*` → `use target_crate::module::*`
4. Removes source crate from workspace members
5. Deletes source crate directory

**After applying:** Manually add `pub mod module;` to `target-crate/src/lib.rs`.

### Rust-Specific: File Rename Updates

Renaming Rust files automatically updates:

```rust
// Before: src/lib.rs
pub mod utils;
use utils::helper;
utils::another();

// After renaming src/utils.rs → src/helpers.rs
pub mod helpers;
use helpers::helper;
helpers::another();
```text
**Coverage:**
- ✅ Module declarations in parent files (`pub mod`)
- ✅ Use statements (`use utils::*`)
- ✅ Qualified paths in code (`utils::helper()`)
- ✅ Nested module paths (`parent::utils::*`)
- ✅ Cross-crate imports

### Comprehensive Rename Coverage

When `scope: "standard"` (default), directory/file renames update:

1. **Code files** (.rs, .ts, .js): imports, module declarations, qualified paths, string literal paths
2. **Documentation** (.md): markdown links, inline code references
3. **Configuration** (.toml, .yaml): path values in any field
4. **Cargo.toml**: workspace members, package names, path dependencies

**Example - Renaming `old-dir/` → `new-dir/` updates:**
- ✅ Rust imports: `use old_dir::*` → `use new_dir::*`
- ✅ String paths: `"old-dir/file.rs"` → `"new-dir/file.rs"`
- ✅ Markdown links: `[doc](old-dir/README.md)` → `[doc](new-dir/README.md)`
- ✅ Cargo.toml: `members = ["old-dir"]` → `members = ["new-dir"]`

**Scope control:**
```json
{
  "name": "rename",
  "arguments": {
    "target": {"kind": "directory", "path": "old-dir"},
    "newName": "new-dir",
    "options": {
      "scope": "code"  // Only update code files, skip .md, .toml, .yaml
    }
  }
}
```text
**Available scopes:**
- `"code"`: Code only (imports, module declarations, string literal paths)
- `"standard"` (default): Code + docs + configs (recommended for most renames)
- `"comments"`: Standard scope + code comments
- `"everything"`: Comments scope + markdown prose text
- `"custom"`: Fine-grained control with exclude patterns

---

**Last Updated:** 2025-10-25
**API Version:** 1.0.0-rc5