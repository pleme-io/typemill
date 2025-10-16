# Rust File Movement Test Coverage

## Problem

Rust file and directory movement operations lack comprehensive test coverage, allowing broken functionality to pass CI:

1. **No tests for `mod` declaration updates** – When renaming `utils.rs` → `helpers.rs`, parent `mod.rs` containing `mod utils;` is never verified to update to `mod helpers;`
2. **No Rust directory rename tests** – Moving workspace members (e.g., `integration-tests/` → `tests/e2e/`) doesn't verify:
   - Cargo.toml workspace member updates
   - Path dependency updates in dependent crates
   - Cross-crate `use` statement updates
3. **Dry-run preview data never validated** – Tests check `dry_run` flag but never verify `import_updates.files_to_modify` or `files` list accuracy
4. **Direct tool calls untested** – Internal `rename_file` and `rename_directory` tools only tested indirectly via unified API, missing their standalone behavior

## Solution

Add comprehensive test fixtures and test cases covering Rust-specific file movement scenarios.

## Checklists

### Add Rust File Rename Test Fixtures

- [x] Add `RUST_RENAME_FILE_TESTS` to `crates/cb-test-support/src/harness/mcp_fixtures.rs`
- [x] Test case: Rename file with `mod` declaration in parent `mod.rs`
- [x] Test case: Rename file with `mod` declaration in parent `lib.rs`
- [x] Test case: Rename module file affecting multiple `use` statements across crate
- [x] Test case: Rename affecting both `mod` and `use` in same file
- [x] Test case: Rename module referenced by sibling `mod.rs` (nested module tree)

### Add Rust Directory Rename Test Fixtures

- [x] Add `RUST_RENAME_DIRECTORY_TESTS` to `crates/cb-test-support/src/harness/mcp_fixtures.rs`
- [x] Test case: Rename workspace member directory
- [x] Test case: Verify `Cargo.toml` workspace members array updates
- [x] Test case: Verify path dependencies update in dependent crates
- [x] Test case: Verify cross-crate `use` statements update
- [x] Test case: Rename nested module directory affecting internal `use` statements

### Add Integration Tests

- [x] Create `integration-tests/src/test_rust_mod_declarations.rs`
- [x] Test renaming file requiring `mod` declaration update
- [x] Test renaming multiple files in same module
- [x] Create `integration-tests/src/test_rust_directory_rename.rs`
- [x] Test workspace member rename end-to-end
- [x] Test Cargo.toml updates across workspace

### Add Dry-Run Verification Tests

- [x] Add test to `integration-tests/src/dry_run_integration.rs`
- [x] Verify `import_updates.files_to_modify` count matches expected
- [x] Verify `import_updates.affected_files` contains expected paths
- [x] Verify `files` list in directory dry-run matches actual file count
- [x] Test dry-run doesn't modify filesystem

### Add Direct Tool Call Tests

- [x] Add test calling `rename_file` tool directly (not via unified API)
- [x] Add test calling `rename_directory` tool directly (not via unified API)
- [x] Verify import updates occur with direct tool invocation
- [x] Verify dry-run flag works with direct tool invocation
- [x] Verify directory dry-run (`rename_directory`) reports accurate `files_to_modify` / `affected_files`

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
