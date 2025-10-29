# Proposals

Task-focused proposals showing **what** needs to be done and execution dependencies.

## Numbering Schema

- `00_name.md` - Standalone (in-progress work, no blockers)
- `01_name.md` - Sequential (must complete before Phase 2)
- `02a_name.md, 02b_name.md, ...` - Parallel execution (can run simultaneously)
- `03_name.md` - Sequential (waits for Phase 2)
- `05_name.md` - Major changes (requires planning)

## Active Proposals

### Phase 0: In Progress
- `00_actionable_suggestions_integration.md` - ⚠️ PARTIAL (2/6 complete, 5 tests failing)

### Phase 1: Build System
- `01_xtask_pattern_adoption.md` - Replace shell scripts with Rust automation

### Phase 2: Code Quality (Parallel)
All can run simultaneously:
- `02a_split_move_handler.md` - Split 720-line handler into modules
- `02b_markdown_link_detection.md` - Add markdown link path extraction
- `02c_split_workspace_apply_handler.md` - Split 870-line handler
- `02d_fix_lsp_zombie_processes.md` - Fix 2,500+ process leak
- `02e_lsp_progress_notifications.md` - LSP $/progress support
- `02f_comprehensive_rename_updates.md` - 9% → 93% rename coverage

### Phase 3: Language Features
- `03_single_language_builds.md` - Capability traits for optional languages
  - ⚠️ **Prerequisite for Phase 4**

### Phase 4: Language Expansion
- `04_language_expansion.md` - Add C++, C, PHP support
  - ⚠️ **PAUSED** - Awaiting unified API completion
  - Depends on: Phase 3 completion

### Phase 5: Major Breaking Changes
- `05_rename_to_typemill.md` - Rename project mill → typemill
  - ⚠️ **DISRUPTIVE** - Requires team consensus

## Archived Proposals

Completed proposals moved to `archived/` for reference:
- `00_rust_move_test_coverage.md` ✅
- `01_fix_same_crate_moves.md` ✅
- `01_plugin_refactoring.md` ✅
- `01a_rust_directory_structure.md` ✅
- `01b_cargo_deny_integration.md` ✅
- `02_plugin_helpers_consolidation.md` ✅
- `02a_extract_rust_imports_module.md` ✅
- `02b_split_reference_updater.md` ✅
- `06_auto_download_lsp.md` ✅
- `09a_refactor_plan_trait.md` ✅
- `09b_segregate_import_trait.md` ✅

## Execution Guide

**Immediate priorities:**
1. Complete `00_actionable_suggestions_integration` (4 remaining kinds)
2. Execute `01_xtask_pattern_adoption`
3. Run Phase 2 proposals in parallel (6 independent tasks)

**Sequential dependencies:**
- Phase 3 must complete before Phase 4
- Phase 5 requires separate planning/approval

**Parallelization:**
- All `02x_` proposals can run simultaneously
- No blockers between Phase 2 tasks