# Rust File Movement Test Coverage

## Problem

Rust file and directory movement operations lack comprehensive test coverage, allowing broken functionality to pass CI:

1. **No tests for `mod` declaration updates** - When renaming `utils.rs` → `helpers.rs`, parent `mod.rs` containing `mod utils;` is never verified to update to `mod helpers;`

2. **No Rust directory rename tests** - Moving workspace members (e.g., `integration-tests/` → `tests/e2e/`) doesn't verify:
   - Cargo.toml workspace member updates
   - Path dependency updates in dependent crates
   - Cross-crate `use` statement updates

3. **Dry-run preview data never validated** - Tests check `dry_run` flag but never verify `import_updates.files_to_modify` or `files` list accuracy

4. **Direct tool calls untested** - Internal `rename_file` and `rename_directory` tools only tested indirectly via unified API, missing their standalone behavior

## Solution

Add comprehensive test fixtures and test cases covering Rust-specific file movement scenarios.

## Checklists

### Add Rust File Rename Test Fixtures

- [ ] Add `RUST_RENAME_FILE_TESTS` to `crates/cb-test-support/src/harness/mcp_fixtures.rs`
- [ ] Test case: Rename file with `mod` declaration in parent `mod.rs`
- [ ] Test case: Rename file with `mod` declaration in parent `lib.rs`
- [ ] Test case: Rename file with `mod` declaration in sibling `mod.rs` (e.g., `src/utils/mod.rs` declaring `mod helpers;` when renaming `src/utils/helpers.rs`)
- [ ] Test case: Nested `mod.rs` tree - rename affecting multiple levels of module hierarchy (verifies `compute_module_path_from_file` + rewrite logic alignment)
- [ ] Test case: Rename module file affecting multiple `use` statements across crate
- [ ] Test case: Rename affecting both `mod` and `use` in same file

### Add Rust Directory Rename Test Fixtures

- [ ] Add `RUST_RENAME_DIRECTORY_TESTS` to `crates/cb-test-support/src/harness/mcp_fixtures.rs`
- [ ] Test case: Rename workspace member directory
- [ ] Test case: Verify `Cargo.toml` workspace members array updates
- [ ] Test case: Verify path dependencies update in dependent crates
- [ ] Test case: Verify cross-crate `use` statements update
- [ ] Test case: Rename nested module directory affecting internal `use` statements

### Add Integration Tests

- [ ] Create `integration-tests/src/test_rust_mod_declarations.rs`
- [ ] Test renaming file requiring `mod` declaration update
- [ ] Test renaming multiple files in same module
- [ ] Create `integration-tests/src/test_rust_directory_rename.rs`
- [ ] Test workspace member rename end-to-end
- [ ] Test Cargo.toml updates across workspace

### Add Dry-Run Verification Tests

- [ ] Add test to `integration-tests/src/dry_run_integration.rs`
- [ ] Verify `import_updates.files_to_modify` count matches expected for file renames
- [ ] Verify `import_updates.affected_files` contains expected paths for file renames
- [ ] Verify `files` list in directory dry-run matches actual file count
- [ ] Verify `import_updates.files_to_modify` count for directory renames (exercises preview path)
- [ ] Verify `import_updates.affected_files` for directory renames (exercises execution path)
- [ ] Test dry-run doesn't modify filesystem

### Add Direct Tool Call Tests

- [ ] Add test calling `rename_file` tool directly (not via unified API)
- [ ] Add test calling `rename_directory` tool directly (not via unified API)
- [ ] Add dry-run test for `rename_directory` tool directly (verifies `files_to_modify`/`affected_files` populated correctly)
- [ ] Verify import updates occur with direct tool invocation
- [ ] Verify dry-run flag works with direct tool invocation for both preview and execution paths

## Success Criteria

- All new test fixtures pass
- Tests catch the `mod` declaration update bug (currently passes incorrectly)
- Tests catch dry-run preview inaccuracy (currently shows 0 files to modify)
- Tests verify Rust workspace member rename operations
- CI catches regressions in Rust file movement before merge

## Benefits

- Prevents shipping broken Rust file movement functionality
- Catches `mod` declaration bugs before production
- Validates dry-run previews show accurate data
- Ensures Cargo workspace operations work correctly
- Improves confidence in refactoring tools for Rust codebases
