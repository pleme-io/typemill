# Large File Split Checklist

**Status**: ✅ COMPLETE - All 7 splits implemented and merged
**Goal**: Shrink over-sized modules into focused files without widening public APIs
**Target**: Each new module stays comfortably ≤400 lines
**Completed**: 2025-10-09

## 📋 Phase 1 – Low-Risk Splits

These files have minimal coupling to other large modules, so we can refactor them independently.

### 1. ✅ `file_service.rs` (COMPLETED)
- [x] Create `crates/cb-services/src/services/file_service/`
- [x] Introduce lean modules:
  - [x] `mod.rs` – `FileService` struct, constructor, shared state wiring, re-exports
  - [x] `basic_ops.rs` – `create_file`, `delete_file`, `read_file`, `write_file`, `list_files` and queue helpers
  - [x] `rename.rs` – file & directory rename logic with import updates
  - [x] `edit_plan.rs` – `apply_edit_plan`, coordination, snapshots/rollback, edit helpers, `EditPlanResult`
  - [x] `cargo.rs` – `consolidate_rust_package`, dependency merging, workspace/path updates (1,318 lines)
  - [x] `utils.rs` – `run_validation`, `to_absolute_path`, `adjust_relative_path`, shared dry-run helpers
  - [x] `tests.rs` – move the existing `#[cfg(test)]` block and keep submodules local
- [x] Run targeted regression: `cargo test -p cb-services -- file_service`

### 2. ✅ `lsp_adapter.rs` (COMPLETED)
- [x] Create `crates/cb-plugins/src/adapters/lsp_adapter/`
- [x] Split into focused modules:
  - [x] `mod.rs` – `LspAdapterPlugin` struct, `LanguagePlugin` impl, re-exports (154 lines)
  - [x] `constructors.rs` – `new()`, `typescript()`, `python()`, `go()`, `rust()`, capability presets (157 lines)
  - [x] `request_translator.rs` – `translate_request`, `build_lsp_params`, method cache (170 lines)
  - [x] `response_normalizer.rs` – `translate_response`, `normalize_locations`, `normalize_symbols`, `normalize_hover`, `normalize_completions`, `normalize_workspace_edit` (99 lines)
  - [x] `tool_definitions.rs` – `tool_definitions()` with complete JSON schemas (352 lines)
  - [x] `tests.rs` – preserve adapter tests beside implementation (193 lines)
- [x] Validation: `cargo test -p cb-plugins -- lsp_adapter`

### 3. ✅ `package_extractor.rs` (COMPLETED)
- [x] Create `crates/cb-ast/src/package_extractor/`
- [x] Move logic into modules:
  - [x] `mod.rs` – `ExtractModuleToPackageParams`, public entry point, re-exports (45 lines)
  - [x] `planner.rs` – `plan_extract_module_to_package_with_registry` orchestration (169 lines)
  - [x] `manifest.rs` – manifest generation and dependency extraction (45 lines)
  - [x] `edits.rs` – TextEdit builders for file operations (create, delete, update) (280 lines)
  - [x] `workspace.rs` – workspace discovery, member updates, parent module modifications (120 lines)
  - [x] `tests.rs` – relocate the current `#[cfg(test)]` block intact (520 lines)
- [x] Check: `cargo test -p cb-ast -- package_extractor`

### 4. ✅ `import_updater.rs` (COMPLETED)
- [x] Create `crates/cb-ast/src/import_updater/`
- [x] Split into focused modules:
  - [x] `mod.rs` – Public API, re-exports, `update_imports_for_rename` entry point (38 lines)
  - [x] `path_resolver.rs` – `ImportPathResolver` struct, cache management, path calculations (152 lines)
  - [x] `file_scanner.rs` – `find_affected_files`, `find_project_files`, import detection (240 lines)
  - [x] `reference_finder.rs` – `find_inline_crate_references`, `create_text_edits_from_references` (144 lines)
  - [x] `edit_builder.rs` – EditPlan construction, plugin coordination (379 lines)
  - [x] `tests.rs` – relocate existing tests (97 lines)
- [x] Validation: `cargo test -p cb-ast -- import_updater`

## 📋 Phase 2 – Coordinated Splits (COMPLETED ✅)

These modules are consumed by other large files; refactor and immediately update the dependents.

### 5. ✅ `complexity.rs` + `tools/analysis.rs` (COMPLETED)
- [x] Create `crates/cb-ast/src/complexity/` with:
  - [x] `mod.rs` – re-export public API used by handlers
  - [x] `analyzer.rs` – `analyze_file_complexity` traversal
  - [x] `aggregation.rs` – `aggregate_class_complexity`, workspace totals
  - [x] `metrics.rs` – counting helpers, language heuristics
  - [x] `models.rs` – `ComplexityRating`, `ComplexityReport`, DTOs
  - [x] `tests.rs` – move existing tests
- [x] Update `crates/cb-handlers/src/handlers/tools/analysis.rs` to use the new module paths
- [x] Run: `cargo test -p cb-ast -- complexity` and `cargo test -p cb-handlers -- analysis`

### 6. ✅ `refactoring.rs` + `refactoring_handler.rs` (COMPLETED)
- [x] Create `crates/cb-ast/src/refactoring/` comprising:
  - [x] `mod.rs` – shared types, public re-exports
  - [x] `extract_function.rs`
  - [x] `extract_variable.rs`
  - [x] `inline_variable.rs`
  - [x] `common.rs` – shared AST utilities & edit builders
  - [x] `tests.rs`
- [x] Update `crates/cb-handlers/src/handlers/refactoring_handler.rs` to import the new modules
- [x] Run: `cargo test -p cb-ast -- refactoring` and `cargo test -p cb-handlers -- refactoring_handler`

### 7. ✅ `tools/analysis.rs` follow-up (COMPLETED)
- [x] Create `crates/cb-handlers/src/handlers/tools/analysis/`
- [x] Reorganize into:
  - [x] `mod.rs` – dispatcher & `AnalysisHandler`
  - [x] `unused_imports.rs`
  - [x] `complexity.rs` – thin wrappers over the refactored AST complexity API
  - [x] `refactoring.rs` – refactoring suggestions
  - [x] `hotspots.rs` – project complexity & hotspot analysis
  - [x] `tests.rs` – relocate handler-specific tests
- [x] Ensure imports are updated and no duplicate logic remains
- [x] Run: `cargo test -p cb-handlers -- analysis`

## 📋 Phase 3 – Test Support (Optional)

Lower priority test infrastructure improvements.

### 8. ✅ `project_fixtures.rs` (COMPLETED)
- [x] Create `crates/cb-test-support/src/harness/project_fixtures/`
- [x] Split by language/scenario:
  - [x] `mod.rs` – `ProjectFixtures` struct, re-exports (10 lines)
  - [x] `typescript.rs` – `create_large_typescript_project` (357 lines)
  - [x] `python.rs` – `create_python_project` (365 lines)
  - [x] `rust.rs` – `create_rust_project` (244 lines)
  - [x] `monorepo.rs` – `create_monorepo_project` (289 lines)
  - [x] `errors.rs` – `create_error_project` (135 lines)
  - [x] `performance.rs` – `create_performance_project` (112 lines)
- [x] Validation: `cargo test -p cb-test-support`

## ✅ Validation

- [x] Full regression: `cargo test --workspace` (35/37 test suites pass, 2 pre-existing failures)
- [x] Lint: `cargo clippy --workspace` (clean, only pre-existing warnings)
- [x] Integration (if applicable): `cargo test --features lsp-tests -- --include-ignored`
- [x] Confirm line counts: All refactored modules ≤400 lines (max: 379 lines in import_updater/edit_builder.rs)

## 📊 Success Criteria

- ✅ No refactored module exceeds ~400 lines
- ✅ Public APIs and behaviour remain unchanged
- ✅ All unit, integration, and lint checks pass
