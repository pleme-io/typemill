# Proposal: Unified Refactoring API

**Status**: Draft
**Author**: Project Team
**Date**: 2025-10-10

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
    "file_path": "src/old.rs",
    "position": { "line": 10, "character": 5 },
    "range": { "start": {...}, "end": {...} }  // for multi-line moves
  },
  "destination": {
    "file_path": "src/new.rs",
    "module_path": "crate::new::module",
    "namespace": "new_namespace"
  },
  "options": {
    "dry_run": true,
    "update_imports": true
  }
}
```

**Examples**:
- `move.plan("symbol", { file_path: "old.rs", position: {...} }, { file_path: "new.rs" })`
- `move.plan("to_module", { file_path: "app.rs", range: {...} }, { module_path: "utils" })`
- `move.plan("consolidate", { source_dir: "crates/old" }, { target_dir: "crates/new/module" })`

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
    "file_path": "src/app.rs",
    "scope": "workspace" | "file" | "directory",
    "range": { "start": {...}, "end": {...} }  // for specific ranges
  },
  "options": {
    "dry_run": true,
    "aggressive": false  // for dead code detection
  }
}
```

**Examples**:
- `delete.plan("unused_imports", { file_path: "app.rs" })`
- `delete.plan("dead_code", { scope: "workspace" }, { aggressive: true })`
- `delete.plan("file", { file_path: "old.rs" })`

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
    "rollback_on_error": true
  }
}
```

**Validation behavior**:
- `validate_checksums`: Rejects plans if file content has changed since plan creation
- `validate_plan_type`: Ensures plan structure matches expected discriminated type
- `force`: Bypasses all validation (dangerous, use only for recovery scenarios)

**Result**:
```json
{
  "success": true,
  "applied_files": ["src/lib.rs", "src/app.rs"],
  "created_files": ["src/new.rs"],
  "deleted_files": [],
  "warnings": [],
  "rollback_available": true
}
```

---

## Implementation Approach

**No migration needed**: This is a beta product with no external users.

**Direct implementation**:
1. Implement all 14 new commands (`*.plan` + `workspace.apply_edit`)
2. Remove all 35 legacy commands immediately
3. Update all internal callsites to use new API
4. Update documentation

**No deprecation period, no legacy wrappers, no telemetry tracking.**

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

### 5. Batch Operations: Phase 3+ (DEFERRED)
**Decision**: Add `workspace.apply_batch([plan1, plan2])` in Phase 3 if needed.

**Rationale**:
- Not critical for MVP
- Can evaluate need after Phase 1-2 usage data
- Atomic multi-plan apply requires careful transaction design

---

## Success Criteria

- [ ] All 14 new commands implemented and tested
- [ ] `workspace.apply_edit` handles all 7 plan types
- [ ] All 35 legacy commands removed from codebase
- [ ] Integration tests cover all operation kinds
- [ ] All internal callsites updated to new API
- [ ] Documentation updated
- [ ] CI validates no direct file writes in `*.plan` commands

---

## Conclusion

This unified API reduces complexity while improving safety and composability. The plan/apply pattern provides a foundation for advanced features like plan validation, batch operations, and workflow automation.

**Recommendation**: Approve and begin Phase 1 implementation.
