//! Integration tests for move.plan and workspace.apply_edit
//!
//! Tests move operations (fully functional):
//! - Move file between directories
//! - Move with import updates

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

#[tokio::test]
async fn test_move_file_plan_and_apply() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create source and destination directories
    workspace.create_directory("src");
    workspace.create_directory("lib");
    workspace.create_file("src/helper.rs", "pub fn helper() -> i32 { 42 }\n");

    let source_path = workspace.absolute_path("src/helper.rs");
    let dest_path = workspace.absolute_path("lib/helper.rs");

    // 2. Generate move.plan
    let plan_result = client
        .call_tool(
            "move.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": source_path.to_string_lossy()
                },
                "destination": dest_path.to_string_lossy()
            }),
        )
        .await
        .expect("move.plan should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    assert_eq!(
        plan.get("plan_type").and_then(|v| v.as_str()),
        Some("MovePlan"),
        "Should be MovePlan"
    );

    // 3. Apply plan
    let apply_result = client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dry_run": false,
                    "validate_checksums": true
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
        "Move should succeed"
    );

    // 4. Verify file was moved
    assert!(
        !workspace.file_exists("src/helper.rs"),
        "Source file should be deleted"
    );
    assert!(
        workspace.file_exists("lib/helper.rs"),
        "Destination file should exist"
    );
    assert_eq!(
        workspace.read_file("lib/helper.rs"),
        "pub fn helper() -> i32 { 42 }\n",
        "Content should be preserved"
    );
}

#[tokio::test]
async fn test_move_file_dry_run_preview() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_directory("source");
    workspace.create_directory("target");
    workspace.create_file("source/file.rs", "pub fn test() {}\n");

    let source = workspace.absolute_path("source/file.rs");
    let dest = workspace.absolute_path("target/file.rs");

    // 2. Generate plan
    let plan_result = client
        .call_tool(
            "move.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": source.to_string_lossy()
                },
                "destination": dest.to_string_lossy()
            }),
        )
        .await
        .expect("move.plan should succeed");

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
                    "dry_run": true
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

    // 4. Verify file was NOT moved
    assert!(
        workspace.file_exists("source/file.rs"),
        "Source file should still exist after dry run"
    );
    assert!(
        !workspace.file_exists("target/file.rs"),
        "Target file should NOT exist after dry run"
    );
}

#[tokio::test]
async fn test_move_file_checksum_validation() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_directory("dir1");
    workspace.create_directory("dir2");
    workspace.create_file("dir1/data.rs", "pub const DATA: i32 = 100;\n");

    let source = workspace.absolute_path("dir1/data.rs");
    let dest = workspace.absolute_path("dir2/data.rs");

    // 2. Generate plan
    let plan_result = client
        .call_tool(
            "move.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": source.to_string_lossy()
                },
                "destination": dest.to_string_lossy()
            }),
        )
        .await
        .expect("move.plan should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    // 3. Modify file to invalidate checksum
    workspace.create_file("dir1/data.rs", "pub const DATA: i32 = 200;\n");

    // 4. Try to apply with checksum validation
    let apply_result = client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "validate_checksums": true
                }
            }),
        )
        .await;

    // Should fail due to checksum mismatch
    assert!(
        apply_result.is_err() || apply_result.unwrap().get("error").is_some(),
        "Apply should fail due to checksum mismatch"
    );

    // Verify file was NOT moved
    assert!(
        workspace.file_exists("dir1/data.rs"),
        "File should still be in source location"
    );
}

#[tokio::test]
async fn test_move_module_plan_structure() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_directory("old_location");
    workspace.create_directory("new_location");
    workspace.create_file("old_location/module.rs", "pub mod items {}\n");

    let source = workspace.absolute_path("old_location/module.rs");
    let dest = workspace.absolute_path("new_location/module.rs");

    // 2. Generate plan
    let plan_result = client
        .call_tool(
            "move.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": source.to_string_lossy()
                },
                "destination": dest.to_string_lossy()
            }),
        )
        .await
        .expect("move.plan should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    // Verify plan structure
    assert!(plan.get("metadata").is_some(), "Should have metadata");
    assert!(plan.get("summary").is_some(), "Should have summary");
    assert!(
        plan.get("file_checksums").is_some(),
        "Should have checksums"
    );
    assert!(plan.get("edits").is_some(), "Should have edits");

    let metadata = plan.get("metadata").unwrap();
    assert_eq!(
        metadata.get("kind").and_then(|v| v.as_str()),
        Some("move"),
        "Kind should be move"
    );
}
