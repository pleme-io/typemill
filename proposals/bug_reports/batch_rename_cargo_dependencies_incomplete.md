# Bug Report: Batch Rename Misses Critical Cargo.toml References

**Date**: 2025-10-22
**Reporter**: Dogfooding session (Claude Code)
**Severity**: High
**Component**: `rename.plan` / `rename` (batch mode with `targets` parameter)
**Affects**: Cargo workspace projects with complex dependency graphs

## Summary

The batch rename tool (`rename` with `targets` array) successfully renames directories and updates many files, but **misses critical Cargo.toml references** that prevent the workspace from building. Manual fixes are required for 5+ config files.

## Reproduction Steps

1. **Setup**: Cargo workspace with crates that have:
   - Workspace members list in root `Cargo.toml`
   - Workspace dependencies section in root `Cargo.toml`
   - Feature-gated dependencies in other crates
   - Cargo alias commands in `.cargo/config.toml`
   - cargo-deny rules in `deny.toml`

2. **Execute batch rename**:
   ```bash
   codebuddy tool rename '{
     "targets": [
       {
         "kind": "directory",
         "path": "crates/cb-lang-rust",
         "new_name": "crates/mill-lang-rust"
       },
       {
         "kind": "directory",
         "path": "crates/cb-lang-typescript",
         "new_name": "crates/mill-lang-typescript"
       }
     ],
     "options": {
       "scope": "all"
     }
   }'
   ```

3. **Result**: Directories renamed, 28 files updated, but `cargo check` fails

## Expected Behavior

With `"scope": "all"`, **all** Cargo.toml references should be updated atomically:

- ✅ Root `Cargo.toml` workspace members: `"crates/cb-lang-rust"` → `"crates/mill-lang-rust"`
- ✅ Root `Cargo.toml` workspace dependencies: `cb-lang-rust = { path = ... }` → `mill-lang-rust = { path = ... }`
- ✅ Dependent crate `Cargo.toml` files: `cb-lang-rust = { workspace = true }` → `mill-lang-rust = { workspace = true }`
- ✅ Feature flags: `lang-rust = ["dep:cb-lang-rust"]` → `lang-rust = ["dep:mill-lang-rust"]`
- ✅ `.cargo/config.toml` aliases
- ✅ `deny.toml` ban rules

**The workspace should build immediately after the rename.**

## Actual Behavior

The tool **partially** updated references:

### ✅ What Was Updated (28 files):
- Directory renames: `crates/cb-lang-rust/` → `crates/mill-lang-rust/`
- Package names in renamed crate's Cargo.toml: `name = "mill-lang-rust"`
- Some import statements in .rs files
- Some documentation .md files
- Some proposal documents

### ❌ What Was **NOT** Updated:

1. **Root `Cargo.toml` workspace members** (lines 10-11):
   ```toml
   # MISSED - Still references old paths
   members = [
       "crates/cb-lang-rust",           # ❌ Should be mill-lang-rust
       "crates/cb-lang-typescript",     # ❌ Should be mill-lang-typescript
   ]
   ```

2. **Root `Cargo.toml` workspace dependencies** (lines 99-100):
   ```toml
   # MISSED - Still references old crate names
   cb-lang-rust = { path = "crates/cb-lang-rust", default-features = false }
   cb-lang-typescript = { path = "crates/cb-lang-typescript", default-features = false }
   ```

3. **`crates/mill-services/Cargo.toml`** (line 47):
   ```toml
   [dev-dependencies]
   cb-lang-rust = { workspace = true }  # ❌ Breaks build - workspace dependency not found
   ```

4. **`crates/mill-plugin-bundle/Cargo.toml`** (lines 13, 24):
   ```toml
   # MISSED - Path dependency
   cb-lang-rust = { path = "../cb-lang-rust", optional = true }

   [features]
   lang-rust = ["dep:cb-lang-rust"]  # ❌ Feature reference
   ```

5. **`.cargo/config.toml`** (lines 115-116):
   ```toml
   # MISSED - Cargo alias commands
   check-lang = "check -p mill-lang-common -p cb-lang-rust -p cb-lang-typescript"
   test-lang = "nextest run -p mill-lang-common -p cb-lang-rust -p cb-lang-typescript"
   ```

6. **`deny.toml`** (lines 264, 268):
   ```toml
   # MISSED - Dependency ban rules
   [[bans.deny]]
   name = "cb-lang-rust"

   [[bans.deny]]
   name = "cb-lang-typescript"
   ```

## Build Failure

```
error: failed to load manifest for workspace member `/workspace/crates/mill-services`

Caused by:
  error inheriting `cb-lang-rust` from workspace root manifest's `workspace.dependencies.cb-lang-rust`

Caused by:
  `dependency.cb-lang-rust` was not found in `workspace.dependencies`
```

## Manual Fixes Required

After rename, **5 files** had to be manually edited:
1. Root `Cargo.toml` (2 sections: members + dependencies)
2. `crates/mill-services/Cargo.toml`
3. `crates/mill-plugin-bundle/Cargo.toml` (2 sections: dependencies + features)
4. `.cargo/config.toml`
5. `deny.toml`

**Total manual edits**: ~12 lines across 5 files

## Root Cause Analysis

The rename tool appears to use **different update strategies** for different file types:

### Working Strategies:
- ✅ **Directory names in paths** - Updated correctly
- ✅ **Package names** in renamed crate's own `Cargo.toml`
- ✅ **Import statements** in Rust source files

### Broken Strategies:
- ❌ **Workspace members list** - Not detected as path reference?
- ❌ **Workspace dependency keys** - Crate name not updated
- ❌ **Workspace dependency consumers** - `{ workspace = true }` references miss the rename
- ❌ **Feature flag dependencies** - `dep:crate-name` syntax not recognized
- ❌ **Cargo config commands** - `-p crate-name` flags not updated
- ❌ **TOML [[array.entry]] name fields** - Not recognized as crate references

## Hypotheses

**Theory 1: String literal path detection is too conservative**
- The tool updates paths like `"crates/cb-lang-rust/src"` (contains `/`)
- But misses `"crates/cb-lang-rust"` in workspace members (also contains `/`)
- **Inconsistent behavior for same pattern**

**Theory 2: Workspace dependency graph not analyzed**
- Tool doesn't understand Cargo workspace dependency semantics
- Doesn't know that `cb-lang-rust = { workspace = true }` references `workspace.dependencies.cb-lang-rust`
- **Missing Cargo-specific knowledge**

**Theory 3: TOML plugin limited scope**
- TOML plugin may only update path values, not name fields
- `[[bans.deny]]` with `name = "cb-lang-rust"` is a name, not a path
- Feature flags `dep:crate-name` are not path strings
- **TOML plugin needs Cargo-aware semantics**

## Impact

**High severity** because:
- ❌ Workspace doesn't build after rename
- ❌ Manual intervention required (defeats purpose of automation)
- ❌ Easy to miss references (found 5 files, could miss more in larger workspaces)
- ❌ Error messages are cryptic (workspace dependency inheritance errors)
- ❌ Breaks dogfooding (can't use our own tools for cb-* → mill-* refactor)

## Comparison: Single vs Batch Rename

**Same issue occurs in both modes:**
- Single rename: `codebuddy tool rename '{"target": {...}, "new_name": "..."}'`
- Batch rename: `codebuddy tool rename '{"targets": [...]}'`

This is **not** a batch-specific bug, but a general Cargo.toml update coverage issue.

## Workaround

After running rename command:

1. Manually grep for old crate name:
   ```bash
   grep -r "cb-lang-rust" --include="*.toml" .
   ```

2. Edit each file found:
   - Root `Cargo.toml`: members + workspace.dependencies
   - Dependent crate `Cargo.toml` files
   - `.cargo/config.toml`
   - `deny.toml`

3. Run `cargo check` to verify

## Proposed Fix

### Option A: Enhance TOML Plugin (Recommended)

Add Cargo-specific awareness to TOML language plugin:

```rust
// In crates/mill-lang-toml/src/cargo_support.rs
pub fn update_cargo_references(content: &str, old_name: &str, new_name: &str) -> String {
    // 1. Workspace members: "crates/old-name" → "crates/new-name"
    // 2. Workspace dependencies: old_name = { ... } → new_name = { ... }
    // 3. Feature deps: dep:old-name → dep:new-name
    // 4. Ban rules: name = "old-name" → name = "new-name"
}
```

### Option B: Add Post-Rename Hook

After directory rename, scan workspace for:
- All `Cargo.toml` files
- All `.cargo/config.toml` files
- All `deny.toml` files

Apply crate-name updates across all matches.

### Option C: Improve Path Detection

Expand string literal detection to include:
- Workspace member paths (even without file extension)
- Crate names in dependency sections
- Package names in ban rules

## Test Cases Needed

Add integration tests for Cargo workspace scenarios:

```rust
#[test]
fn test_rename_updates_workspace_members() {
    // Rename crate, verify workspace members list updated
}

#[test]
fn test_rename_updates_workspace_dependencies() {
    // Rename crate, verify workspace.dependencies key updated
}

#[test]
fn test_rename_updates_dependent_crates() {
    // Rename crate, verify dependents using { workspace = true } still build
}

#[test]
fn test_rename_updates_feature_flags() {
    // Rename crate, verify feature flags updated
}

#[test]
fn test_rename_updates_cargo_config() {
    // Rename crate, verify .cargo/config.toml aliases updated
}

#[test]
fn test_rename_updates_deny_toml() {
    // Rename crate, verify deny.toml ban rules updated
}
```

## Related Issues

- Similar to Python package renames (need to update `pyproject.toml`, `setup.py`)
- Similar to Node.js package renames (need to update `package.json` dependencies)
- **Pattern**: Rename tools need language ecosystem awareness

## Files for Investigation

- `crates/mill-lang-toml/src/lib.rs` - TOML plugin implementation
- `crates/mill-handlers/src/handlers/rename_handler/directory_rename.rs` - Directory rename logic
- `crates/mill-handlers/src/handlers/rename_handler/mod.rs` - Batch rename orchestration
- `crates/mill-lang-toml/src/import_support_impl.rs` - TOML path rewriting

## Stash Reference

Changes stashed in: `WIP: Batch rename cb-lang-rust and cb-lang-typescript - incomplete tool updates`

Contains:
- Manual fixes for all 5 config files
- Working state after manual intervention
- Can be used to diff expected vs actual tool behavior

## Next Steps

1. [ ] Add comprehensive Cargo workspace test fixtures
2. [ ] Implement Cargo-aware TOML updates (Option A)
3. [ ] Add test coverage for workspace dependency scenarios
4. [ ] Document Cargo workspace rename limitations (if partial fix)
5. [ ] Consider similar fixes for other package managers (npm, poetry)
