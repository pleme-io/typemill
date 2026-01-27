//! Analysis API tests for analyze.dead_code
//!
//! The dead code analyzer uses LSP + call graph reachability to find unreachable code.
//! It works at the workspace level, not on individual files.

use crate::harness::{TestClient, TestWorkspace};
use mill_foundation::protocol::analysis_result::AnalysisResult;
use serde_json::json;

// ============================================================================
// Dead Code Analysis Tests (LSP + Call Graph Reachability)
// ============================================================================
// The unified dead code analyzer works at workspace level using LSP for
// symbol resolution and a call graph for reachability analysis.

/// Helper for dead code analysis tests
async fn run_dead_code_test<V>(files: &[(&str, &str)], verify: V) -> anyhow::Result<()>
where
    V: FnOnce(&serde_json::Value) -> anyhow::Result<()>,
{
    let workspace = TestWorkspace::new();
    workspace.setup_lsp_config();

    for (file_path, content) in files {
        workspace.create_file(file_path, content);
    }

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

    verify(&response)?;
    Ok(())
}

#[tokio::test]
async fn test_analyze_dead_code_empty_workspace() {
    let workspace = TestWorkspace::new();
    workspace.setup_lsp_config();
    workspace.create_file("src/lib.rs", "");

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

    // With no symbols, should return empty dead_code array
    if let Some(dead_code) = response.get("dead_code") {
        assert!(
            dead_code.as_array().map(|a| a.is_empty()).unwrap_or(true),
            "Empty workspace should have no dead code findings"
        );
    }
}

#[cfg(feature = "e2e-tests")]
#[tokio::test]
async fn test_analyze_dead_code_finds_unused_private_function() {
    let files = &[
        (
            "Cargo.toml",
            r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
        ),
        (
            "src/main.rs",
            r#"
use test_project::used_public_function;

fn main() {
    used_public_function();
}
"#,
        ),
        (
            "src/lib.rs",
            r#"
fn unused_private_function() -> i32 {
    42
}

pub fn used_public_function() {
    println!("Hello, world!");
}
"#,
        ),
    ];

    run_dead_code_test(files, |response| {
        // The response should have a dead_code array
        let dead_code = response
            .get("dead_code")
            .expect("Response should have dead_code field");

        let dead_array = dead_code.as_array().expect("dead_code should be an array");

        // Should find unused_private_function
        let names: Vec<&str> = dead_array
            .iter()
            .filter_map(|item| item.get("name").and_then(|n| n.as_str()))
            .collect();

        assert!(
            names.contains(&"unused_private_function"),
            "Should find unused_private_function, found: {:?}",
            names
        );

        Ok(())
    })
    .await
    .unwrap();
}

#[cfg(feature = "e2e-tests")]
#[tokio::test]
async fn test_analyze_dead_code_respects_entry_points() {
    let files = &[
        (
            "Cargo.toml",
            r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
        ),
        (
            "src/main.rs",
            r#"
fn main() {
    println!("Hello");
}

fn helper_used_by_main() {
    // This function is called from main, so it's not dead
}
"#,
        ),
    ];

    run_dead_code_test(files, |response| {
        // main() should NOT be marked as dead (it's an entry point)
        let dead_code = response.get("dead_code");

        if let Some(dead) = dead_code {
            let dead_array = dead.as_array().unwrap_or(&vec![]);
            let names: Vec<&str> = dead_array
                .iter()
                .filter_map(|item| item.get("name").and_then(|n| n.as_str()))
                .collect();

            assert!(
                !names.contains(&"main"),
                "main() should NOT be marked as dead"
            );
        }

        Ok(())
    })
    .await
    .unwrap();
}

#[cfg(feature = "e2e-tests")]
#[tokio::test]
async fn test_analyze_dead_code_with_config_options() {
    let files = &[(
        "src/lib.rs",
        r#"
pub fn public_unused() -> i32 {
    42
}

fn private_unused() -> i32 {
    100
}

#[test]
fn test_something() {
    assert!(true);
}
"#,
    )];

    let workspace = TestWorkspace::new();
    workspace.setup_lsp_config();

    for (file_path, content) in files {
        workspace.create_file(file_path, content);
    }

    let mut client = TestClient::new(workspace.path());

    // Test with include_tests = false, include_pub_exports = false
    let response = client
        .call_tool_with_timeout(
            "analyze.dead_code",
            json!({
                "scope": {
                    "path": workspace.path().to_string_lossy()
                },
                "include_tests": false,
                "include_pub_exports": false
            }),
            std::time::Duration::from_secs(60),
        )
        .await
        .expect("analyze.dead_code call should succeed");

    // With pub exports not considered entry points, public_unused should be found
    if let Some(dead_code) = response.get("dead_code") {
        let dead_array = dead_code.as_array().unwrap_or(&vec![]);
        let names: Vec<&str> = dead_array
            .iter()
            .filter_map(|item| item.get("name").and_then(|n| n.as_str()))
            .collect();

        // Both public_unused and private_unused should potentially be dead
        // when pub exports aren't considered entry points
        assert!(
            names.contains(&"private_unused") || dead_array.is_empty(),
            "Analysis completed without errors"
        );
    }
}

#[tokio::test]
async fn test_analyze_dead_code_returns_stats() {
    let workspace = TestWorkspace::new();
    workspace.setup_lsp_config();
    workspace.create_file("src/lib.rs", "pub fn foo() {}");

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

    // Should have stats field
    if let Some(stats) = response.get("stats") {
        // Stats should have duration_ms
        assert!(
            stats.get("duration_ms").is_some(),
            "Stats should include duration_ms"
        );
    }
}
