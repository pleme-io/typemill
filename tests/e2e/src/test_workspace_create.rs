//! workspace create_package tests migrated to closure-based helpers (v2)
//!
//! BEFORE: 397 lines with manual workspace setup
//! AFTER: Focused workspace operation verification
//!
//! Tests workspace package creation tool.

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;
use std::fs;

#[tokio::test]
async fn test_create_library_package() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = []

[workspace.package]
version = "0.1.0"
edition = "2021"
"#,
    );
    workspace.create_directory("crates");

    let package_path = workspace.absolute_path("crates/my-lib");

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "create_package",
                "params": {
                    "packagePath": package_path.to_string_lossy(),
                    "packageType": "library"
                },
                "options": {
                    "dryRun": false,
                    "addToWorkspace": true,
                    "template": "minimal"
                }
            }),
        )
        .await
        .expect("workspace create_package should succeed");

    let content = result.get("result").expect("Result should exist");

    // M7 response: status at top level, action-specific data in changes
    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success")
    );
    let changes = content.get("changes").expect("Changes should exist");
    assert_eq!(
        changes.get("workspaceUpdated").and_then(|v| v.as_bool()),
        Some(true)
    );

    // Verify package structure
    assert!(workspace.file_exists("crates/my-lib/Cargo.toml"));
    assert!(workspace.file_exists("crates/my-lib/src/lib.rs"));

    let cargo_toml = workspace.read_file("crates/my-lib/Cargo.toml");
    assert!(cargo_toml.contains("name = \"my-lib\""));
    assert!(cargo_toml.contains("[dependencies]"));

    let lib_rs = workspace.read_file("crates/my-lib/src/lib.rs");
    assert!(lib_rs.contains("crate"));

    let workspace_toml = workspace.read_file("Cargo.toml");
    assert!(workspace_toml.contains("\"crates/my-lib\""));
}

#[tokio::test]
async fn test_create_binary_package() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = []

[workspace.package]
version = "0.1.0"
edition = "2021"
"#,
    );
    workspace.create_directory("crates");

    let package_path = workspace.absolute_path("crates/my-bin");

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "create_package",
                "params": {
                    "packagePath": package_path.to_string_lossy(),
                    "packageType": "binary"
                },
                "options": {
                    "dryRun": false,
                    "addToWorkspace": true,
                    "template": "minimal"
                }
            }),
        )
        .await
        .expect("workspace create_package should succeed");

    let content = result.get("result").expect("Result should exist");

    // M7 response: status at top level, action-specific data in changes
    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success")
    );
    let changes = content.get("changes").expect("Changes should exist");
    assert_eq!(
        changes.get("workspaceUpdated").and_then(|v| v.as_bool()),
        Some(true)
    );

    // Verify binary structure
    assert!(workspace.file_exists("crates/my-bin/src/main.rs"));

    let main_rs = workspace.read_file("crates/my-bin/src/main.rs");
    assert!(main_rs.contains("fn main()"));
}

#[tokio::test]
async fn test_create_package_without_workspace_registration() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = []

[workspace.package]
version = "0.1.0"
edition = "2021"
"#,
    );
    workspace.create_directory("standalone");

    let package_path = workspace.absolute_path("standalone/my-lib");

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "create_package",
                "params": {
                    "packagePath": package_path.to_string_lossy(),
                    "packageType": "library"
                },
                "options": {
                    "dryRun": false,
                    "addToWorkspace": false,
                    "template": "minimal"
                }
            }),
        )
        .await
        .expect("workspace create_package should succeed");

    let content = result.get("result").expect("Result should exist");

    // M7 response: status at top level, action-specific data in changes
    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success")
    );
    let changes = content.get("changes").expect("Changes should exist");

    assert_eq!(
        changes.get("workspaceUpdated").and_then(|v| v.as_bool()),
        Some(false)
    );

    assert!(workspace.file_exists("standalone/my-lib/Cargo.toml"));

    let workspace_toml = workspace.read_file("Cargo.toml");
    assert!(!workspace_toml.contains("standalone/my-lib"));
}

#[tokio::test]
async fn test_create_package_dry_run() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let package_path = workspace.absolute_path("crates/test-lib");

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "create_package",
                "params": {
                    "packagePath": package_path.to_string_lossy(),
                    "packageType": "library"
                },
                "options": {
                    "dryRun": true,
                    "addToWorkspace": true,
                    "template": "minimal"
                }
            }),
        )
        .await;

    // Should return error (dry run not supported)
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("dry_run") || error_msg.contains("not yet supported"),
        "Error should mention dry_run: {}",
        error_msg
    );

    assert!(!workspace.file_exists("crates/test-lib/Cargo.toml"));
}

#[tokio::test]
async fn test_create_package_full_template() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = []

[workspace.package]
version = "0.1.0"
edition = "2021"
"#,
    );
    workspace.create_directory("crates");

    let package_path = workspace.absolute_path("crates/full-lib");

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "create_package",
                "params": {
                    "packagePath": package_path.to_string_lossy(),
                    "packageType": "library"
                },
                "options": {
                    "dryRun": false,
                    "addToWorkspace": true,
                    "template": "full"
                }
            }),
        )
        .await
        .expect("workspace create_package should succeed");

    let content = result.get("result").expect("Result should exist");

    // M7 response: status at top level, action-specific data in changes
    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success")
    );
    let changes = content.get("changes").expect("Changes should exist");
    assert_eq!(
        changes.get("workspaceUpdated").and_then(|v| v.as_bool()),
        Some(true)
    );

    // Verify full template files
    assert!(workspace.file_exists("crates/full-lib/README.md"));
    assert!(workspace.file_exists("crates/full-lib/tests/integration_test.rs"));
    assert!(workspace.file_exists("crates/full-lib/examples/basic.rs"));
}

#[tokio::test]
async fn test_create_package_file_list() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = []

[workspace.package]
version = "0.1.0"
edition = "2021"
"#,
    );
    workspace.create_directory("crates");

    let package_path = workspace.absolute_path("crates/test-lib");

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "create_package",
                "params": {
                    "packagePath": package_path.to_string_lossy(),
                    "packageType": "library"
                },
                "options": {
                    "dryRun": false,
                    "addToWorkspace": true,
                    "template": "minimal"
                }
            }),
        )
        .await
        .expect("workspace create_package should succeed");

    let content = result.get("result").expect("Result should exist");

    // M7 response: status at top level, action-specific data in changes
    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success")
    );
    let changes = content.get("changes").expect("Changes should exist");

    // Verify created_files list
    let created_files = changes
        .get("createdFiles")
        .and_then(|v| v.as_array())
        .expect("created_files should be an array");

    assert!(created_files.len() >= 2);

    // Verify all reported files exist
    for file in created_files {
        let file_path = file.as_str().expect("File path should be string");
        let exists = fs::metadata(file_path).is_ok();
        assert!(exists, "Created file should exist: {}", file_path);
    }
}
