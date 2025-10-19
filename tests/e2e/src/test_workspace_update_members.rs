//! Workspace member management integration tests
//!
//! Tests for workspace.update_members tool (Proposal 50: Crate Extraction Tooling)
//!
//! Tests:
//! - Adding new members
//! - Removing existing members
//! - Listing members
//! - Duplicate detection (adding existing member)
//! - Dry-run mode
//! - Creating [workspace] section if missing
//! - Error handling (nonexistent workspace file)

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

#[tokio::test]
async fn test_add_members_basic() {
    // Test adding new members to workspace
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a workspace Cargo.toml with initial members
    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = ["crates/existing-crate"]

[workspace.package]
version = "0.1.0"
edition = "2021"
"#,
    );

    // Create member directories (to avoid warnings)
    workspace.create_directory("crates/existing-crate/src");
    workspace.create_directory("crates/new-crate1/src");
    workspace.create_directory("crates/new-crate2/src");

    let manifest_path = workspace.absolute_path("Cargo.toml");

    // Call workspace.update_members to add new members
    let result = client
        .call_tool(
            "workspace.update_members",
            json!({
                "workspace_manifest": manifest_path.to_string_lossy(),
                "action": "add",
                "members": ["crates/new-crate1", "crates/new-crate2"],
                "options": {
                    "dry_run": false,
                    "create_if_missing": false
                }
            }),
        )
        .await
        .expect("workspace.update_members should succeed");

    let content = result.get("result").expect("Result should exist");

    // Verify the operation succeeded
    assert_eq!(
        content.get("action").and_then(|v| v.as_str()),
        Some("add"),
        "Action should be 'add'"
    );

    assert_eq!(
        content.get("changes_made").and_then(|v| v.as_u64()),
        Some(2),
        "Should have made 2 changes"
    );

    assert_eq!(
        content.get("workspace_updated").and_then(|v| v.as_bool()),
        Some(true),
        "Workspace should be updated"
    );

    // Verify members_before and members_after
    let members_before = content
        .get("members_before")
        .and_then(|v| v.as_array())
        .expect("members_before should be an array");
    assert_eq!(members_before.len(), 1);

    let members_after = content
        .get("members_after")
        .and_then(|v| v.as_array())
        .expect("members_after should be an array");
    assert_eq!(members_after.len(), 3);

    // Verify the file was actually updated
    let cargo_toml = workspace.read_file("Cargo.toml");
    assert!(
        cargo_toml.contains("crates/new-crate1"),
        "Should contain new-crate1"
    );
    assert!(
        cargo_toml.contains("crates/new-crate2"),
        "Should contain new-crate2"
    );
}

#[tokio::test]
async fn test_remove_members_basic() {
    // Test removing members from workspace
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a workspace Cargo.toml with multiple members
    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = ["crates/crate1", "crates/crate2", "crates/crate3"]

[workspace.package]
version = "0.1.0"
edition = "2021"
"#,
    );

    let manifest_path = workspace.absolute_path("Cargo.toml");

    // Call workspace.update_members to remove a member
    let result = client
        .call_tool(
            "workspace.update_members",
            json!({
                "workspace_manifest": manifest_path.to_string_lossy(),
                "action": "remove",
                "members": ["crates/crate2"],
                "options": {
                    "dry_run": false
                }
            }),
        )
        .await
        .expect("workspace.update_members should succeed");

    let content = result.get("result").expect("Result should exist");

    // Verify the operation
    assert_eq!(
        content.get("action").and_then(|v| v.as_str()),
        Some("remove"),
        "Action should be 'remove'"
    );

    assert_eq!(
        content.get("changes_made").and_then(|v| v.as_u64()),
        Some(1),
        "Should have made 1 change"
    );

    assert_eq!(
        content.get("workspace_updated").and_then(|v| v.as_bool()),
        Some(true),
        "Workspace should be updated"
    );

    // Verify members count
    let members_after = content
        .get("members_after")
        .and_then(|v| v.as_array())
        .expect("members_after should be an array");
    assert_eq!(members_after.len(), 2);

    // Verify the file was actually updated
    let cargo_toml = workspace.read_file("Cargo.toml");
    assert!(
        cargo_toml.contains("crates/crate1"),
        "Should still contain crate1"
    );
    assert!(
        !cargo_toml.contains("crates/crate2"),
        "Should not contain crate2"
    );
    assert!(
        cargo_toml.contains("crates/crate3"),
        "Should still contain crate3"
    );
}

#[tokio::test]
async fn test_list_members() {
    // Test listing workspace members
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a workspace Cargo.toml with members
    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = ["crates/crate1", "crates/crate2", "crates/crate3"]

[workspace.package]
version = "0.1.0"
edition = "2021"
"#,
    );

    let manifest_path = workspace.absolute_path("Cargo.toml");

    // Call workspace.update_members to list members
    let result = client
        .call_tool(
            "workspace.update_members",
            json!({
                "workspace_manifest": manifest_path.to_string_lossy(),
                "action": "list"
            }),
        )
        .await
        .expect("workspace.update_members should succeed");

    let content = result.get("result").expect("Result should exist");

    // Verify the operation
    assert_eq!(
        content.get("action").and_then(|v| v.as_str()),
        Some("list"),
        "Action should be 'list'"
    );

    assert_eq!(
        content.get("changes_made").and_then(|v| v.as_u64()),
        Some(0),
        "Should have made 0 changes"
    );

    assert_eq!(
        content.get("workspace_updated").and_then(|v| v.as_bool()),
        Some(false),
        "Workspace should not be updated"
    );

    // Verify members list
    let members_before = content
        .get("members_before")
        .and_then(|v| v.as_array())
        .expect("members_before should be an array");
    assert_eq!(members_before.len(), 3);

    let members_after = content
        .get("members_after")
        .and_then(|v| v.as_array())
        .expect("members_after should be an array");
    assert_eq!(members_after.len(), 3);

    // Verify members are correct
    let member_strings: Vec<String> = members_after
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
    assert!(member_strings.contains(&"crates/crate1".to_string()));
    assert!(member_strings.contains(&"crates/crate2".to_string()));
    assert!(member_strings.contains(&"crates/crate3".to_string()));
}

#[tokio::test]
async fn test_add_duplicate_member() {
    // Test that adding an already existing member is handled gracefully
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a workspace Cargo.toml with a member
    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = ["crates/existing-crate"]

[workspace.package]
version = "0.1.0"
edition = "2021"
"#,
    );

    let manifest_path = workspace.absolute_path("Cargo.toml");

    // Try to add the same member again
    let result = client
        .call_tool(
            "workspace.update_members",
            json!({
                "workspace_manifest": manifest_path.to_string_lossy(),
                "action": "add",
                "members": ["crates/existing-crate"],
                "options": {
                    "dry_run": false
                }
            }),
        )
        .await
        .expect("workspace.update_members should succeed");

    let content = result.get("result").expect("Result should exist");

    // Verify no changes were made
    assert_eq!(
        content.get("changes_made").and_then(|v| v.as_u64()),
        Some(0),
        "Should have made 0 changes"
    );

    assert_eq!(
        content.get("workspace_updated").and_then(|v| v.as_bool()),
        Some(false),
        "Workspace should not be updated"
    );

    // Verify members list unchanged
    let members_after = content
        .get("members_after")
        .and_then(|v| v.as_array())
        .expect("members_after should be an array");
    assert_eq!(members_after.len(), 1);
}

#[tokio::test]
async fn test_dry_run_mode() {
    // Test dry-run mode doesn't modify the file
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a workspace Cargo.toml
    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = ["crates/crate1"]

[workspace.package]
version = "0.1.0"
edition = "2021"
"#,
    );

    let manifest_path = workspace.absolute_path("Cargo.toml");
    let original_content = workspace.read_file("Cargo.toml");

    // Call workspace.update_members with dry_run = true
    let result = client
        .call_tool(
            "workspace.update_members",
            json!({
                "workspace_manifest": manifest_path.to_string_lossy(),
                "action": "add",
                "members": ["crates/new-crate"],
                "options": {
                    "dry_run": true
                }
            }),
        )
        .await
        .expect("workspace.update_members should succeed");

    let content = result.get("result").expect("Result should exist");

    // Verify dry_run flag
    assert_eq!(
        content.get("dry_run").and_then(|v| v.as_bool()),
        Some(true),
        "dry_run should be true"
    );

    assert_eq!(
        content.get("workspace_updated").and_then(|v| v.as_bool()),
        Some(false),
        "Workspace should not be updated in dry-run"
    );

    // Verify the file was NOT modified
    let current_content = workspace.read_file("Cargo.toml");
    assert_eq!(
        original_content, current_content,
        "File should not be modified in dry-run"
    );

    // Verify we got the preview of changes
    assert_eq!(
        content.get("changes_made").and_then(|v| v.as_u64()),
        Some(1),
        "Should report 1 change would be made"
    );

    let members_after = content
        .get("members_after")
        .and_then(|v| v.as_array())
        .expect("members_after should show preview");
    assert_eq!(members_after.len(), 2);
}

#[tokio::test]
async fn test_create_workspace_section_if_missing() {
    // Test create_if_missing option creates [workspace] section
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a Cargo.toml WITHOUT workspace section
    workspace.create_file(
        "Cargo.toml",
        r#"[package]
name = "my-package"
version = "0.1.0"
edition = "2021"
"#,
    );

    let manifest_path = workspace.absolute_path("Cargo.toml");

    // Call workspace.update_members with create_if_missing = true
    let result = client
        .call_tool(
            "workspace.update_members",
            json!({
                "workspace_manifest": manifest_path.to_string_lossy(),
                "action": "add",
                "members": ["crates/new-crate"],
                "options": {
                    "dry_run": false,
                    "create_if_missing": true
                }
            }),
        )
        .await
        .expect("workspace.update_members should succeed");

    let content = result.get("result").expect("Result should exist");

    // Verify the operation succeeded
    assert_eq!(
        content.get("workspace_updated").and_then(|v| v.as_bool()),
        Some(true),
        "Workspace should be updated"
    );

    assert_eq!(
        content.get("changes_made").and_then(|v| v.as_u64()),
        Some(1),
        "Should have made 1 change"
    );

    // Verify [workspace] section was created
    let cargo_toml = workspace.read_file("Cargo.toml");
    assert!(
        cargo_toml.contains("[workspace]"),
        "Should have created [workspace] section"
    );
    assert!(
        cargo_toml.contains("crates/new-crate"),
        "Should contain new member"
    );
}

#[tokio::test]
async fn test_error_on_missing_workspace_section() {
    // Test that without create_if_missing, error is returned
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a Cargo.toml WITHOUT workspace section
    workspace.create_file(
        "Cargo.toml",
        r#"[package]
name = "my-package"
version = "0.1.0"
edition = "2021"
"#,
    );

    let manifest_path = workspace.absolute_path("Cargo.toml");

    // Call workspace.update_members WITHOUT create_if_missing
    let result = client
        .call_tool(
            "workspace.update_members",
            json!({
                "workspace_manifest": manifest_path.to_string_lossy(),
                "action": "add",
                "members": ["crates/new-crate"],
                "options": {
                    "dry_run": false,
                    "create_if_missing": false
                }
            }),
        )
        .await
        .expect("call_tool should return a response");

    // Should have an error
    let has_error = result.get("error").is_some();
    assert!(has_error, "Should return an error response");

    if let Some(error) = result.get("error") {
        let message = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("");
        assert!(
            message.contains("workspace"),
            "Error should mention workspace section: {}",
            message
        );
    }
}

#[tokio::test]
async fn test_error_on_nonexistent_manifest() {
    // Test error handling for nonexistent workspace file
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let manifest_path = workspace.absolute_path("nonexistent/Cargo.toml");

    // Call workspace.update_members on nonexistent file
    let result = client
        .call_tool(
            "workspace.update_members",
            json!({
                "workspace_manifest": manifest_path.to_string_lossy(),
                "action": "list"
            }),
        )
        .await
        .expect("call_tool should return a response");

    // Should have an error
    let has_error = result.get("error").is_some();
    assert!(has_error, "Should return an error response");

    if let Some(error) = result.get("error") {
        let message = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("");
        assert!(
            message.contains("not found") || message.contains("does not exist"),
            "Error should mention file not found: {}",
            message
        );
    }
}

#[tokio::test]
async fn test_remove_nonexistent_member() {
    // Test that removing a nonexistent member doesn't error
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a workspace Cargo.toml
    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = ["crates/crate1"]

[workspace.package]
version = "0.1.0"
edition = "2021"
"#,
    );

    let manifest_path = workspace.absolute_path("Cargo.toml");

    // Try to remove a member that doesn't exist
    let result = client
        .call_tool(
            "workspace.update_members",
            json!({
                "workspace_manifest": manifest_path.to_string_lossy(),
                "action": "remove",
                "members": ["crates/nonexistent"],
                "options": {
                    "dry_run": false
                }
            }),
        )
        .await
        .expect("workspace.update_members should succeed");

    let content = result.get("result").expect("Result should exist");

    // Verify no changes were made
    assert_eq!(
        content.get("changes_made").and_then(|v| v.as_u64()),
        Some(0),
        "Should have made 0 changes"
    );

    assert_eq!(
        content.get("workspace_updated").and_then(|v| v.as_bool()),
        Some(false),
        "Workspace should not be updated"
    );
}

#[tokio::test]
async fn test_path_normalization() {
    // Test that paths with backslashes are normalized to forward slashes
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a workspace Cargo.toml
    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = []

[workspace.package]
version = "0.1.0"
edition = "2021"
"#,
    );

    let manifest_path = workspace.absolute_path("Cargo.toml");

    // Add member with backslashes (Windows-style path)
    let result = client
        .call_tool(
            "workspace.update_members",
            json!({
                "workspace_manifest": manifest_path.to_string_lossy(),
                "action": "add",
                "members": ["crates\\my-crate"],
                "options": {
                    "dry_run": false
                }
            }),
        )
        .await
        .expect("workspace.update_members should succeed");

    let content = result.get("result").expect("Result should exist");

    // Verify member was added
    assert_eq!(
        content.get("changes_made").and_then(|v| v.as_u64()),
        Some(1),
        "Should have made 1 change"
    );

    // Verify the file uses forward slashes
    let cargo_toml = workspace.read_file("Cargo.toml");
    assert!(
        cargo_toml.contains("crates/my-crate"),
        "Should use forward slashes"
    );
    assert!(
        !cargo_toml.contains("crates\\my-crate"),
        "Should not contain backslashes"
    );
}
