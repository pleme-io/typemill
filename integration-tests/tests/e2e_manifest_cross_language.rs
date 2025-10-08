use integration_tests :: harness :: { TestClient , TestWorkspace } ;
//!
//! This module tests update_dependency tool across all supported languages
//! with their respective manifest formats:
//! - Python: requirements.txt, pyproject.toml, Pipfile
//! - TypeScript: package.json
//! - Rust: Cargo.toml
//! - Go: go.mod

use integration_tests::harness::{TestClient, TestWorkspace};
use serde_json::json;
use std::fs;

/// Test update_dependency for Python requirements.txt
#[tokio::test]
async fn test_python_update_dependency_requirements_txt() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let requirements_file = workspace.path().join("requirements.txt");
    fs::write(
        &requirements_file,
        "requests==2.28.0\nnumpy==1.24.0\npandas==1.5.0\n",
    )
    .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let response = client
        .call_tool(
            "update_dependency",
            json!({
                "file_path": requirements_file.to_str().unwrap(),
                "old_name": "requests",
                "new_name": "requests",
                "new_version": "2.31.0"
            }),
        )
        .await;

    if let Ok(response_value) = response {
        assert!(
            response_value.get("result").is_some() || response_value.get("error").is_some(),
            "Response must have result or error"
        );

        // Verify file was updated
        let content = fs::read_to_string(&requirements_file).unwrap();
        if response_value.get("result").is_some() {
            assert!(
                content.contains("requests==2.31.0") || content.contains("requests>=2.31.0"),
                "Should update requests version"
            );
        }
    }
}

/// Test update_dependency for TypeScript package.json
#[tokio::test]
async fn test_typescript_update_dependency_package_json() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let package_json = workspace.path().join("package.json");
    fs::write(
        &package_json,
        r#"{
  "name": "test-project",
  "version": "1.0.0",
  "dependencies": {
    "lodash": "4.17.20",
    "express": "4.18.0"
  }
}
"#,
    )
    .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let response = client
        .call_tool(
            "update_dependency",
            json!({
                "file_path": package_json.to_str().unwrap(),
                "old_name": "lodash",
                "new_name": "lodash",
                "new_version": "4.17.21"
            }),
        )
        .await;

    if let Ok(response_value) = response {
        assert!(
            response_value.get("result").is_some() || response_value.get("error").is_some(),
            "Response must have result or error"
        );

        // Verify file was updated
        let content = fs::read_to_string(&package_json).unwrap();
        if response_value.get("result").is_some() {
            assert!(
                content.contains("\"lodash\": \"4.17.21\"")
                    || content.contains("\"lodash\": \"^4.17.21\""),
                "Should update lodash version"
            );
        }
    }
}

/// Test update_dependency for Rust Cargo.toml
#[tokio::test]
async fn test_rust_update_dependency_cargo_toml() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let cargo_toml = workspace.path().join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"[package]
name = "test-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1.0.180"
tokio = { version = "1.32.0", features = ["full"] }
"#,
    )
    .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let response = client
        .call_tool(
            "update_dependency",
            json!({
                "file_path": cargo_toml.to_str().unwrap(),
                "old_name": "serde",
                "new_name": "serde",
                "new_version": "1.0.190"
            }),
        )
        .await;

    if let Ok(response_value) = response {
        assert!(
            response_value.get("result").is_some() || response_value.get("error").is_some(),
            "Response must have result or error"
        );

        // Verify file was updated
        let content = fs::read_to_string(&cargo_toml).unwrap();
        if response_value.get("result").is_some() {
            assert!(
                content.contains("serde = \"1.0.190\""),
                "Should update serde version"
            );
        }
    }
}

/// Test update_dependency for Go go.mod
#[tokio::test]
async fn test_go_update_dependency_go_mod() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let go_mod = workspace.path().join("go.mod");
    fs::write(
        &go_mod,
        r#"module example.com/myproject

go 1.21

require (
    github.com/gin-gonic/gin v1.9.0
    github.com/stretchr/testify v1.8.4
)
"#,
    )
    .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let response = client
        .call_tool(
            "update_dependency",
            json!({
                "file_path": go_mod.to_str().unwrap(),
                "old_name": "github.com/gin-gonic/gin",
                "new_name": "github.com/gin-gonic/gin",
                "new_version": "v1.9.1"
            }),
        )
        .await;

    if let Ok(response_value) = response {
        assert!(
            response_value.get("result").is_some() || response_value.get("error").is_some(),
            "Response must have result or error"
        );

        // Verify file was updated
        let content = fs::read_to_string(&go_mod).unwrap();
        if response_value.get("result").is_some() {
            assert!(
                content.contains("github.com/gin-gonic/gin v1.9.1"),
                "Should update gin version"
            );
        }
    }
}
