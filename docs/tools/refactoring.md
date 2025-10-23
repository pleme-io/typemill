# Editing & Refactoring Tools

**15 tools following the unified plan → apply pattern for safe, reviewable refactoring operations**

The Unified Refactoring API provides two execution modes for all refactoring operations:

1. **Two-step pattern (recommended):** Generate a plan with `*.plan` tools (always dry-run, never modifies files), review changes, then apply with `workspace.apply_edit`
2. **One-step pattern (quick):** Use tools without `.plan` suffix to combine plan + execute in one call for trusted operations

All plan types support checksum validation, rollback on error, and post-apply validation.

**Tool count:** 15 tools
**Related categories:** [Navigation](navigation.md), [Analysis](analysis.md), [Workspace](workspace.md)

## Table of Contents

- [Tools](#tools)
  - [rename.plan](#renameplan)
  - [rename](#rename)
  - [extract.plan](#extractplan)
  - [extract](#extract)
  - [inline.plan](#inlineplan)
  - [inline](#inline)
  - [move.plan](#moveplan)
  - [move](#move)
  - [reorder.plan](#reorderplan)
  - [reorder](#reorder)
  - [transform.plan](#transformplan)
  - [transform](#transform)
  - [delete.plan](#deleteplan)
  - [delete](#delete)
  - [workspace.apply_edit](#workspaceapply_edit)
- [Common Patterns](#common-patterns)
  - [Two-Step Workflow (Recommended)](#two-step-workflow-recommended)
  - [Dry-Run Preview](#dry-run-preview)
  - [Checksum Validation](#checksum-validation)
  - [Post-Apply Validation](#post-apply-validation)
  - [Batch Operations](#batch-operations)
  - [Rust-Specific: Crate Consolidation](#rust-specific-crate-consolidation)
  - [Rust-Specific: File Rename Updates](#rust-specific-file-rename-updates)
  - [Comprehensive Rename Coverage](#comprehensive-rename-coverage)

---

## Tools

### rename.plan

**Purpose:** Generate a plan for renaming symbols, files, or directories with automatic import/reference updates.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `target` | `object` | Yes | Target to rename (see target structure below) |
| `target.kind` | `string` | Yes | Type of target: `"symbol"`, `"file"`, or `"directory"` |
| `target.path` | `string` | Yes | Absolute path to file/directory, or file path for symbol |
| `target.line` | `number` | Conditional | Line number (0-indexed) - required for symbol renames |
| `target.character` | `number` | Conditional | Column position (0-indexed) - required for symbol renames |
| `new_name` | `string` | Yes | New name or path |
| `options` | `object` | No | Rename options (see options below) |
| `options.consolidate` | `boolean` | No | Enable Rust crate consolidation mode (auto-detected if not specified) |
| `options.scope` | `string` | No | Update scope: `"all"` (default), `"code-only"`, or `"custom"` |

**Returns:**

A `RenamePlan` object containing:
- `plan_type`: `"RenamePlan"`
- `edits`: LSP `WorkspaceEdit` with all file changes
- `summary`: Counts of affected/created/deleted files
- `warnings`: Array of warnings (e.g., missing LSP server)
- `metadata`: Plan version, kind, language, estimated impact, timestamp
- `file_checksums`: SHA-256 checksums for validation
- `is_consolidation`: Boolean flag for crate consolidation (Rust-specific)

**Example:**

```json
// Request - Rename a Rust file
{
  "method": "tools/call",
  "params": {
    "name": "rename.plan",
    "arguments": {
      "target": {
        "kind": "file",
        "path": "/workspace/src/utils.rs"
      },
      "new_name": "/workspace/src/helpers.rs"
    }
  }
}

// Response
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
        "created_at": "2025-10-22T10:30:00Z"
      },
      "file_checksums": {
        "/workspace/src/utils.rs": "a1b2c3...",
        "/workspace/src/lib.rs": "d4e5f6..."
      },
      "is_consolidation": false
    }
  }
}
```

**Notes:**
- **Rust file renames** automatically update module declarations (`pub mod utils;` → `pub mod helpers;`), import statements (`use utils::*`), and qualified paths (`utils::helper()`)
- **Directory renames** update all string literal paths, markdown links, config file paths, and Cargo.toml entries (100% coverage when `scope: "all"`)
- **Crate consolidation mode** merges dependencies and removes source crate from workspace when renaming into another crate's `src/` directory
- **Checksum validation** prevents applying stale plans after file modifications

---

### rename

**Purpose:** Execute rename operation in one step (combines `rename.plan` + `workspace.apply_edit`).

**Parameters:**

Same as `rename.plan` - all parameters pass through to the planning phase.

**Returns:**

Same structure as `workspace.apply_edit` result (see `workspace.apply_edit` section).

**Example:**

```json
// Request - Quick rename without review
{
  "method": "tools/call",
  "params": {
    "name": "rename",
    "arguments": {
      "target": {
        "kind": "file",
        "path": "/workspace/src/old.rs"
      },
      "new_name": "/workspace/src/new.rs"
    }
  }
}

// Response
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
```

**Notes:**
- Less safe than two-step pattern - no preview before execution
- Use for small, low-risk refactorings when you trust the operation
- Automatically applies with `dry_run: false`

---

### extract.plan

**Purpose:** Generate a plan for extracting code into functions, variables, constants, or modules.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `kind` | `string` | Yes | Extraction type: `"function"`, `"variable"`, `"constant"`, or `"module"` |
| `source` | `object` | Yes | Source code selection (see source structure) |
| `source.file_path` | `string` | Yes | Absolute path to source file |
| `source.range` | `object` | Yes | LSP range of code to extract |
| `source.range.start` | `object` | Yes | Start position: `{line: number, character: number}` |
| `source.range.end` | `object` | Yes | End position: `{line: number, character: number}` |
| `source.name` | `string` | Yes | Name for extracted element |
| `source.destination` | `string` | No | Destination file path (for module extraction) |
| `options` | `object` | No | Extract options (see options below) |
| `options.visibility` | `string` | No | Visibility modifier: `"public"` or `"private"` |
| `options.destination_path` | `string` | No | Override destination path |

**Returns:**

An `ExtractPlan` object containing:
- `plan_type`: `"ExtractPlan"`
- `edits`: LSP `WorkspaceEdit` with code changes
- `summary`: File counts (typically only affected_files, no created/deleted)
- `warnings`: Array of warnings
- `metadata`: Plan metadata
- `file_checksums`: SHA-256 checksums

**Example:**

```json
// Request - Extract function from code block
{
  "method": "tools/call",
  "params": {
    "name": "extract.plan",
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

// Response
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
        "created_at": "2025-10-22T10:35:00Z"
      },
      "file_checksums": {
        "/workspace/src/calc.rs": "f7g8h9..."
      }
    }
  }
}
```

**Notes:**
- Uses AST-based refactoring (no LSP required)
- Automatically infers parameters and return types
- Module extraction requires language plugin support

---

### extract

**Purpose:** Execute extract operation in one step (combines `extract.plan` + `workspace.apply_edit`).

**Parameters:**

Same as `extract.plan`.

**Returns:**

Same structure as `workspace.apply_edit` result.

**Notes:**
- Convenient for quick extractions
- Skips plan review step

---

### inline.plan

**Purpose:** Generate a plan for inlining variables, functions, or constants by replacing references with definitions.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `kind` | `string` | Yes | Inline type: `"variable"`, `"function"`, or `"constant"` |
| `target` | `object` | Yes | Target to inline (see target structure) |
| `target.file_path` | `string` | Yes | Absolute path to file containing definition |
| `target.position` | `object` | Yes | Position of definition: `{line: number, character: number}` |
| `options` | `object` | No | Inline options (see options below) |
| `options.inline_all` | `boolean` | No | Inline all usages (true) or current only (false, default) |

**Returns:**

An `InlinePlan` object containing:
- `plan_type`: `"InlinePlan"`
- `edits`: LSP `WorkspaceEdit` replacing references
- `summary`: File counts
- `warnings`: Array of warnings
- `metadata`: Plan metadata
- `file_checksums`: SHA-256 checksums

**Example:**

```json
// Request - Inline a variable
{
  "method": "tools/call",
  "params": {
    "name": "inline.plan",
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

// Response
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
        "created_at": "2025-10-22T10:40:00Z"
      },
      "file_checksums": {
        "/workspace/src/vars.rs": "j1k2l3..."
      }
    }
  }
}
```

**Notes:**
- Uses AST-based refactoring
- Removes definition after inlining
- Does not create or delete files

---

### inline

**Purpose:** Execute inline operation in one step (combines `inline.plan` + `workspace.apply_edit`).

**Parameters:**

Same as `inline.plan`.

**Returns:**

Same structure as `workspace.apply_edit` result.

---

### move.plan

**Purpose:** Generate a plan for moving symbols or code blocks to different files/modules.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `source` | `object` | Yes | Source location (file, range, or symbol) |
| `destination` | `string` | Yes | Destination file path |
| `options` | `object` | No | Move options (scope, visibility, etc.) |

**Returns:**

A `MovePlan` object with structure similar to other plan types.

**Example:**

```json
// Request - Move a function to different file
{
  "method": "tools/call",
  "params": {
    "name": "move.plan",
    "arguments": {
      "source": {
        "file_path": "/workspace/src/utils.rs",
        "range": {
          "start": {"line": 5, "character": 0},
          "end": {"line": 10, "character": 1}
        }
      },
      "destination": "/workspace/src/helpers.rs"
    }
  }
}
```

**Notes:**
- Automatically updates imports and references
- Handles cross-file symbol moves

---

### move

**Purpose:** Execute move operation in one step.

**Parameters:**

Same as `move.plan`.

**Returns:**

Same structure as `workspace.apply_edit` result.

---

### reorder.plan

**Purpose:** Generate a plan for reordering function parameters, struct fields, imports, or statements.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `target` | `object` | Yes | Target to reorder (see target structure) |
| `target.kind` | `string` | Yes | Reorder type: `"parameters"`, `"fields"`, `"imports"`, or `"statements"` |
| `target.file_path` | `string` | Yes | Absolute path to file |
| `target.position` | `object` | Yes | Position: `{line: number, character: number}` |
| `new_order` | `array` | Yes | Array of strings specifying new order |
| `options` | `object` | No | Reorder options (see options below) |
| `options.preserve_formatting` | `boolean` | No | Preserve code formatting (default: true) |
| `options.update_call_sites` | `boolean` | No | Update all call sites for parameter reordering (default: true) |

**Returns:**

A `ReorderPlan` object with standard plan structure.

**Example:**

```json
// Request - Reorder function parameters
{
  "method": "tools/call",
  "params": {
    "name": "reorder.plan",
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
```

**Notes:**
- Parameter reordering updates all call sites across the codebase
- Requires LSP server support for best results

---

### reorder

**Purpose:** Execute reorder operation in one step.

**Parameters:**

Same as `reorder.plan`.

**Returns:**

Same structure as `workspace.apply_edit` result.

---

### transform.plan

**Purpose:** Generate a plan for syntax transformations (if-to-match, add/remove async, etc.).

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `transformation` | `object` | Yes | Transformation specification (see structure below) |
| `transformation.kind` | `string` | Yes | Type: `"if_to_match"`, `"match_to_if"`, `"add_async"`, `"remove_async"`, `"fn_to_closure"`, `"closure_to_fn"` |
| `transformation.file_path` | `string` | Yes | Absolute path to file |
| `transformation.range` | `object` | Yes | LSP range to transform |
| `options` | `object` | No | Transform options (see options below) |
| `options.preserve_formatting` | `boolean` | No | Preserve code formatting (default: true) |
| `options.preserve_comments` | `boolean` | No | Preserve comments (default: true) |

**Returns:**

A `TransformPlan` object with standard plan structure.

**Example:**

```json
// Request - Convert if-else to match
{
  "method": "tools/call",
  "params": {
    "name": "transform.plan",
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
```

**Notes:**
- Uses LSP code actions when available
- Falls back to AST-based transformations
- Language-specific transformation support

---

### transform

**Purpose:** Execute transform operation in one step.

**Parameters:**

Same as `transform.plan`.

**Returns:**

Same structure as `workspace.apply_edit` result.

---

### delete.plan

**Purpose:** Generate a plan for deleting symbols, files, directories, or dead code.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `target` | `object` | Yes | Target to delete (see target structure) |
| `target.kind` | `string` | Yes | Delete type: `"symbol"`, `"file"`, `"directory"`, or `"dead_code"` |
| `target.path` | `string` | Yes | Absolute path to file/directory, or file for symbol |
| `target.selector` | `object` | Conditional | Symbol selector - required for symbol deletes |
| `target.selector.line` | `number` | Conditional | Line number (0-indexed) |
| `target.selector.character` | `number` | Conditional | Column position (0-indexed) |
| `target.selector.symbol_name` | `string` | No | Optional symbol name hint |
| `options` | `object` | No | Delete options (see options below) |
| `options.cleanup_imports` | `boolean` | No | Remove unused imports (default: true) |
| `options.remove_tests` | `boolean` | No | Also delete associated tests (default: false) |
| `options.force` | `boolean` | No | Force delete without safety checks (default: false) |

**Returns:**

A `DeletePlan` object containing:
- `plan_type`: `"DeletePlan"`
- `deletions`: Array of deletion targets with path and kind
- `summary`: File counts
- `warnings`: Array of warnings
- `metadata`: Plan metadata
- `file_checksums`: SHA-256 checksums

**Example:**

```json
// Request - Delete a file
{
  "method": "tools/call",
  "params": {
    "name": "delete.plan",
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

// Response
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
        "created_at": "2025-10-22T10:50:00Z"
      },
      "file_checksums": {
        "/workspace/src/deprecated.rs": "m4n5o6..."
      }
    }
  }
}
```

**Notes:**
- File/directory deletes are implemented, symbol/dead_code deletes are placeholders
- Cleanup imports automatically when deleting files
- DeletePlan uses `deletions` field instead of LSP `WorkspaceEdit`

---

### delete

**Purpose:** Execute delete operation in one step.

**Parameters:**

Same as `delete.plan`.

**Returns:**

Same structure as `workspace.apply_edit` result.

---

### workspace.apply_edit

**Purpose:** Apply a refactoring plan generated by any `*.plan` tool with validation and rollback support.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `plan` | `object` | Yes | Any plan type from `*.plan` tools (discriminated union) |
| `options` | `object` | No | Apply options (see options below) |
| `options.dry_run` | `boolean` | No | Preview mode - don't actually apply changes (default: false) |
| `options.validate_checksums` | `boolean` | No | Validate file checksums before applying (default: true) |
| `options.rollback_on_error` | `boolean` | No | Automatically rollback all changes if any error occurs (default: true) |
| `options.validation` | `object` | No | Post-apply validation configuration |
| `options.validation.command` | `string` | Conditional | Shell command to run (e.g., "cargo test") |
| `options.validation.timeout_seconds` | `number` | No | Command timeout in seconds (default: 300) |

**Returns:**

An `ApplyResult` object containing:
- `success`: Boolean indicating operation success
- `applied_files`: Array of file paths that were modified
- `created_files`: Array of newly created file paths
- `deleted_files`: Array of deleted file paths
- `warnings`: Array of warning messages
- `validation`: Optional validation result object
- `rollback_available`: Boolean indicating if rollback is still possible

**Example:**

```json
// Request - Apply a rename plan with validation
{
  "method": "tools/call",
  "params": {
    "name": "workspace.apply_edit",
    "arguments": {
      "plan": {
        "plan_type": "RenamePlan",
        "edits": { /* ... */ },
        "summary": { /* ... */ },
        "warnings": [],
        "metadata": { /* ... */ },
        "file_checksums": { /* ... */ },
        "is_consolidation": false
      },
      "options": {
        "dry_run": false,
        "validate_checksums": true,
        "rollback_on_error": true,
        "validation": {
          "command": "cargo check",
          "timeout_seconds": 60
        }
      }
    }
  }
}

// Response
{
  "result": {
    "content": {
      "success": true,
      "applied_files": [
        "/workspace/src/helpers.rs",
        "/workspace/src/lib.rs",
        "/workspace/src/main.rs"
      ],
      "created_files": [],
      "deleted_files": ["/workspace/src/utils.rs"],
      "warnings": [],
      "validation": {
        "success": true,
        "command": "cargo check",
        "exit_code": 0,
        "stdout": "Checking workspace...\nFinished",
        "stderr": "",
        "duration_ms": 1234
      },
      "rollback_available": false
    }
  }
}
```

**Notes:**
- **Only command that writes files** - all `*.plan` tools are dry-run only
- Supports all 7 plan types: RenamePlan, ExtractPlan, InlinePlan, MovePlan, ReorderPlan, TransformPlan, DeletePlan
- Checksum validation prevents applying stale plans (files modified after planning)
- Post-apply validation runs custom commands (e.g., tests) and rolls back on failure
- Atomic operations with automatic rollback on any error
- Dry-run mode provides final preview before execution

---

## Common Patterns

### Two-Step Workflow (Recommended)

The safest refactoring approach:

```json
// Step 1: Generate plan (always dry-run, never modifies files)
{
  "name": "rename.plan",
  "arguments": {
    "target": {"kind": "file", "path": "/workspace/src/old.rs"},
    "new_name": "/workspace/src/new.rs"
  }
}

// Step 2: Review the plan output (edits, summary, warnings)

// Step 3: Apply with validation
{
  "name": "workspace.apply_edit",
  "arguments": {
    "plan": { /* plan from step 1 */ },
    "options": {
      "dry_run": false,
      "validate_checksums": true,
      "validation": {
        "command": "cargo test",
        "timeout_seconds": 120
      }
    }
  }
}
```

**Benefits:**
- Preview all changes before execution
- Verify estimated impact and affected files
- Catch issues early with warnings
- Post-apply validation ensures correctness
- Automatic rollback on validation failure

### Dry-Run Preview

Use `dry_run: true` for final review before applying:

```json
{
  "name": "workspace.apply_edit",
  "arguments": {
    "plan": { /* plan */ },
    "options": {
      "dry_run": true  // Returns success without modifying files
    }
  }
}
```

**Use cases:**
- Final safety check before large refactorings
- Debugging plan generation
- Testing refactoring logic

### Checksum Validation

Prevent applying stale plans after file modifications:

```json
// Generate plan at T0
{"name": "rename.plan", "arguments": {...}}

// File modified at T1 (plan now stale)

// Try to apply at T2 - will fail with checksum mismatch
{
  "name": "workspace.apply_edit",
  "arguments": {
    "plan": { /* stale plan */ },
    "options": {
      "validate_checksums": true  // Prevents stale apply
    }
  }
}
// Error: "Checksum mismatch for /workspace/src/file.rs"
```

### Post-Apply Validation

Run tests or checks after applying changes:

```json
{
  "name": "workspace.apply_edit",
  "arguments": {
    "plan": { /* plan */ },
    "options": {
      "validation": {
        "command": "cargo test --workspace",
        "timeout_seconds": 300
      }
    }
  }
}
```

**Behavior:**
- Applies changes first
- Runs validation command
- If validation fails: automatic rollback to pre-apply state
- If validation succeeds: changes are permanent

### Batch Operations

Apply multiple refactorings atomically:

```json
// Generate multiple plans
const renamePlan = await call("rename.plan", {...});
const extractPlan = await call("extract.plan", {...});

// Apply each plan separately (each is atomic)
await call("workspace.apply_edit", {plan: renamePlan});
await call("workspace.apply_edit", {plan: extractPlan});
```

**Note:** Each `workspace.apply_edit` is atomic, but multiple calls are not. For true multi-plan atomicity, combine operations at the planning stage.

### Rust-Specific: Crate Consolidation

Merge a Rust crate into another crate's module:

```json
{
  "name": "rename.plan",
  "arguments": {
    "target": {
      "kind": "directory",
      "path": "/workspace/crates/source-crate"
    },
    "new_name": "/workspace/crates/target-crate/src/module",
    "options": {
      "consolidate": true  // Optional - auto-detected
    }
  }
}
```

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
```

**Coverage:**
- ✅ Module declarations in parent files
- ✅ Use statements
- ✅ Qualified paths in code
- ✅ Nested module paths
- ✅ Cross-crate imports

### Comprehensive Rename Coverage

When `scope: "all"` (default), directory/file renames update:

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
  "options": {
    "scope": "code-only"  // Skip .md, .toml, .yaml files
  }
}
```

---

**Last Updated:** 2025-10-22
**API Version:** 1.0.0-rc4
