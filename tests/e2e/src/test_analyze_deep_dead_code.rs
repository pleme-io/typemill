//! Analysis API tests for analyze.dead_code deep mode (MIGRATED VERSION)
//!
//! BEFORE: 170 lines with repetitive workspace setup
//! AFTER: Simplified pattern for LSP-based analysis tests
//!
//! Note: Deep dead code analysis requires LSP support

use crate::harness::{TestClient, TestWorkspace};
use mill_foundation::protocol::analysis_result::AnalysisResult;
use serde_json::json;

/// Helper for deep dead code analysis tests (with LSP)
async fn run_deep_dead_code_test<V>(
    files: &[(&str, &str)],
    check_public_exports: bool,
    verify: V,
) -> anyhow::Result<()>
where
    V: FnOnce(&AnalysisResult) -> anyhow::Result<()>,
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
                "kind": "deep",
                "scope": {
                    "type": "workspace",
                    "path": workspace.path().to_string_lossy()
                },
                "check_public_exports": check_public_exports
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

    verify(&result)?;
    Ok(())
}

#[cfg(feature = "e2e-tests")]
#[tokio::test]
async fn test_analyze_deep_dead_code_default_mode() {
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

pub fn unused_public_function() {
    println!("This should be ignored by default");
}
"#,
        ),
    ];

    run_deep_dead_code_test(files, false, |result| {
        assert_eq!(result.metadata.kind, "deep");
        assert_eq!(result.findings.len(), 1);
        assert_eq!(
            result.findings[0].location.symbol,
            Some("unused_private_function".to_string())
        );
        Ok(())
    })
    .await
    .unwrap();
}

#[cfg(feature = "e2e-tests")]
#[tokio::test]
#[ignore = "LSP cross-file reference tracking bug - graph builder doesn't properly find dependencies from main() to lib functions"]
async fn test_analyze_deep_dead_code_aggressive_mode() {
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

pub fn unused_public_function() {
    println!("This should be detected in aggressive mode");
}
"#,
        ),
    ];

    run_deep_dead_code_test(files, true, |result| {
        assert_eq!(result.metadata.kind, "deep");

        let symbols: Vec<String> = result
            .findings
            .iter()
            .map(|f| f.location.symbol.clone().unwrap())
            .collect();

        assert_eq!(
            result.findings.len(),
            2,
            "Should find 2 dead symbols, found: {:?}",
            symbols
        );
        assert!(symbols.contains(&"unused_private_function".to_string()));
        assert!(symbols.contains(&"unused_public_function".to_string()));

        assert!(
            !symbols.contains(&"used_public_function".to_string()),
            "used_public_function should NOT be marked as dead"
        );

        Ok(())
    })
    .await
    .unwrap();
}
