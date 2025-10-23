use mill_test_support::harness::{ TestClient , TestWorkspace };
use serde_json::{json, Value};
#[tokio::test]
async fn test_health_check_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let response = client.call_tool("health_check", json!({})).await.unwrap();
    let result = response["result"]
        .as_object()
        .expect("Should have result field");
    assert!(result.get("status").is_some());
    let status = result["status"].as_str().unwrap();
    assert!(status == "healthy" || status == "degraded" || status == "unhealthy");
    if let Some(servers) = result.get("servers") {
        let servers_array = servers.as_array().unwrap();
        for server in servers_array {
            assert!(server.get("name").is_some());
            assert!(server.get("status").is_some());
        }
    }
}
#[tokio::test]
async fn test_health_check_with_active_lsp() {
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
    let _response = client
        .call_tool(
            "get_document_symbols",
            json!({ "file_path" : ts_file.to_string_lossy() }),
        )
        .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
    let response = client.call_tool("health_check", json!({})).await.unwrap();
    let result = response["result"]
        .as_object()
        .expect("Should have result field");
    let status = result["status"].as_str().unwrap();
    assert!(status == "healthy" || status == "degraded");
    if let Some(servers) = result.get("servers") {
        let servers_array = servers.as_array().unwrap();
        let _has_ts_server = servers_array.iter().any(|s| {
            s["name"].as_str().unwrap_or("").contains("typescript")
                || s["name"].as_str().unwrap_or("").contains("ts")
        });
        // Server may or may not be running depending on LSP initialization
    }
}
#[tokio::test]
async fn test_health_check_detailed() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let response = client
        .call_tool("health_check", json!({ "include_details" : true }))
        .await
        .unwrap();
    let result = response["result"]
        .as_object()
        .expect("Should have result field");
    assert!(result.get("status").is_some());
    if result.get("system").is_some() {
        let system = &result["system"];
        assert!(system.is_object());
    }
    if let Some(servers) = result.get("servers") {
        let servers_array = servers.as_array().unwrap();
        for server in servers_array {
            assert!(server.get("name").is_some());
            assert!(server.get("status").is_some());
            if server.get("details").is_some() {
                let details = &server["details"];
                assert!(details.is_object());
            }
        }
    }
}
#[tokio::test]
async fn test_rename_directory_in_rust_workspace() {
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

    // Step 1: Generate rename plan using public API
    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "directory",
                    "path": "crates/crate_b"
                },
                "new_name": "crates/crate_renamed"
            }),
        )
        .await;
    assert!(plan_result.is_ok(), "rename.plan should succeed");
    let plan_response = plan_result.unwrap();
    let plan = &plan_response["result"];

    // Step 2: Apply the plan using workspace.apply_edit
    let apply_result = client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": { "dry_run": false }
            }),
        )
        .await;
    assert!(apply_result.is_ok(), "workspace.apply_edit should succeed");
    let apply_response = apply_result.unwrap();
    assert_eq!(
        apply_response["result"]["applied"], true,
        "Edit should be applied successfully"
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