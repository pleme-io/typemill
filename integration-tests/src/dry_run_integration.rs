//! Dry run integration tests
//!
//! This test suite ensures that when dry_run=true is specified, no actual
//! file system modifications occur. This is critical for safety and user trust.

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

    // 2. Call rename_file with dry_run=true
    let response = client
        .call_tool(
            "move_file",
            json!({
                "old_path": old_file_path.to_str().unwrap(),
                "new_path": new_file_path.to_str().unwrap(),
                "dry_run": true
            }),
        )
        .await
        .unwrap();

    // 3. Assertions on response
    let result = &response["result"];
    assert_eq!(
        result["status"], "preview",
        "Response should indicate a preview"
    );

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
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let new_file_path = workspace.path().join("new_file.txt");

    // 2. Call create_file with dry_run=true
    let response = client
        .call_tool(
            "create_file",
            json!({
                "file_path": new_file_path.to_str().unwrap(),
                "content": "This should not be written",
                "dry_run": true
            }),
        )
        .await
        .unwrap();

    // 3. Assertions on response
    let result = &response["result"];
    assert_eq!(
        result["status"], "preview",
        "Response should indicate a preview"
    );
    assert_eq!(
        result["operation"], "create_file",
        "Operation should be create_file"
    );

    // 4. CRITICAL: Verify file was NOT created
    assert!(
        !workspace.file_exists("new_file.txt"),
        "File should NOT exist after dry run create"
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

    // 2. Call delete_file with dry_run=true
    let response = client
        .call_tool(
            "delete_file",
            json!({
                "file_path": file_path.to_str().unwrap(),
                "dry_run": true
            }),
        )
        .await
        .unwrap();

    // 3. Assertions on response
    let result = &response["result"];
    assert_eq!(
        result["status"], "preview",
        "Response should indicate a preview"
    );
    assert_eq!(
        result["operation"], "delete_file",
        "Operation should be delete_file"
    );

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

    // 2. Call rename_directory with dry_run=true
    let response = client
        .call_tool(
            "move_directory",
            json!({
                "old_path": old_dir.to_str().unwrap(),
                "new_path": new_dir.to_str().unwrap(),
                "dry_run": true
            }),
        )
        .await
        .unwrap();

    // 3. Assertions on response
    let result = &response["result"];
    assert_eq!(
        result["status"], "preview",
        "Response should indicate a preview"
    );

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
