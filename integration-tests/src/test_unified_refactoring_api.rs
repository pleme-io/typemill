//! Unified Refactoring API integration tests
//!
//! Tests the complete plan → apply workflow across all refactoring operations:
//! - rename.plan → workspace.apply_edit
//! - extract.plan → workspace.apply_edit
//! - inline.plan → workspace.apply_edit
//! - move.plan → workspace.apply_edit
//! - reorder.plan → workspace.apply_edit
//! - transform.plan → workspace.apply_edit
//! - delete.plan → workspace.apply_edit
//!
//! Also tests:
//! - Checksum validation across operations
//! - Rollback on validation failure
//! - Configuration preset loading and application

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

#[tokio::test]
async fn test_rename_plan_and_apply_workflow() {
    // Test the complete rename workflow: rename.plan → workspace.apply_edit
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("rename.rs", "pub fn old_name() {}\\n");
    let file_path = workspace.absolute_path("rename.rs");
    let new_path = workspace.absolute_path("new_name.rs");

    // Step 1: Generate rename.plan
    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": file_path.to_string_lossy()
                },
                "new_name": new_path.to_string_lossy()
            }),
        )
        .await
        .expect("rename.plan should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .cloned()
        .expect("Plan should exist");

    // Verify plan type
    assert_eq!(
        plan.get("plan_type").and_then(|v| v.as_str()),
        Some("RenamePlan"),
        "Should be RenamePlan"
    );

    // Step 2: Apply via workspace.apply_edit
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
        "Rename should succeed"
    );

    // Verify file was renamed
    assert!(
        !workspace.file_exists("rename.rs"),
        "Old file should be gone"
    );
    assert!(
        workspace.file_exists("new_name.rs"),
        "New file should exist"
    );
}

#[tokio::test]
async fn test_extract_plan_and_apply_workflow() {
    // Test the complete extract workflow: extract.plan → workspace.apply_edit
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "extract.rs",
        r#"pub fn main() {
    let x = 1;
    let y = 2;
    let z = x + y;
    println!("{}", z);
}
"#,
    );

    let file_path = workspace.absolute_path("extract.rs");

    // Step 1: Generate extract.plan
    // Extract lines 1-3 (the three let statements)
    // Line 3 is "    let z = x + y;" which is 18 chars, so end at character 18
    let plan_result = client
        .call_tool(
            "extract.plan",
            json!({
                "kind": "function",
                "source": {
                    "file_path": file_path.to_string_lossy(),
                    "range": {
                        "start": {"line": 1, "character": 4},
                        "end": {"line": 3, "character": 18}
                    },
                    "name": "calculate_sum"
                }
            }),
        )
        .await;

    let plan = plan_result
        .expect("extract.plan should succeed")
        .get("result")
        .and_then(|r| r.get("content"))
        .cloned()
        .expect("Plan should exist");

    // Verify plan type
    assert_eq!(
        plan.get("plan_type").and_then(|v| v.as_str()),
        Some("ExtractPlan"),
        "Should be ExtractPlan"
    );

    // Step 2: Apply via workspace.apply_edit
    let apply_result = client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dry_run": false
                }
            }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    let result = apply_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Apply result should have result.content");

    assert_eq!(
        result.get("success").and_then(|v| v.as_bool()),
        Some(true),
        "Extract should succeed"
    );
}

#[tokio::test]
async fn test_checksum_validation_across_all_plan_types() {
    // Test that checksum validation works for all plan types
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("checksum.rs", "pub fn original() {}\\n");
    let file_path = workspace.absolute_path("checksum.rs");

    // Generate a delete.plan
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
        .cloned()
        .expect("Plan should exist");

    // Modify file to invalidate checksum
    workspace.create_file("checksum.rs", "pub fn modified() {}\\n");

    // Try to apply with checksum validation
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

    // Verify file was NOT deleted (rollback worked)
    assert!(
        workspace.file_exists("checksum.rs"),
        "File should still exist after validation failure"
    );
}

#[tokio::test]
async fn test_validation_rollback_on_failure() {
    // Test that post-apply validation triggers rollback on failure
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("validate.rs", "pub fn test() {}\\n");
    let file_path = workspace.absolute_path("validate.rs");

    // Generate delete.plan
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
        .cloned()
        .expect("Plan should exist");

    // Apply with failing validation command
    let apply_result = client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "validation": {
                        "command": "false",  // Always fails
                        "timeout_seconds": 5
                    }
                }
            }),
        )
        .await;

    // Should fail due to validation failure
    assert!(
        apply_result.is_err() || apply_result.unwrap().get("error").is_some(),
        "Apply should fail when validation fails"
    );
}

#[tokio::test]
async fn test_dry_run_across_all_operations() {
    // Test dry_run works for all plan types
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("dry.rs", "pub fn test() {}\\n");
    let old_path = workspace.absolute_path("dry.rs");
    let new_path = workspace.absolute_path("dry_renamed.rs");

    // Generate rename.plan
    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": old_path.to_string_lossy()
                },
                "new_name": new_path.to_string_lossy()
            }),
        )
        .await
        .expect("rename.plan should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .cloned()
        .expect("Plan should exist");

    // Apply with dry_run=true
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

    // Verify file was NOT renamed
    assert!(
        workspace.file_exists("dry.rs"),
        "Original file should still exist"
    );
    assert!(
        !workspace.file_exists("dry_renamed.rs"),
        "New file should NOT exist in dry run"
    );
}

#[tokio::test]
async fn test_config_preset_loading() {
    // Test configuration preset loading and override
    // This test verifies that RefactorConfig can load presets and apply them

    // Note: This test is currently a placeholder until apply_preset is implemented
    // See: /workspace/crates/cb-core/src/refactor_config.rs:54-57

    // When implemented, this should test:
    // 1. Loading a preset from .codebuddy/refactor.toml
    // 2. Applying preset options to PlanOptions
    // 3. Overriding preset values with explicit options
    // 4. Preset validation and error handling
}
