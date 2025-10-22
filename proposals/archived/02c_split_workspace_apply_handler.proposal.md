# Split WorkspaceApplyHandler

## Problem

`WorkspaceApplyHandler` has 870 lines mixing 6 distinct concerns:
1. Plan conversion (extract edits, warnings, checksums)
2. Checksum validation (SHA-256 calculation, verification)
3. Dry-run preview generation
4. Post-apply validation (external command execution)
5. Rollback coordination
6. Cross-plan type handling

This violates Single Responsibility Principle and creates merge conflicts when multiple developers modify validation, conversion, or execution logic simultaneously.

**File:** `../../crates/mill-handlers/src/handlers/workspace_apply_handler.rs`

## Solution

Extract 5 focused services, leaving orchestration handler at ~150 lines:

```rust
ChecksumValidator       // Validation only
PlanConverter          // Plan -> WorkspaceEdit extraction
DryRunGenerator        // Preview creation
PostApplyValidator     // Command execution
WorkspaceApplyHandler  // Orchestration only
```

## Checklists

- [ ] Create `../../crates/mill-services/src/services/checksum_validator.rs`
- [ ] Move `validate_checksums()` function
- [ ] Move `calculate_checksum()` function
- [ ] Add unit tests for validation logic
- [ ] Export ChecksumValidator from services module
- [ ] Create `../../crates/mill-services/src/services/plan_converter.rs`
- [ ] Move `convert_to_edit_plan()` function (210 lines)
- [ ] Move all `extract_*()` helper functions
- [ ] Move all `get_*()` helper functions
- [ ] Add unit tests for each plan type conversion
- [ ] Export PlanConverter from services module
- [ ] Create `../../crates/mill-services/src/services/dry_run_generator.rs`
- [ ] Move dry-run preview creation logic
- [ ] Move result formatting code
- [ ] Add unit tests for preview generation
- [ ] Export DryRunGenerator from services module
- [ ] Create `../../crates/mill-services/src/services/post_apply_validator.rs`
- [ ] Move `run_validation()` function
- [ ] Move external command execution logic
- [ ] Move timeout handling code
- [ ] Add unit tests for validation execution
- [ ] Export PostApplyValidator from services module
- [ ] Replace inline validation with `ChecksumValidator` calls in WorkspaceApplyHandler
- [ ] Replace inline conversion with `PlanConverter` calls
- [ ] Replace inline dry-run with `DryRunGenerator` calls
- [ ] Replace inline validation with `PostApplyValidator` calls
- [ ] Handler becomes pure orchestration (~150 lines)
- [ ] Update handler to receive services via dependency injection
- [ ] Add `checksum_validator: Arc<ChecksumValidator>` to `AppState`
- [ ] Add `plan_converter: Arc<PlanConverter>` to `AppState`
- [ ] Add `dry_run_generator: Arc<DryRunGenerator>` to `AppState`
- [ ] Add `post_apply_validator: Arc<PostApplyValidator>` to `AppState`
- [ ] Update `AppStateFactory` to create service instances
- [ ] Verify all existing workspace apply tests pass
- [ ] Add integration test verifying services work together
- [ ] Add test for concurrent modifications to different services

## Success Criteria

- `WorkspaceApplyHandler` is ~150 lines (orchestration only)
- Each service has single, focused responsibility
- Services can be tested independently
- Multiple developers can modify validation, conversion, execution in parallel
- All existing tests pass
- No functional changes to workspace apply behavior

## Benefits

- Enables parallel development without merge conflicts
- Each service can be tested in isolation
- Reduces cognitive load (understand one concern at a time)
- Easier to add new validation types or conversion logic
- Improves Single Responsibility compliance
- Services reusable across other handlers
