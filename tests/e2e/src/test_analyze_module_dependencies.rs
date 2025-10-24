//! analyze.module_dependencies tests migrated to closure-based helpers (v2)
//!
//! BEFORE: 498 lines with manual setup/verification
//! AFTER: Simplified result verification
//!
//! Module dependency analysis tests for Rust crate extraction.

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

/// Helper: Call analyze.module_dependencies
async fn analyze_module_dependencies(
    workspace: &TestWorkspace,
    client: &mut TestClient,
    path: &str,
    kind: &str,
    include_workspace_deps: bool,
) -> serde_json::Value {
    let target_path = workspace.absolute_path(path);
    let response = client
        .call_tool(
            "analyze.module_dependencies",
            json!({
                "target": {
                    "kind": kind,
                    "path": target_path.to_string_lossy()
                },
                "options": {
                    "includeWorkspaceDeps": include_workspace_deps,
                    "resolve_features": true
                }
            }),
        )
        .await
        .expect("analyze.module_dependencies call should succeed");

    response
        .get("result")
        .expect("Response should have result field")
        .clone()
}

#[tokio::test]
async fn test_analyze_single_file_dependencies() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Rust file with various import types
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

    let result = analyze_module_dependencies(&workspace, &mut client, "service.rs", "file", true).await;

    // Verify external dependencies
    let external_deps = result["externalDependencies"].as_object().unwrap();
    assert!(external_deps.contains_key("tokio"));
    assert!(external_deps.contains_key("serde"));
    assert!(external_deps.contains_key("anyhow"));

    // Verify std dependencies
    let std_deps = result["stdDependencies"].as_array().unwrap();
    assert!(std_deps.iter().any(|v| v.as_str() == Some("std")));

    // Verify import analysis
    assert!(result["importAnalysis"]["totalImports"].as_u64().unwrap() > 0);
    assert!(result["importAnalysis"]["externalCrates"].as_u64().unwrap() >= 3);

    // Verify files analyzed
    let files = result["filesAnalyzed"].as_array().unwrap();
    assert_eq!(files.len(), 1);
}

#[tokio::test]
async fn test_analyze_workspace_internal_dependencies() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create workspace structure
    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = ["crate-a", "crate-b"]

[workspace.dependencies]
tokio = "1.0"
"#,
    );

    workspace.create_file("crate-a/Cargo.toml", r#"[package]
name = "crate-a"
version = "0.1.0"
edition = "2021"
"#);
    workspace.create_file("crate-a/src/lib.rs", "pub struct ServiceA;");

    workspace.create_file("crate-b/Cargo.toml", r#"[package]
name = "crate-b"
version = "0.1.0"
edition = "2021"

[dependencies]
crate-a = { path = "../crate-a" }
tokio = { workspace = true }
"#);

    workspace.create_file("crate-b/src/lib.rs", r#"
use crate_a::ServiceA;
use tokio::runtime::Runtime;

pub struct ServiceB {
    service_a: ServiceA,
    runtime: Runtime,
}
"#);

    let result = analyze_module_dependencies(&workspace, &mut client, "crate-b/src/lib.rs", "file", true).await;

    // Verify workspace dependencies detected
    let workspace_deps = result["workspaceDependencies"].as_array().unwrap();
    let workspace_dep_names: Vec<&str> = workspace_deps.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        workspace_dep_names.contains(&"crate_a") || workspace_dep_names.contains(&"crate-a"),
        "Should detect internal workspace dependency, got: {:?}",
        workspace_dep_names
    );

    // Verify external dependencies don't include workspace crates
    let external_deps = result["externalDependencies"].as_object().unwrap();
    assert!(!external_deps.contains_key("crate_a") && !external_deps.contains_key("crate-a"));
    assert!(external_deps.contains_key("tokio"));
}

#[tokio::test]
async fn test_analyze_std_only_dependencies() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // File with only std imports
    let code = r#"
use std::collections::HashMap;
use std::sync::Arc;
use core::fmt::Display;

pub fn process(data: HashMap<String, Arc<dyn Display>>) {
    // Implementation
}
"#;
    workspace.create_file("std_only.rs", code);

    let result = analyze_module_dependencies(&workspace, &mut client, "std_only.rs", "file", true).await;

    // Verify std dependencies
    let std_deps = result["stdDependencies"].as_array().unwrap();
    let std_dep_names: Vec<&str> = std_deps.iter().filter_map(|v| v.as_str()).collect();
    assert!(std_dep_names.contains(&"std"));
    assert!(std_dep_names.contains(&"core"));

    // Verify no external dependencies
    let external_deps = result["externalDependencies"].as_object().unwrap();
    assert_eq!(external_deps.len(), 0);
}

#[tokio::test]
async fn test_analyze_directory_dependencies() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create multiple files in directory
    workspace.create_file("auth/mod.rs", "pub mod jwt;\npub mod session;");
    workspace.create_file("auth/jwt.rs", r#"
use jsonwebtoken::{encode, decode};
use serde::{Serialize, Deserialize};

pub fn create_token() -> String { String::new() }
"#);
    workspace.create_file("auth/session.rs", r#"
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SessionStore {
    data: Arc<RwLock<()>>,
}
"#);

    let result = analyze_module_dependencies(&workspace, &mut client, "auth", "directory", true).await;

    // Verify multiple files analyzed
    let files = result["filesAnalyzed"].as_array().unwrap();
    assert!(files.len() >= 3);

    // Verify external dependencies from both files
    let external_deps = result["externalDependencies"].as_object().unwrap();
    assert!(external_deps.contains_key("jsonwebtoken"));
    assert!(external_deps.contains_key("tokio"));

    // Verify import analysis
    assert!(result["importAnalysis"]["totalImports"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn test_error_nonexistent_path() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let error = client
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
        .expect_err("Should return an error");

    let error_msg = error.to_string();
    assert!(
        error_msg.contains("does not exist") || error_msg.contains("not found"),
        "Should return error for nonexistent path: {}",
        error_msg
    );
}

#[tokio::test]
async fn test_exclude_workspace_deps_option() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create workspace with internal and external deps
    workspace.create_file("Cargo.toml", "[workspace]\nmembers = [\"lib-a\", \"lib-b\"]");
    workspace.create_file("lib-a/Cargo.toml", "[package]\nname = \"lib-a\"\nversion = \"0.1.0\"\nedition = \"2021\"");
    workspace.create_file("lib-a/src/lib.rs", "pub struct A;");

    workspace.create_file("lib-b/Cargo.toml", r#"[package]
name = "lib-b"
version = "0.1.0"
edition = "2021"

[dependencies]
lib-a = { path = "../lib-a" }
serde = "1.0"
"#);

    workspace.create_file("lib-b/src/lib.rs", r#"
use lib_a::A;
use serde::Serialize;
use std::collections::HashMap;

pub struct B {
    a: A,
}
"#);

    let result = analyze_module_dependencies(&workspace, &mut client, "lib-b/src/lib.rs", "file", false).await;

    // Verify workspace dependencies excluded
    let workspace_deps = result["workspaceDependencies"].as_array().unwrap();
    assert_eq!(workspace_deps.len(), 0);

    // Verify std dependencies also excluded
    let std_deps = result["stdDependencies"].as_array().unwrap();
    assert_eq!(std_deps.len(), 0);

    // Verify external dependencies still present
    let external_deps = result["externalDependencies"].as_object().unwrap();
    assert!(external_deps.len() > 0);
}
