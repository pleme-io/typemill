# Extract Rust Imports Module

## Problem

Import-related logic in `crates/cb-lang-rust/src/lib.rs` is scattered across 650+ lines, mixing:
- Import rewriting (`rewrite_imports_for_rename`)
- Module path computation (`compute_module_path_from_file`)
- Crate name extraction (`find_crate_name_from_cargo_toml`)
- Plugin trait implementations

This makes the code hard to navigate, test in isolation, and evolve independently.

## Solution

Create a dedicated `imports/` module under `crates/cb-lang-rust/src/` with clear separation of concerns:

```
crates/cb-lang-rust/src/imports/
├── mod.rs           # ImportSupport implementation
├── rewrite.rs       # rewrite_imports_for_rename logic
├── module_path.rs   # compute_module_path_from_file, path helpers
└── crate_name.rs    # find_crate_name_from_cargo_toml, Cargo.toml parsing
```

## Checklists

### Module Structure
- [x] Create `crates/cb-lang-rust/src/imports/mod.rs`
- [x] Create `crates/cb-lang-rust/src/imports/module_path.rs`
- [x] Create `crates/cb-lang-rust/src/imports/crate_name.rs`
- [x] Note: `rewrite_imports_for_rename` kept in `lib.rs` (part of plugin impl, better location)

### Move Functions
- [x] Move `compute_module_path_from_file` to `module_path.rs`
- [x] Move `find_crate_name_from_cargo_toml` to `crate_name.rs`
- [x] Keep `rewrite_imports_for_rename` in `lib.rs` (uses plugin's `import_support`, better location)
- [x] Keep `import_support.rs` as-is (implements trait)

### Public API
- [x] Re-export `compute_module_path_from_file` and `find_crate_name_from_cargo_toml` from `imports/mod.rs`
- [x] Update `lib.rs` to import from `imports` module
- [x] Ensure existing public API remains stable (all tests pass)

### Testing
- [x] Move existing `compute_module_path_from_file` tests to `module_path.rs`
- [x] Add unit tests for `module_path::compute_module_path_from_file` (6 tests)
- [x] Keep `import_support` tests in `import_support.rs`
- [x] Verify all 45 tests still pass (✅ PASSED)

### Documentation
- [x] Add module-level docs to `imports/mod.rs` explaining organization
- [x] Add module-level docs to `module_path.rs` with examples
- [x] Add module-level docs to `crate_name.rs` with usage notes
- [x] Document `mod.rs`, `lib.rs`, `main.rs` special cases in function docs

## Success Criteria

- ✅ `lib.rs` reduced by 95 lines (940 → 845 lines)
- ✅ Import helper logic isolated in `imports/` module
- ✅ Each submodule has focused responsibility (module_path, crate_name)
- ✅ All existing tests pass without modification (45/45 tests passing)
- ✅ Unit tests cover path conversion edge cases (6 test cases for module_path)

## Benefits

- Easier to reason about import logic in isolation
- Clear entry points for debugging import issues
- Facilitates adding new import features without touching `lib.rs`
- Better test coverage through focused unit tests
- Simplifies onboarding for new contributors
