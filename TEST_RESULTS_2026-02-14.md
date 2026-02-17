# Test Run Report (2026-02-14)

## Commands Run

1. `time -p cargo test --workspace --exclude e2e`
   - Result: **PASS**
   - Summary: workspace regular/unit/integration/doc tests passed.
   - Totals observed in output:
     - `real 861.58`
     - `user 1138.19`
     - `sys 391.19`

2. `time -p cargo test -p e2e`
   - Result: **FAIL**
   - Summary line: `test result: FAILED. 229 passed; 16 failed; 26 ignored; 0 measured; 0 filtered out; finished in 2211.51s`
   - Totals observed in output:
     - `real 2370.50`
     - `user 358.06`
     - `sys 149.35`

   - Failing tests:
     - `dry_run_integration::test_dry_run_rename_directory_shows_files_list`
     - `test_cargo_package_rename::test_complete_cargo_package_rename`
     - `test_comprehensive_rename_coverage::test_alice_string_literal_updates`
     - `test_cross_workspace_import_updates::test_rename_crate_updates_all_workspace_imports`
     - `test_file_discovery_regression::test_file_discovery_in_non_standard_locations`
     - `test_move_with_imports::test_rust_move_file_updates_imports_from_fixtures`
     - `test_real_project_zod::test_zod_rename_symbol_execute`
     - `test_rust_refactoring::test_rust_nested_module_directory_rename`
     - `test_rust_refactoring::test_rust_rename_affects_both_mod_and_use`
     - `test_rust_refactoring::test_rust_rename_nested_mod_tree`
     - `test_rust_refactoring::test_rust_rename_updates_mod_in_lib_rs`
     - `test_rust_refactoring::test_rust_rename_updates_mod_in_parent_mod_rs`
     - `test_rust_refactoring::test_rust_rename_updates_sibling_mod_rs`
     - `test_rust_refactoring::test_same_crate_file_move_updates_use_statements`
     - `test_rust_refactoring::test_same_crate_nested_file_move_multiple_importers`
     - `user_scenario_test::test_user_scenario`

   - Notable error signal from failing run:
     - rust-analyzer initialization/request timeouts around diagnostics and indexing in `user_scenario_test` (initialize timeout at 60s; eventual panic waiting 180s for indexing).

3. `cargo test -p e2e -- --ignored --list`
   - Result: **PASS**
   - Summary: listed all 26 ignored e2e tests.

4. `time -p cargo test -p e2e -- --ignored`
   - Result: **INCOMPLETE (manually stopped)**
   - Observed completed tests before stop:
     - `test_lsp_ast_performance::test_lsp_ast_performance_python_httpx` ✅
     - `test_lsp_ast_performance::test_lsp_ast_performance_rust_ripgrep` ✅
     - `test_lsp_ast_performance::test_lsp_ast_performance_typescript_sveltekit` ✅
     - `test_lsp_ast_performance::test_lsp_ast_performance_typescript_zod` ✅
     - `test_lsp_ast_performance::test_lsp_ast_quick_benchmark` ✅
     - `test_refactoring_matrix::test_isolated_single_file_rename_zod` ✅
     - `test_refactoring_matrix::test_matrix_py_httpx` ✅
     - `test_refactoring_matrix::test_matrix_py_fastapi` ✅
     - `test_refactoring_matrix::test_matrix_py_pydantic` ✅
   - Run was manually terminated after prolonged execution with no additional output while entering larger matrix suite.

## Current Situation

- **Regular (non-e2e) test suite:** green.
- **e2e standard suite:** currently red with 16 failures, concentrated in Rust refactoring/move/rename and one Zod rename execution path.
- **Ignored matrix/perf e2e suite:** partially green on the tests reached, but full run not completed in this session.
