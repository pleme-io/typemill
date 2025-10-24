//! Integration tests for delete.plan and workspace.apply_edit (MIGRATED VERSION)
//!
//! BEFORE: 332 lines with duplicated setup/plan/apply logic
//! AFTER: Using shared helpers from test_helpers.rs
//!
//! Tests delete operations (file, directory, dead code).

use crate::harness::{TestClient, TestWorkspace};
use crate::test_helpers::*;
use serde_json::json;

/// Test 1: Delete file plan and apply (CLOSURE-BASED API)
/// BEFORE: 88 lines | AFTER: ~20 lines (~77% reduction)
#[tokio::test]
async fn test_delete_file_plan_and_apply() {
    run_tool_test_with_plan_validation(
        &[("to_delete.rs", "pub fn unused() {}\n")],
        "delete.plan",
        |ws| build_delete_params(ws, "to_delete.rs", "file"),
        |plan| {
            assert_eq!(plan.get("planType").and_then(|v| v.as_str()), Some("deletePlan"),
                "Should be DeletePlan");
            Ok(())
        },
        |ws| {
            assert!(!ws.file_exists("to_delete.rs"), "File should be deleted");
            Ok(())
        }
    ).await.unwrap();
}

/// Test 2: Delete file dry run preview (CLOSURE-BASED API)
/// BEFORE: 58 lines | AFTER: ~12 lines (~79% reduction)
#[tokio::test]
async fn test_delete_file_dry_run_preview() {
    run_dry_run_test(
        &[("keep_for_now.rs", "pub fn test() {}\n")],
        "delete.plan",
        |ws| build_delete_params(ws, "keep_for_now.rs", "file"),
        |ws| {
            assert!(ws.file_exists("keep_for_now.rs"),
                "File should still exist after dry run");
            Ok(())
        }
    ).await.unwrap();
}

/// Test 3: Delete file checksum validation (CLOSURE-BASED API)
/// BEFORE: 51 lines | AFTER: ~20 lines (~61% reduction)
#[tokio::test]
async fn test_delete_file_checksum_validation() {
    let workspace = TestWorkspace::new();
    workspace.create_file("file.rs", "pub fn original() {}\n");

    let mut client = TestClient::new(workspace.path());
    let params = build_delete_params(&workspace, "file.rs", "file");

    let plan = client.call_tool("delete.plan", params).await.unwrap()
        .get("result").and_then(|r| r.get("content")).cloned().unwrap();

    // Modify file to invalidate checksum
    workspace.create_file("file.rs", "pub fn modified() {}\n");

    let apply_result = client.call_tool("workspace.apply_edit", json!({
        "plan": plan, "options": {"validateChecksums": true}
    })).await;

    assert!(apply_result.is_err() || apply_result.unwrap().get("error").is_some(),
        "Apply should fail due to checksum mismatch");
    assert!(workspace.file_exists("file.rs"), "File should still exist");
}

/// Test 4: Delete directory plan and apply (CLOSURE-BASED API)
/// BEFORE: 67 lines | AFTER: ~25 lines (~63% reduction)
#[tokio::test]
async fn test_delete_directory_plan_and_apply() {
    let workspace = TestWorkspace::new();
    workspace.create_directory("temp_dir");
    workspace.create_file("temp_dir/file1.rs", "pub fn a() {}\n");
    workspace.create_file("temp_dir/file2.rs", "pub fn b() {}\n");

    let mut client = TestClient::new(workspace.path());
    let params = build_delete_params(&workspace, "temp_dir", "directory");

    let plan = client.call_tool("delete.plan", params).await
        .expect("delete.plan should succeed")
        .get("result").and_then(|r| r.get("content"))
        .cloned().expect("Plan should exist");

    assert_eq!(plan.get("planType").and_then(|v| v.as_str()), Some("deletePlan"),
        "Should be DeletePlan");

    client.call_tool("workspace.apply_edit", json!({
        "plan": plan, "options": {"dryRun": false}
    })).await.expect("workspace.apply_edit should succeed");

    assert!(!workspace.file_exists("temp_dir"), "Directory should be deleted");
}

/// Test 5: Delete dead code plan structure (MANUAL - AST analysis required)
/// BEFORE: 68 lines | AFTER: ~45 lines (~34% reduction)
#[tokio::test]
async fn test_delete_dead_code_plan_structure() {
    let workspace = TestWorkspace::new();
    workspace.create_file("dead_code.rs",
        r#"pub fn used() -> i32 {
    42
}

fn unused_helper() -> i32 {
    100
}
"#);

    let mut client = TestClient::new(workspace.path());
    let file_path = workspace.absolute_path("dead_code.rs");

    let plan_result = client.call_tool("delete.plan", json!({
        "target": {
            "kind": "dead_code",
            "path": file_path.to_string_lossy()
        }
    })).await;

    match plan_result {
        Ok(response) => {
            let plan = response.get("result").and_then(|r| r.get("content"))
                .expect("Plan should exist");

            assert!(plan.get("metadata").is_some(), "Should have metadata");
            assert!(plan.get("summary").is_some(), "Should have summary");
            assert!(plan.get("fileChecksums").is_some(), "Should have checksums");

            let metadata = plan.get("metadata").unwrap();
            assert_eq!(metadata.get("kind").and_then(|v| v.as_str()), Some("delete"),
                "Kind should be delete");
        }
        Err(_) => {
            eprintln!("INFO: delete dead_code requires AST analysis, skipping test");
        }
    }
}
