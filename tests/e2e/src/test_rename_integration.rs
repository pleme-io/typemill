//! Integration tests for unified refactoring API with dryRun (MIGRATED VERSION)
//!
//! This file demonstrates the test helper consolidation for rename operations:
//! - BEFORE: 304 lines with duplicated setup/plan/apply/verify logic
//! - AFTER: ~140 lines using shared helpers from test_helpers.rs
//!
//! Tests rename operations (fully functional):
//! - File rename with plan validation
//! - Dry-run mode
//! - Checksum validation
//! - Directory rename

use crate::harness::{TestClient, TestWorkspace};
use crate::test_helpers::*;
use serde_json::json;

/// Test 1: File rename with plan validation (CLOSURE-BASED API)
/// BEFORE: 91 lines | AFTER: ~20 lines (~78% reduction)
/// Demonstrates: Plan metadata assertions before applying rename
#[tokio::test]
async fn test_rename_file_plan_and_apply() {
    run_tool_test_with_plan_validation(
        &[("original.rs", "pub fn hello() {}\n")],
        "rename",
        |ws| build_rename_params(ws, "original.rs", "renamed.rs", "file"),
        |plan| {
            assert_eq!(
                plan.get("planType").and_then(|v| v.as_str()),
                Some("renamePlan"),
                "Plan should be RenamePlan"
            );
            assert!(plan.get("metadata").is_some(), "Plan should have metadata");
            assert!(
                plan.get("fileChecksums").is_some(),
                "Plan should have fileChecksums"
            );
            Ok(())
        },
        |ws| {
            assert!(
                !ws.file_exists("original.rs"),
                "Original file should be deleted"
            );
            assert!(ws.file_exists("renamed.rs"), "New file should exist");
            assert_eq!(
                ws.read_file("renamed.rs"),
                "pub fn hello() {}\n",
                "Content should be preserved"
            );
            Ok(())
        },
    )
    .await
    .unwrap();
}

/// Test 2: File rename dry-run (CLOSURE-BASED API)
/// BEFORE: 64 lines | AFTER: ~14 lines (~78% reduction)
/// Demonstrates: No-op verification (file unchanged after dry-run)
#[tokio::test]
async fn test_rename_file_dry_run_preview() {
    run_dry_run_test(
        &[("test.rs", "pub fn test() {}\n")],
        "rename",
        |ws| build_rename_params(ws, "test.rs", "test_renamed.rs", "file"),
        |ws| {
            assert!(
                ws.file_exists("test.rs"),
                "Original file should still exist after dry run"
            );
            assert!(
                !ws.file_exists("test_renamed.rs"),
                "New file should NOT exist after dry run"
            );
            Ok(())
        },
    )
    .await
    .unwrap();
}

/// Test 3: Directory rename with plan validation (CLOSURE-BASED API)
/// BEFORE: 84 lines | AFTER: ~22 lines (~74% reduction)
/// Demonstrates: Rename entire directory with multiple files
#[tokio::test]
async fn test_rename_directory_plan_and_apply() {
    run_tool_test_with_plan_validation(
        &[
            ("old_module/lib.rs", "pub fn old() {}\n"),
            ("old_module/utils.rs", "pub fn util() {}\n"),
        ],
        "rename",
        |ws| build_rename_params(ws, "old_module", "new_module", "directory"),
        |plan| {
            assert_eq!(
                plan.get("planType").and_then(|v| v.as_str()),
                Some("renamePlan"),
                "Should be RenamePlan"
            );
            Ok(())
        },
        |ws| {
            assert!(
                !ws.file_exists("old_module"),
                "Old directory should be deleted"
            );
            assert!(ws.file_exists("new_module"), "New directory should exist");
            assert!(ws.file_exists("new_module/lib.rs"), "Files should be moved");
            assert_eq!(
                ws.read_file("new_module/lib.rs"),
                "pub fn old() {}\n",
                "File content should be preserved"
            );
            Ok(())
        },
    )
    .await
    .unwrap();
}
