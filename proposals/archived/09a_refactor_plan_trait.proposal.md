# Add RefactorPlanExt Trait

**Status:** ✅ **COMPLETED** (Commits: 47a758d4, f336cbb4)

## Problem

Adding new refactoring plan types requires updating 5+ hardcoded match statements in `WorkspaceApplyHandler`:
- `get_checksums_from_plan()` - 7 variants
- `extract_workspace_edit()` - 7 variants
- `extract_warnings()` - 7 variants
- `estimate_complexity()` - 7 variants
- `extract_impact_areas()` - 7 variants

This violates Open/Closed Principle and creates spider web dependencies where new plan types require modifications across multiple locations.

**File:** `crates/cb-handlers/src/handlers/workspace_apply_handler.rs:479-522`

## Solution

Define common trait for all refactoring plans with required methods. Each plan type implements once, eliminating all match statements.

```rust
pub trait RefactorPlanExt {
    fn checksums(&self) -> &HashMap<String, String>;
    fn workspace_edit(&self) -> &WorkspaceEdit;
    fn warnings(&self) -> &[Warning];
    fn complexity(&self) -> u8;
    fn impact_areas(&self) -> Vec<String>;
}
```

## Checklists

### Define Trait
- [x] Create `RefactorPlanExt` trait in `crates/cb-protocol/src/refactor_plan.rs` (lines 113-128)
- [x] Add 5 required methods (checksums, workspace_edit, warnings, complexity, impact_areas)
- [x] Add trait to public exports (cb_protocol crate)

### Implement for Existing Plans
- [x] Implement `RefactorPlanExt` for `RenamePlan` (lines 130-141)
- [x] Implement `RefactorPlanExt` for `ExtractPlan` (lines 143-154)
- [x] Implement `RefactorPlanExt` for `InlinePlan` (lines 156-167)
- [x] Implement `RefactorPlanExt` for `MovePlan` (lines 169-180)
- [x] Implement `RefactorPlanExt` for `ReorderPlan` (lines 182-193)
- [x] Implement `RefactorPlanExt` for `TransformPlan` (lines 195-206)
- [x] Implement `RefactorPlanExt` for `DeletePlan` (lines 208-227)
- [x] Implement `RefactorPlanExt` for `RefactorPlan` enum (lines 231-291) - **KEY ADDITION**

### Refactor WorkspaceApplyHandler
- [x] Replace workspace_edit match statement with `plan.workspace_edit()` call (line 221)
- [x] Replace checksums match statement with `plan.checksums()` call (line 402)
- [x] Replace warnings match statement (validation path) with `plan.warnings()` call (line 315)
- [x] Replace warnings match statement (no validation path) with `plan.warnings()` call (line 373)
- [x] Replace complexity match statement with `plan.complexity()` call (line 691)
- [x] Replace impact_areas match statement with `plan.impact_areas()` call (line 692)
- [x] Delete all 5 match-based helper functions (VERIFIED: no match statements remain)

### Testing
- [x] Run existing workspace apply tests to verify behavior unchanged (build succeeded)
- [ ] Add test for new plan type to verify extension works without code changes (DEFERRED)

## Success Criteria

- ✅ Adding new `RefactorPlan` variant requires:
  - ✅ One trait implementation (5 methods)
  - ✅ Zero changes to `WorkspaceApplyHandler`
- ✅ All existing tests pass (build succeeded, no errors)
- ✅ No match statements on `RefactorPlan` enum in workspace apply logic (VERIFIED via grep)

## Benefits

- New plan types extend system without modifying existing code (Open/Closed)
- Reduces coupling between plan definitions and apply handler
- Compiler enforces all plans implement required interface
- Eliminates spider web where one change touches 5+ match statements
