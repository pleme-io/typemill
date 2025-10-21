use crate::harness::{TestClient, TestWorkspace};
use codebuddy_foundation::protocol::analysis_result::AnalysisResult;
use serde_json::json;

#[cfg(feature = "e2e-tests")]
#[tokio::test]
async fn test_analyze_deep_dead_code_default_mode() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a Rust project with a mix of used and unused symbols
    workspace.create_file(
        "Cargo.toml",
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
    );
    workspace.create_file(
        "src/main.rs",
        r#"
use test_project::used_public_function;

fn main() {
    used_public_function();
}
"#,
    );
    workspace.create_file(
        "src/lib.rs",
        r#"
fn unused_private_function() -> i32 {
    42
}

pub fn used_public_function() {
    println!("Hello, world!");
}

pub fn unused_public_function() {
    println!("This should be ignored by default");
}
"#,
    );

    let response = client
        .call_tool_with_timeout(
            "analyze.dead_code",
            json!({
                "kind": "deep",
                "scope": {
                    "scope_type": "workspace",
                    "path": workspace.path().to_string_lossy()
                }
            }),
            std::time::Duration::from_secs(60),
        )
        .await
        .expect("analyze.dead_code call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    assert_eq!(result.metadata.kind, "deep");
    assert_eq!(result.findings.len(), 1);
    assert_eq!(
        result.findings[0].location.symbol,
        Some("unused_private_function".to_string())
    );
}

#[cfg(feature = "e2e-tests")]
#[tokio::test]
async fn test_analyze_deep_dead_code_aggressive_mode() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create the same Rust project
    workspace.create_file(
        "Cargo.toml",
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
    );
    workspace.create_file(
        "src/main.rs",
        r#"
use test_project::used_public_function;

fn main() {
    used_public_function();
}
"#,
    );
    workspace.create_file(
        "src/lib.rs",
        r#"
fn unused_private_function() -> i32 {
    42
}

pub fn used_public_function() {
    println!("Hello, world!");
}

pub fn unused_public_function() {
    println!("This should be detected in aggressive mode");
}
"#,
    );

    let response = client
        .call_tool_with_timeout(
            "analyze.dead_code",
            json!({
                "kind": "deep",
                "scope": {
                    "scope_type": "workspace",
                    "path": workspace.path().to_string_lossy()
                },
                "check_public_exports": true
            }),
            std::time::Duration::from_secs(60),
        )
        .await
        .expect("analyze.dead_code call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    assert_eq!(result.metadata.kind, "deep");
    assert_eq!(result.findings.len(), 2);
    let symbols: Vec<String> = result
        .findings
        .iter()
        .map(|f| f.location.symbol.clone().unwrap())
        .collect();
    assert!(symbols.contains(&"unused_private_function".to_string()));
    assert!(symbols.contains(&"unused_public_function".to_string()));
}
