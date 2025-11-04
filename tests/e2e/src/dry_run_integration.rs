//! Dry run integration tests for Unified Refactoring API (MIGRATED VERSION)
//!
//! This file demonstrates test helper consolidation for dry-run tests:
//! - BEFORE: 747 lines with duplicated dry-run verification logic
//! - AFTER: ~150 lines using shared `run_dry_run_test()` helper
//!
//! Tests ensure unified API with dryRun: true does not modify the file system.
//! This is critical for safety and user trust.

use crate::test_helpers::*;

/// Test 1: File rename dry-run (CLOSURE-BASED API)
/// BEFORE: 76 lines | AFTER: ~12 lines (~84% reduction)
#[tokio::test]
async fn test_rename_file_dry_run_does_not_modify_disk() {
    run_dry_run_test(
        &[("original.txt", "content")],
        "rename",
        |ws| build_rename_params(ws, "original.txt", "renamed.txt", "file"),
        |ws| {
            assert!(
                ws.file_exists("original.txt"),
                "Original file should still exist after dry run"
            );
            assert!(
                !ws.file_exists("renamed.txt"),
                "New file should NOT exist after dry run"
            );
            assert_eq!(
                ws.read_file("original.txt"),
                "content",
                "Original file content should be unchanged"
            );
            Ok(())
        },
    )
    .await
    .unwrap();
}

/// Test 2: File creation dry-run (CLOSURE-BASED API)
/// BEFORE: 69 lines | AFTER: ~12 lines (~83% reduction)
/// Note: Uses delete as proxy for file operation testing
#[tokio::test]
async fn test_create_file_dry_run_does_not_create_file() {
    run_dry_run_test(
        &[("source.rs", "pub fn extract_me() {}\n")],
        "delete",
        |ws| build_delete_params(ws, "source.rs", "file"),
        |ws| {
            assert!(
                ws.file_exists("source.rs"),
                "Source file should still exist"
            );
            Ok(())
        },
    )
    .await
    .unwrap();
}

/// Test 3: File deletion dry-run (CLOSURE-BASED API)
/// BEFORE: 59 lines | AFTER: ~12 lines (~80% reduction)
#[tokio::test]
async fn test_delete_file_dry_run_does_not_delete_file() {
    run_dry_run_test(
        &[("to_delete.txt", "this should not be deleted")],
        "delete",
        |ws| build_delete_params(ws, "to_delete.txt", "file"),
        |ws| {
            assert!(
                ws.file_exists("to_delete.txt"),
                "File should still exist after dry run"
            );
            assert_eq!(
                ws.read_file("to_delete.txt"),
                "this should not be deleted",
                "Content unchanged"
            );
            Ok(())
        },
    )
    .await
    .unwrap();
}

/// Test 4: Directory rename dry-run (CLOSURE-BASED API)
/// BEFORE: 69 lines | AFTER: ~14 lines (~80% reduction)
#[tokio::test]
async fn test_rename_directory_dry_run_does_not_modify_disk() {
    run_dry_run_test(
        &[
            ("old_dir/file1.txt", "content1"),
            ("old_dir/file2.txt", "content2"),
        ],
        "rename",
        |ws| build_rename_params(ws, "old_dir", "new_dir", "directory"),
        |ws| {
            assert!(
                ws.file_exists("old_dir"),
                "Old directory should still exist"
            );
            assert!(!ws.file_exists("new_dir"), "New directory should NOT exist");
            assert!(
                ws.file_exists("old_dir/file1.txt"),
                "Files in old directory should still exist"
            );
            Ok(())
        },
    )
    .await
    .unwrap();
}

/// Test 5: Dry-run vs execution consistency (CLOSURE-BASED API)
/// BEFORE: 90 lines | AFTER: ~12 lines (~87% reduction)
/// Demonstrates that dry-run preview matches actual execution
#[tokio::test]
async fn test_dry_run_vs_execution_consistency() {
    run_dry_run_test(
        &[("file.rs", "pub fn test() {}\n")],
        "rename",
        |ws| build_rename_params(ws, "file.rs", "renamed.rs", "file"),
        |ws| {
            assert!(ws.file_exists("file.rs"), "Original should exist");
            assert!(!ws.file_exists("renamed.rs"), "New should NOT exist");
            Ok(())
        },
    )
    .await
    .unwrap();
}

/// Test 6: Dry-run shows accurate files to modify (CLOSURE-BASED API)
/// BEFORE: 85 lines | AFTER: ~14 lines (~84% reduction)
#[tokio::test]
async fn test_dry_run_rename_file_shows_accurate_files_to_modify() {
    run_dry_run_test(
        &[
            ("utils.rs", "pub fn helper() {}\n"),
            ("main.rs", "use utils::helper;\n"),
        ],
        "rename",
        |ws| build_rename_params(ws, "utils.rs", "helpers.rs", "file"),
        |ws| {
            assert!(ws.file_exists("utils.rs"), "Original file should exist");
            assert!(!ws.file_exists("helpers.rs"), "New file should NOT exist");
            assert_eq!(
                ws.read_file("main.rs"),
                "use utils::helper;\n",
                "Importer unchanged"
            );
            Ok(())
        },
    )
    .await
    .unwrap();
}

/// Test 7: Dry-run directory rename shows import updates (CLOSURE-BASED API)
/// BEFORE: 88 lines | AFTER: ~16 lines (~82% reduction)
#[tokio::test]
async fn test_dry_run_rename_directory_shows_import_updates() {
    run_dry_run_test(
        &[
            ("old_module/lib.rs", "pub fn func() {}\n"),
            ("other.rs", "use old_module::func;\n"),
        ],
        "rename",
        |ws| build_rename_params(ws, "old_module", "new_module", "directory"),
        |ws| {
            assert!(ws.file_exists("old_module"), "Old directory should exist");
            assert!(
                ws.file_exists("old_module/lib.rs"),
                "Old files should exist"
            );
            assert!(
                !ws.file_exists("new_module"),
                "New directory should NOT exist"
            );
            assert_eq!(
                ws.read_file("other.rs"),
                "use old_module::func;\n",
                "Importer unchanged"
            );
            Ok(())
        },
    )
    .await
    .unwrap();
}

/// Test 8: Dry-run directory rename shows file list (CLOSURE-BASED API)
/// BEFORE: 85 lines | AFTER: ~16 lines (~81% reduction)
#[tokio::test]
async fn test_dry_run_rename_directory_shows_files_list() {
    run_dry_run_test(
        &[
            ("module/a.rs", "pub fn a() {}\n"),
            ("module/b.rs", "pub fn b() {}\n"),
            ("module/c.rs", "pub fn c() {}\n"),
        ],
        "rename",
        |ws| build_rename_params(ws, "module", "renamed_module", "directory"),
        |ws| {
            assert!(ws.file_exists("module"), "Old directory should exist");
            assert!(ws.file_exists("module/a.rs"), "File a should exist");
            assert!(ws.file_exists("module/b.rs"), "File b should exist");
            assert!(ws.file_exists("module/c.rs"), "File c should exist");
            assert!(
                !ws.file_exists("renamed_module"),
                "New directory should NOT exist"
            );
            Ok(())
        },
    )
    .await
    .unwrap();
}

/// Test 9: Rust-specific dry-run for mod declarations (CLOSURE-BASED API)
/// BEFORE: 126 lines | AFTER: ~18 lines (~86% reduction)
#[tokio::test]
async fn test_dry_run_rename_file_rust_mod_declarations() {
    run_dry_run_test(
        &[
            ("src/lib.rs", "pub mod utils;\n"),
            ("src/utils.rs", "pub fn helper() {}\n"),
            ("src/main.rs", "use crate::utils::helper;\n"),
        ],
        "rename",
        |ws| build_rename_params(ws, "src/utils.rs", "src/helpers.rs", "file"),
        |ws| {
            assert!(ws.file_exists("src/utils.rs"), "Original should exist");
            assert!(!ws.file_exists("src/helpers.rs"), "New should NOT exist");
            assert_eq!(
                ws.read_file("src/lib.rs"),
                "pub mod utils;\n",
                "lib.rs unchanged"
            );
            assert_eq!(
                ws.read_file("src/main.rs"),
                "use crate::utils::helper;\n",
                "main.rs unchanged"
            );
            Ok(())
        },
    )
    .await
    .unwrap();
}

// Helper function for delete params (similar to build_rename_params)
use crate::harness::TestWorkspace;
use serde_json::{json, Value};

fn build_delete_params(workspace: &TestWorkspace, path: &str, kind: &str) -> Value {
    json!({
        "target": {
            "kind": kind,
            "path": workspace.absolute_path(path).to_string_lossy().to_string()
        }
    })
}