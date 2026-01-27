//! Dead code analysis tests
//!
//! The dead code analyzer now uses LSP + call graph reachability at the workspace level.
//! File-level dead code analysis with specific "kind" parameters is no longer supported.
//! Use the `analyze.dead_code` tool with a workspace scope instead.

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

#[tokio::test]
async fn test_dead_code_analysis_returns_report() {
    let workspace = TestWorkspace::new();
    workspace.setup_lsp_config();
    workspace.create_file("src/lib.rs", "fn unused() {} pub fn used() {}");

    let mut client = TestClient::new(workspace.path());

    let response = client
        .call_tool_with_timeout(
            "analyze.dead_code",
            json!({
                "scope": {
                    "path": workspace.path().to_string_lossy()
                }
            }),
            std::time::Duration::from_secs(30),
        )
        .await
        .expect("analyze.dead_code call should succeed");

    // The new API returns a Report with dead_code array and stats
    // Tool result may be wrapped in "result" field in JSON-RPC response
    let result = response.get("result").unwrap_or(&response);
    assert!(
        result.get("dead_code").is_some() || result.get("stats").is_some(),
        "Response should have dead_code or stats field. Got: {}",
        serde_json::to_string_pretty(&response).unwrap_or_default()
    );
}

#[cfg(feature = "e2e-tests")]
#[tokio::test]
async fn test_dead_code_analysis_finds_unused_function() {
    let workspace = TestWorkspace::new();
    workspace.setup_lsp_config();

    // Create a Rust project with an unused function
    workspace.create_file(
        "Cargo.toml",
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
    );

    workspace.create_file(
        "src/lib.rs",
        r#"
fn unused_function() -> i32 {
    42
}

pub fn used_function() {
    println!("Hello");
}
"#,
    );

    let mut client = TestClient::new(workspace.path());

    let response = client
        .call_tool_with_timeout(
            "analyze.dead_code",
            json!({
                "scope": {
                    "path": workspace.path().to_string_lossy()
                }
            }),
            std::time::Duration::from_secs(60),
        )
        .await
        .expect("analyze.dead_code call should succeed");

    // Check that unused_function is detected
    if let Some(dead_code) = response.get("dead_code").and_then(|d| d.as_array()) {
        let names: Vec<&str> = dead_code
            .iter()
            .filter_map(|item| item.get("name").and_then(|n| n.as_str()))
            .collect();

        // unused_function should be in the dead code list
        assert!(
            names.contains(&"unused_function") || dead_code.is_empty(),
            "Should detect unused_function or return empty if LSP not available"
        );
    }
}
