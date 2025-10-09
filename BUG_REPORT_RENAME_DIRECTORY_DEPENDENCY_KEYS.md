# Bug Report: `rename_directory` Does Not Update Cargo Dependency Keys

**Date:** 2025-10-09
**Reporter:** Claude Code
**Severity:** Medium
**Component:** `rename_directory` tool, Rust language plugin
**Status:** Identified

---

## Summary

When using `codebuddy tool rename_directory` to rename a Rust crate (e.g., `test-support` → `cb-test-support`), the tool successfully:
- ✅ Moves the directory and files
- ✅ Updates the workspace `members` list in root `Cargo.toml`
- ✅ Updates the crate's own `Cargo.toml` name field
- ✅ Updates documentation references

However, it **fails to update dependency keys** in other crates' `Cargo.toml` files that depend on the renamed crate.

---

## Steps to Reproduce

1. Create a workspace with a crate named `test-support`:
   ```toml
   # Cargo.toml
   [workspace]
   members = ["crates/test-support", "apps/myapp"]
   ```

2. Add `test-support` as a dependency in another crate:
   ```toml
   # apps/myapp/Cargo.toml
   [dev-dependencies]
   test-support = { path = "../../crates/test-support" }
   ```

3. Rename the crate using codebuddy:
   ```bash
   codebuddy tool rename_directory '{
     "old_path": "crates/test-support",
     "new_path": "crates/cb-test-support"
   }'
   ```

4. Run `cargo check`

---

## Expected Behavior

The tool should update **all** references to the renamed crate:

```toml
# apps/myapp/Cargo.toml (EXPECTED)
[dev-dependencies]
cb-test-support = { path = "../../crates/cb-test-support" }
```

**And** update all corresponding `use` statements:
```rust
// Before
use test_support::harness::TestClient;

// After (EXPECTED)
use cb_test_support::harness::TestClient;
```

---

## Actual Behavior

After running `rename_directory`, the dependency **key** remains unchanged:

```toml
# apps/myapp/Cargo.toml (ACTUAL - BROKEN)
[dev-dependencies]
test-support = { path = "../../crates/cb-test-support" }
#  ^^^^^^^^^^^^ OLD KEY NAME
#                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ UPDATED PATH
```

This causes a build error:
```
error: no matching package found
searched package name: `test-support`
perhaps you meant:      cb-test-support
location searched: /workspace/crates/cb-test-support
required by package `codebuddy v0.0.0 (/workspace/apps/codebuddy)`
```

---

## Root Cause Analysis

The `rename_directory` tool's manifest update logic:

1. ✅ **Updates workspace members** (root `Cargo.toml`)
2. ✅ **Updates crate name** (target crate's `Cargo.toml`)
3. ✅ **Updates dependency paths** (path field)
4. ❌ **Does NOT update dependency keys** (left side of `=`)

### Expected File Updates

| File | Field | Updated? |
|------|-------|----------|
| `Cargo.toml` (workspace) | `members = ["crates/cb-test-support"]` | ✅ Yes |
| `crates/cb-test-support/Cargo.toml` | `name = "cb-test-support"` | ✅ Yes |
| `apps/myapp/Cargo.toml` | `path = "../../crates/cb-test-support"` | ✅ Yes |
| `apps/myapp/Cargo.toml` | **Dependency key** `test-support` → `cb-test-support` | ❌ **NO** |

---

## Workaround

Manually update dependency keys in all dependent `Cargo.toml` files:

```bash
# Find all references
grep -r "test-support" apps/*/Cargo.toml crates/*/Cargo.toml integration-tests/Cargo.toml

# Manually edit each file
# OR use sed (risky):
find . -name "Cargo.toml" -exec sed -i 's/^test-support = /cb-test-support = /' {} +
```

Then update Rust import statements:

```bash
find . -name "*.rs" -exec sed -i 's/use test_support/use cb_test_support/g' {} +
```

---

## Suggested Fix

### Location
`crates/cb-lang-rust/src/manifest.rs` or `crates/cb-services/src/services/file_service/cargo.rs`

### Implementation

When renaming a Rust crate directory, the manifest updater should:

1. Extract old crate name from path: `test-support`
2. Extract new crate name from path: `cb-test-support`
3. Scan all workspace `Cargo.toml` files for dependency declarations
4. Update dependency keys using regex or TOML parsing:

```rust
// Pseudocode
fn update_dependency_keys(
    workspace_root: &Path,
    old_name: &str,
    new_name: &str,
) -> Result<()> {
    for cargo_toml in find_all_cargo_tomls(workspace_root) {
        let content = read_to_string(&cargo_toml)?;
        let mut doc = content.parse::<DocumentMut>()?;

        // Update in [dependencies], [dev-dependencies], [build-dependencies]
        for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
            if let Some(deps) = doc.get_mut(section).and_then(|t| t.as_table_mut()) {
                if let Some(old_dep) = deps.remove(old_name) {
                    deps.insert(new_name, old_dep);
                }
            }
        }

        fs::write(&cargo_toml, doc.to_string())?;
    }
    Ok(())
}
```

### Test Case

```rust
#[tokio::test]
async fn test_rename_directory_updates_dependency_keys() {
    let workspace = TestWorkspace::new();

    // Create crate structure
    workspace.create_cargo_package("crates/old-name");
    workspace.create_cargo_package("apps/consumer");

    // Add dependency
    workspace.add_dependency("apps/consumer/Cargo.toml",
        "old-name",
        "{ path = \"../../crates/old-name\" }");

    // Rename
    rename_directory("crates/old-name", "crates/new-name").await?;

    // Verify dependency key updated
    let cargo_toml = workspace.read("apps/consumer/Cargo.toml")?;
    assert!(cargo_toml.contains("new-name = { path"));
    assert!(!cargo_toml.contains("old-name = { path"));

    // Verify build works
    assert!(Command::new("cargo").arg("check").status()?.success());
}
```

---

## Impact

**Severity: Medium**

- **Build breakage**: Immediate `cargo check` failure after rename
- **Manual intervention required**: Users must manually fix dependency keys
- **Incomplete operation**: Tool claims success but leaves workspace in broken state
- **User confusion**: Path is updated but key is not, creating inconsistency

**Affected workflows:**
- Crate renaming for consistency (e.g., adding `cb-` prefix)
- Crate consolidation/refactoring
- Workspace reorganization

---

## Related Issues

This may also affect:
- Import statement updates (observed as needing manual `sed` replacement)
- Workspace-level dependency specifications
- Feature flag references in other crates

---

## Tool Output (Actual)

```json
{
  "documentation_updates": {
    "files_updated": 2,
    "references_updated": 8
  },
  "files_moved": 33,
  "import_updates": {
    "edits_applied": 0,
    "files_updated": 0
  },
  "manifest_updates": {
    "files_updated": 4,
    "updated_files": [
      "/workspace/Cargo.toml",
      "/workspace/apps/codebuddy/Cargo.toml",
      "/workspace/crates/cb-test-support/Cargo.toml",
      "/workspace/integration-tests/Cargo.toml"
    ]
  },
  "success": true
}
```

**Note:** `"import_updates": { "edits_applied": 0 }` suggests the import rewriting system may also need investigation.

---

## Recommendations

1. **Short term**: Document this limitation in `API_REFERENCE.md` under `rename_directory`
2. **Medium term**: Implement dependency key updates in Rust manifest handler
3. **Long term**: Consider a `rename_crate` tool specifically for Rust that handles:
   - Dependency key updates
   - Import statement rewriting (`use old_crate::*` → `use new_crate::*`)
   - Feature flag references
   - Documentation links
   - Example code in README files

---

## Additional Context

**Command used:**
```bash
./target/release/codebuddy tool rename_directory '{
  "old_path": "crates/test-support",
  "new_path": "crates/cb-test-support"
}'
```

**Workaround applied:**
```bash
# Manual fixes required
sed -i 's/"crates\/test-support"/"crates\/cb-test-support"/' Cargo.toml
sed -i 's/^test-support = /cb-test-support = /' apps/codebuddy/Cargo.toml
sed -i 's/^test-support = /cb-test-support = /' integration-tests/Cargo.toml
find . -name "*.rs" -exec sed -i 's/use test_support/use cb_test_support/g' {} +
```

**Environment:**
- Codebuddy version: `1.0.0-rc3`
- Rust version: `1.81.0` (per `rust-toolchain.toml`)
- Platform: Linux
