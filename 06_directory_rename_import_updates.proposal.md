# Proposal 06: Fix Directory Rename Import Updates

**Status**: 🚧 In Progress
**Priority**: High
**Complexity**: Medium

## Problem Statement

When renaming a directory (e.g., `src/core` → `src/legacy`), imports in external files are not updated. Files importing from the renamed directory retain old paths:

```typescript
// Before: src/app.ts
import { api } from './core/api';  // ❌ Should update to './legacy/api'
```

**Root Cause**: The TypeScript plugin's `rewrite_file_references()` receives directory paths and calculates `from './core'` as the import string to replace. This never matches real imports like `from './core/api'`, so no edits are generated.

## Current Status

### Completed ✅
1. **Comprehensive integration tests** (`test_rename_with_imports.rs`)
   - 3/4 tests passing (file renames work correctly)
   - 1 failing test clearly demonstrates the directory rename bug

2. **Root cause analysis**
   - Plugin expects file paths, not directory paths
   - Regex matching fails when looking for `'./core'` vs `'./core/api'`

3. **Implementation started** (Commit 7f597941)
   - Per-file iteration through renamed directory
   - Calculate new path for each file
   - Call plugin with individual file paths

### Remaining Work 🚧
1. **Debug why edits don't reach the plan**
   - Add logging to verify files are found
   - Confirm plugin is called with correct paths
   - Verify TextEdits are created and added to plan

2. **Verify fix works**
   - All 4 integration tests should pass
   - Directory rename test should update external imports

## Technical Approach

### Option 1: Per-File Processing (Current Implementation)
For each file inside the renamed directory:
```rust
// OLD: Pass directory paths
plugin.rewrite_file_references(&content, "src/core", "src/legacy", ...);
// Plugin generates: from './core' (doesn't match './core/api')

// NEW: Pass individual file paths
plugin.rewrite_file_references(&content, "src/core/api.ts", "src/legacy/api.ts", ...);
// Plugin generates: from './core/api' → from './legacy/api' ✅
```

**Advantages**:
- ✅ No plugin API changes
- ✅ Works with all language plugins
- ✅ Leverages existing tested logic

## Files Modified

- `crates/cb-ast/src/import_updater/edit_builder.rs` - Per-file iteration logic
- `crates/cb-handlers/src/handlers/rename_handler.rs` - Call `plan_rename_directory_with_imports()`
- `crates/cb-services/src/services/file_service/rename.rs` - Add `plan_rename_directory_with_imports()` method
- `integration-tests/src/test_rename_with_imports.rs` - Comprehensive test coverage

## Test Coverage

**File:** `integration-tests/src/test_rename_with_imports.rs`

| Test | Status | Description |
|------|--------|-------------|
| `test_rename_file_updates_imports_from_fixtures` | ✅ Pass | Fixture-based file rename tests |
| `test_rename_file_updates_parent_directory_importer` | ✅ Pass | File in subdirectory |
| `test_rename_file_updates_sibling_directory_importer` | ✅ Pass | Cross-directory imports |
| `test_directory_rename_updates_all_imports` | ❌ Fail | **Directory rename (target fix)** |

## Next Steps

1. **Add debug logging** to trace execution path
2. **Verify** files inside directory are found correctly
3. **Confirm** plugin calls generate TextEdits
4. **Ensure** edits are included in the RenamePlan
5. **Test** that all 4 integration tests pass
6. **Commit** working solution

## References

- Initial commit: 5b96bbc9 (tests + infrastructure)
- Implementation: 7f597941 (per-file processing)
- Related: `cb-lang-typescript/src/import_support.rs` (plugin implementation)
