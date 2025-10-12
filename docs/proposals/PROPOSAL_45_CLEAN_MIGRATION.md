# Proposal 45: Clean Legacy Handler Migration (No Shims)

**Status**: DRAFT
**Created**: 2025-10-12
**Strategy**: Complete removal of legacy handlers, full test migration

---

## Goal

**Remove all legacy analysis handlers completely** - no shims, no backward compatibility layer. Migrate all callers to the unified API.

---

## Current State Analysis

### Legacy Handlers Still Exist

1. **`analyze_project`** (crates/cb-handlers/src/handlers/tools/analysis/mod.rs:98)
   - Currently: Thin shim routing to `analyze.quality("maintainability")`
   - Callers: E2E tests (apps/codebuddy/tests/e2e_analysis_features.rs:438, :571)
   - Action: Delete shim, migrate tests

2. **`analyze_imports`** (crates/cb-handlers/src/handlers/tools/analysis/mod.rs:99)
   - Currently: Delegates to SystemToolsPlugin
   - Plugin: crates/cb-plugins/src/system_tools_plugin.rs:194
   - Action: Delete handler, ensure `analyze.dependencies("imports")` works

3. **`find_dead_code`** (crates/cb-handlers/src/handlers/analysis_handler.rs:139)
   - Currently: Legacy LSP-powered implementation
   - Callers: E2E tests (e2e_analysis_features.rs:52, e2e_workflow_execution.rs:362)
   - Action: Delete handler, migrate tests to `analyze.dead_code`

### Unified API Already Exists

✅ **analyze.quality("maintainability")** - workspace scope implemented
✅ **analyze.dependencies("imports")** - plugin-backed parsing implemented
✅ **analyze.dead_code** - LSP workspace mode implemented

**The functionality is there. We just need to remove the old entry points.**

---

## Migration Plan

### Phase 1: Test Migration (2-3 hours)

**Step 1: Migrate E2E Tests**

**File: apps/codebuddy/tests/e2e_analysis_features.rs**

```rust
// OLD (line 438):
client.call_tool("analyze_project", json!({
    "directory_path": workspace.path().to_str().unwrap(),
    "report_format": "full"
}))

// NEW:
client.call_tool("analyze.quality", json!({
    "kind": "maintainability",
    "scope": {
        "type": "workspace",
        "path": workspace.path().to_str().unwrap()
    }
}))
```

**File: apps/codebuddy/tests/e2e_analysis_features.rs**

```rust
// OLD (line 52):
client.call_tool("find_dead_code", json!({}))

// NEW:
client.call_tool("analyze.dead_code", json!({
    "kind": "unused_symbols",
    "scope": {
        "type": "workspace",
        "path": workspace.path().to_str().unwrap()
    }
}))
```

**Step 2: Migrate Workflow Tests**

**File: apps/codebuddy/tests/e2e_workflow_execution.rs**

```rust
// OLD (line 362):
client.call_tool("find_dead_code", json!({}))

// NEW:
client.call_tool("analyze.dead_code", json!({
    "kind": "unused_symbols",
    "scope": { "type": "workspace" }
}))
```

**Step 3: Update Test Assertions**

- Unified API returns structured `AnalysisResult` (not old formats)
- Update assertions to check: `result.findings`, `result.summary`, `result.scope`
- Remove expectations of old fields like `deadSymbols`, `workspacePath`

---

### Phase 2: Handler Removal (1 hour)

**Step 1: Delete Legacy Handlers**

```bash
# Remove analyze_project and analyze_imports from AnalysisHandler
# File: crates/cb-handlers/src/handlers/tools/analysis/mod.rs

# Remove tool_names entries (lines 97-100)
# Remove handle_tool_call cases (lines 117-143)

# Result: AnalysisHandler becomes empty (can be deleted entirely)
```

**Step 2: Remove find_dead_code from AnalysisHandler (analysis_handler.rs)**

```bash
# File: crates/cb-handlers/src/handlers/analysis_handler.rs
# Delete entire find_dead_code implementation (lines 139-300+)
```

**Step 3: Remove Legacy Plugin Handler**

```bash
# File: crates/cb-plugins/src/system_tools_plugin.rs
# Delete handle_analyze_imports (line 194)
```

---

### Phase 3: Cleanup (1 hour)

**Step 1: Remove Unused Types**

```bash
# File: crates/cb-types/src/model/mcp.rs
# Delete ProjectReportFormat enum (line 111)
# Delete any other unused analysis types
```

**Step 2: Update Tool Registration**

```rust
// File: crates/cb-server/tests/tool_registration_test.rs
// Remove expectations for:
// - "analyze_project"
// - "analyze_imports"
// - "find_dead_code"

// Update internal tool count: 23 → 20
```

**Step 3: Update Plugin Dispatcher**

```rust
// File: crates/cb-handlers/src/handlers/plugin_dispatcher.rs
// Remove legacy tool registrations (line 190)
// Update count: 23 → 20 internal tools
```

---

### Phase 4: Documentation (30 min)

**Update Files:**
- API_REFERENCE.md - Remove legacy tool sections
- QUICK_REFERENCE.md - Update internal tool counts
- TOOLS_VISIBILITY_SPEC.md - Remove legacy tool listings
- 45_PROPOSAL_LEGACY_HANDLER_RETIREMENT.md - Mark COMPLETE

---

## Expected Changes

### Files Deleted
- (None - handlers are methods, not files)

### Files Modified
- `apps/codebuddy/tests/e2e_analysis_features.rs` - Migrate 3 test functions
- `apps/codebuddy/tests/e2e_workflow_execution.rs` - Migrate 1 test function
- `crates/cb-handlers/src/handlers/tools/analysis/mod.rs` - Delete AnalysisHandler
- `crates/cb-handlers/src/handlers/analysis_handler.rs` - Delete find_dead_code
- `crates/cb-plugins/src/system_tools_plugin.rs` - Delete handle_analyze_imports
- `crates/cb-types/src/model/mcp.rs` - Delete ProjectReportFormat
- `crates/cb-server/tests/tool_registration_test.rs` - Update counts
- `crates/cb-handlers/src/handlers/plugin_dispatcher.rs` - Update registrations
- 4 documentation files

**Total**: ~10 files modified, 0 files deleted
**Net Lines**: ~-500 lines (removing legacy code)

---

## Success Criteria

**Tests:**
- [ ] All E2E tests pass with unified API names
- [ ] Workflow tests pass with unified API
- [ ] Tool registration tests pass with new counts
- [ ] No legacy tool names in test suite

**Code:**
- [ ] analyze_project handler deleted
- [ ] analyze_imports handler deleted
- [ ] find_dead_code handler deleted
- [ ] ProjectReportFormat type deleted
- [ ] Tool counts accurate (20 internal tools)

**Documentation:**
- [ ] API_REFERENCE.md updated
- [ ] QUICK_REFERENCE.md updated
- [ ] TOOLS_VISIBILITY_SPEC.md updated
- [ ] Proposal 45 marked COMPLETE

---

## Risk Assessment

**Low Risk:**
- Unified API already implemented and tested
- Plugin integration working
- LSP workspace mode functional

**Medium Risk:**
- Test assertions need updating (response format changes)
- E2E tests may need timeout adjustments
- Potential edge cases in response structure

**Mitigation:**
- Run tests frequently during migration
- Compare old vs new response formats
- Keep git commits small and focused

---

## Timeline

**Total: 4-5 hours**
- Phase 1 (Tests): 2-3 hours
- Phase 2 (Handlers): 1 hour
- Phase 3 (Cleanup): 1 hour
- Phase 4 (Docs): 30 min

**Can be done in single session or split into:**
- Session 1: Phase 1 (test migration)
- Session 2: Phases 2-4 (cleanup)

---

## Execution Strategy

### Option A: Sequential (Safe)
1. Migrate tests first
2. Verify tests pass with unified API
3. Delete legacy handlers
4. Run tests again
5. Clean up and document

### Option B: Parallel (Fast)
1. Agent 1: Migrate E2E tests
2. Agent 2: Migrate workflow tests
3. Merge and verify
4. Single agent: Delete handlers, cleanup, docs

**Recommendation**: Option A (sequential) - safer, easier to debug

---

## Rollback Plan

If tests fail or issues arise:

1. Revert test changes: `git revert <commit>`
2. Investigate failures with legacy handlers still in place
3. Fix unified API issues
4. Retry migration

**Git Strategy:**
- Commit 1: Test migration
- Commit 2: Handler removal
- Commit 3: Cleanup
- Commit 4: Documentation

Each commit is independently revertible.

---

## Open Questions

1. Should we remove thin shim commit (6be453ce) as part of this?
   - **Yes** - it was temporary, no longer needed

2. Should we update CHANGELOG.md with breaking changes?
   - **No** - these are internal tools, not public API

3. Should we add deprecation warnings first?
   - **No** - these are internal tools, direct removal is fine

---

## Approval Required

**Before proceeding:**
- [ ] User confirms: "Remove all legacy handlers, no shims"
- [ ] User confirms: "Migrate all tests to unified API"
- [ ] User confirms: Timeline acceptable (4-5 hours)

---

**This is the clean migration approach you wanted - no shims, no legacy code, just unified API.**

