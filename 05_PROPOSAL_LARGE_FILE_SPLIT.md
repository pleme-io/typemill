# Large File Split Checklist

**Status**: Partially complete - 3 splits done, 4 remaining
**Goal**: Shrink over-sized modules into focused files without widening public APIs
**Target**: Each new module stays comfortably ≤400 lines

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

### 2. `lsp_adapter.rs` (1,100 lines → ≤300 each)
- [ ] Create `crates/cb-plugins/src/adapters/lsp_adapter/`
- [ ] Split into focused modules:
  - [ ] `mod.rs` – `LspAdapterPlugin` struct, `LanguagePlugin` impl, re-exports (~200 lines)
  - [ ] `constructors.rs` – `new()`, `typescript()`, `python()`, `go()`, `rust()`, capability presets (~200 lines)
  - [ ] `request_translator.rs` – `translate_request`, `build_lsp_params`, method cache (~260 lines)
  - [ ] `response_normalizer.rs` – `translate_response`, `normalize_locations`, `normalize_symbols`, `normalize_hover`, `normalize_completions`, `normalize_workspace_edit` (~200 lines)
  - [ ] `tool_definitions.rs` – `tool_definitions()` with complete JSON schemas (~350 lines)
  - [ ] `tests.rs` – preserve adapter tests beside implementation (~200 lines)
- [ ] Validation: `cargo test -p cb-plugins -- lsp_adapter`

### 3. `package_extractor.rs` (1,148 lines → ≤300 each)
- [ ] Create `crates/cb-ast/src/package_extractor/`
- [ ] Move logic into modules:
  - [ ] `mod.rs` – `ExtractModuleToPackageParams`, public entry point, re-exports (~100 lines)
  - [ ] `planner.rs` – `plan_extract_module_to_package_with_registry` orchestration (~300 lines)
  - [ ] `manifest.rs` – manifest generation and dependency extraction (~150 lines)
  - [ ] `edits.rs` – TextEdit builders for file operations (create, delete, update) (~250 lines)
  - [ ] `workspace.rs` – workspace discovery, member updates, parent module modifications (~200 lines)
  - [ ] `tests.rs` – relocate the current `#[cfg(test)]` block intact (~450 lines)
- [ ] Check: `cargo test -p cb-ast -- package_extractor`

### 4. `import_updater.rs` (1,011 lines → ≤300 each)
- [ ] Create `crates/cb-ast/src/import_updater/`
- [ ] Split into focused modules:
  - [ ] `mod.rs` – Public API, re-exports, `update_imports_for_rename` entry point (~150 lines)
  - [ ] `path_resolver.rs` – `ImportPathResolver` struct, cache management, path calculations (~300 lines)
  - [ ] `file_scanner.rs` – `find_affected_files`, `find_project_files`, import detection (~250 lines)
  - [ ] `reference_finder.rs` – `find_inline_crate_references`, `create_text_edits_from_references` (~150 lines)
  - [ ] `edit_builder.rs` – EditPlan construction, plugin coordination (~200 lines)
  - [ ] `tests.rs` – relocate existing tests (~100 lines)
- [ ] Validation: `cargo test -p cb-ast -- import_updater`

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

### 8. `project_fixtures.rs` (1,506 lines → ≤300 each) [OPTIONAL]
- [ ] Create `crates/cb-test-support/src/harness/project_fixtures/`
- [ ] Split by language/scenario:
  - [ ] `mod.rs` – `ProjectFixtures` struct, re-exports (~50 lines)
  - [ ] `typescript.rs` – `create_large_typescript_project` (~400 lines)
  - [ ] `python.rs` – `create_python_project` (~350 lines)
  - [ ] `rust.rs` – `create_rust_project` (~250 lines)
  - [ ] `monorepo.rs` – `create_monorepo_project` (~280 lines)
  - [ ] `errors.rs` – `create_error_project` (~130 lines)
  - [ ] `performance.rs` – `create_performance_project` (~100 lines)
- [ ] Validation: `cargo test -p cb-test-support`

## ✅ Validation

- [ ] Full regression: `cargo test --workspace`
- [ ] Lint: `cargo clippy --workspace`
- [ ] Integration (if applicable): `cargo test --features lsp-tests -- --include-ignored`
- [ ] Confirm line counts: `find crates -name '*.rs' -exec wc -l {} + | awk '$1 > 400 {print}'`

## 📊 Success Criteria

- ✅ No refactored module exceeds ~400 lines
- ✅ Public APIs and behaviour remain unchanged
- ✅ All unit, integration, and lint checks pass
