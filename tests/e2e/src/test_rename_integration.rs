//! Integration tests for rename.plan and workspace.apply_edit
//!
//! Tests the complete plan → apply workflow for:
//! - File rename (fully functional)
//! - Directory rename (fully functional)
//! - Symbol rename (requires LSP, may fail gracefully)

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

#[tokio::test]
async fn test_rename_file_plan_and_apply() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a test file
    workspace.create_file("original.rs", "pub fn hello() {}\n");
    let old_path = workspace.absolute_path("original.rs");
    let new_path = workspace.absolute_path("renamed.rs");

    // 2. Generate rename plan
    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": old_path.to_string_lossy()
                },
                "newName": new_path.to_string_lossy()
            }),
        )
        .await
        .expect("rename.plan should succeed");

    // Verify plan structure
    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should have result.content");

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

    // 3. Apply plan via workspace.apply_edit
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
        .expect("Apply should have result.content");

    assert_eq!(
        result.get("success").and_then(|v| v.as_bool()),
        Some(true),
        "Apply should succeed"
    );

    // 4. Verify file was renamed
    assert!(
        !workspace.file_exists("original.rs"),
        "Original file should be deleted"
    );
    assert!(workspace.file_exists("renamed.rs"), "New file should exist");
    assert_eq!(
        workspace.read_file("renamed.rs"),
        "pub fn hello() {}\n",
        "Content should be preserved"
    );
}

#[tokio::test]
async fn test_rename_file_dry_run_preview() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("test.rs", "pub fn test() {}\n");
    let old_path = workspace.absolute_path("test.rs");
    let new_path = workspace.absolute_path("test_renamed.rs");

    // 2. Generate plan
    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": old_path.to_string_lossy()
                },
                "newName": new_path.to_string_lossy()
            }),
        )
        .await
        .expect("rename.plan should succeed");

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
        .expect("workspace.apply_edit dry run should succeed");

    let result = apply_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Dry run should have result");

    assert_eq!(
        result.get("success").and_then(|v| v.as_bool()),
        Some(true),
        "Dry run should succeed"
    );

    // 4. CRITICAL: Verify file was NOT renamed
    assert!(
        workspace.file_exists("test.rs"),
        "Original file should still exist after dry run"
    );
    assert!(
        !workspace.file_exists("test_renamed.rs"),
        "New file should NOT exist after dry run"
    );
}

#[tokio::test]
async fn test_rename_checksum_validation_rejects_stale_plan() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("file.rs", "pub fn original() {}\n");
    let file_path = workspace.absolute_path("file.rs");
    let new_path = workspace.absolute_path("renamed.rs");

    // 2. Generate plan
    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": file_path.to_string_lossy()
                },
                "newName": new_path.to_string_lossy()
            }),
        )
        .await
        .expect("rename.plan should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    // 3. Modify file to invalidate checksum
    workspace.create_file("file.rs", "pub fn modified() {}\n");

    // 4. Try to apply plan with checksum validation
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

    // Verify file was NOT modified
    assert_eq!(
        workspace.read_file("file.rs"),
        "pub fn modified() {}\n",
        "File should remain unchanged"
    );
}

#[tokio::test]
async fn test_rename_directory_plan_and_apply() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create directory with files
    workspace.create_directory("old_module");
    workspace.create_file("old_module/lib.rs", "pub fn old() {}\n");
    workspace.create_file("old_module/utils.rs", "pub fn util() {}\n");

    let old_dir = workspace.absolute_path("old_module");
    let new_dir = workspace.absolute_path("new_module");

    // 2. Generate rename plan
    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "directory",
                    "path": old_dir.to_string_lossy()
                },
                "newName": new_dir.to_string_lossy()
            }),
        )
        .await
        .expect("rename.plan for directory should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    assert_eq!(
        plan.get("planType").and_then(|v| v.as_str()),
        Some("renamePlan"),
        "Should be RenamePlan"
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

    assert_eq!(
        result.get("success").and_then(|v| v.as_bool()),
        Some(true),
        "Directory rename should succeed"
    );

    // 4. Verify directory was renamed
    assert!(
        !workspace.file_exists("old_module"),
        "Old directory should be deleted"
    );
    assert!(
        workspace.file_exists("new_module"),
        "New directory should exist"
    );
    assert!(
        workspace.file_exists("new_module/lib.rs"),
        "Files should be moved"
    );
    assert_eq!(
        workspace.read_file("new_module/lib.rs"),
        "pub fn old() {}\n",
        "File content should be preserved"
    );
}
