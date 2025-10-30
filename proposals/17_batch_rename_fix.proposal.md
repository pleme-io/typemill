# Proposal 17: Fix Batch Rename Workspace Manifest Updates

## Problem

Commit 8b9b6344 attempted to implement batch workspace manifest updates for multi-target rename operations but contains multiple critical bugs that prevent compilation:

1. **Import violation**: Tries to import `mill_lang_rust` directly in `mill-handlers` (not a dependency)
2. **API mismatch**: Uses `context.project_root` instead of `context.app_state.project_root`
3. **Type error**: Uses `lsp_types::Url` instead of `url::Url`
4. **Architecture violation**: Handlers calling language-specific code breaks plugin abstraction

**Impact:**
- Batch rename workspace updates never executed (didn't compile)
- Each target plans workspace manifest changes independently against original state
- Last edit overwrites previous changes (e.g., gitignore member not updated when renamed with cpp)
- Duplicate full-file edits require filtering

**Current Workaround:**
Individual `plan_directory_rename` calls handle workspace updates per-target, creating duplicate edits that get filtered. Works for single renames but fails for batch operations.

## Solution

Implement batch workspace manifest updates through the proper abstraction layers:

**Architecture:**
```
RenameHandler (batch mode)
    ↓
MoveService.plan_batch_directory_moves()
    ↓
PluginRegistry.get_workspace_plugin()
    ↓
RustPlugin.plan_workspace_updates_for_batch()
```

**Key Changes:**
1. Add `plan_batch_directory_moves()` method to MoveService
2. Add `WorkspaceManifestSupport` trait with `plan_workspace_updates_for_batch()` method
3. Implement trait in RustPlugin using existing `cargo_util::plan_workspace_manifest_updates_for_batch()`
4. Call MoveService batch method from RenameHandler before individual plans
5. Keep existing full-file edit deduplication logic

**Maintains:**
- ✅ Language-agnostic handler design
- ✅ Plugin-based architecture
- ✅ Service layer abstraction
- ✅ Existing individual rename behavior

## Checklists

### MoveService Enhancement
- [ ] Add `plan_batch_directory_moves()` method to MoveService trait
- [ ] Detect workspace manifest files (Cargo.toml with `[workspace]`)
- [ ] Query PluginRegistry for workspace plugin by file extension
- [ ] Return combined workspace manifest updates for all moves
- [ ] Add unit tests for batch planning

### Plugin Trait & Implementation
- [ ] Define `WorkspaceManifestSupport` trait in mill-plugin-api
- [ ] Add `plan_workspace_updates_for_batch(&[(PathBuf, PathBuf)])` method signature
- [ ] Implement trait for RustPlugin using existing cargo_util function
- [ ] Register trait capability in RustPlugin metadata
- [ ] Update PluginRegistry to query for workspace manifest capability

### RenameHandler Integration
- [ ] Replace buggy batch workspace code in `plan_batch_rename()` (mod.rs:536-550)
- [ ] Call `move_service.plan_batch_directory_moves()` for directory targets
- [ ] Merge workspace updates into `all_document_changes` before individual plans
- [ ] Keep existing full-file edit deduplication logic (lines 624-638)
- [ ] Verify workspace updates appear first in final edit list

### Testing
- [ ] Add integration test: batch rename 2 Rust crates, verify both workspace members updated
- [ ] Add test: batch rename with non-Rust files, verify no workspace updates
- [ ] Add test: batch rename within single crate, verify no workspace manifest changes
- [ ] Verify existing single-rename tests still pass
- [ ] Test with cpp + gitignore scenario from investigation

### Documentation
- [ ] Add debug logging to new MoveService method
- [ ] Document WorkspaceManifestSupport trait in plugin development docs
- [ ] Update CLAUDE.md batch rename section with fixed implementation details
- [ ] Remove TODO comment from rename_handler/mod.rs after fix

## Success Criteria

1. **Compilation**: `cargo build --release -p mill` succeeds with no errors
2. **Batch Workspace Updates**: Renaming `[crates/a, crates/b]` → `[languages/a, languages/b]` updates both workspace members in single Cargo.toml edit
3. **Architecture Compliance**: No direct language-specific imports in mill-handlers
4. **Test Coverage**: All batch rename integration tests pass
5. **Backward Compatibility**: Existing single-rename operations unchanged

## Benefits

- **Correctness**: Workspace manifests updated atomically for all batch targets
- **Architecture**: Maintains clean separation between handlers, services, and plugins
- **Maintainability**: Future workspace types (npm workspaces, etc.) follow same pattern
- **Performance**: Single workspace manifest edit instead of N conflicting edits
- **Reliability**: No more "last edit wins" behavior in batch operations
