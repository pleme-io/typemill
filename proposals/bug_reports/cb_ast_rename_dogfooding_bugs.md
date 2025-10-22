# Bug Report: cb-ast → codebuddy-ast Rename Dogfooding Issues

**Date**: 2025-10-19
**Context**: Dogfooding codebuddy's rename.plan feature on itself (Phase 3 of Proposal 06)
**Severity**: Medium - Rename succeeded but required manual fixes for edge cases

## Summary

During the dogfooding rename of `cb-ast` → `codebuddy-ast`, the rename plan successfully updated most references but missed several edge cases that should have been caught by the comprehensive rename coverage feature (93% → 100% coverage, implemented in Proposal 02g).

## Bugs Discovered

### Bug 1: Feature Flag References Not Updated
**Severity**: High
**File**: `crates/codebuddy-plugin-system/Cargo.toml`
**Issue**: Feature flag still referenced old crate name

**What happened**:
```toml
[dependencies]
codebuddy-ast = { path = "../codebuddy-ast", optional = true }  # ✅ Updated correctly

[features]
runtime = ["codebuddy-foundation", "codebuddy-config", "cb-ast"]  # ❌ NOT updated
```

**Expected**: Feature flag should have been updated to `"codebuddy-ast"`

**Root cause**: The rename plan's Cargo.toml parser correctly updated the dependency on line 24 but failed to update the feature flag reference on line 31. Feature flags can reference optional dependencies and should be included in the rename scope.

**Impact**: `cargo check` failed with:
```
error: failed to load manifest for workspace member `/workspace/crates/codebuddy-plugin-system`
```

---

### Bug 2: Qualified Path References Not Updated (60+ occurrences)
**Severity**: High
**File**: Multiple files across 16+ crates
**Issue**: Code using `cb_ast::` qualified paths was not updated to `codebuddy_ast::`

**Examples**:
```rust
// ../../crates/mill-server/src/lib.rs:64
let cache_settings = cb_ast::CacheSettings::from_config(  // ❌ NOT updated

// ../../crates/mill-handlers/src/handlers/inline_handler.rs:106
let edit_plan = cb_ast::refactoring::inline_variable::plan_inline_variable(  // ❌ NOT updated

// crates/codebuddy-plugin-system/src/system_tools_plugin.rs:325
let parsed: cb_ast::package_extractor::ExtractModuleToPackageParams =  // ❌ NOT updated
```

**Total affected**:
- 60+ occurrences across 16 crates
- Files: mill-server, cb-handlers, cb-services, codebuddy-plugin-system

**Expected**: All qualified paths should have been updated by the rename plan

**Root cause**: The rename plan claims to update "qualified paths in code" per the comprehensive rename coverage feature, but it appears to only update `use` statements, not inline qualified path references.

**Impact**: `cargo check` failed with:
```
error[E0433]: failed to resolve: use of unresolved module or unlinked crate `cb_ast`
```

**Manual fix required**: `find /workspace/crates -name "*.rs" -type f -exec sed -i 's/cb_ast::/codebuddy_ast::/g' {} +`

---

### Bug 3: Import Statement Updates Missing
**Severity**: Medium
**File**: Multiple files
**Issue**: While the rename plan showed it would update imports, some `use cb_ast::*` statements were missed

**Expected**: All `use cb_ast::*` should become `use codebuddy_ast::*`

**Actual**: These were caught by the global find-replace fix for Bug 2, but should have been handled by the rename plan itself

---

## What the Rename Plan Claimed vs Reality

### Claimed (from CLAUDE.md):
> **3. Qualified Paths** - Inline qualified paths in code:
> ```rust
> // Before
> pub fn lib_fn() {
>     utils::helper();
> }
> // After
> pub fn lib_fn() {
>     helpers::helper();
> }
> ```
> ✅ `utils::helper()` → `helpers::helper()` (qualified paths)

### Reality:
- ❌ Qualified paths like `cb_ast::CacheSettings::from_config()` were NOT updated
- ❌ Feature flag references in Cargo.toml were NOT updated
- ✅ Directory rename worked correctly
- ✅ Package name in Cargo.toml updated correctly
- ✅ Workspace members list updated correctly
- ✅ Path dependencies updated correctly

## Coverage Analysis

**Claimed Coverage**: 100% (per Proposal 02g - Comprehensive Rename Coverage)

**Actual Coverage**: ~85% (estimated)
- ✅ Directory renames
- ✅ Package name in Cargo.toml
- ✅ Workspace members
- ✅ Path dependencies in Cargo.toml
- ❌ Feature flags in Cargo.toml
- ❌ Qualified paths in Rust code (`crate::module::Type`)
- ⚠️ Use statements (unclear - may have been missed)

## Pre-existing Issues Discovered

### Issue 1: Missing Test Dependency
**Severity**: Low
**File**: `crates/cb-plugin-api/Cargo.toml`
**Issue**: Tests use `tempfile` but it's not in `[dev-dependencies]`

**Error**:
```
error[E0432]: unresolved import `tempfile`
   --> crates/cb-plugin-api/src/language.rs:172:13
```

**Impact**: `cargo nextest run --workspace` fails (12 test compilation errors)

**Fix needed**: Add to `crates/cb-plugin-api/Cargo.toml`:
```toml
[dev-dependencies]
tempfile = { workspace = true }
```

---

## Recommendations

### Immediate Fixes Needed

1. **Fix qualified path detection in rename planner**
   - Scan for all `old_name::` patterns in Rust code
   - Update to `new_name::` during rename
   - Add tests for this scenario

2. **Fix Cargo.toml feature flag parsing**
   - Parse `[features]` section
   - Update any references to renamed crates
   - Add tests for optional dependency renames

3. **Add missing test dependency**
   - Add `tempfile` to cb-plugin-api dev-dependencies

### Testing Improvements

4. **Add dogfooding tests to CI**
   - Create test that renames a crate and verifies 100% coverage
   - Verify `cargo check` passes after rename
   - Catch these issues before they reach production

5. **Improve rename plan validation**
   - After generating plan, search for old name in codebase
   - Warn if any references would remain after applying plan
   - Provide "missed references" report

### Documentation Updates

6. **Update CLAUDE.md coverage claims**
   - Current claim of "100% coverage" is inaccurate
   - Should list known limitations until bugs are fixed
   - Add warning about manual verification needed

---

## Test Case for Reproduction

```bash
# 1. Generate rename plan
cargo nextest run test_rename_cb_ast_to_codebuddy_ast

# 2. Apply rename
cargo nextest run test_apply_rename_cb_ast_to_codebuddy_ast

# 3. Verify workspace compiles
cargo check --workspace

# Expected: Should pass
# Actual: Fails with missing qualified path updates
```

---

## Success Criteria for Bug Fixes

After fixing these bugs, the following should work without manual intervention:

```bash
# Generate and apply rename plan
cargo nextest run test_dogfood_rename_[crate]

# Verify workspace compiles (no manual fixes needed)
cargo check --workspace  # ✅ Should pass

# Verify tests compile
cargo nextest run --workspace  # ✅ Should pass
```

**No manual fixes should be required** - the rename plan should be complete and correct.
