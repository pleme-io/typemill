# Compat Layer Removal Tracking

**Created:** 2025-10-09
**Status:** Planned
**Effort:** 1-2 days
**Priority:** Medium (technical debt)

---

## Overview

The codebase currently has a compatibility layer (`crates/cb-handlers/src/handlers/compat.rs`) that wraps legacy tool handlers. This was used during a migration but is now creating technical debt.

## Affected Files (13 total)

1. `crates/cb-handlers/src/handlers/tools/workspace.rs`
2. `crates/cb-handlers/src/handlers/tools/system.rs`
3. `crates/cb-handlers/src/handlers/tools/file_ops.rs`
4. `crates/cb-handlers/src/handlers/tools/editing.rs`
5. `crates/cb-handlers/src/handlers/tools/advanced.rs`
6. `crates/cb-handlers/src/handlers/refactoring_handler.rs`
7. `crates/cb-handlers/src/handlers/file_operation_handler.rs`
8. `crates/cb-handlers/src/handlers/analysis_handler.rs`
9. `crates/cb-handlers/src/handlers/workflow_handler.rs`
10. `crates/cb-handlers/src/handlers/tools/lifecycle.rs`
11. `crates/cb-handlers/src/handlers/tools/internal_editing.rs`
12. `crates/cb-handlers/src/handlers/system_handler.rs`
13. `crates/cb-handlers/src/handlers/dependency_handler.rs`

## Migration Pattern

All files follow this pattern:
```rust
use crate::handlers::compat::*;  // Remove this

// Old: Wrap legacy handler
pub struct NewHandler {
    inner: LegacyHandler,
}

// New: Direct implementation
pub struct NewHandler {
    // Direct fields
}
```

## Tasks

### Phase 1: Analysis (2 hours)
- [ ] For each file, identify what `compat::*` imports
- [ ] Document dependencies on `LegacyRefactoringHandler` and similar
- [ ] Create file-by-file migration checklist

### Phase 2: Migration (1 day)
- [ ] Refactor editing.rs (remove LegacyRefactoringHandler wrapping)
- [ ] Refactor workspace.rs
- [ ] Refactor system.rs
- [ ] Refactor file_ops.rs
- [ ] Refactor advanced.rs
- [ ] Refactor remaining 8 files

### Phase 3: Cleanup (2 hours)
- [ ] Delete `crates/cb-handlers/src/handlers/compat.rs`
- [ ] Delete `delegate_to_legacy!` macro in `macros.rs`
- [ ] Remove compatibility type alias `pub type ToolContext = ToolHandlerContext`
- [ ] Update tests

### Phase 4: Verification (1 hour)
- [ ] Run full test suite
- [ ] Verify no references to compat module remain
- [ ] Update documentation

## Success Criteria

- ✅ Zero imports of `handlers::compat`
- ✅ `compat.rs` file deleted
- ✅ All tests passing
- ✅ No `LegacyRefactoringHandler` wrapping
- ✅ Type alias `ToolContext` removed

## Notes

- This is **purely internal refactoring** - no API changes
- Can be done incrementally (one handler at a time)
- Low risk if done with tests running

## Related Documentation

- See [30_PROPOSAL_CODE_QUALITY.md](../../30_PROPOSAL_CODE_QUALITY.md) for full context
- Part of MEDIUM RISK technical debt cleanup
- HIGH RISK items already verified clean (no active usage of deprecated methods)
