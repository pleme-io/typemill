//! End-to-end tests for analyze.module_dependencies tool
//!
//! Tests module dependency analysis functionality including:
//! - Single file analysis
//! - Directory analysis
//! - External vs internal dependency classification
//! - Standard library detection
//! - Workspace dependency resolution

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

#[tokio::test]
async fn test_analyze_single_file_dependencies() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a Rust file with various import types
    let code = r#"
use std::collections::HashMap;
use tokio::runtime::Runtime;
use serde::{Serialize, Deserialize};
use anyhow::Result;

pub struct Service {
    runtime: Runtime,
    data: HashMap<String, String>,
}
"#;

    workspace.create_file("service.rs", code);
    let test_file = workspace.absolute_path("service.rs");

    let response = client
        .call_tool(
            "analyze.module_dependencies",
            json!({
                "target": {
                    "kind": "file",
                    "path": test_file.to_string_lossy()
                },
                "options": {
                    "include_workspace_deps": true,
                    "resolve_features": true
                }
            }),
        )
        .await
        .expect("analyze.module_dependencies call should succeed");

    let result = response
        .get("result")
        .expect("Response should have result field");

    // Verify external dependencies
    let external_deps = result["external_dependencies"]
        .as_object()
        .expect("Should have external_dependencies");

    assert!(
        external_deps.contains_key("tokio"),
        "Should detect tokio dependency"
    );
    assert!(
        external_deps.contains_key("serde"),
        "Should detect serde dependency"
    );
    assert!(
        external_deps.contains_key("anyhow"),
        "Should detect anyhow dependency"
    );

    // Verify std dependencies
    let std_deps = result["std_dependencies"]
        .as_array()
        .expect("Should have std_dependencies");

    assert!(
        std_deps.iter().any(|v| v.as_str() == Some("std")),
        "Should detect std library usage"
    );

    // Verify import analysis
    let import_analysis = &result["import_analysis"];
    assert!(
        import_analysis["total_imports"].as_u64().unwrap() > 0,
        "Should count imports"
    );
    assert!(
        import_analysis["external_crates"].as_u64().unwrap() >= 3,
        "Should count external crates"
    );

    // Verify files analyzed
    let files = result["files_analyzed"]
        .as_array()
        .expect("Should have files_analyzed");
    assert_eq!(files.len(), 1, "Should analyze exactly one file");
}

#[tokio::test]
async fn test_analyze_workspace_internal_dependencies() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create workspace structure with internal dependencies
    workspace.create_file(
        "Cargo.toml",
        r#"
[workspace]
members = ["crate-a", "crate-b"]

[workspace.dependencies]
tokio = "1.0"
"#,
    );

    workspace.create_file(
        "crate-a/Cargo.toml",
        r#"
[package]
name = "crate-a"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
    );

    workspace.create_file(
        "crate-a/src/lib.rs",
        r#"
pub struct ServiceA;
"#,
    );

    workspace.create_file(
        "crate-b/Cargo.toml",
        r#"
[package]
name = "crate-b"
version = "0.1.0"
edition = "2021"

[dependencies]
crate-a = { path = "../crate-a" }
tokio = { workspace = true }
"#,
    );

    let code = r#"
use crate_a::ServiceA;
use tokio::runtime::Runtime;

pub struct ServiceB {
    service_a: ServiceA,
    runtime: Runtime,
}
"#;

    workspace.create_file("crate-b/src/lib.rs", code);
    let test_file = workspace.absolute_path("crate-b/src/lib.rs");

    let response = client
        .call_tool(
            "analyze.module_dependencies",
            json!({
                "target": {
                    "kind": "file",
                    "path": test_file.to_string_lossy()
                },
                "options": {
                    "include_workspace_deps": true
                }
            }),
        )
        .await
        .expect("analyze.module_dependencies call should succeed");

    let result = response
        .get("result")
        .expect("Response should have result field");

    // Verify workspace dependencies detected
    let workspace_deps = result["workspace_dependencies"]
        .as_array()
        .expect("Should have workspace_dependencies");

    let workspace_dep_names: Vec<&str> = workspace_deps.iter().filter_map(|v| v.as_str()).collect();

    assert!(
        workspace_dep_names.contains(&"crate_a") || workspace_dep_names.contains(&"crate-a"),
        "Should detect internal workspace dependency, got: {:?}",
        workspace_dep_names
    );

    // Verify external dependencies don't include workspace crates
    let external_deps = result["external_dependencies"]
        .as_object()
        .expect("Should have external_dependencies");

    assert!(
        !external_deps.contains_key("crate_a") && !external_deps.contains_key("crate-a"),
        "Workspace crate should not be in external dependencies"
    );

    // Verify tokio is in external dependencies
    assert!(
        external_deps.contains_key("tokio"),
        "Should detect tokio as external dependency"
    );
}

#[tokio::test]
async fn test_analyze_std_only_dependencies() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a file with only std imports
    let code = r#"
use std::collections::HashMap;
use std::sync::Arc;
use core::fmt::Display;

pub fn process(data: HashMap<String, Arc<dyn Display>>) {
    // Implementation
}
"#;

    workspace.create_file("std_only.rs", code);
    let test_file = workspace.absolute_path("std_only.rs");

    let response = client
        .call_tool(
            "analyze.module_dependencies",
            json!({
                "target": {
                    "kind": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.module_dependencies call should succeed");

    let result = response
        .get("result")
        .expect("Response should have result field");

    // Verify std dependencies detected
    let std_deps = result["std_dependencies"]
        .as_array()
        .expect("Should have std_dependencies");

    let std_dep_names: Vec<&str> = std_deps.iter().filter_map(|v| v.as_str()).collect();

    assert!(
        std_dep_names.contains(&"std"),
        "Should detect std library usage"
    );
    assert!(
        std_dep_names.contains(&"core"),
        "Should detect core library usage"
    );

    // Verify no external dependencies
    let external_deps = result["external_dependencies"]
        .as_object()
        .expect("Should have external_dependencies");

    assert_eq!(
        external_deps.len(),
        0,
        "Should have no external dependencies"
    );
}

#[tokio::test]
async fn test_analyze_directory_dependencies() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create multiple files in a directory
    workspace.create_file(
        "auth/mod.rs",
        r#"
pub mod jwt;
pub mod session;
"#,
    );

    workspace.create_file(
        "auth/jwt.rs",
        r#"
use jsonwebtoken::{encode, decode};
use serde::{Serialize, Deserialize};

pub fn create_token() -> String {
    String::new()
}
"#,
    );

    workspace.create_file(
        "auth/session.rs",
        r#"
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SessionStore {
    data: Arc<RwLock<()>>,
}
"#,
    );

    let auth_dir = workspace.absolute_path("auth");

    let response = client
        .call_tool(
            "analyze.module_dependencies",
            json!({
                "target": {
                    "kind": "directory",
                    "path": auth_dir.to_string_lossy()
                },
                "options": {
                    "include_workspace_deps": true
                }
            }),
        )
        .await
        .expect("analyze.module_dependencies call should succeed");

    let result = response
        .get("result")
        .expect("Response should have result field");

    // Verify multiple files analyzed
    let files = result["files_analyzed"]
        .as_array()
        .expect("Should have files_analyzed");

    assert!(
        files.len() >= 3,
        "Should analyze all .rs files in directory"
    );

    // Verify external dependencies from both files
    let external_deps = result["external_dependencies"]
        .as_object()
        .expect("Should have external_dependencies");

    assert!(
        external_deps.contains_key("jsonwebtoken"),
        "Should detect jsonwebtoken from jwt.rs"
    );
    assert!(
        external_deps.contains_key("tokio"),
        "Should detect tokio from session.rs"
    );

    // Verify import analysis counts
    let import_analysis = &result["import_analysis"];
    assert!(
        import_analysis["total_imports"].as_u64().unwrap() > 0,
        "Should count imports from all files"
    );
}

#[tokio::test]
async fn test_error_nonexistent_path() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let result = client
        .call_tool(
            "analyze.module_dependencies",
            json!({
                "target": {
                    "kind": "file",
                    "path": "/nonexistent/path/file.rs"
                }
            }),
        )
        .await
        .expect("call_tool should return a response");

    // Should have an error in the JSON-RPC response
    let has_error = result.get("error").is_some();
    assert!(has_error, "Should return error for nonexistent path");
}

#[tokio::test]
async fn test_exclude_workspace_deps_option() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create workspace with internal and external deps
    workspace.create_file(
        "Cargo.toml",
        r#"
[workspace]
members = ["lib-a", "lib-b"]
"#,
    );

    workspace.create_file(
        "lib-a/Cargo.toml",
        r#"
[package]
name = "lib-a"
version = "0.1.0"
edition = "2021"
"#,
    );

    workspace.create_file("lib-a/src/lib.rs", "pub struct A;");

    workspace.create_file(
        "lib-b/Cargo.toml",
        r#"
[package]
name = "lib-b"
version = "0.1.0"
edition = "2021"

[dependencies]
lib-a = { path = "../lib-a" }
serde = "1.0"
"#,
    );

    let code = r#"
use lib_a::A;
use serde::Serialize;
use std::collections::HashMap;

pub struct B {
    a: A,
}
"#;

    workspace.create_file("lib-b/src/lib.rs", code);
    let test_file = workspace.absolute_path("lib-b/src/lib.rs");

    let response = client
        .call_tool(
            "analyze.module_dependencies",
            json!({
                "target": {
                    "kind": "file",
                    "path": test_file.to_string_lossy()
                },
                "options": {
                    "include_workspace_deps": false
                }
            }),
        )
        .await
        .expect("analyze.module_dependencies call should succeed");

    let result = response
        .get("result")
        .expect("Response should have result field");

    // Verify workspace dependencies are excluded
    let workspace_deps = result["workspace_dependencies"]
        .as_array()
        .expect("Should have workspace_dependencies");

    assert_eq!(
        workspace_deps.len(),
        0,
        "Workspace dependencies should be excluded"
    );

    // Verify std dependencies also excluded
    let std_deps = result["std_dependencies"]
        .as_array()
        .expect("Should have std_dependencies");

    assert_eq!(std_deps.len(), 0, "Std dependencies should be excluded");

    // Verify external dependencies still present
    let external_deps = result["external_dependencies"]
        .as_object()
        .expect("Should have external_dependencies");

    assert!(
        external_deps.len() > 0,
        "External dependencies should still be included"
    );
}
