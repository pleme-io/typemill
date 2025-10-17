# Proposal 02g: Complete Cargo Package Rename Coverage

## Problem

Directory renames involving Cargo packages fail to update critical manifest references, requiring manual fixes and causing build failures:

1. **Root workspace `Cargo.toml` not updated** - Workspace members list still references old directory name
2. **Package name in moved `Cargo.toml` not updated** - Package retains old name after directory rename
3. **Dev-dependency references not updated** - Other crates referencing the renamed package via dev-dependencies fail to resolve
4. **String literals in Rust code status unknown** - `rewrite_string_literals()` exists but unclear if wired into rename workflow

**Impact:** `integration-tests → tests` rename required manual fixes to 3 Cargo.toml files before build would succeed.

## Solution

Extend `cargo::plan_workspace_manifest_updates` to detect and update all Cargo manifest references during directory renames:

### 1. Root Workspace Manifest Updates

Scan and update the root workspace `Cargo.toml` members list when renaming a directory that is a workspace member.

**Current behavior:**
```toml
# /workspace/Cargo.toml - NOT UPDATED
members = [
    "integration-tests",  # ❌ Breaks build after rename
]
```

**Expected behavior:**
```toml
# /workspace/Cargo.toml - UPDATED
members = [
    "tests",  # ✅ Auto-updated during rename
]
```

### 2. Package Name Updates

Update the `[package] name` field in the moved directory's `Cargo.toml` to match the new directory name.

**Current behavior:**
```toml
# /workspace/tests/Cargo.toml - NOT UPDATED
[package]
name = "integration-tests"  # ❌ Mismatch with directory name
```

**Expected behavior:**
```toml
# /workspace/tests/Cargo.toml - UPDATED
[package]
name = "tests"  # ✅ Matches new directory name
```

### 3. Dev-Dependency References

Scan all `Cargo.toml` files in workspace for `[dev-dependencies]` referencing the old package name/path.

**Current behavior:**
```toml
# /workspace/apps/codebuddy/Cargo.toml - NOT UPDATED
[dev-dependencies]
integration-tests = { path = "../../integration-tests" }  # ❌ Path broken
```

**Expected behavior:**
```toml
# /workspace/apps/codebuddy/Cargo.toml - UPDATED
[dev-dependencies]
tests = { path = "../../tests" }  # ✅ Both name and path updated
```

### 4. Verify String Literal Support

Confirm `rewrite_string_literals()` is called during Rust file renames to update hardcoded paths like:
```rust
let config = "integration-tests/fixtures/test.toml";
```

## Checklists

### Root Workspace Updates
- [x] Add `find_workspace_root()` helper to locate `/workspace/Cargo.toml`
- [x] Scan workspace members list for old directory name
- [x] Generate TextEdit to update workspace members array
- [x] Add test: rename workspace member, verify root manifest updated

### Package Name Updates
- [x] After directory rename, read moved `Cargo.toml`
- [x] Parse `[package] name` field
- [x] Generate TextEdit if name matches old directory name
- [x] Add test: rename package directory, verify package name updated

### Dev-Dependency Scanning
- [x] Extend `scan_cargo_files()` to check `[dev-dependencies]` sections
- [x] Match both package name and path in dependency declarations
- [x] Generate TextEdits for both name and path fields
- [x] Add test: rename package, verify dev-dependency references updated across workspace

### String Literal Integration
- [x] Verify `rewrite_string_literals()` called in `reference_updater::update_references()`
- [x] If not wired: integrate string literal rewriting for Rust files
- [x] Add test: rename directory, verify hardcoded path strings updated in .rs files

### Integration Tests
- [x] Test full workflow: rename `integration-tests → tests`
- [x] Assert workspace Cargo.toml updated
- [x] Assert package name updated
- [x] Assert dev-dependencies updated
- [x] Assert build succeeds without manual fixes

## Success Criteria

1. **Zero manual Cargo.toml edits required** after directory rename of workspace member
2. **`cargo build` succeeds immediately** after rename operation
3. **All 4 critical issues resolved:**
   - ✅ Root workspace manifest updated
   - ✅ Package name updated
   - ✅ Dev-dependency references updated
   - ✅ String literals handled (confirmed working or integrated)
4. **Test coverage:** New integration test demonstrates complete rename without manual intervention

## Benefits

- **Eliminates manual fixes** for Cargo package renames
- **Prevents build failures** after rename operations
- **Achieves true "comprehensive rename coverage"** as documented
- **Improved reliability** for Rust-specific rename operations
- **Consistent behavior** across all Cargo manifest fields (members, dependencies, dev-dependencies)
