//! Dry run integration tests for Unified Refactoring API
//!
//! This test suite ensures that workspace.apply_edit with dry_run=true does not
//! modify the file system. This is critical for safety and user trust.
//!
//! NOTE: These tests use the Unified Refactoring API pattern:
//! 1. Generate a plan with *.plan() command
//! 2. Apply with workspace.apply_edit(plan, { dry_run: true })
//! 3. Verify no file system modifications occurred

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

#[tokio::test]
async fn test_rename_file_dry_run_does_not_modify_disk() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a test file
    workspace.create_file("original.txt", "content");
    let old_file_path = workspace.path().join("original.txt");
    let new_file_path = workspace.path().join("renamed.txt");

    // 2. Generate rename plan using unified API
    let plan_response = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": old_file_path.to_str().unwrap()
                },
                "new_name": new_file_path.to_str().unwrap()
            }),
        )
        .await
        .unwrap();

    // Debug: Print the full response to understand what's happening
    eprintln!("DEBUG plan_response: {}", serde_json::to_string_pretty(&plan_response).unwrap());

    // Check if there's an error instead of a result
    if !plan_response["error"].is_null() {
        panic!("rename.plan failed with error: {}", plan_response["error"]);
    }

    let plan = &plan_response["result"]["content"];
    assert_eq!(plan["plan_type"], "RenamePlan", "Should generate a RenamePlan");

    // 3. Apply plan with dry_run=true
    let apply_response = client
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
        .unwrap();

    let result = &apply_response["result"]["content"];
    assert_eq!(result["success"], true, "Dry run should succeed");

    // 4. CRITICAL: Verify file system is unchanged
    assert!(
        workspace.file_exists("original.txt"),
        "Original file should still exist after dry run"
    );
    assert!(
        !workspace.file_exists("renamed.txt"),
        "New file should NOT exist after dry run"
    );
    assert_eq!(
        workspace.read_file("original.txt"),
        "content",
        "Original file content should be unchanged"
    );
}

#[tokio::test]
async fn test_create_file_dry_run_does_not_create_file() {
    // NOTE: File creation is typically part of extract/move operations in unified API
    // This test demonstrates using delete.plan (which can represent file operations)
    // For creating files outside refactoring context, use FileService directly or write_file utility

    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a source file to extract from
    workspace.create_file("source.rs", "const VALUE: i32 = 42;");
    let source_path = workspace.path().join("source.rs");
    let new_file_path = workspace.path().join("extracted.rs");

    // 2. Generate extract plan that creates a new file
    let plan_response = client
        .call_tool(
            "extract.plan",
            json!({
                "kind": "constant",
                "source": {
                    "file_path": source_path.to_str().unwrap(),
                    "range": {
                        "start": { "line": 0, "character": 0 },
                        "end": { "line": 0, "character": 23 }
                    },
                    "name": "VALUE",
                    "destination": new_file_path.to_str().unwrap()
                }
            }),
        )
        .await
        .unwrap();

    let plan = &plan_response["result"]["content"];
    assert_eq!(plan["plan_type"], "ExtractPlan", "Should generate an ExtractPlan");

    // 3. Apply plan with dry_run=true
    let apply_response = client
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
        .unwrap();

    let result = &apply_response["result"]["content"];
    assert_eq!(result["success"], true, "Dry run should succeed");

    // 4. CRITICAL: Verify no new file was created
    assert!(
        !workspace.file_exists("extracted.rs"),
        "New file should NOT be created after dry run"
    );
    assert!(
        workspace.file_exists("source.rs"),
        "Source file should still exist"
    );
}

#[tokio::test]
async fn test_delete_file_dry_run_does_not_delete_file() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a file to delete
    workspace.create_file("to_delete.txt", "important content");
    let file_path = workspace.path().join("to_delete.txt");

    // 2. Generate delete plan using unified API
    let plan_response = client
        .call_tool(
            "delete.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": file_path.to_str().unwrap()
                }
            }),
        )
        .await
        .unwrap();

    let plan = &plan_response["result"]["content"];
    assert_eq!(plan["plan_type"], "DeletePlan", "Should generate a DeletePlan");

    // 3. Apply plan with dry_run=true
    let apply_response = client
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
        .unwrap();

    let result = &apply_response["result"]["content"];
    assert_eq!(result["success"], true, "Dry run should succeed");

    // 4. CRITICAL: Verify file still exists
    assert!(
        workspace.file_exists("to_delete.txt"),
        "File should still exist after dry run delete"
    );
    assert_eq!(
        workspace.read_file("to_delete.txt"),
        "important content",
        "File content should be unchanged"
    );
}

#[tokio::test]
async fn test_rename_directory_dry_run_does_not_modify_disk() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a directory with a file
    workspace.create_directory("old_dir");
    workspace.create_file("old_dir/file.txt", "content in directory");
    let old_dir = workspace.path().join("old_dir");
    let new_dir = workspace.path().join("new_dir");

    // 2. Generate rename plan for directory using unified API
    let plan_response = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "directory",
                    "path": old_dir.to_str().unwrap()
                },
                "new_name": new_dir.to_str().unwrap()
            }),
        )
        .await
        .unwrap();

    let plan = &plan_response["result"]["content"];
    assert_eq!(plan["plan_type"], "RenamePlan", "Should generate a RenamePlan");

    // 3. Apply plan with dry_run=true
    let apply_response = client
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
        .unwrap();

    let result = &apply_response["result"]["content"];
    assert_eq!(result["success"], true, "Dry run should succeed");

    // 4. CRITICAL: Verify directory is unchanged
    assert!(
        workspace.file_exists("old_dir"),
        "Original directory should still exist after dry run"
    );
    assert!(
        !workspace.file_exists("new_dir"),
        "New directory should NOT exist after dry run"
    );
    assert!(
        workspace.file_exists("old_dir/file.txt"),
        "Files in original directory should still exist"
    );
}
