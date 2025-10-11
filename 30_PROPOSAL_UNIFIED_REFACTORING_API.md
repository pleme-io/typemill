# Proposal: Unified Refactoring API

**Status**: Draft (not implemented on `main`)
**Author**: Project Team
**Date**: 2025-10-10
**Formal Spec**: [docs/design/unified_api_contracts.md](docs/design/unified_api_contracts.md)

---

## Executive Summary

Consolidate 35 refactoring commands into **14 unified commands** using a consistent **plan → apply** pattern. This reduces API surface by 60% while improving safety, composability, and discoverability.

**Context**: This is a beta product with no external users. We can make breaking changes immediately without migration paths or legacy support.

---

## Current Implementation Status (main @ 2025-10-10)

The codebase still uses the legacy refactoring stack; the unified API has not begun implementation.

- Legacy tools such as `rename_symbol`, `extract_function`, `extract_variable`, and `inline_variable` remain the active handlers in `crates/cb-handlers/src/handlers/refactoring_handler.rs`.
- The dispatcher continues to register the legacy editing tools and does not expose any `*.plan` or `workspace.apply_edit` commands (`crates/cb-handlers/src/handlers/plugin_dispatcher.rs`).
- The protocol crate exports only the historical modules; there are no `RefactorPlan` types or related metadata structures in `crates/cb-protocol/src/lib.rs`.
- Supporting pieces described in this proposal—`workspace_apply_handler.rs`, `refactor_config.rs`, unified integration tests, and documentation updates—do not yet exist in the repository.

The remainder of this document captures the target design for when implementation work begins.

---

## Problem

Current API has fragmentation:
- **35 separate commands** for refactoring operations
- **Inconsistent interfaces** across similar operations
- **No unified dry-run or preview** mechanism
- **Difficult to compose** multi-step refactorings
- **High cognitive load** for users and AI agents

---

## Solution

### Pillar 1: Refactoring Primitives (Code Transformation)

These atomic operations provide the building blocks for restructuring code safely:

- **Rename** – change the name of a symbol (variable, function, class, file, or directory).
- **Extract** – pull a block of code into a new function, file, or module.
- **Inject / Insert** – add code to an existing structure without disturbing surrounding logic.
- **Move** – relocate code between files, modules, or directories.
- **Inline** – replace a reference with its value or implementation.
- **Reorder** – adjust the sequence of code elements to improve clarity or enforce conventions.
- *(Optional)* **Duplicate / Delete** – copy or remove code snippets when higher-level workflows require it.

These refactoring primitives compose into the richer plan-based workflows defined in this proposal.

### Core Pattern: Plan → Apply

Every refactoring operation follows two steps:

1. **`<operation>.plan(...)`** - Returns a plan with edits, warnings, metadata (never writes files)
2. **`workspace.apply_edit(plan)`** - Executes any plan atomically with rollback support

### Unified Plan Structure

All `*.plan` commands return a discriminated union type with validation metadata:

```json
{
  "plan_type": "RenamePlan" | "ExtractPlan" | "InlinePlan" | "MovePlan" | "ReorderPlan" | "TransformPlan" | "DeletePlan",
  "plan_version": "1.0",
  "edits": [ /* LSP workspace edits */ ],
  "summary": {
    "affected_files": 3,
    "created_files": 1,
    "deleted_files": 0
  },
  "warnings": [
    { "code": "AMBIGUOUS_TARGET", "message": "...", "candidates": [...] }
  ],
  "metadata": {
    "kind": "rename",
    "language": "rust",
    "estimated_impact": "low",
    "created_at": "2025-10-10T12:00:00Z"
  },
  "file_checksums": {
    "src/lib.rs": "sha256:abc123...",
    "src/app.rs": "sha256:def456..."
  }
}
```

**Key fields**:
- `plan_type`: Discriminator for type-safe validation in `workspace.apply_edit`
- `plan_version`: API version for backward compatibility
- `file_checksums`: SHA-256 hashes to detect stale plans
- `created_at`: Timestamp for plan expiration checks

---

## New API Surface

### 1. Rename Operations

**Commands**: 2 (was 6)

```javascript
rename.plan(target, new_name, options) → RenamePlan
workspace.apply_edit(plan) → Result
```

**Arguments**:
```json
{
  "target": {
    "kind": "symbol" | "parameter" | "type" | "file" | "directory",
    "path": "src/lib.rs",
    "selector": {
      "position": { "line": 12, "character": 8 },
      "name": "oldName"  // optional fallback
    }
  },
  "new_name": "newName",
  "options": {
    "dry_run": true,
    "strict": false,
    "update_imports": true,
    "validate_scope": true,
    "workspace_limits": ["src/"]
  }
}
```

**Examples**:
- `rename.plan({ kind: "symbol", path: "lib.rs", selector: { position: {...} } }, "new_name")`
- `rename.plan({ kind: "file", path: "old.rs" }, "new.rs")`
- `rename.plan({ kind: "directory", path: "crates/old" }, "crates/new", { update_imports: true })`

---

### 2. Extract Operations

**Commands**: 2 (was 7)

```javascript
extract.plan(kind, source, options) → ExtractPlan
workspace.apply_edit(plan) → Result
```

**Arguments**:
```json
{
  "kind": "function" | "variable" | "module" | "interface" | "class" | "constant" | "type_alias",
  "source": {
    "file_path": "src/app.rs",
    "range": { "start": {...}, "end": {...} },
    "name": "extracted_item",
    "destination": "src/extracted.rs"  // optional
  },
  "options": {
    "dry_run": true,
    "visibility": "public" | "private",
    "destination_path": "src/new_module.rs",
    "language_hints": {}
  }
}
```

**Examples**:
- `extract.plan("function", { file_path: "app.rs", range: {...}, name: "helper" })`
- `extract.plan("constant", { file_path: "app.rs", range: {...}, name: "MAX_SIZE" })`
- `extract.plan("module", { file_path: "lib.rs", range: {...}, destination: "utils.rs" })`

---

### 3. Inline Operations

**Commands**: 2 (was 4)

```javascript
inline.plan(kind, target, options) → InlinePlan
workspace.apply_edit(plan) → Result
```

**Arguments**:
```json
{
  "kind": "variable" | "function" | "constant" | "type_alias",
  "target": {
    "file_path": "src/app.rs",
    "position": { "line": 10, "character": 5 }
  },
  "options": {
    "dry_run": true,
    "inline_all": false  // inline all usages vs current only
  }
}
```

**Examples**:
- `inline.plan("variable", { file_path: "app.rs", position: {...} })`
- `inline.plan("function", { file_path: "lib.rs", position: {...} }, { inline_all: true })`

---

### 4. Move Operations

**Commands**: 2 (was 4)

```javascript
move.plan(kind, source, destination, options) → MovePlan
workspace.apply_edit(plan) → Result
```

**Arguments**:
```json
{
  "kind": "symbol" | "to_module" | "to_namespace" | "consolidate",
  "source": {
    // For symbol/module moves
    "file_path": "src/old.rs",
    "position": { "line": 10, "character": 5 },
    "range": { "start": {...}, "end": {...} },

    // For consolidate moves (Rust crate consolidation)
    "directory": "crates/old-crate"
  },
  "destination": {
    // For symbol/module moves
    "file_path": "src/new.rs",
    "module_path": "crate::new::module",
    "namespace": "new_namespace",

    // For consolidate moves
    "directory": "crates/target-crate/src/module"
  },
  "options": {
    "dry_run": true,
    "update_imports": true,
    "merge_dependencies": true  // for consolidate: merge Cargo.toml deps
  }
}
```

**Schema rules**:
- `kind="symbol" | "to_module" | "to_namespace"`: Use `source.file_path` + `destination.file_path` or `module_path`
- `kind="consolidate"`: Use `source.directory` + `destination.directory`

**Examples**:
```javascript
// Move symbol to different file
move.plan("symbol",
  { file_path: "old.rs", position: { line: 10, character: 5 } },
  { file_path: "new.rs" }
)

// Move code block to module
move.plan("to_module",
  { file_path: "app.rs", range: { start: {...}, end: {...} } },
  { module_path: "crate::utils" }
)

// Consolidate Rust crate (directory-level move)
move.plan("consolidate",
  { directory: "crates/old-crate" },
  { directory: "crates/target-crate/src/module" },
  { merge_dependencies: true }
)
```

---

### 5. Reorder Operations

**Commands**: 2 (was 4)

```javascript
reorder.plan(kind, target, new_order, options) → ReorderPlan
workspace.apply_edit(plan) → Result
```

**Arguments**:
```json
{
  "kind": "parameters" | "imports" | "members" | "statements",
  "target": {
    "file_path": "src/app.rs",
    "position": { "line": 10, "character": 5 },
    "range": { "start": {...}, "end": {...} }
  },
  "new_order": [2, 0, 1],  // for parameters
  "options": {
    "dry_run": true,
    "strategy": "alphabetical" | "visibility" | "dependency"  // for auto-ordering
  }
}
```

**Examples**:
- `reorder.plan("parameters", { file_path: "lib.rs", position: {...} }, { new_order: [1,0,2] })`
- `reorder.plan("imports", { file_path: "app.rs" }, { strategy: "alphabetical" })`
- `reorder.plan("members", { file_path: "lib.rs", position: {...} }, { strategy: "visibility" })`

---

### 6. Transform Operations

**Commands**: 2 (was 6)

```javascript
transform.plan(kind, target, options) → TransformPlan
workspace.apply_edit(plan) → Result
```

**Arguments**:
```json
{
  "kind": "to_arrow_function" | "to_async" | "loop_to_iterator" |
          "callback_to_promise" | "add_null_check" | "remove_dead_branch",
  "target": {
    "file_path": "src/app.ts",
    "position": { "line": 10, "character": 5 },
    "range": { "start": {...}, "end": {...} }
  },
  "options": {
    "dry_run": true,
    "language_specific": {}
  }
}
```

**Examples**:
- `transform.plan("to_async", { file_path: "app.js", position: {...} })`
- `transform.plan("loop_to_iterator", { file_path: "lib.rs", range: {...} })`
- `transform.plan("add_null_check", { file_path: "app.ts", range: {...} })`

---

### 7. Delete Operations

**Commands**: 2 (was 4)

```javascript
delete.plan(kind, target, options) → DeletePlan
workspace.apply_edit(plan) → Result
```

**Arguments**:
```json
{
  "kind": "unused_imports" | "dead_code" | "redundant_code" | "file",
  "target": {
    // For file-scoped deletions (unused_imports, file)
    "file_path": "src/app.rs",

    // For workspace/directory-scoped deletions (dead_code, redundant_code)
    "scope": "workspace" | "file" | "directory",  // optional, inferred from kind
    "path": "src/",  // optional, for directory scope

    // For range-specific deletions
    "range": { "start": {...}, "end": {...} }
  },
  "options": {
    "dry_run": true,
    "aggressive": false  // for dead code detection
  }
}
```

**Scope inference rules**:
- `kind="file"`: `scope` inferred as `"file"` from `file_path`, explicit `scope` ignored
- `kind="unused_imports"`: `scope` inferred as `"file"` from `file_path`
- `kind="dead_code"`: `scope` required (can be `"workspace"`, `"file"`, or `"directory"`)
- `kind="redundant_code"`: `scope` optional, defaults to `"file"` if `file_path` provided

**Examples**:
```javascript
// Delete unused imports from single file (scope inferred)
delete.plan("unused_imports", { file_path: "app.rs" })

// Delete dead code workspace-wide (scope explicit)
delete.plan("dead_code", { scope: "workspace" }, { aggressive: true })

// Delete dead code in directory (scope + path)
delete.plan("dead_code", { scope: "directory", path: "src/legacy/" })

// Delete specific file (scope inferred)
delete.plan("file", { file_path: "old.rs" })

// Delete redundant code in range (scope inferred from file_path)
delete.plan("redundant_code", {
  file_path: "app.rs",
  range: { start: { line: 10, character: 0 }, end: { line: 20, character: 0 } }
})
```

---

### 8. Shared Apply Command

**Single executor for all plans**:

```javascript
workspace.apply_edit(plan, options) → Result
```

**Arguments**:
```json
{
  "plan": { /* any plan from above */ },
  "options": {
    "dry_run": false,
    "validate_checksums": true,  // fail if files changed since plan creation
    "validate_plan_type": true,  // verify plan_type matches expected schema
    "force": false,              // skip all validation
    "rollback_on_error": true,
    "validation": {              // post-apply validation (optional)
      "command": "cargo check --workspace",
      "timeout_seconds": 60,
      "working_dir": ".",        // optional, defaults to workspace root
      "fail_on_stderr": false    // some tools write warnings to stderr
    }
  }
}
```

**Validation behavior**:
- `validate_checksums`: Rejects plans if file content has changed since plan creation
- `validate_plan_type`: Ensures plan structure matches expected discriminated type
- `force`: Bypasses all validation (dangerous, use only for recovery scenarios)
- `validation`: If provided, runs command after applying edits. If command fails (non-zero exit), automatically rolls back changes.

**Result**:
```json
{
  "success": true,
  "applied_files": ["src/lib.rs", "src/app.rs"],
  "created_files": ["src/new.rs"],
  "deleted_files": [],
  "warnings": [],
  "validation": {
    "passed": true,
    "command": "cargo check --workspace",
    "exit_code": 0,
    "stdout": "    Checking codebuddy v1.0.0\n    Finished check in 2.34s",
    "stderr": "",
    "duration_ms": 2340
  },
  "rollback_available": false  // consumed by validation if run
}
```

**Validation workflow**:
1. Apply edits to filesystem
2. If `validation` specified, run validation command
3. If validation fails (non-zero exit), automatically rollback all changes
4. Return result with validation details (passed/failed, output, timing)

---

## Implementation Approach

**IMPORTANT**: This proposal should be implemented as a **single, complete implementation** with all components delivered together. This is a beta product with no external users, so we can make breaking changes immediately.

**Prerequisites**: Phase 0 (self-registration system) is already complete ✅

### Complete Implementation Scope

Implement all components in a single pass:

1. **Core Plan Types** (`crates/cb-protocol/src/refactor_plan.rs`)
   - Define all 7 plan types: `RenamePlan`, `ExtractPlan`, `InlinePlan`, `MovePlan`, `ReorderPlan`, `TransformPlan`, `DeletePlan`
   - Include checksums, metadata, warnings, validation fields
   - Add plan version discriminators

2. **All 14 Commands** (7 operation families × 2 commands each)
   - `rename.plan()` + handler
   - `extract.plan()` + handler
   - `inline.plan()` + handler
   - `move.plan()` + handler
   - `reorder.plan()` + handler
   - `transform.plan()` + handler
   - `delete.plan()` + handler
   - `workspace.apply_edit()` - single executor for all plan types

3. **Plan Validation & Safety**
   - File checksum validation (SHA-256)
   - Plan type discriminator validation
   - Atomic rollback mechanism
   - Post-apply validation with automatic rollback

4. **Configuration Support**
   - `.codebuddy/refactor.toml` loader
   - Preset system with override support
   - Default configuration values

5. **Legacy Command Removal**
   - Remove all 35 legacy refactoring commands
   - Update all internal callsites to new API
   - Remove old handler implementations

6. **Testing & Documentation**
   - Integration tests for all 7 operation families
   - Validation scenario tests (pass/fail/timeout)
   - Update API_REFERENCE.md
   - Update CLAUDE.md with new tool list

**No phased rollout, no partial implementations, no legacy compatibility.**

---

## Command Reduction Summary

| Operation Family | Old Commands | New Commands | Reduction |
|-----------------|-------------|--------------|-----------|
| Rename | 6 | 2 | -67% |
| Extract | 7 | 2 | -71% |
| Inline | 4 | 2 | -50% |
| Move | 4 | 2 | -50% |
| Reorder | 4 | 2 | -50% |
| Transform | 6 | 2 | -67% |
| Delete | 4 | 2 | -50% |
| **TOTAL** | **35** | **14** | **-60%** |

---

## Benefits

### 1. Consistency
- Every operation follows identical `plan → apply` pattern
- Uniform error handling and validation
- Consistent dry-run behavior

### 2. Safety
- All operations preview-by-default
- Atomic apply with automatic rollback
- Validation before any file writes

### 3. Composability
- Plans can be inspected and validated
- Multiple plans can be merged before applying
- AI agents can reason about plans before execution

### 4. Simplicity
- 60% fewer commands to learn
- Single apply mechanism to understand
- Clear separation: planning vs execution

### 5. Extensibility
- New operation `kind` values added without new commands
- Options extend without breaking changes
- Language-specific features via `kind` + `options`

### 6. Discoverability
- `kind` parameter self-documents available operations
- Shared structure across all operations
- Better IDE autocomplete and validation

---

## Design Decisions

### 1. Naming: `workspace.apply_edit` (LOCKED)
**Decision**: Use `workspace.apply_edit` as the single executor.

**Rationale**:
- Aligns with LSP terminology (`WorkspaceEdit`)
- Familiar to developers using language servers
- Alternative `refactor.apply` considered but rejected for consistency

### 2. Plan Validation: Checksums Required (LOCKED)
**Decision**: All plans include `file_checksums` and `plan_type` discriminators.

**Rationale**:
- Prevents stale plans from corrupting code
- Type-safe validation before apply
- `validate_checksums` option defaults to `true`
- `force: true` escape hatch for recovery scenarios

### 3. No Legacy Support (LOCKED)
**Decision**: Remove all 35 legacy commands immediately, no wrappers.

**Rationale**:
- Beta product with no external users
- No migration burden or compatibility constraints
- Clean slate implementation of new API
- Simpler codebase without dual API support

### 4. Dry-run Default: False (LOCKED)
**Decision**: `dry_run` defaults to `false` in `workspace.apply_edit`.

**Rationale**:
- `*.plan` commands already provide preview (never write files)
- Explicit opt-in for dry-run in apply step
- Matches typical "preview then execute" workflow

### 5. Project-Level Configuration (PROMOTED TO PHASE 1)
**Decision**: Support `.codebuddy/refactor.toml` for preset configurations.

**Rationale**:
- Dramatically improves DX by eliminating repetitive option passing
- Ensures consistency across team members and AI agents
- Config file serves as living documentation of project standards
- Can be overridden per-call when needed

**Configuration format**:
```toml
# .codebuddy/refactor.toml
[presets.safe]
strict = true
validate_scope = true
update_imports = true

[presets.aggressive]
strict = false
force = true

[defaults]
dry_run = false
rollback_on_error = true
validate_checksums = true
```

**Usage**:
```javascript
// Use preset
rename.plan(target, new_name, { preset: "safe" })

// Override specific options
rename.plan(target, new_name, { preset: "safe", strict: false })
```

### 6. Plan Formatting Utility: Phase 2 (CLIENT LIBRARY)
**Decision**: Provide `formatPlan(plan)` utility in client libraries, NOT in plan structure.

**Rationale**:
- Keeps plan structure lightweight and focused on data
- Avoids redundancy (description duplicates structured summary)
- Allows customization of formatting without plan versioning concerns
- Enables localization if needed in future
- No maintenance burden for keeping descriptions accurate

**Example usage**:
```javascript
// Client-side utility (not part of plan)
import { formatPlan } from '@codebuddy/client';

const plan = await rename.plan(...);
const description = formatPlan(plan);
// Returns: "Renames function 'process_data' to 'parse_and_process_data' across 3 files"

// Use for logging, debugging, human-readable output
console.log(`Plan: ${description}`);
```

### 7. Batch Operations: Phase 3+ (DEFERRED)
**Decision**: Add `workspace.apply_batch([plan1, plan2])` in Phase 3 if needed.

**Rationale**:
- Not critical for MVP
- Can evaluate need after Phase 1-2 usage data
- Atomic multi-plan apply requires careful transaction design

---

## Success Criteria

- [ ] All 14 new commands implemented and tested
- [ ] `workspace.apply_edit` handles all 7 plan types
- [ ] Post-apply validation with automatic rollback implemented and tested
- [ ] Project-level configuration (`.codebuddy/refactor.toml`) with preset support
- [ ] Plan formatting utility (`formatPlan(plan)`) in client library
- [ ] All 35 legacy commands removed from codebase
- [ ] Integration tests cover all operation kinds
- [ ] Integration tests cover validation scenarios (pass/fail/timeout)
- [ ] All internal callsites updated to new API
- [ ] Documentation updated with validation and config examples
- [ ] CI validates no direct file writes in `*.plan` commands
- [ ] CI validates preset loading and override behavior

---

## Conclusion

This unified API reduces complexity while improving safety and composability. The plan/apply pattern provides a foundation for advanced features like plan validation, batch operations, and workflow automation.

**Recommendation**: Implement this proposal in its entirety as a single, complete deliverable.

---

## Implementation Instructions for AI Agents

### Step-by-Step Implementation Guide

This section provides detailed instructions for implementing the entire Unified Refactoring API in one complete pass.

#### Step 1: Create Core Plan Types (NEW FILE)

**File**: `crates/cb-protocol/src/refactor_plan.rs` (~300-400 lines)

Create all plan type definitions:

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Discriminated union type for all refactoring plans
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "plan_type")]
pub enum RefactorPlan {
    RenamePlan(RenamePlan),
    ExtractPlan(ExtractPlan),
    InlinePlan(InlinePlan),
    MovePlan(MovePlan),
    ReorderPlan(ReorderPlan),
    TransformPlan(TransformPlan),
    DeletePlan(DeletePlan),
}

/// Base structure shared by all plans
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanMetadata {
    pub plan_version: String,  // Always "1.0"
    pub kind: String,
    pub language: String,
    pub estimated_impact: String,  // "low" | "medium" | "high"
    pub created_at: String,  // ISO 8601 timestamp
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSummary {
    pub affected_files: usize,
    pub created_files: usize,
    pub deleted_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanWarning {
    pub code: String,
    pub message: String,
    pub candidates: Option<Vec<String>>,
}

// Define each plan type (RenamePlan, ExtractPlan, etc.) following the spec
// Include: edits, summary, warnings, metadata, file_checksums fields
```

**Export in** `crates/cb-protocol/src/lib.rs`:
```rust
pub mod refactor_plan;
pub use refactor_plan::*;
```

#### Step 2: Implement All 7 Operation Handlers (NEW FILES)

Create these handler files in `crates/cb-handlers/src/handlers/`:

1. **`rename_handler.rs`** (~300 lines)
   - Implement `rename.plan()` for symbols, files, directories
   - Parse target, validate inputs, generate RenamePlan
   - Leverage existing `rename_symbol`, `rename_file`, `rename_directory` logic

2. **`extract_handler.rs`** (~250 lines)
   - Implement `extract.plan()` for functions, variables, modules
   - Migrate logic from `RefactoringHandler::extract_function`, `extract_variable`
   - Generate ExtractPlan with proper edits

3. **`inline_handler.rs`** (~200 lines)
   - Implement `inline.plan()` for variables, functions, constants
   - Migrate logic from `RefactoringHandler::inline_variable`
   - Generate InlinePlan

4. **`move_handler.rs`** (~250 lines)
   - Implement `move.plan()` for symbols, modules, consolidation
   - Support crate consolidation mode (Rust-specific)
   - Generate MovePlan

5. **`reorder_handler.rs`** (~200 lines)
   - Implement `reorder.plan()` for parameters, imports, members
   - Support both manual order (`new_order: [1,0,2]`) and strategy (`"alphabetical"`)
   - Generate ReorderPlan

6. **`transform_handler.rs`** (~200 lines)
   - Implement `transform.plan()` for language-specific transformations
   - Support: `to_async`, `loop_to_iterator`, etc.
   - Generate TransformPlan

7. **`delete_handler.rs`** (~200 lines)
   - Implement `delete.plan()` for unused imports, dead code, files
   - Support scope inference (file/directory/workspace)
   - Generate DeletePlan

#### Step 3: Implement Unified Apply Handler (NEW FILE)

**File**: `crates/cb-handlers/src/handlers/workspace_apply_handler.rs` (~400 lines)

```rust
pub struct WorkspaceApplyHandler;

impl WorkspaceApplyHandler {
    /// Single executor for all plan types
    pub async fn apply_edit(
        plan: RefactorPlan,
        options: ApplyOptions,
        context: &ToolHandlerContext,
    ) -> ServerResult<ApplyResult> {
        // 1. Validate plan_type discriminator
        // 2. Validate file checksums if enabled
        // 3. Apply edits atomically using file_service.apply_edit_plan()
        // 4. If validation command provided, run it
        // 5. If validation fails, automatic rollback
        // 6. Return result with validation details
    }

    fn validate_checksums(plan: &RefactorPlan, file_service: &FileService) -> Result<()>;
    fn run_validation(validation: &ValidationConfig) -> Result<ValidationResult>;
}
```

#### Step 4: Register All Handlers

**File**: `crates/cb-handlers/src/handlers/plugin_dispatcher.rs`

Add to `create_tool_handlers()`:
```rust
handlers.push(Arc::new(RenameHandler::new()) as Arc<dyn ToolHandler>);
handlers.push(Arc::new(ExtractHandler::new()) as Arc<dyn ToolHandler>);
handlers.push(Arc::new(InlineHandler::new()) as Arc<dyn ToolHandler>);
handlers.push(Arc::new(MoveHandler::new()) as Arc<dyn ToolHandler>);
handlers.push(Arc::new(ReorderHandler::new()) as Arc<dyn ToolHandler>);
handlers.push(Arc::new(TransformHandler::new()) as Arc<dyn ToolHandler>);
handlers.push(Arc::new(DeleteHandler::new()) as Arc<dyn ToolHandler>);
handlers.push(Arc::new(WorkspaceApplyHandler::new()) as Arc<dyn ToolHandler>);
```

**File**: `crates/cb-handlers/src/handlers/mod.rs`

Export new handlers:
```rust
pub mod rename_handler;
pub mod extract_handler;
pub mod inline_handler;
pub mod move_handler;
pub mod reorder_handler;
pub mod transform_handler;
pub mod delete_handler;
pub mod workspace_apply_handler;

pub use rename_handler::RenameHandler;
pub use extract_handler::ExtractHandler;
// ... export all
```

#### Step 5: Implement Configuration Support (NEW FILE)

**File**: `crates/cb-core/src/refactor_config.rs` (~200 lines)

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactorConfig {
    pub presets: HashMap<String, RefactorPreset>,
    pub defaults: RefactorDefaults,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactorPreset {
    pub strict: Option<bool>,
    pub validate_scope: Option<bool>,
    pub update_imports: Option<bool>,
    // ... other options
}

impl RefactorConfig {
    pub fn load() -> Result<Self> {
        // Load from .codebuddy/refactor.toml
        // Merge with defaults
    }

    pub fn apply_preset(&self, preset_name: &str, options: &mut PlanOptions) -> Result<()>;
}
```

#### Step 6: Remove Legacy Commands

**Delete or deprecate**:
- `crates/cb-handlers/src/handlers/refactoring_handler.rs` (or adapt to delegate to new handlers)
- Remove legacy tool registrations from `plugin_dispatcher.rs`
- Remove from `tool_names()` lists

**Update callsites** (search for):
- `extract_function` → use `extract.plan("function", ...)`
- `inline_variable` → use `inline.plan("variable", ...)`
- `rename_symbol` → use `rename.plan({ kind: "symbol", ... }, ...)`
- etc.

#### Step 7: Add Tests

**File**: `integration-tests/test_unified_refactoring_api.rs` (~500-800 lines)

Test all 7 operation families:
```rust
#[tokio::test]
async fn test_rename_symbol_plan_and_apply() { }

#[tokio::test]
async fn test_extract_function_plan_and_apply() { }

#[tokio::test]
async fn test_validation_rollback_on_failure() { }

#[tokio::test]
async fn test_checksum_validation() { }

#[tokio::test]
async fn test_config_preset_loading() { }
// ... more tests
```

#### Step 8: Update Documentation

1. **API_REFERENCE.md**: Add new tools, remove legacy tools
2. **CLAUDE.md**: Update MCP tools list
3. **docs/design/unified_api_contracts.md**: Add formal API specs (if doesn't exist, create it)

### Files to Create (7 new files)

1. `crates/cb-protocol/src/refactor_plan.rs` - Plan types
2. `crates/cb-handlers/src/handlers/rename_handler.rs`
3. `crates/cb-handlers/src/handlers/extract_handler.rs`
4. `crates/cb-handlers/src/handlers/inline_handler.rs`
5. `crates/cb-handlers/src/handlers/move_handler.rs`
6. `crates/cb-handlers/src/handlers/reorder_handler.rs`
7. `crates/cb-handlers/src/handlers/transform_handler.rs`
8. `crates/cb-handlers/src/handlers/delete_handler.rs`
9. `crates/cb-handlers/src/handlers/workspace_apply_handler.rs`
10. `crates/cb-core/src/refactor_config.rs` - Config support
11. `integration-tests/test_unified_refactoring_api.rs` - Tests

### Files to Modify (8-12 files)

1. `crates/cb-protocol/src/lib.rs` - Export plan types
2. `crates/cb-handlers/src/handlers/mod.rs` - Export new handlers
3. `crates/cb-handlers/src/handlers/plugin_dispatcher.rs` - Register handlers
4. `crates/cb-handlers/src/handlers/refactoring_handler.rs` - Deprecate or adapt
5. `crates/cb-handlers/src/handlers/file_operation_handler.rs` - May need adaptation
6. `crates/cb-handlers/src/handlers/tools/editing.rs` - Update tool wrappers
7. `crates/cb-core/src/lib.rs` - Export refactor_config
8. `API_REFERENCE.md` - Update tool list
9. `CLAUDE.md` - Update tool list
10. `docs/design/unified_api_contracts.md` - Add/update formal specs

### Validation Checklist

After implementation, verify:

- [ ] All 14 commands (7 `*.plan` + 7 handlers + 1 `workspace.apply_edit`) implemented
- [ ] All plan types include: `plan_type`, `plan_version`, `edits`, `summary`, `warnings`, `metadata`, `file_checksums`
- [ ] Checksum validation works (rejects stale plans)
- [ ] Rollback works (reverts changes on error)
- [ ] Post-apply validation works (runs command, rolls back on failure)
- [ ] Configuration loading works (`.codebuddy/refactor.toml`)
- [ ] Preset system works (apply preset, override options)
- [ ] All 35 legacy commands removed
- [ ] All tests pass (`cargo nextest run --workspace`)
- [ ] Documentation updated (API_REFERENCE.md, CLAUDE.md)

### Expected Outcome

After completing these steps, the codebase should have:
- ✅ 14 new unified refactoring commands (60% reduction from 35 legacy commands)
- ✅ Consistent plan/apply pattern across all operations
- ✅ Type-safe plan validation with checksums
- ✅ Atomic rollback on errors
- ✅ Post-apply validation with automatic rollback
- ✅ Configuration system with presets
- ✅ Zero legacy refactoring commands
- ✅ Complete test coverage
- ✅ Updated documentation

**Estimated Total**: ~15-25 files modified/created, ~3,000-4,000 lines of new code (mostly straightforward adaptations of existing logic).
