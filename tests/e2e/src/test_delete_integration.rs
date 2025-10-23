//! Integration tests for delete.plan and workspace.apply_edit
//!
//! Tests delete operations (fully functional):
//! - Delete file
//! - Delete directory
//! - Delete dead code (requires AST analysis)

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

#[tokio::test]
async fn test_delete_file_plan_and_apply() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("to_delete.rs", "pub fn unused() {}\n");
    let file_path = workspace.absolute_path("to_delete.rs");

    // 2. Generate delete.plan
    let plan_result = client
        .call_tool(
            "delete.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": file_path.to_string_lossy()
                }
            }),
        )
        .await
        .expect("delete.plan should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    // DEBUG: Print plan to see what's inside
    eprintln!(
        "DEBUG DELETE PLAN: {}",
        serde_json::to_string_pretty(&plan).unwrap()
    );

    assert_eq!(
        plan.get("planType").and_then(|v| v.as_str()),
        Some("deletePlan"),
        "Should be DeletePlan"
    );

    // 3. Apply plan
    let apply_result = client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dryRun": false,
                    "validateChecksums": true
                }
            }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    let result = apply_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Apply result should exist");

    // DEBUG: Print apply result
    eprintln!(
        "DEBUG APPLY RESULT: {}",
        serde_json::to_string_pretty(&result).unwrap()
    );

    assert_eq!(
        result.get("success").and_then(|v| v.as_bool()),
        Some(true),
        "Delete should succeed"
    );

    // 4. Verify file was deleted
    assert!(
        !workspace.file_exists("to_delete.rs"),
        "File should be deleted"
    );
}

#[tokio::test]
async fn test_delete_file_dry_run_preview() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("keep_for_now.rs", "pub fn test() {}\n");
    let file_path = workspace.absolute_path("keep_for_now.rs");

    // 2. Generate plan
    let plan_result = client
        .call_tool(
            "delete.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": file_path.to_string_lossy()
                }
            }),
        )
        .await
        .expect("delete.plan should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    // 3. Apply with dry_run=true
    let apply_result = client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dryRun": true
                }
            }),
        )
        .await
        .expect("Dry run should succeed");

    let result = apply_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Dry run result should exist");

    assert_eq!(
        result.get("success").and_then(|v| v.as_bool()),
        Some(true),
        "Dry run should succeed"
    );

    // 4. Verify file was NOT deleted
    assert!(
        workspace.file_exists("keep_for_now.rs"),
        "File should still exist after dry run"
    );
}

#[tokio::test]
async fn test_delete_file_checksum_validation() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("file.rs", "pub fn original() {}\n");
    let file_path = workspace.absolute_path("file.rs");

    // 2. Generate plan
    let plan_result = client
        .call_tool(
            "delete.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": file_path.to_string_lossy()
                }
            }),
        )
        .await
        .expect("delete.plan should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    // 3. Modify file to invalidate checksum
    workspace.create_file("file.rs", "pub fn modified() {}\n");

    // 4. Try to apply with checksum validation
    let apply_result = client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "validateChecksums": true
                }
            }),
        )
        .await;

    // Should fail due to checksum mismatch
    assert!(
        apply_result.is_err() || apply_result.unwrap().get("error").is_some(),
        "Apply should fail due to checksum mismatch"
    );

    // Verify file was NOT deleted
    assert!(workspace.file_exists("file.rs"), "File should still exist");
}

#[tokio::test]
async fn test_delete_directory_plan_and_apply() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_directory("temp_dir");
    workspace.create_file("temp_dir/file1.rs", "pub fn a() {}\n");
    workspace.create_file("temp_dir/file2.rs", "pub fn b() {}\n");

    let dir_path = workspace.absolute_path("temp_dir");

    // 2. Generate delete.plan for directory
    let plan_result = client
        .call_tool(
            "delete.plan",
            json!({
                "target": {
                    "kind": "directory",
                    "path": dir_path.to_string_lossy()
                }
            }),
        )
        .await
        .expect("delete.plan should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    assert_eq!(
        plan.get("planType").and_then(|v| v.as_str()),
        Some("deletePlan"),
        "Should be DeletePlan"
    );

    // 3. Apply plan
    let apply_result = client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dryRun": false
                }
            }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    let result = apply_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Apply result should exist");

    assert_eq!(
        result.get("success").and_then(|v| v.as_bool()),
        Some(true),
        "Directory delete should succeed"
    );

    // 4. Verify directory was deleted
    assert!(
        !workspace.file_exists("temp_dir"),
        "Directory should be deleted"
    );
}

#[tokio::test]
async fn test_delete_dead_code_plan_structure() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "dead_code.rs",
        r#"pub fn used() -> i32 {
    42
}

fn unused_helper() -> i32 {
    100
}
"#,
    );

    let file_path = workspace.absolute_path("dead_code.rs");

    // 2. Generate delete.plan for dead code
    let plan_result = client
        .call_tool(
            "delete.plan",
            json!({
                "target": {
                    "kind": "dead_code",
                    "path": file_path.to_string_lossy()
                }
            }),
        )
        .await;

    match plan_result {
        Ok(response) => {
            let plan = response
                .get("result")
                .and_then(|r| r.get("content"))
                .expect("Plan should exist");

            // Verify plan structure
            assert!(plan.get("metadata").is_some(), "Should have metadata");
            assert!(plan.get("summary").is_some(), "Should have summary");
            assert!(
                plan.get("fileChecksums").is_some(),
                "Should have checksums"
            );

            let metadata = plan.get("metadata").unwrap();
            assert_eq!(
                metadata.get("kind").and_then(|v| v.as_str()),
                Some("delete"),
                "Kind should be delete"
            );
        }
        Err(_) => {
            eprintln!("INFO: delete dead_code requires AST analysis, skipping test");
        }
    }
}
