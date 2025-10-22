# Bug Report: Markdown and YAML Files Not Discovered During Rename Operations

**Status**: 🔴 **CRITICAL** - Causes incomplete renames, broken references
**Affects**: `rename.plan`, `move.plan` - any operation that scans for file references
**Discovered**: 2025-10-21

## Summary

When renaming a crate or directory, the file discovery mechanism fails to find markdown (`.md`) and YAML (`.yaml`, `.yml`) files that contain references to the renamed item. This results in broken documentation links, outdated configuration files, and incomplete refactoring plans.

## Impact

**Real-world example from mill-test-support → some-test-support rename:**

- **Expected**: 29 files with references should be updated
- **Actual**: Only 22 files in the plan (76% coverage)
- **Missing**: 9 critical files including:
  - `docs/architecture/layers.md` - Documentation
  - `deny.toml` - Configuration (not YAML but similar issue)
  - `proposals/*.md` - 4 proposal documents
  - `apps/codebuddy/tests/e2e_analysis_features.rs` - Wait, this is a .rs file! (separate issue)

**For markdown/YAML specifically:**

Log evidence:
```
Found files for extension, extension: "markdown", files_found: 0
```

But `docs/guide.md` exists in the workspace and contains references.

## Root Cause

The file discovery logic in the reference updater is not properly scanning for markdown and YAML files.

**Evidence from logs:**

```rust
// From ../../crates/mill-services/src/services/move_service/planner.rs:273
INFO Found files for extension, extension: "markdown", files_found: 0
INFO Found files for extension, extension: "toml", files_found: 3  ✅
INFO Found files for extension, extension: "yaml", files_found: 0
INFO Found files for extension, extension: "yml", files_found: 0
INFO No documentation or config file updates needed  ❌ WRONG!
```

The scanner:
- ✅ Finds `.rs` files correctly
- ✅ Finds `.toml` files correctly
- ❌ **Fails to find `.md` files**
- ❌ **Fails to find `.yaml`/`.yml` files**

## Reproduction

### Test Case 1: Simplified Reproduction (FAILS as expected)

```bash
cargo nextest run -p e2e test_file_discovery_in_non_standard_locations
```

**Test creates:**
```
/tmp/test-workspace/
  crates/my-crate/           # Source crate
  docs/guide.md              # Contains: "use my_crate::helper;"
  proposals/01_feature.md    # Contains: "uses my_crate"
```

**Expected behavior:**
- Both markdown files should be in the rename plan
- Markdown links/references should be updated

**Actual behavior:**
```
Found files for extension, extension: "markdown", files_found: 0
❌ BUG: docs/guide.md not in plan
```

**Files found**: Only 5 files (3 Cargo.toml, 2 .rs files)
**Files missing**: 2 markdown files

### Test Case 2: Real-world Reproduction

```bash
./target/debug/codebuddy tool rename.plan '{
  "target": {"kind": "directory", "path": "../../crates/mill-test-support"},
  "new_name": "crates/mill-test-support"
}'
```

**Missing files include:**
- `docs/architecture/layers.md`
- `proposals/00_rename_to_typemill.proposal.md`
- `proposals/archived/06_workspace_consolidation.proposal.md`
- `proposals/bug_reports/codebuddy_core_consolidation_issues.md`
- `proposals/bug_reports/consolidation_dogfooding_issues.md`

All contain `cb-test-support` or `cb_test_support` references.

## Expected Behavior

When `scope: "all"` is specified (the default), the rename planner should:

1. **Scan all relevant file types**:
   - ✅ Rust files (.rs)
   - ✅ TOML files (.toml)
   - ❌ Markdown files (.md, .markdown)
   - ❌ YAML files (.yaml, .yml)

2. **Update all discovered references**:
   - Code imports/uses
   - Documentation links
   - Configuration paths
   - Markdown code examples

3. **Report accurate coverage**:
   - Summary should show all affected files
   - User should see comprehensive list before execution

## Affected Code Paths

### Primary suspect: File discovery logic

**Location**: `../../crates/mill-services/src/services/reference_updater/mod.rs`

The `find_project_files()` function appears to have special handling for different file types, but markdown/YAML may not be included in the scan.

**Log evidence points to**: `../../crates/mill-services/src/services/move_service/planner.rs:273`

This is where the scanner reports finding 0 markdown files despite them existing in the workspace.

### Relevant code sections:

1. **File scanning**:
   ```rust
   // ../../crates/mill-services/src/services/reference_updater/mod.rs
   pub async fn find_project_files(
       project_root: &Path,
       old_path: &Path,
       plugins: &[Arc<dyn LanguagePlugin>],
   ) -> Result<Vec<PathBuf>> {
       // BUG: This function is not finding markdown/YAML files
   }
   ```

2. **Extension-based scanning**:
   ```rust
   // ../../crates/mill-services/src/services/move_service/planner.rs:273
   // Logs: "Found files for extension, extension: \"markdown\", files_found: 0"
   ```

3. **Scope handling**:
   ```rust
   // crates/codebuddy-foundation/src/core/rename_scope.rs
   pub fn all() -> Self {
       Self {
           update_docs: true,      // ✅ Enabled
           update_configs: true,    // ✅ Enabled
           // But files aren't being FOUND in the first place!
       }
   }
   ```

## Why Previous Test Didn't Catch This

The first regression test I wrote (`test_cross_workspace_import_updates`) **PASSED** because:

- It only tested `.rs` files (which work correctly)
- It created a simple workspace structure
- All test files were in standard Rust locations (apps/, crates/, tests/)

The bug was only caught when testing with markdown files in docs/ and proposals/ directories.

## Fix Strategy

### Investigation needed:

1. **Check `find_project_files()` implementation**:
   - Does it filter by plugin-handled extensions?
   - Is markdown/YAML plugin registered for file scanning?
   - Is there a hardcoded allowlist that excludes these types?

2. **Verify plugin registration**:
   - Is `MarkdownPlugin` registered in the plugin system?
   - Does it implement the required traits for file discovery?
   - Is `YamlPlugin` even instantiated?

3. **Check glob patterns**:
   - Are there glob patterns that exclude docs/ or proposals/ directories?
   - Is there a .gitignore-style filter removing these files?

### Likely fix locations:

1. **Option A: Plugin registration issue**
   ```rust
   // Ensure markdown/YAML plugins are registered for scanning
   // Location: Plugin initialization code
   ```

2. **Option B: File scanner filter**
   ```rust
   // Remove or fix filter that excludes non-code files
   // Location: reference_updater/mod.rs::find_project_files()
   ```

3. **Option C: Extension mapping**
   ```rust
   // Add markdown/YAML extensions to scanner allowlist
   // Location: move_service/planner.rs
   ```

## Regression Tests

### Test 1: File Discovery (FAILS - catches bug)
**Location**: `tests/e2e/src/test_file_discovery_bug.rs`

```rust
#[tokio::test]
async fn test_file_discovery_in_non_standard_locations()
```

**Status**: ❌ **FAILS** - This is expected and proves the bug exists

**Verifies**:
- Markdown files in docs/ are discovered
- Markdown files in proposals/ are discovered
- YAML files are discovered
- References in all file types are updated

### Test 2: Cross-Workspace Imports (PASSES)
**Location**: `tests/e2e/src/test_cross_workspace_import_updates.rs`

```rust
#[tokio::test]
async fn test_rename_crate_updates_all_workspace_imports()
```

**Status**: ✅ PASSES - But doesn't test markdown/YAML

**Covers**: Rust file imports only

## Success Criteria

After fix, the following should work:

1. **Test suite passes**:
   ```bash
   cargo nextest run test_file_discovery_in_non_standard_locations
   # Should PASS (currently fails)
   ```

2. **Real-world rename includes all files**:
   ```bash
   ./target/debug/codebuddy tool rename.plan '{
     "target": {"kind": "directory", "path": "../../crates/mill-test-support"},
     "new_name": "crates/mill-test-support"
   }'
   # Should show 29 files (currently shows 22)
   ```

3. **Coverage verification**:
   ```bash
   # All files with references should be in plan
   rg "cb-test-support|cb_test_support" --files-with-matches | wc -l  # 29
   # vs
   # Files in rename.plan
   jq '.content.edits.documentChanges | length' plan.json  # Should also be 29
   ```

4. **Log output**:
   ```
   Found files for extension, extension: "markdown", files_found: 2  ✅ (not 0)
   Found files for extension, extension: "yaml", files_found: 0  ✅ (if no YAML files)
   ```

## Related Issues

- This may be related to the file extension routing in the plugin system
- The capability trait pattern refactor (Proposal 07) may have inadvertently broken this
- See: `docs/architecture/internal_tools.md` for background on file operation changes

## Priority

**CRITICAL** - Blocks TypeMill rename project

- Without this fix, renaming crates will leave broken references
- Documentation will become stale and misleading
- Configuration files won't reflect new names
- Manual cleanup required for every rename operation

**Workaround**: Manual search-and-replace for .md and .yaml files after rename completes.

---

**Reporter**: Claude (AI Assistant)
**Date**: 2025-10-21
**Test coverage**: Regression test added in `tests/e2e/src/test_file_discovery_bug.rs`
