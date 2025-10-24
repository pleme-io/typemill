//! workspace.update_members tests migrated to closure-based helpers (v2)
//!
//! BEFORE: 636 lines with repetitive setup
//! AFTER: Focused workspace member management verification
//!
//! Tests workspace member add/remove/list operations.

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

/// Helper: Create workspace with initial members
fn setup_workspace(workspace: &TestWorkspace, members: &[&str]) -> std::path::PathBuf {
    let members_str = members
        .iter()
        .map(|m| format!("\"{}\"", m))
        .collect::<Vec<_>>()
        .join(", ");

    workspace.create_file(
        "Cargo.toml",
        &format!(
            r#"[workspace]
members = [{}]

[workspace.package]
version = "0.1.0"
edition = "2021"
"#,
            members_str
        ),
    );

    workspace.absolute_path("Cargo.toml")
}

#[tokio::test]
async fn test_add_members_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let manifest_path = setup_workspace(&workspace, &["crates/existing-crate"]);

    // Create directories
    workspace.create_directory("crates/existing-crate/src");
    workspace.create_directory("crates/new-crate1/src");
    workspace.create_directory("crates/new-crate2/src");

    let result = client
        .call_tool(
            "workspace.update_members",
            json!({
                "workspaceManifest": manifest_path.to_string_lossy(),
                "action": "add",
                "members": ["crates/new-crate1", "crates/new-crate2"],
                "options": {
                    "dryRun": false,
                    "createIfMissing": false
                }
            }),
        )
        .await
        .expect("workspace.update_members should succeed");

    let content = result.get("result").expect("Result should exist");

    assert_eq!(
        content.get("action").and_then(|v| v.as_str()),
        Some("add")
    );
    assert_eq!(
        content.get("changesMade").and_then(|v| v.as_u64()),
        Some(2)
    );
    assert_eq!(
        content.get("workspaceUpdated").and_then(|v| v.as_bool()),
        Some(true)
    );

    let members_before = content.get("membersBefore").and_then(|v| v.as_array()).unwrap();
    assert_eq!(members_before.len(), 1);

    let members_after = content.get("membersAfter").and_then(|v| v.as_array()).unwrap();
    assert_eq!(members_after.len(), 3);

    let cargo_toml = workspace.read_file("Cargo.toml");
    assert!(cargo_toml.contains("crates/new-crate1"));
    assert!(cargo_toml.contains("crates/new-crate2"));
}

#[tokio::test]
async fn test_remove_members_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let manifest_path = setup_workspace(&workspace, &["crates/crate1", "crates/crate2", "crates/crate3"]);

    let result = client
        .call_tool(
            "workspace.update_members",
            json!({
                "workspaceManifest": manifest_path.to_string_lossy(),
                "action": "remove",
                "members": ["crates/crate2"],
                "options": {
                    "dryRun": false
                }
            }),
        )
        .await
        .expect("workspace.update_members should succeed");

    let content = result.get("result").expect("Result should exist");

    assert_eq!(
        content.get("action").and_then(|v| v.as_str()),
        Some("remove")
    );
    assert_eq!(
        content.get("changesMade").and_then(|v| v.as_u64()),
        Some(1)
    );
    assert_eq!(
        content.get("workspaceUpdated").and_then(|v| v.as_bool()),
        Some(true)
    );

    let members_after = content.get("membersAfter").and_then(|v| v.as_array()).unwrap();
    assert_eq!(members_after.len(), 2);

    let cargo_toml = workspace.read_file("Cargo.toml");
    assert!(cargo_toml.contains("crates/crate1"));
    assert!(!cargo_toml.contains("crates/crate2"));
    assert!(cargo_toml.contains("crates/crate3"));
}

#[tokio::test]
async fn test_list_members() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let manifest_path = setup_workspace(&workspace, &["crates/crate1", "crates/crate2", "crates/crate3"]);

    let result = client
        .call_tool(
            "workspace.update_members",
            json!({
                "workspaceManifest": manifest_path.to_string_lossy(),
                "action": "list"
            }),
        )
        .await
        .expect("workspace.update_members should succeed");

    let content = result.get("result").expect("Result should exist");

    assert_eq!(
        content.get("action").and_then(|v| v.as_str()),
        Some("list")
    );
    assert_eq!(
        content.get("changesMade").and_then(|v| v.as_u64()),
        Some(0)
    );
    assert_eq!(
        content.get("workspaceUpdated").and_then(|v| v.as_bool()),
        Some(false)
    );

    let members_before = content.get("membersBefore").and_then(|v| v.as_array()).unwrap();
    assert_eq!(members_before.len(), 3);

    let members_after = content.get("membersAfter").and_then(|v| v.as_array()).unwrap();
    assert_eq!(members_after.len(), 3);

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
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let manifest_path = setup_workspace(&workspace, &["crates/existing-crate"]);

    let result = client
        .call_tool(
            "workspace.update_members",
            json!({
                "workspaceManifest": manifest_path.to_string_lossy(),
                "action": "add",
                "members": ["crates/existing-crate"],
                "options": {
                    "dryRun": false
                }
            }),
        )
        .await
        .expect("workspace.update_members should succeed");

    let content = result.get("result").expect("Result should exist");

    assert_eq!(
        content.get("changesMade").and_then(|v| v.as_u64()),
        Some(0)
    );
    assert_eq!(
        content.get("workspaceUpdated").and_then(|v| v.as_bool()),
        Some(false)
    );

    let members_after = content.get("membersAfter").and_then(|v| v.as_array()).unwrap();
    assert_eq!(members_after.len(), 1);
}

#[tokio::test]
async fn test_dry_run_mode() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let manifest_path = setup_workspace(&workspace, &["crates/crate1"]);
    let original_content = workspace.read_file("Cargo.toml");

    let result = client
        .call_tool(
            "workspace.update_members",
            json!({
                "workspaceManifest": manifest_path.to_string_lossy(),
                "action": "add",
                "members": ["crates/new-crate"],
                "options": {
                    "dryRun": true
                }
            }),
        )
        .await
        .expect("workspace.update_members should succeed");

    let content = result.get("result").expect("Result should exist");

    assert_eq!(
        content.get("dryRun").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        content.get("workspaceUpdated").and_then(|v| v.as_bool()),
        Some(false)
    );

    // Verify file unchanged
    let current_content = workspace.read_file("Cargo.toml");
    assert_eq!(original_content, current_content);

    // Verify preview of changes
    assert_eq!(
        content.get("changesMade").and_then(|v| v.as_u64()),
        Some(1)
    );

    let members_after = content.get("membersAfter").and_then(|v| v.as_array()).unwrap();
    assert_eq!(members_after.len(), 2);
}

#[tokio::test]
async fn test_create_workspace_section_if_missing() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "Cargo.toml",
        r#"[package]
name = "my-package"
version = "0.1.0"
edition = "2021"
"#,
    );

    let manifest_path = workspace.absolute_path("Cargo.toml");

    let result = client
        .call_tool(
            "workspace.update_members",
            json!({
                "workspaceManifest": manifest_path.to_string_lossy(),
                "action": "add",
                "members": ["crates/new-crate"],
                "options": {
                    "dryRun": false,
                    "createIfMissing": true
                }
            }),
        )
        .await
        .expect("workspace.update_members should succeed");

    let content = result.get("result").expect("Result should exist");

    assert_eq!(
        content.get("workspaceUpdated").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        content.get("changesMade").and_then(|v| v.as_u64()),
        Some(1)
    );

    let cargo_toml = workspace.read_file("Cargo.toml");
    assert!(cargo_toml.contains("[workspace]"));
    assert!(cargo_toml.contains("crates/new-crate"));
}

#[tokio::test]
async fn test_error_on_missing_workspace_section() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "Cargo.toml",
        r#"[package]
name = "my-package"
version = "0.1.0"
edition = "2021"
"#,
    );

    let manifest_path = workspace.absolute_path("Cargo.toml");

    let error = client
        .call_tool(
            "workspace.update_members",
            json!({
                "workspaceManifest": manifest_path.to_string_lossy(),
                "action": "add",
                "members": ["crates/new-crate"],
                "options": {
                    "dryRun": false,
                    "createIfMissing": false
                }
            }),
        )
        .await
        .expect_err("Should return an error");

    let error_msg = error.to_string();
    assert!(
        error_msg.contains("workspace"),
        "Error should mention workspace section: {}",
        error_msg
    );
}

#[tokio::test]
async fn test_error_on_nonexistent_manifest() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let manifest_path = workspace.absolute_path("nonexistent/Cargo.toml");

    let error = client
        .call_tool(
            "workspace.update_members",
            json!({
                "workspaceManifest": manifest_path.to_string_lossy(),
                "action": "list"
            }),
        )
        .await
        .expect_err("Should return an error");

    let error_msg = error.to_string();
    assert!(
        error_msg.contains("not found") || error_msg.contains("does not exist"),
        "Error should mention file not found: {}",
        error_msg
    );
}

#[tokio::test]
async fn test_remove_nonexistent_member() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let manifest_path = setup_workspace(&workspace, &["crates/crate1"]);

    let result = client
        .call_tool(
            "workspace.update_members",
            json!({
                "workspaceManifest": manifest_path.to_string_lossy(),
                "action": "remove",
                "members": ["crates/nonexistent"],
                "options": {
                    "dryRun": false
                }
            }),
        )
        .await
        .expect("workspace.update_members should succeed");

    let content = result.get("result").expect("Result should exist");

    assert_eq!(
        content.get("changesMade").and_then(|v| v.as_u64()),
        Some(0)
    );
    assert_eq!(
        content.get("workspaceUpdated").and_then(|v| v.as_bool()),
        Some(false)
    );
}

#[tokio::test]
async fn test_path_normalization() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let manifest_path = setup_workspace(&workspace, &[]);

    let result = client
        .call_tool(
            "workspace.update_members",
            json!({
                "workspaceManifest": manifest_path.to_string_lossy(),
                "action": "add",
                "members": ["crates\\my-crate"],
                "options": {
                    "dryRun": false
                }
            }),
        )
        .await
        .expect("workspace.update_members should succeed");

    let content = result.get("result").expect("Result should exist");

    assert_eq!(
        content.get("changesMade").and_then(|v| v.as_u64()),
        Some(1)
    );

    let cargo_toml = workspace.read_file("Cargo.toml");
    assert!(cargo_toml.contains("crates/my-crate"));
    assert!(!cargo_toml.contains("crates\\my-crate"));
}
