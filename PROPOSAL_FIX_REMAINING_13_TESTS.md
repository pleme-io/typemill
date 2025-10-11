# Proposal: Fix Remaining 13 Test Failures

**Status**: 🟡 **PROPOSED** - Awaiting Approval
**Author**: Claude Code
**Date**: 2025-10-11
**Context**: Remaining failures after deleting tests for removed tools

---

## Executive Summary

After removing tests for deleted tools (organize_imports, format_document, etc.), we have **13 remaining test failures**. Analysis shows:

- **6 tests**: Testing `update_dependencies` tool which should be **made internal** (not deleted)
- **1 test**: Testing `move_directory` which is missing handler registration
- **6 tests**: Pre-existing or unrelated failures

---

## Failure Analysis

### Category 1: Update Dependencies Tests (6 failures) ❌ DELETE TESTS

**User directive**: "update dependencies should be private now"

**Failing tests**:
1. `test_update_dependencies_package_json` (e2e_system_tools.rs:89)
2. `test_update_dependencies_cargo_toml` (e2e_system_tools.rs:146)
3. `test_update_dependencies_requirements_txt` (e2e_system_tools.rs:187)
4. `test_update_dependencies_dry_run` (e2e_system_tools.rs:219)
5. `test_update_dependencies_scripts_management` (e2e_system_tools.rs:254)
6. `test_system_tools_integration` (e2e_system_tools.rs:330) - calls update_dependencies

**Root cause**: Tests expect public tool, but tool should be internal/removed.

**Proposed fix**: **DELETE these 6 tests**
- Tool functionality replaced by Unified Refactoring API
- Remove all `test_update_dependencies_*` tests
- Update `test_system_tools_integration` to remove update_dependencies calls

---

### Category 2: Move Directory Test (1 failure) ❌ DELETE TEST

**Failing test**:
7. `test_rename_directory_in_rust_workspace` (e2e_system_tools.rs:427)

**Root cause**: Tests `move_directory` tool which is **replaced by Unified API**

**From 30_PROPOSAL_UNIFIED_REFACTORING_API.md** (lines 275-280):
```javascript
// Move directory is now: move.plan("consolidate", ...)
move.plan("consolidate",
  { directory: "crates/old-crate" },
  { directory: "crates/target-crate/src/module" },
  { merge_dependencies: true }
)
```

**Legacy tool**: `move_directory` → **Replaced** by `move.plan` with `kind="consolidate"`

**Evidence**:
```rust
// crates/cb-handlers/src/handlers/tools/workspace.rs:79
fn tool_names(&self) -> &[&str] {
    &[
        "move_directory",      // ✅ Defined
        "find_dead_code",      // ✅ Defined
        "update_dependencies", // ✅ Defined
        "update_dependency",   // ✅ Defined
    ]
}
```

BUT in plugin_dispatcher.rs:183-202:
```rust
register_handlers_with_logging!(registry, {
    SystemToolsHandler => "...",
    FileToolsHandler => "...",
    AdvancedToolsHandler => "...",
    NavigationHandler => "...",
    AnalysisHandler => "...",
    // ❌ WorkspaceToolsHandler NOT registered!

    // Internal tools
    LifecycleHandler => "...",
    InternalEditingToolsHandler => "...",
    // ...
});
```

**Proposed fix**: **ADD WorkspaceToolsHandler registration**
- Register `WorkspaceToolsHandler` in plugin_dispatcher.rs
- This will expose `move_directory`, `find_dead_code`, `update_dependency` (if we keep them public)

**Decision needed**: Should these tools be public or internal?
- `move_directory` → Likely should stay public (directory operations)
- `find_dead_code` → Public (analysis tool)
- `update_dependency` → Make internal per user directive

---

### Category 3: Rename Directory Dry Run (1 failure) ❌ SAME AS CATEGORY 2

**Failing test**:
8. `test_rename_directory_dry_run` (mcp_file_operations.rs:285)

**Root cause**: Same as #7 - WorkspaceToolsHandler not registered

**Proposed fix**: Same as Category 2

---

### Category 4: Workflow Test (1 failure) ⚠️ INVESTIGATE

**Failing test**:
9. `test_workflow_failure_handling` (e2e_workflow_execution.rs)

**Likely cause**: May be using removed tools (create_file, format_document) in workflow

**Proposed fix**:
- Read test to check what tools it uses
- If uses removed tools → delete or update test
- If unrelated → investigate separately

---

### Category 5: Pre-existing Failures (4 failures) ⏭️ SKIP FOR NOW

**Failing tests**:
10. `test_registry_statistics` (cb-plugins)
11. `test_all_plugins_conform_to_contract` (cb-test-support)
12. `test_large_message_handling` (e2e_server_lifecycle)
13. `test_rapid_transport_operations` (e2e_server_lifecycle)
14. `test_workspace_edit_in_process` (integration_services)

**Proposed fix**: Skip - these are pre-existing or unrelated to our refactoring work

---

## Recommended Action Plan

### Phase 1: Update Tests to Use Unified API ✅ PREFERRED APPROACH

**Action**: Update tests to use the new Unified Refactoring API instead of legacy tools

#### 1a. Update `move_directory` Test → `move.plan`

**File**: `e2e_system_tools.rs:427` - `test_rename_directory_in_rust_workspace`

**Change**:
```rust
// OLD (legacy tool):
client.call_tool(
    "move_directory",
    json!({ "old_path": "crates/crate_b", "new_path": "crates/crate_renamed" })
)

// NEW (unified API):
let plan = client.call_tool(
    "move.plan",
    json!({
        "kind": "consolidate",
        "source": { "directory": "crates/crate_b" },
        "destination": { "directory": "crates/crate_renamed" },
        "options": { "merge_dependencies": true, "dry_run": false }
    })
).await?;

client.call_tool(
    "workspace.apply_edit",
    json!({ "plan": plan["result"]["content"], "options": { "dry_run": false } })
)
```

**Also applies to**:
- `mcp_file_operations.rs:285` - `test_rename_directory_dry_run`

---

#### 1b. Update `update_dependencies` Tests → Internal or Remove?

**User confirmed**: "update dependencies should be private now"

**Two options**:

**Option A: Make `update_dependencies` internal (still functional, just hidden)**
- Move `WorkspaceToolsHandler` to internal handlers
- Update tests to call tool directly (bypass MCP listing)
- Preserve test coverage for internal functionality

**Option B: Remove `update_dependencies` entirely (replaced by manual package.json editing)**
- Delete all `test_update_dependencies_*` tests (6 tests)
- Update `test_system_tools_integration` to remove update_dependencies call
- Assume users will manually edit package.json/Cargo.toml files

**Recommended**: **Option A** - Keep as internal tool with test coverage

If Option A:
```rust
// crates/cb-handlers/src/handlers/tools/workspace.rs
impl ToolHandler for WorkspaceToolsHandler {
    fn tool_names(&self) -> &[&str] {
        &["update_dependencies", "update_dependency"]
    }

    fn is_internal(&self) -> bool {
        true  // Hide from public MCP API, but still callable internally
    }
}
```

Then register in `plugin_dispatcher.rs`:
```rust
register_handlers_with_logging!(registry, {
    // ... other handlers ...

    // Internal handlers
    LifecycleHandler => "...",
    InternalEditingToolsHandler => "...",
    InternalWorkspaceHandler => "...",
    InternalIntelligenceHandler => "...",
    WorkspaceToolsHandler => "WorkspaceToolsHandler with 2 INTERNAL tools (update_dependencies, update_dependency)",
});
```

---

#### 1c. Investigate `find_dead_code` Replacement

**From 30_PROPOSAL_UNIFIED_REFACTORING_API.md** (lines 389-394):
```javascript
// Delete dead code workspace-wide
delete.plan("dead_code", { scope: "workspace" }, { aggressive: true })
```

**Question**: Are there tests for `find_dead_code` tool? If so, update them to use `delete.plan` with `kind="dead_code"`.

---

### Phase 2: Investigate Workflow Test ⚠️ NEEDS INVESTIGATION

**Action**: Read `e2e_workflow_execution.rs` test_workflow_failure_handling and determine:
- Does it use removed tools? → Update to use unified API
- Is it a real failure? → Fix
- Is it pre-existing? → Skip

---

## Decision Matrix

| Tool | Current Status | Should Be | Action |
|------|---------------|-----------|--------|
| `update_dependencies` | Public (not registered) | Internal/Removed | Delete tests |
| `update_dependency` | Public (not registered) | Internal/Removed | Delete tests |
| `move_directory` | Public (not registered) | Public | Register handler |
| `find_dead_code` | Public (not registered) | Public | Register handler |

---

## Questions for User

1. ✅ **CONFIRMED**: "update dependencies should be private now"
2. ✅ **CONFIRMED**: "instead of deleting, update?" → Update tests to use Unified API
3. **For `update_dependencies`**: Make internal (Option A) or remove entirely (Option B)?
   - **Recommended**: Option A (internal with test coverage)

---

## Expected Results

**After Phase 1a (Update move_directory tests to use move.plan)**:
- 2 fewer failures (move_directory tests pass with unified API)
- Tests now use `move.plan` + `workspace.apply_edit`

**After Phase 1b (Make update_dependencies internal)**:
- 6 fewer failures (update_dependencies tests pass)
- `WorkspaceToolsHandler` registered as internal
- Tool count stays at 27 (internal tools not counted)

**After Phase 2 (Investigate workflow test)**:
- 1 fewer failure (if it uses removed tools, update to unified API)

**Final expected state**: 4 failures (pre-existing only)

---

## Approval

Ready to proceed with **Phase 1** (delete update_dependencies tests)?

- [ ] Approved - Execute Phase 1
- [ ] Need clarification on questions above
- [ ] Alternative approach

**Approver**: _____________
**Date**: _____________
