# Proposal 06: Workspace Consolidation & Architectural Hardening

**Status**: ✅ **Phases 06a & 06b COMPLETED** (2025-10-19)

**Completed Phases:**
- ✅ **Phase 06a**: Preparation & Enforcement (docs/architecture/layers.md + cargo-deny enforcement)
- ✅ **Phase 06b**: Foundational Consolidation (6 crates → 3 consolidated crates)

**Remaining Phases:**
- ⏳ **Phase 07a**: Workspace Standardization (rename 19 crates: cb-* → mill-*)
- ⏳ **Phase 08a**: Verification & Documentation

---

## Problem

The current workspace has several issues hindering maintainability and developer velocity:
1.  **Crate Sprawl:** The workspace contains many small, tightly-coupled crates (e.g., `cb-core`, `cb-types`, `cb-protocol`) that are rarely modified independently, increasing cognitive overhead.
2.  **High-Friction Changes:** A single logical change often requires editing multiple crates, leading to complex pull requests and internal version churn.
3.  **Inconsistent Structure & Naming:** Crates use a `cb-*` prefix while the binary is `mill`, and the `analysis` workspace is a separate top-level island, creating an inconsistent structure.
4.  **Implicit Architecture:** The desired layered architecture is documented but not programmatically enforced, creating a risk of dependency violations ("spider webs") over time.

## Solution(s)

This proposal adopts the "Pragmatic Layered Workspace" strategy to refactor the codebase into a more cohesive and maintainable structure. All file and directory operations will be performed using `mill`'s own refactoring tools to dogfood the product.

1.  **Consolidate Core Crates:** Merge the most tightly-coupled crates into logical components.
2.  **Standardize Naming:** Rename all workspace crates to use a consistent `mill-*` prefix.
3.  **Unify Tooling Directory:** Move the `analysis` workspace under a new top-level `tooling/` directory.
4.  **Enforce Architectural Layers:** Use `cargo-deny` to programmatically enforce the documented layered dependency model.

## Checklists

### 06a: Preparation & Enforcement ✅ COMPLETE
- [x] Create `docs/architecture/layers.md` to formally document the layered dependency model.
- [x] Add `cargo-deny` to the workspace and create a `deny.toml` configuration with graph rules to enforce the documented layers.

**Completion Notes:**
- `docs/architecture/layers.md` created with 7-layer hierarchy (Foundation → Application)
- `deny.toml` enforcement rules enabled for:
  - Plugins cannot depend on Services or higher
  - Cross-plugin isolation (no plugin-to-plugin dependencies)
  - Services cannot depend on Handlers
  - Handlers cannot depend on Application
  - Production cannot depend on cb-test-support
  - Analysis crates properly isolated
- Verification: `cargo deny check bans` passing ✅
- Commit: `708f446c feat: Enable cargo-deny architectural layer enforcement (Phase 06a)`

### 06b: Foundational Consolidation ✅ COMPLETE
- [x] Create the target directory and manifest for the new `../../crates/mill-foundation` crate.
- [x] For `cb-core`, `cb-types`, and `cb-protocol`, generate a `rename.plan` with the `consolidate: true` option, targeting `../../crates/mill-foundation` as the destination.
- [x] Execute the generated plans using `workspace.apply_edit`.
- [x] Manually add the public modules (`pub mod core;` etc.) to `../../crates/mill-foundation/src/lib.rs` as required by the consolidation workflow.
- [x] Create `../../crates/mill-plugin-system` and use the same `rename.plan(consolidate) -> workspace.apply_edit` workflow to merge `cb-plugins` and `cb-plugin-registry`.
- [x] Use the `rename.plan(consolidate)` workflow to merge `cb-bench` into `cb-test-support`.

**Completion Notes:**
- ✅ mill-foundation (cb-core + cb-types + cb-protocol) - merged and verified
- ✅ mill-plugin-system (cb-plugins + cb-plugin-registry) - merged and verified
- ✅ cb-test-support (cb-bench merged) - completed
- All builds passing, old crates deleted, workspace members updated
- Circular dependency detection integrated into consolidation workflow
- Comprehensive import path updates for 100% coverage
- Related commits:
  - `c01c1c96 feat: Create mill-foundation crate`
  - `e9d5c049 feat: Consolidate cb-types into mill-foundation`
  - `841a5d7d feat: Complete cb-protocol consolidation into mill-foundation`
  - `fb25c4ef feat: Consolidate mill-core into mill-foundation`
  - Multiple bug fix and enhancement commits

### 07a: Workspace Standardization
- [ ] For each remaining `cb-*` crate, generate a `rename.plan` to rename it to `mill-*` (e.g., `mill-lsp` -> `mill-lsp`).
- [ ] Execute all rename plans using `workspace.apply_edit`. The tool will update all `use` statements and `Cargo.toml` references across the workspace.
- [ ] Generate a `rename.plan` to move the `analysis` directory to `tooling/analysis`.
- [ ] Execute the move plan using `workspace.apply_edit`.
- [ ] Sequentially run `rename.plan` on each crate within `tooling/analysis/` to apply the `mill-analysis-*` prefix.

### 08a: Verification & Documentation
- [ ] Run `cargo test --workspace` to ensure all functionality remains intact after the refactoring.
- [ ] Run `cargo deny check` to confirm the new architectural layers are correctly enforced.
- [ ] Use `analyze.dead_code` to confirm that no orphaned modules or files remain from the old crate structure.
- [ ] Use `analyze.dependencies` to generate and review the new dependency graph, ensuring it is cleaner and adheres to the layered model.
- [ ] Verify that the original source directories (e.g., `crates/cb-core`) have been deleted by running `delete.plan` with `dry_run: true` and confirming it fails with a "not found" error.
- [ ] Update `SOC_LAYER_DIAGRAM.md` and other relevant documentation to reflect the new, consolidated structure.

## Success Criteria

1.  The number of crates in the `crates/` directory is reduced from 22 to 16.
2.  All crates in the workspace (including those in `tooling/analysis/`) follow the `mill-*` naming convention.
3.  The `analysis` workspace is located at `tooling/analysis/`.
4.  `cargo test --workspace` completes successfully.
5.  `cargo deny check` passes with zero violations.
6.  `analyze.dead_code` reports no unexpected dead code from the refactor.
7.  `analyze.dependencies` confirms a simplified dependency graph.
8.  All file and directory manipulations were executed using the project's own `mill` refactoring tools.

## Benefits

-   A more intuitive and cohesive codebase with reduced cognitive overhead.
-   Reduced friction for making changes to core components.
-   A standard, idiomatic, and professional workspace structure that is familiar to Rust developers.
-   Programmatically enforced architectural boundaries that prevent dependency violations.
-   Successful "dogfooding" of the product's own advanced refactoring capabilities.
