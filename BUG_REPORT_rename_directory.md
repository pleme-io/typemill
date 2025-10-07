# [BUG] rename_directory doesn't update Cargo.toml path dependencies

## Status: Active Issue

**Severity**: Medium - Workaround available but breaks expected behavior

**Last Updated**: 2025-10-07

## Describe the bug

The `rename_directory` MCP tool successfully:
- ✅ Moves all files physically
- ✅ Updates documentation references (Markdown, text files)
- ❌ **Does NOT update Cargo.toml path dependencies**

This leaves the Rust workspace in a **broken state** where `cargo check` fails because Cargo.toml files still reference the old directory path.

## To Reproduce

1. **Run rename_directory on a Rust crate:**
   ```bash
   ./target/release/codebuddy tool rename_directory '{
     "old_path": "crates/languages/cb-lang-java",
     "new_path": "crates/cb-lang-java"
   }'
   ```

2. **Result returned shows success:**
   ```json
   {
     "documentation_updates": {
       "files_updated": 7,
       "references_updated": 15
     },
     "files_moved": 10,
     "import_updates": {
       "edits_applied": 0,
       "errors": [],
       "files_updated": 0
     },
     "success": true
   }
   ```

3. **Try to build:**
   ```bash
   cargo check
   ```

4. **Build fails:**
   ```
   error: failed to load manifest for workspace member

   Caused by:
     failed to load manifest for dependency `cb-lang-java`

   Caused by:
     failed to read `/workspace/crates/languages/cb-lang-java/Cargo.toml`

   Caused by:
     No such file or directory (os error 2)
   ```

## Expected behavior

`rename_directory` should detect and update **ALL references** to the moved directory, including:

1. ✅ Documentation files (Markdown, text) - **Currently works**
2. ✅ Source code imports - **Works for some languages**
3. ❌ **Cargo.toml path dependencies** - **Does NOT work**

## Actual behavior

### What gets updated automatically:
```diff
# docs/development/languages/PLUGIN_DEVELOPMENT_GUIDE.md
-cb-lang-java = { path = "crates/languages/cb-lang-java" } # ← Add this
+cb-lang-java = { path = "crates/cb-lang-java" } # ← Add this
```

### What does NOT get updated (requires manual fix):
```diff
# Cargo.toml (root)
-cb-lang-java = { path = "crates/languages/cb-lang-java" }
+cb-lang-java = { path = "crates/cb-lang-java" }  # ← Manual fix required

# crates/cb-handlers/Cargo.toml
-cb-lang-java = { path = "../languages/cb-lang-java", optional = true }
+cb-lang-java = { path = "../cb-lang-java", optional = true }  # ← Manual fix required

# crates/cb-services/Cargo.toml
-cb-lang-java = { path = "../languages/cb-lang-java", optional = true }
+cb-lang-java = { path = "../cb-lang-java", optional = true }  # ← Manual fix required
```

## Root Cause Analysis

The `rename_directory` tool appears to use different update strategies:

1. **Documentation scanner** - Finds markdown/text references and updates them ✅
2. **Import updater** - Updates source code imports (TypeScript, Python, Rust `use` statements) ✅
3. **Cargo.toml dependencies** - ❌ **Not implemented**

The tool does NOT recognize `path = "..."` dependencies in Cargo.toml as references that need updating.

## Workaround

After running `rename_directory`, manually update all Cargo.toml files:

```bash
# Search for old path references
grep -r "crates/languages/cb-lang-java" . --include="Cargo.toml"

# Manually edit each Cargo.toml to update path dependencies
```

## Impact

**Medium severity:**
- ❌ Breaks the build immediately
- ❌ Defeats purpose of "automatic import updates"
- ✅ Easy to detect (build fails)
- ✅ Straightforward manual fix
- ✅ Doesn't corrupt data

**Affects:** Any Rust workspace using `rename_directory` on crates with path dependencies

## Environment

- **OS**: Linux aarch64-unknown-linux-gnu
- **Rust version**: 1.90.0
- **codebuddy version**: 1.0.0-beta
- **MCP client**: Claude Code
- **Project**: Rust workspace with 20+ crates

## Suggested Fix

Add Cargo.toml path dependency detection to `rename_directory`:

**Pseudo-code:**
```rust
// In rename_directory handler:
1. Scan for Cargo.toml files in workspace
2. Parse each Cargo.toml
3. Find dependencies with `path = "..."` matching old directory
4. Update path to point to new directory
5. Write updated Cargo.toml back
```

**Similar to existing functionality:**
- The tool already updates documentation file references
- Just needs to add Cargo.toml to the list of files to scan/update

## Testing

Test case to verify fix:
```rust
#[test]
fn test_rename_directory_updates_cargo_toml() {
    // 1. Create test workspace with Cargo.toml path dependency
    // 2. Run rename_directory
    // 3. Assert Cargo.toml path updated
    // 4. Assert cargo check passes
}
```

## Related Issues

This was discovered during dogfooding in **00_PROPOSAL_TREE.md** - using CodeBuddy's own tools to reorganize the codebase.

**Test commit**: 76bef76 "refactor: move cb-lang-java to flat crates layout (Phase 2)"

## Notes

- **Previous bug** (column position errors) was FIXED in commit 81e79e2
- This is a **separate issue** - missing functionality rather than broken functionality
- Tool works correctly for what it does, just doesn't do enough
