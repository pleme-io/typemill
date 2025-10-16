# Split Reference Updater into Detector Modules

## Problem

`crates/cb-services/src/services/reference_updater.rs` (550+ lines) mixes multiple concerns:
- Project file scanning
- Cache management
- Rust-specific cross-crate detection
- Generic import resolution
- Directory rename handling
- Edit plan assembly

This makes it difficult to:
- Understand detection strategy selection
- Add new language-specific detectors
- Test detection logic in isolation
- Maintain cache performance

## Solution

Split `reference_updater.rs` into focused modules:

```
crates/cb-services/src/services/reference_updater/
├── mod.rs              # Public API (update_references, orchestration)
├── cache.rs            # import_cache logic and FileImportInfo
├── detectors/
│   ├── mod.rs          # Detector trait and strategy selection
│   ├── rust.rs         # Rust cross-crate and same-crate detection
│   ├── generic.rs      # Generic import resolution fallback
│   └── directory.rs    # Directory-specific detection
└── edit_builder.rs     # TextEdit assembly, EditPlan construction
```

## Checklists

### Module Structure
- [x] Create `reference_updater/` directory
- [x] Create `mod.rs` with public API
- [x] Create `cache.rs` for import caching
- [x] Create `detectors/mod.rs` with re-exports
- [x] Create `detectors/rust.rs` for Rust-specific detection
- [x] Create `detectors/generic.rs` for fallback logic
- [x] Note: Skipped `detectors/directory.rs` (directory logic kept in `update_references`)
- [x] Note: Skipped `edit_builder.rs` (edit assembly kept in `update_references`)

### Extract Detection Logic
- [x] Move `FileImportInfo` to `cache.rs`
- [x] Removed duplicate `compute_module_path_from_file` and `find_crate_name_from_cargo_toml` (~90 lines)
- [x] Extract Rust cross-crate + same-crate detection (~200 lines) to `detectors/rust.rs`
- [x] Extract generic import resolution (~100 lines) to `detectors/generic.rs`
- [x] Extract `get_all_imported_files` and `extract_import_path` to `detectors/generic.rs`

### Strategy Selection
- [x] Implement strategy via conditional dispatch (Rust-first, then generic fallback)
- [x] Preserve single-pass scanning over `project_files`
- [x] Note: Skipped formal `DetectionStrategy` enum (simple conditional is clearer)

### Edit Building
- [x] Keep `TextEdit` assembly in `update_references` (better location)
- [x] Keep `EditPlan` construction in `update_references` (better location)
- [x] Keep `update_references` as orchestrator in `mod.rs`

### Public API
- [x] Maintain `ReferenceUpdater` struct in `mod.rs`
- [x] Keep `update_references` signature unchanged
- [x] Preserve `find_affected_files_for_rename` as public method
- [x] Ensure backwards compatibility (all tests pass)

### Testing
- [x] Keep integration tests in `mod.rs::tests`
- [x] Add unit test for `extract_import_path` in `detectors/generic.rs`
- [x] Verify all existing tests pass (3/3 tests passing)
- [x] Removed obsolete helper tests (used deleted functions)

### Performance
- [x] Verify single-pass file scanning is preserved
- [x] Cache access patterns unchanged
- [x] No duplicate reads of file content

## Success Criteria

- ✅ `reference_updater.rs` reduced from 1058 → 618 lines (440 lines removed!)
- ✅ Rust detection isolated in `detectors/rust.rs` (220 lines)
- ✅ Generic fallback isolated in `detectors/generic.rs` (150 lines)
- ✅ Cache structure isolated in `cache.rs` (15 lines)
- ✅ Removed ~90 lines of duplicate code (now uses `cb-lang-rust` imports module)
- ✅ All existing tests pass (3/3 tests passing)
- ✅ No performance regression (same algorithm, cleaner organization)

## Benefits

- Clear separation between detection strategies
- Easy to add new language-specific detectors
- Cache logic can be optimized independently
- Testable in isolation without full MCP harness
- Better error messages showing which detector triggered
- Facilitates debugging of "why wasn't this file detected?" issues
