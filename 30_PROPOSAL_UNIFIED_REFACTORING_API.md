# Proposal: Unified Refactoring API

**Status**: Draft
**Author**: Project Team
**Date**: 2025-10-10
**Formal Spec**: [docs/design/unified_api_contracts.md](docs/design/unified_api_contracts.md)

---

## Executive Summary

Consolidate 35 refactoring commands into **14 unified commands** using a consistent **plan → apply** pattern. This reduces API surface by 60% while improving safety, composability, and discoverability.

**Context**: This is a beta product with no external users. We can make breaking changes immediately without migration paths or legacy support.

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

**No migration needed**: This is a beta product with no external users.

**Phased implementation** (see [35_IMPLEMENTATION_SEQUENCING.md](35_IMPLEMENTATION_SEQUENCING.md) for detailed timeline):

### Phase 0: Foundation (PREREQUISITE)
- **Self-registration system** for plugin capability discovery
- Registry descriptors enable dynamic validation of `kind` values
- **Blocks**: All unified API work until complete
- **Timeline**: 2-3 weeks

### Phase 1A: Core Refactoring (4-5 weeks)
1. Implement all 14 new commands (`*.plan` + `workspace.apply_edit`)
2. Plan validation (checksums, types)
3. Rollback mechanism
4. **No config/validation yet** - inline options only

### Phase 1B: Configuration (1-2 weeks, parallel with 1C)
1. `.codebuddy/refactor.toml` loader
2. Preset resolution with overrides
3. Config validation against registry

### Phase 1C: Post-Apply Validation (1-2 weeks, parallel with 1B)
1. Command executor in `workspace.apply_edit`
2. Automatic rollback on validation failure
3. Timeout handling

### Phase 4: Client Utilities (1-2 weeks)
1. `formatPlan(plan)` utility in client library
2. Plan diff visualization

### Legacy Removal
- Remove all 35 legacy commands after Phase 1C complete
- Update all internal callsites to use new API
- Update documentation

**Critical dependency**: Phase 0 (self-registration) must complete before Phase 1A.

**No deprecation period, no legacy wrappers during beta.**

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

**Recommendation**: Approve and begin Phase 1 implementation.
