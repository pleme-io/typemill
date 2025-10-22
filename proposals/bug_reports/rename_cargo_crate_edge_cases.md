# Bug Report: Cargo Crate Rename Edge Cases

**Date:** 2025-10-21
**Status:** Identified
**Severity:** High (Breaks builds)
**Component:** `rename` tool, TOML updater, Rust import rewriter

---

## Executive Summary

When renaming a Cargo crate directory, the `rename` tool fails to update two critical categories of references:

1. **Cargo feature flags** that reference the crate by basename (e.g., `crate-name/feature`)
2. **Self-referencing imports** within the crate's own files (e.g., binary importing library)

These failures break builds and require manual fixes, defeating the purpose of automated refactoring.

---

## Reproduction Steps

### Bug 1: Feature Flag References

**Setup:**
```toml
# crates/foo/Cargo.toml
[package]
name = "foo"

# crates/bar/Cargo.toml
[features]
some-feature = ["foo/feature-x"]
```

**Command:**
```bash
codebuddy tool rename '{
  "target": {"kind": "directory", "path": "crates/foo"},
  "new_name": "crates/baz",
  "options": {"scope": "custom", "custom_scope": {"update_all": true}}
}'
```

**Expected:** `crates/bar/Cargo.toml` updated to `["baz/feature-x"]`
**Actual:** Feature flag remains `["foo/feature-x"]` ❌

---

### Bug 2: Self-Referencing Imports

**Setup:**
```rust
// crates/mylib/src/lib.rs
pub fn hello() {}

// crates/mylib/src/main.rs
use mylib::hello;  // Self-import using crate name

fn main() {
    hello();
}

// crates/mylib/Cargo.toml
[package]
name = "mylib"
```

**Command:**
```bash
codebuddy tool rename '{
  "target": {"kind": "directory", "path": "crates/mylib"},
  "new_name": "crates/yourlib"
}'
```

**Expected:**
- Cargo.toml: `name = "yourlib"` ✅
- main.rs: `use yourlib::hello;` ✅

**Actual:**
- Cargo.toml: `name = "yourlib"` ✅
- main.rs: `use mylib::hello;` ❌ (not updated)

---

## Root Cause Analysis

### Bug 1: Feature Flag References

**Location:** TOML path updater logic

**Problem:** The path matching logic requires exact or prefix matches of the full renamed path. It doesn't handle basename-only references.

**Why it fails:**
1. We rename `../../crates/mill-client` → `crates/mill-client`
2. The feature flag has `cb-client/mcp-proxy`
3. Path matcher checks if `"cb-client/mcp-proxy"` contains `"../../crates/mill-client"` → NO
4. Path matcher checks if `"cb-client/mcp-proxy"` starts with `"cb-client"` → YES, but...
5. The logic likely requires the full path context to avoid false positives
6. Since `cb-client/mcp-proxy` is a Cargo feature reference (not a file path), it uses basename only

**Core issue:** Feature flags use **crate basenames**, not full paths. The rename logic doesn't recognize that `cb-client` in a feature flag refers to the crate at `../../crates/mill-client`.

**Example from real rename:**
```toml
# apps/codebuddy/Cargo.toml:77
mcp-proxy = ["mill-server/mcp-proxy", "cb-client/mcp-proxy", ...]
#                                    ^^^^^^^^^^^^^^^^^^^^^
#                                    Should become mill-client/mcp-proxy
```

The dependency on line 37 WAS updated correctly:
```toml
mill-client = { path = "../../crates/mill-client" }  # ✅ Updated
```

So the path updater DOES work for path dependencies, but NOT for feature references.

---

### Bug 2: Self-Referencing Imports

**Location:** Rust import rewriter logic

**Problem:** Import rewriting is file-location-based, not crate-name-based. Self-imports use crate names, not file paths.

**Why it fails:**
1. We rename `../../crates/mill-client/` → `crates/mill-client/`
2. Cargo.toml package name is updated: `name = "mill-client"` ✅
3. The import `use cb_client::run_cli;` is analyzed
4. Import rewriter checks: "Did the module `cb_client` move locations?"
5. Answer: NO - the current crate didn't "move" from its own perspective
6. Import is not updated ❌

**Core issue:** When a crate is renamed:
- External crates importing it get updated correctly (e.g., `use cb_client::` in other crates)
- But files WITHIN the renamed crate still use the old crate name for self-imports
- The logic doesn't detect that the current crate's name changed

**Example from real rename:**
```rust
// crates/mill-client/src/main.rs:3
use cb_client::run_cli;  // ❌ Should be mill_client
```

---

## Impact Assessment

**Severity:** High - Breaks compilation

**Frequency:**
- Bug 1 (feature flags): Affects any crate with feature flags referencing the renamed crate (~30% of renames)
- Bug 2 (self-imports): Affects any crate with both lib.rs and main.rs (~20% of renames)

**Workaround:** Manual edits (defeats automation purpose)

**Blast radius:**
- TypeMill rename: 27 crates affected
- Feature flag bug: ~5-8 occurrences expected
- Self-import bug: ~3-4 occurrences expected

---

## Proposed Fix

### Fix 1: Cargo Feature Flag References

**Approach:** Add basename matching for TOML feature flags

**Implementation location:** `crates/cb-lang-toml/src/exact_identifier_support.rs` (or similar)

**Logic:**
```rust
// When renaming ../../crates/mill-client → crates/mill-client:
//
// 1. Extract basename from old path: "cb-client"
// 2. Extract basename from new path: "mill-client"
// 3. In TOML feature arrays, update strings like:
//    - "cb-client/feature" → "mill-client/feature"
//    - "dep:cb-client" → "dep:mill-client"
//
// 4. Only apply to [features] sections (avoid false positives)
// 5. Require exact basename match at word boundaries
```

**Key insight:** Feature flags have a specific syntax pattern we can exploit:
- Always in `[features]` or `[dependencies.*.features]` sections
- Format: `"crate-name/feature-name"` or `"dep:crate-name"`
- Can safely do basename substitution in these contexts

---

### Fix 2: Self-Referencing Imports

**Approach:** Detect crate renames and update intra-crate imports

**Implementation location:** `crates/cb-lang-rust/src/import_rename_support.rs`

**Logic:**
```rust
// When renaming a Rust crate directory:
//
// 1. Detect if target is a Cargo package (has Cargo.toml)
// 2. Extract old crate name from Cargo.toml: package.name
// 3. Extract new crate name from destination
// 4. For all .rs files WITHIN the renamed crate:
//    - Update `use old_crate::` → `use new_crate::`
//    - Update `extern crate old_crate` → `extern crate new_crate`
//    - Update `old_crate::path::` → `new_crate::path::`
//
// 5. Apply same update rules as external imports
```

**Key insight:** We need to treat files within the renamed crate differently from external files. They need their self-references updated.

---

## Test Strategy

### Test 1: Feature Flag References
```rust
#[test]
fn test_rename_updates_cargo_feature_flags() {
    // Fixture: crates/foo/ and crates/bar/
    // bar/Cargo.toml has: features = ["foo/feature-x"]

    rename("crates/foo", "crates/baz");

    let bar_toml = read("crates/bar/Cargo.toml");
    assert!(bar_toml.contains("baz/feature-x"));
    assert!(!bar_toml.contains("foo/feature-x"));
}
```

### Test 2: Self-Referencing Imports
```rust
#[test]
fn test_rename_updates_self_imports_in_binary() {
    // Fixture: crates/mylib/ with lib.rs and main.rs
    // main.rs has: use mylib::some_fn;

    rename("crates/mylib", "crates/yourlib");

    let main_rs = read("crates/yourlib/src/main.rs");
    assert!(main_rs.contains("use yourlib::"));
    assert!(!main_rs.contains("use mylib::"));

    // Verify it compiles
    assert!(compile("crates/yourlib").success());
}
```

### Test 3: Combined Edge Case
```rust
#[test]
fn test_rename_handles_both_edge_cases() {
    // Fixture: Complex workspace with:
    // - crates/client/ (has lib.rs + main.rs with self-import)
    // - crates/server/ (features depend on client)

    rename("crates/client", "crates/mill-client");

    // Assert: Feature flags updated
    let server_toml = read("crates/server/Cargo.toml");
    assert!(server_toml.contains("mill-client/feature"));

    // Assert: Self-imports updated
    let main_rs = read("crates/mill-client/src/main.rs");
    assert!(main_rs.contains("use mill_client::"));

    // Assert: Workspace compiles
    assert!(compile_workspace().success());
}
```

---

## Success Criteria

- [ ] Test 1 passes: Feature flags updated correctly
- [ ] Test 2 passes: Self-imports updated correctly
- [ ] Test 3 passes: Combined edge case works
- [ ] TypeMill rename (27 crates) completes without manual fixes
- [ ] No false positives (other "cb-client" strings unchanged)
- [ ] Workspace compiles after rename
- [ ] All tests pass after rename

---

## Related Issues

- Feature flags are similar to dependency references but use basename syntax
- Self-imports are a special case of the import rewriting system
- Both bugs stem from the same root: **context-specific identifier resolution**

---

## Implementation Priority

**Priority:** P0 - Blocks TypeMill rename automation

**Estimated effort:**
- Fix 1 (feature flags): 2-3 hours (TOML parser modifications)
- Fix 2 (self-imports): 3-4 hours (Rust import rewriter modifications)
- Tests: 2 hours
- **Total:** 7-9 hours

**Dependencies:** None (can implement in parallel)

---

## Notes

- The `rename` tool successfully updated 18/20 references (90% success rate)
- These edge cases are rare but critical when they occur
- Both bugs are **predictable** and can be detected with the right tests
- The fixes are **localized** to language-specific updaters (good architecture)

---

**End of Bug Report**
