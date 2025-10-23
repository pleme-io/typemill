//! Workspace package creation integration tests
//!
//! Tests for workspace.create_package tool (Proposal 50: Crate Extraction Tooling)
//!
//! Tests:
//! - Creating library packages
//! - Creating binary packages
//! - Workspace member registration
//! - Directory structure generation
//! - Cargo.toml generation
//! - Template support (minimal vs full)

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;
use std::fs;

#[tokio::test]
async fn test_create_library_package() {
    // Test creating a new library package in a workspace
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

    // Create the parent directory
    workspace.create_directory("crates");

    let package_path = workspace.absolute_path("crates/my-lib");

    // Call workspace.create_package
    let result = client
        .call_tool(
            "workspace.create_package",
            json!({
                "packagePath": package_path.to_string_lossy(),
                "packageType": "library",
                "options": {
                    "dryRun": false,
                    "addToWorkspace": true,
                    "template": "minimal"
                }
            }),
        )
        .await
        .expect("workspace.create_package should succeed");

    let content = result.get("result").expect("Result should exist");

    // Verify the operation succeeded
    assert_eq!(
        content.get("workspaceUpdated").and_then(|v| v.as_bool()),
        Some(true),
        "Workspace should be updated"
    );

    // Verify package directory was created
    assert!(
        workspace.file_exists("crates/my-lib/Cargo.toml"),
        "Package manifest should exist"
    );
    assert!(
        workspace.file_exists("crates/my-lib/src/lib.rs"),
        "Library entry point should exist"
    );

    // Verify Cargo.toml content
    let cargo_toml = workspace.read_file("crates/my-lib/Cargo.toml");
    assert!(
        cargo_toml.contains("name = \"my-lib\""),
        "Package name should be set"
    );
    assert!(
        cargo_toml.contains("[dependencies]"),
        "Dependencies section should exist"
    );

    // Verify lib.rs exists and has content
    let lib_rs = workspace.read_file("crates/my-lib/src/lib.rs");
    assert!(
        lib_rs.contains("crate"),
        "Library should have initial content"
    );

    // Verify workspace was updated
    let workspace_toml = workspace.read_file("Cargo.toml");
    assert!(
        workspace_toml.contains("\"crates/my-lib\""),
        "Workspace should include new package"
    );
}

#[tokio::test]
async fn test_create_binary_package() {
    // Test creating a new binary package
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

    // Create the parent directory
    workspace.create_directory("crates");

    let package_path = workspace.absolute_path("crates/my-bin");

    // Call workspace.create_package with binary type
    let result = client
        .call_tool(
            "workspace.create_package",
            json!({
                "packagePath": package_path.to_string_lossy(),
                "packageType": "binary",
                "options": {
                    "dryRun": false,
                    "addToWorkspace": true,
                    "template": "minimal"
                }
            }),
        )
        .await
        .expect("workspace.create_package should succeed");

    let content = result.get("result").expect("Result should exist");

    // Verify the operation succeeded
    assert_eq!(
        content.get("workspaceUpdated").and_then(|v| v.as_bool()),
        Some(true),
        "Workspace should be updated"
    );

    // Verify binary entry point was created
    assert!(
        workspace.file_exists("crates/my-bin/src/main.rs"),
        "Binary entry point should exist"
    );

    // Verify main.rs has main function
    let main_rs = workspace.read_file("crates/my-bin/src/main.rs");
    assert!(
        main_rs.contains("fn main()"),
        "Binary should have main function"
    );
}

#[tokio::test]
async fn test_create_package_without_workspace_registration() {
    // Test creating a package without adding to workspace
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

    // Create the parent directory
    workspace.create_directory("standalone");

    let package_path = workspace.absolute_path("standalone/my-lib");

    // Call workspace.create_package with add_to_workspace = false
    let result = client
        .call_tool(
            "workspace.create_package",
            json!({
                "packagePath": package_path.to_string_lossy(),
                "packageType": "library",
                "options": {
                    "dryRun": false,
                    "addToWorkspace": false,
                    "template": "minimal"
                }
            }),
        )
        .await
        .expect("workspace.create_package should succeed");

    let content = result.get("result").expect("Result should exist");

    // Verify workspace was NOT updated
    assert_eq!(
        content.get("workspaceUpdated").and_then(|v| v.as_bool()),
        Some(false),
        "Workspace should not be updated"
    );

    // Verify package was still created
    assert!(
        workspace.file_exists("standalone/my-lib/Cargo.toml"),
        "Package should be created"
    );

    // Verify workspace members list was not updated
    let workspace_toml = workspace.read_file("Cargo.toml");
    assert!(
        !workspace_toml.contains("standalone/my-lib"),
        "Workspace should not include standalone package"
    );
}

#[tokio::test]
async fn test_create_package_dry_run() {
    // Test dry run mode (should fail with not yet supported error)
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let package_path = workspace.absolute_path("crates/test-lib");

    // Call workspace.create_package with dry_run = true
    let result = client
        .call_tool(
            "workspace.create_package",
            json!({
                "packagePath": package_path.to_string_lossy(),
                "packageType": "library",
                "options": {
                    "dryRun": true,
                    "addToWorkspace": true,
                    "template": "minimal"
                }
            }),
        )
        .await;

    // Should return an error (call_tool converts JSON-RPC errors to Result::Err)
    assert!(
        result.is_err(),
        "Dry run mode should return an error, got: {:?}",
        result
    );

    // Verify the error message mentions dry_run
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("dry_run") || error_msg.contains("not yet supported"),
        "Error message should mention dry_run, got: {}",
        error_msg
    );

    // Verify nothing was created
    assert!(
        !workspace.file_exists("crates/test-lib/Cargo.toml"),
        "No files should be created in dry run"
    );
}

#[tokio::test]
async fn test_create_package_full_template() {
    // Test creating a package with full template
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

    // Create the parent directory
    workspace.create_directory("crates");

    let package_path = workspace.absolute_path("crates/full-lib");

    // Call workspace.create_package with full template
    let result = client
        .call_tool(
            "workspace.create_package",
            json!({
                "packagePath": package_path.to_string_lossy(),
                "packageType": "library",
                "options": {
                    "dryRun": false,
                    "addToWorkspace": true,
                    "template": "full"
                }
            }),
        )
        .await
        .expect("workspace.create_package should succeed");

    let content = result.get("result").expect("Result should exist");

    // Verify the operation succeeded
    assert_eq!(
        content.get("workspaceUpdated").and_then(|v| v.as_bool()),
        Some(true),
        "Workspace should be updated"
    );

    // Verify full template files were created
    assert!(
        workspace.file_exists("crates/full-lib/README.md"),
        "Full template should have README.md"
    );
    assert!(
        workspace.file_exists("crates/full-lib/tests/integration_test.rs"),
        "Full template should have integration tests"
    );
    assert!(
        workspace.file_exists("crates/full-lib/examples/basic.rs"),
        "Full template should have examples"
    );
}

#[tokio::test]
async fn test_create_package_file_list() {
    // Test that created_files list is accurate
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

    // Create the parent directory
    workspace.create_directory("crates");

    let package_path = workspace.absolute_path("crates/test-lib");

    let result = client
        .call_tool(
            "workspace.create_package",
            json!({
                "packagePath": package_path.to_string_lossy(),
                "packageType": "library",
                "options": {
                    "dryRun": false,
                    "addToWorkspace": true,
                    "template": "minimal"
                }
            }),
        )
        .await
        .expect("workspace.create_package should succeed");

    let content = result.get("result").expect("Result should exist");

    // Verify created_files list
    let created_files = content
        .get("createdFiles")
        .and_then(|v| v.as_array())
        .expect("created_files should be an array");

    assert!(
        created_files.len() >= 2,
        "Should create at least Cargo.toml and lib.rs"
    );

    // Verify all reported files actually exist
    for file in created_files {
        let file_path = file.as_str().expect("File path should be string");
        let exists = fs::metadata(file_path).is_ok();
        assert!(exists, "Created file should exist: {}", file_path);
    }
}
