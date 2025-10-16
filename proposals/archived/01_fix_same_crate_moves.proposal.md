# Fix Same-Crate Folder Move Detection

## Problem

When moving Rust files within the same crate (e.g., `common/src/utils.rs` → `common/src/helpers.rs`), the import rewriting logic skips the rewrite because it only checks if crate names differ:

```rust
if old_name != new_name {  // Line 559 in lib.rs
    // Rewrite logic...
} else {
    tracing::info!("Crates are the same - no rewrite needed");
    return Ok((content.to_string(), 0));  // ❌ Skips rewrite
}
```

This means imports like `use common::utils::foo` are not updated to `use common::helpers::foo` when the file is moved within the same crate.

## Solution

Change the condition from comparing crate names to comparing full module paths. This allows rewrites when the file moves within the same crate but to a different module path.

```rust
if old_name != new_name {
    // Cross-crate move: compute full module paths
    let old_module_path = compute_module_path_from_file(_old_path, old_name, &canonical_project);
    let new_module_path = compute_module_path_from_file(_new_path, new_name, &canonical_project);

    if old_module_path != new_module_path {
        // Perform rewrite
    }
} else {
    // Same crate: still compute module paths to check if they differ
    let old_module_path = compute_module_path_from_file(_old_path, old_name, &canonical_project);
    let new_module_path = compute_module_path_from_file(_new_path, new_name, &canonical_project);

    if old_module_path != new_module_path {
        // Perform rewrite
    }
}
```

Or refactor to compute paths once and check the condition once:

```rust
// Always compute full module paths
let old_module_path = compute_module_path_from_file(_old_path, old_name, &canonical_project);
let new_module_path = compute_module_path_from_file(_new_path, new_name, &canonical_project);

if old_module_path != new_module_path {
    // Perform rewrite
}
```

## Checklists

### Implementation
- [x] Refactor `rewrite_imports_for_rename` in `crates/cb-lang-rust/src/lib.rs` (lines 552-595)
- [x] Move module path computation outside the crate name check
- [x] Change condition from `old_name != new_name` to `old_module_path != new_module_path`
- [x] Remove redundant "Crates are the same - no rewrite needed" log message

### Testing
- [x] Add unit test: same-crate file move with different module paths
- [x] Add integration test: move `common/src/utils.rs` → `common/src/helpers.rs`
- [x] Verify imports like `use common::utils::foo` are updated to `use common::helpers::foo`
- [x] Add test: same-crate directory move (`common/src/old_dir/` → `common/src/new_dir/`)
- [x] Verify existing cross-crate move tests still pass
- [x] Add test: nested file move with `crate::` prefixed imports

### Reference Updater Integration
- [x] Update `find_affected_files_for_rename` in `reference_updater.rs` to detect same-crate moves
- [x] Ensure same-crate moves trigger the Rust plugin rewrite path (not just generic resolver)
- [x] Add support for detecting files with `crate::` prefixed imports

## Success Criteria

- Same-crate file moves trigger import rewrites when module paths differ
- `use common::utils::foo` → `use common::helpers::foo` when moving `common/src/utils.rs` → `common/src/helpers.rs`
- Cross-crate moves continue to work as before
- Directory moves within the same crate update all affected imports

## Benefits

- Complete import rewriting support for intra-crate refactoring
- Consistent behavior between cross-crate and same-crate moves
- Enables safe file reorganization within Rust crates
