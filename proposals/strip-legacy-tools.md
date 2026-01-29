# Proposal: Strip Legacy Tools - Magnificent Seven Only

## Executive Summary

Remove all 41 internal/legacy tool registrations and only expose the **Magnificent Seven** public API. The codebase will be simplified by eliminating the dual-layer architecture where new handlers wrap legacy handlers.

## Current State

```
Public Tools (7):     inspect_code, search_code, rename_all, relocate, prune, refactor, workspace
Internal Tools (41):  find_definition, find_references, rename, delete, move, extract, inline, etc.
Handler Files (32):   Implementing ToolHandler trait
```

**Problem**: The new Magnificent Seven handlers mostly wrap legacy handlers, creating:
- Unnecessary indirection
- Duplicate code paths
- Maintenance burden
- Confusion about which tools are "real"

## Proposed Architecture

### Keep (7 handlers)
```
crates/mill-handlers/src/handlers/
├── inspect_handler.rs      # Direct LSP calls, no legacy wrapper
├── search_handler.rs       # Direct workspace search
├── rename_all_handler.rs   # Inline rename logic from rename_handler
├── relocate_handler.rs     # Inline move logic from move_handler
├── prune_handler.rs        # Inline delete logic from delete_handler
├── refactor_handler.rs     # Inline extract/inline logic
└── workspace_handler.rs    # Inline workspace operations
```

### Delete (25+ files)

#### Category 1: Legacy Refactoring Handlers (5 files)
```
DELETE: rename_handler/mod.rs        → Logic moves to rename_all_handler.rs
DELETE: move/mod.rs                  → Logic moves to relocate_handler.rs
DELETE: delete_handler.rs            → Logic moves to prune_handler.rs
DELETE: extract_handler.rs           → Logic moves to refactor_handler.rs
DELETE: inline_handler.rs            → Logic moves to refactor_handler.rs
```

#### Category 2: Legacy Navigation Tools (1 file)
```
DELETE: tools/navigation.rs          → Logic moves to inspect_handler.rs
```

#### Category 3: Legacy Workspace Tools (3 files)
```
DELETE: tools/workspace_create.rs    → Logic in workspace_handler.rs
DELETE: tools/workspace_extract_deps.rs → Logic in workspace_handler.rs
DELETE: workspace/find_replace_handler.rs → Logic in workspace_handler.rs
```

#### Category 4: Internal-Only Tools (can delete or keep as services)
```
EVALUATE: tools/lifecycle.rs         → Keep as internal service (no tool)
EVALUATE: tools/internal_editing.rs  → Keep as internal service (no tool)
EVALUATE: tools/internal_workspace.rs → Keep as internal service (no tool)
EVALUATE: tools/internal_intelligence.rs → Keep as internal service (no tool)
EVALUATE: tools/advanced.rs          → Keep execute_batch as internal?
EVALUATE: tools/file_ops.rs          → Keep as internal service
EVALUATE: tools/editing.rs           → Merge into rename_all_handler
EVALUATE: tools/system.rs            → health_check moves to workspace_handler
```

#### Category 5: Unused/Deprecated
```
DELETE: refactoring_handler.rs       → Superseded by refactor_handler.rs
DELETE: workflow_handler.rs          → If unused
DELETE: dependency_handler.rs        → If unused
DELETE: file_operation_handler.rs    → Merge into prune_handler/relocate_handler
```

## Implementation Plan

### Phase 1: Inline Legacy Logic into Magnificent Seven (2-3 days)

For each Magnificent Seven handler, inline the logic from its legacy counterpart:

| New Handler | Inline From | Key Methods |
|-------------|-------------|-------------|
| `rename_all_handler.rs` | `rename_handler/mod.rs` | `handle_symbol_rename`, `handle_file_rename`, `handle_directory_rename` |
| `relocate_handler.rs` | `move/mod.rs` | `handle_symbol_move`, `handle_file_move`, `handle_directory_move` |
| `prune_handler.rs` | `delete_handler.rs` | `handle_symbol_delete`, `handle_file_delete`, `handle_directory_delete` |
| `refactor_handler.rs` | `extract_handler.rs`, `inline_handler.rs` | Extract/inline operations |
| `inspect_handler.rs` | `tools/navigation.rs` | Already direct LSP calls |
| `search_handler.rs` | N/A | Already direct implementation |
| `workspace_handler.rs` | `tools/workspace_*.rs` | Create package, extract deps, find/replace |

### Phase 2: Remove Legacy Handler Registrations (1 day)

```rust
// plugin_dispatcher.rs - BEFORE
register_handlers_with_logging!(registry, {
    // Magnificent Seven
    InspectHandler => "InspectHandler: inspect_code",
    // ... 6 more

    // REMOVE ALL OF THESE:
    RenameHandler => "RenameHandler: rename",
    DeleteHandler => "DeleteHandler: delete",
    MoveHandler => "MoveHandler: move",
    ExtractHandler => "ExtractHandler: extract",
    InlineHandler => "InlineHandler: inline",
    NavigationHandler => "NavigationHandler: find_*",
    // ... 35 more internal tools
});

// AFTER
register_handlers_with_logging!(registry, {
    // Magnificent Seven ONLY
    InspectHandler => "InspectHandler: inspect_code",
    SearchHandler => "SearchHandler: search_code",
    RenameAllHandler => "RenameAllHandler: rename_all",
    RelocateHandler => "RelocateHandler: relocate",
    PruneHandler => "PruneHandler: prune",
    RefactorHandler => "RefactorHandler: refactor",
    WorkspaceHandler => "WorkspaceHandler: workspace"
});
```

### Phase 3: Delete Legacy Files (1 day)

```bash
# Legacy refactoring
rm -rf crates/mill-handlers/src/handlers/rename_handler/
rm -rf crates/mill-handlers/src/handlers/move/
rm crates/mill-handlers/src/handlers/delete_handler.rs
rm crates/mill-handlers/src/handlers/extract_handler.rs
rm crates/mill-handlers/src/handlers/inline_handler.rs

# Legacy tools
rm crates/mill-handlers/src/handlers/tools/navigation.rs
rm crates/mill-handlers/src/handlers/tools/workspace_create.rs
rm crates/mill-handlers/src/handlers/tools/workspace_extract_deps.rs
rm crates/mill-handlers/src/handlers/workspace/find_replace_handler.rs

# Deprecated
rm crates/mill-handlers/src/handlers/refactoring_handler.rs
```

### Phase 4: Update Tool Registry (1 day)

```rust
// tool_registry.rs - Simplified
pub struct ToolRegistry {
    handlers: HashMap<String, Arc<dyn ToolHandler>>,
    // REMOVE: internal_tools tracking
    // REMOVE: is_internal() checks
}

impl ToolRegistry {
    pub fn list_tools(&self) -> Vec<String> {
        // Simply return all registered tools (only 7)
        self.handlers.keys().cloned().collect()
    }

    // REMOVE: list_internal_tools()
    // REMOVE: is_internal_tool()
}
```

### Phase 5: Update Tests (1 day)

```rust
// tool_registration_test.rs
#[tokio::test]
async fn test_only_magnificent_seven_registered() {
    let registry = create_registry();
    let tools = registry.list_tools();

    assert_eq!(tools.len(), 7);
    assert!(tools.contains(&"inspect_code".to_string()));
    assert!(tools.contains(&"search_code".to_string()));
    assert!(tools.contains(&"rename_all".to_string()));
    assert!(tools.contains(&"relocate".to_string()));
    assert!(tools.contains(&"prune".to_string()));
    assert!(tools.contains(&"refactor".to_string()));
    assert!(tools.contains(&"workspace".to_string()));
}

// DELETE: test_all_internal_tools_are_registered_and_hidden
```

### Phase 6: Clean Up (1 day)

- Remove `is_internal()` from `ToolHandler` trait
- Remove internal tool tracking from registry
- Update documentation
- Remove legacy tool schemas from plugin system

## Files to Modify/Delete Summary

### DELETE (17+ files)
```
handlers/rename_handler/mod.rs
handlers/rename_handler/scope.rs (if exists)
handlers/move/mod.rs
handlers/move/symbol_move.rs (if exists)
handlers/delete_handler.rs
handlers/extract_handler.rs
handlers/inline_handler.rs
handlers/refactoring_handler.rs
handlers/tools/navigation.rs
handlers/tools/workspace_create.rs
handlers/tools/workspace_extract_deps.rs
handlers/workspace/find_replace_handler.rs
handlers/tools/system.rs (merge health_check into workspace)
handlers/tools/editing.rs (if superseded)
```

### MODIFY (10+ files)
```
handlers/mod.rs                    # Remove exports
handlers/plugin_dispatcher.rs      # Remove legacy registrations
handlers/tool_registry.rs          # Simplify, remove internal tracking
handlers/rename_all_handler.rs     # Inline legacy logic
handlers/relocate_handler.rs       # Inline legacy logic
handlers/prune_handler.rs          # Inline legacy logic
handlers/refactor_handler.rs       # Inline legacy logic
handlers/workspace_handler.rs      # Inline legacy logic
handlers/tools/mod.rs              # Remove exports
```

### KEEP AS SERVICES (no tool registration)
```
handlers/tools/lifecycle.rs        # File open/save/close notifications
handlers/tools/internal_editing.rs # rename_symbol_with_imports service
handlers/tools/internal_workspace.rs # apply_workspace_edit service
handlers/tools/file_ops.rs         # File CRUD services
handlers/tools/advanced.rs         # execute_batch/execute_edits services
```

## Benefits

1. **Simpler Architecture**: 7 tools instead of 48
2. **Less Code**: ~3000+ lines deleted
3. **Clearer API**: No internal/public distinction
4. **Easier Maintenance**: Single code path per operation
5. **Faster Registration**: 7 handlers instead of 32+
6. **Better Testing**: Test 7 tools, not 48

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Breaking internal tool users | These were internal - external users should use Magnificent Seven |
| Service code needed elsewhere | Keep services without tool registration |
| Complex logic in legacy handlers | Carefully inline, preserve all edge cases |
| Test coverage gaps | Update E2E tests to cover all functionality via new API |

## Timeline

| Phase | Duration | Description |
|-------|----------|-------------|
| 1 | 2-3 days | Inline legacy logic into M7 handlers |
| 2 | 1 day | Remove legacy registrations |
| 3 | 1 day | Delete legacy files |
| 4 | 1 day | Simplify tool registry |
| 5 | 1 day | Update tests |
| 6 | 1 day | Clean up & docs |
| **Total** | **7-8 days** | |

## Decision Points

1. **Keep services without tools?**
   - Recommend: YES - lifecycle, file_ops, internal_editing are useful internally

2. **Keep execute_batch/execute_edits?**
   - Recommend: YES as internal services for workflow execution

3. **Deprecation period?**
   - Recommend: NO - internal tools were never public API

## Next Steps

1. Review and approve this proposal
2. Create feature branch `strip-legacy-tools`
3. Execute phases 1-6
4. PR review and merge
