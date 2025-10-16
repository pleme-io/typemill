# Split Move Handler into Focused Modules

## Problem

`crates/cb-handlers/src/handlers/move_handler.rs` (720+ lines) interleaves logic for:
- File moves (`plan_file_move`)
- Directory moves (`plan_directory_move`)
- Symbol moves (LSP-dependent flows)
- Clipboard detection
- Validation and checksums
- Error handling

This makes it:
- Difficult to locate relevant code for a specific move type
- Hard to reuse logic across different move scenarios
- Challenging to test individual move operations in isolation
- Easy to introduce regressions when modifying shared helpers

## Solution

Split `move_handler.rs` into focused modules:

```
crates/cb-handlers/src/handlers/move/
├── mod.rs            # Handler registration, dispatch logic
├── file_move.rs      # Single-file move planning
├── directory_move.rs # Directory move planning
├── symbol_move.rs    # LSP symbol move flows
├── validation.rs     # Checksum, warnings, conflict detection
└── clipboard.rs      # Clipboard target resolution
```

Each module owns its private helpers, keeping the main dispatcher compact.

## Checklists

- [ ] Create `handlers/move/` directory
- [ ] Create `mod.rs` with handler registration and dispatch
- [ ] Create `file_move.rs` for single-file moves
- [ ] Create `directory_move.rs` for directory moves
- [ ] Create `symbol_move.rs` for LSP-based moves
- [ ] Create `validation.rs` for checksums and warnings
- [ ] Create `clipboard.rs` for clipboard target logic
- [ ] Move `plan_file_move` to `file_move.rs`
- [ ] Move file-specific validation helpers to `file_move.rs`
- [ ] Move rename edit assembly to `file_move.rs`
- [ ] Keep file move tests in `file_move.rs` module
- [ ] Move `plan_directory_move` to `directory_move.rs`
- [ ] Move directory validation helpers to `directory_move.rs`
- [ ] Move recursive rename logic to `directory_move.rs`
- [ ] Keep directory move tests in `directory_move.rs` module
- [ ] Move symbol fallback flows to `symbol_move.rs`
- [ ] Move LSP rename preparation to `symbol_move.rs`
- [ ] Keep symbol move tests in `symbol_move.rs` module
- [ ] Move checksum calculation to `validation.rs`
- [ ] Move conflict detection to `validation.rs`
- [ ] Move warning generation to `validation.rs`
- [ ] Keep validation tests in `validation.rs` module
- [ ] Move clipboard target resolution to `clipboard.rs`
- [ ] Move clipboard validation to `clipboard.rs`
- [ ] Keep clipboard tests in `clipboard.rs` module
- [ ] Implement dispatch in `mod.rs` based on `target` field
- [ ] Route file moves to `file_move::plan`
- [ ] Route directory moves to `directory_move::plan`
- [ ] Route symbol moves to `symbol_move::plan`
- [ ] Keep dispatcher logic under 100 lines
- [ ] Maintain `MoveHandler` registration in `mod.rs`
- [ ] Keep `move.plan` tool signature unchanged
- [ ] Preserve error types and responses
- [ ] Ensure backwards compatibility
- [ ] Verify all existing integration tests pass
- [ ] Add unit tests for `file_move::plan`
- [ ] Add unit tests for `directory_move::plan`
- [ ] Add unit tests for `validation::check_conflicts`
- [ ] Add unit tests for `clipboard::resolve_target`
- [ ] Add module-level docs to each file explaining scope
- [ ] Update handler registration docs in `mod.rs`
- [ ] Add examples showing typical move scenarios

## Success Criteria

- `move_handler.rs` reduced to ~150 lines (dispatcher only)
- Each module handles single move type
- Private helpers scoped to relevant module
- All existing tests pass
- New unit tests cover individual move types
- Handler dispatch logic is clear and maintainable

## Benefits

- Clear separation of file, directory, and symbol move logic
- Easier to locate and modify specific move behavior
- Private helpers scoped to relevant context
- Better testability through focused modules
- Reduced risk of regressions when modifying move logic
- Simpler code reviews focusing on single move type
- Easier onboarding for contributors
