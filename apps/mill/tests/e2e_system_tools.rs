use mill_test_support::harness::{TestClient, TestWorkspace};
use serde_json::json;

/// Test workspace.verify_project action (replaces health_check in public API)
#[tokio::test]
async fn test_workspace_verify_project_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let response = client
        .call_tool("workspace", json!({"action": "verify_project"}))
        .await
        .unwrap();
    // verify_project returns a WriteResponse with status of "success", "preview", or "error"
    let result = response["result"]
        .as_object()
        .expect("Should have result field");
    assert!(
        result.get("status").is_some(),
        "Response should have status field"
    );
    let status = result["status"].as_str().unwrap();
    assert!(
        status == "success" || status == "preview" || status == "error",
        "Status should be 'success', 'preview', or 'error', got: {}",
        status
    );
    // Should also have summary and changes
    assert!(
        result.get("summary").is_some(),
        "Response should have summary"
    );
}
/// Test workspace.verify_project with active LSP server
#[tokio::test]
async fn test_workspace_verify_project_with_active_lsp() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let ts_file = workspace.path().join("trigger.ts");
    std::fs::write(
        &ts_file,
        r#"
interface Test {
    id: number;
}

const test: Test = { id: 1 };
"#,
    )
    .unwrap();

    // Trigger LSP initialization by calling search_code
    let _response = client
        .call_tool(
            "search_code",
            json!({ "query": "Test", "filePath": ts_file.to_string_lossy() }),
        )
        .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

    let response = client
        .call_tool("workspace", json!({"action": "verify_project"}))
        .await
        .unwrap();
    // verify_project returns a WriteResponse with status of "success", "preview", or "error"
    let result = response["result"]
        .as_object()
        .expect("Should have result field");
    let status = result["status"].as_str().unwrap();
    assert!(
        status == "success" || status == "preview" || status == "error",
        "Status should be 'success', 'preview', or 'error', got: {}",
        status
    );
    // Should have summary and changes fields (from WriteResponse)
    assert!(
        result.get("summary").is_some(),
        "Response should have summary"
    );
}
/// Test workspace.verify_project with detailed information
#[tokio::test]
async fn test_workspace_verify_project_detailed() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let response = client
        .call_tool(
            "workspace",
            json!({ "action": "verify_project", "params": { "include_details": true } }),
        )
        .await
        .unwrap();
    // verify_project returns a WriteResponse with status, summary, and changes
    let result = response["result"]
        .as_object()
        .expect("Should have result field");
    assert!(result.get("status").is_some(), "Should have status");
    assert!(result.get("summary").is_some(), "Should have summary");
    // changes field contains plugin and metric details
    if let Some(changes) = result.get("changes") {
        // Check for plugins info in changes
        if let Some(plugins) = changes.get("plugins") {
            assert!(
                plugins.get("loaded").is_some(),
                "Should have loaded plugins count"
            );
        }
        // Check for metrics in changes
        if let Some(metrics) = changes.get("metrics") {
            assert!(metrics.is_object(), "Metrics should be an object");
        }
    }
}
/// Test rename_all for directory rename in Rust workspace (Magnificent Seven API)
#[tokio::test]
async fn test_rename_all_directory_in_rust_workspace() {
    let workspace = TestWorkspace::new();
    workspace.create_file(
        "Cargo.toml",
        r#"
[workspace]
resolver = "2"
members = ["crates/crate_a", "crates/crate_b"]
"#,
    );
    workspace.create_file(
        "crates/crate_a/Cargo.toml",
        r#"
[package]
name = "crate_a"
version = "0.1.0"
edition = "2021"

[dependencies]
crate_b = { path = "../crate_b" }
"#,
    );
    workspace.create_file(
        "crates/crate_a/src/lib.rs",
        "pub fn hello_a() { crate_b::hello_b(); }",
    );
    workspace.create_file(
        "crates/crate_b/Cargo.toml",
        r#"
[package]
name = "crate_b"
version = "0.1.0"
edition = "2021"
"#,
    );
    workspace.create_file(
        "crates/crate_b/src/lib.rs",
        "pub fn hello_b() { println!(\"Hello from B\"); }",
    );
    let cargo_available = std::process::Command::new("cargo")
        .arg("--version")
        .output()
        .is_ok();
    if cargo_available {
        let initial_check = std::process::Command::new("cargo")
            .arg("check")
            .current_dir(workspace.path())
            .output()
            .expect("Failed to run cargo check");
        assert!(
            initial_check.status.success(),
            "Initial workspace should be valid. Stderr: {}",
            String::from_utf8_lossy(&initial_check.stderr)
        );
    } else {
        eprintln!("Note: cargo not available, skipping initial validation");
    }
    let mut client = TestClient::new(workspace.path());

    // Execute rename_all with new Magnificent Seven API (dryRun: false)
    let apply_result = client
        .call_tool(
            "rename_all",
            json!({
                "target": {
                    "kind": "directory",
                    "filePath": "crates/crate_b"
                },
                "newName": "crates/crate_renamed",
                "options": {
                    "dryRun": false
                }
            }),
        )
        .await;
    assert!(apply_result.is_ok(), "rename_all should succeed");
    let apply_response = apply_result.unwrap();
    assert_eq!(
        apply_response["result"]["content"]["status"], "success",
        "rename_all should be applied successfully"
    );
    let ws_manifest = workspace.read_file("Cargo.toml");
    assert!(
        ws_manifest.contains("\"crates/crate_renamed\"")
            || ws_manifest.contains("crates/crate_renamed")
    );
    assert!(!ws_manifest.contains("\"crates/crate_b\"") || !ws_manifest.contains("crate_b\""));
    assert!(
        workspace.file_exists("crates/crate_renamed/Cargo.toml"),
        "Renamed crate should exist"
    );
    assert!(
        workspace.file_exists("crates/crate_renamed/src/lib.rs"),
        "Renamed crate source should exist"
    );
    assert!(
        !workspace.file_exists("crates/crate_b/Cargo.toml"),
        "Old crate directory should not exist"
    );
}
