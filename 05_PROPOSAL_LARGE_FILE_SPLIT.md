# Large File Split Checklist

**Status**: Ready for implementation  
**Goal**: Shrink the six over-sized modules into focused files without widening public APIs  
**Target**: Each new module stays comfortably ≤400 lines

## 📋 Phase 1 – Low-Risk Splits

These files have minimal coupling to other large modules, so we can refactor them independently.

### 1. `file_service.rs` (3,849 → ≤400 each)
- [ ] Create `crates/cb-services/src/services/file_service/`
- [ ] Introduce lean modules:
  - [ ] `mod.rs` – `FileService` struct, constructor, shared state wiring, re-exports (~150 lines)
  - [ ] `basic_ops.rs` – `create_file`, `delete_file`, `read_file`, `write_file`, `list_files` and queue helpers (~400 lines)
  - [ ] `rename.rs` – file & directory rename logic with import updates (~350 lines)
  - [ ] `edit_plan.rs` – `apply_edit_plan`, coordination, snapshots/rollback, edit helpers, `EditPlanResult` (~350 lines)
  - [ ] `cargo.rs` – `consolidate_rust_package`, dependency merging, workspace/path updates (~400 lines)
  - [ ] `utils.rs` – `run_validation`, `to_absolute_path`, `adjust_relative_path`, shared dry-run helpers (~250 lines)
  - [ ] `tests.rs` – move the existing `#[cfg(test)]` block and keep submodules local
- [ ] Run targeted regression: `cargo test -p cb-services -- file_service`

### 2. `lsp_adapter.rs` (≈1,100 → ≤250 each)
- [ ] Create `crates/cb-plugins/src/adapters/lsp_adapter/`
- [ ] Split into focused modules:
  - [ ] `mod.rs` – `LspAdapterPlugin`, `LanguagePlugin` impl, re-exports (~180 lines)
  - [ ] `constructors.rs` – `new()`, language-specific constructors, capability presets (~200 lines)
  - [ ] `translator.rs` – request translation & cache (`translate_request`, `build_lsp_params`) (~250 lines)
  - [ ] `responses.rs` – `translate_response` and `normalize_*` helpers (~220 lines)
  - [ ] `tools.rs` – `tool_definitions()` JSON specs (~200 lines)
  - [ ] `tests.rs` – preserve adapter tests beside implementation
- [ ] Validation: `cargo test -p cb-plugins -- lsp_adapter`

### 3. `package_extractor.rs` (1,147 → ≤250 each)
- [ ] Create `crates/cb-ast/src/package_extractor/`
- [ ] Move logic into four modules:
  - [ ] `mod.rs` – `ExtractModuleToPackageParams`, public entry point (~180 lines)
  - [ ] `planner.rs` – orchestration (`plan_extract_module_to_package_with_registry`) (~250 lines)
  - [ ] `edits.rs` – file copy & edit builders, dependency aggregation (~220 lines)
  - [ ] `workspace.rs` – workspace discovery, manifest updates, membership helpers (~220 lines)
  - [ ] `tests.rs` – relocate the current `#[cfg(test)]` block intact
- [ ] Check: `cargo test -p cb-ast -- package_extractor`

## 📋 Phase 2 – Coordinated Splits

These modules are consumed by other large files; refactor and immediately update the dependents.

### 4. `complexity.rs` + `tools/analysis.rs`
- [ ] Create `crates/cb-ast/src/complexity/` with:
  - [ ] `mod.rs` – re-export public API used by handlers
  - [ ] `analyzer.rs` – `analyze_file_complexity` traversal
  - [ ] `aggregation.rs` – `aggregate_class_complexity`, workspace totals
  - [ ] `metrics.rs` – counting helpers, language heuristics
  - [ ] `models.rs` – `ComplexityRating`, `ComplexityReport`, DTOs
  - [ ] `tests.rs` – move existing tests
- [ ] Update `crates/cb-handlers/src/handlers/tools/analysis.rs` to use the new module paths (consider adding a tiny `complexity::api` facade for stability)
- [ ] Run: `cargo test -p cb-ast -- complexity` and `cargo test -p cb-handlers -- analysis`

### 5. `refactoring.rs` + `refactoring_handler.rs`
- [ ] Create `crates/cb-ast/src/refactoring/` comprising:
  - [ ] `mod.rs` – shared types, public re-exports
  - [ ] `extract_function.rs`
  - [ ] `extract_variable.rs`
  - [ ] `inline_variable.rs`
  - [ ] `common.rs` – shared AST utilities & edit builders
  - [ ] `tests.rs`
- [ ] Update `crates/cb-handlers/src/handlers/refactoring_handler.rs` to import the new modules (optionally split handler helpers once AST modules are in place)
- [ ] Run: `cargo test -p cb-ast -- refactoring` and `cargo test -p cb-handlers -- refactoring_handler`

### 6. `tools/analysis.rs` follow-up
- [ ] Create `crates/cb-handlers/src/handlers/tools/analysis/`
- [ ] Reorganize into:
  - [ ] `mod.rs` – dispatcher & `AnalysisHandler`
  - [ ] `unused_imports.rs`
  - [ ] `complexity.rs` – now thin wrappers over the refactored AST complexity API
  - [ ] `refactoring.rs` – refactoring suggestions
  - [ ] `hotspots.rs` – project complexity & hotspot analysis
  - [ ] `tests.rs` – relocate handler-specific tests
- [ ] Ensure imports are updated and no duplicate logic remains
- [ ] Run: `cargo test -p cb-handlers -- analysis`

## ✅ Validation

- [ ] Full regression: `cargo test --workspace`
- [ ] Lint: `cargo clippy --workspace`
- [ ] Integration (if applicable): `cargo test --features lsp-tests -- --include-ignored`
- [ ] Confirm line counts: `find crates -name '*.rs' -exec wc -l {} + | awk '$1 > 400 {print}'`

## 📊 Success Criteria

- ✅ No refactored module exceeds ~400 lines
- ✅ Public APIs and behaviour remain unchanged
- ✅ All unit, integration, and lint checks pass
