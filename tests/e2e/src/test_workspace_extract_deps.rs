//! Workspace dependency extraction integration tests
//!
//! Tests for workspace.extract_dependencies tool (Proposal 50: Crate Extraction Tooling)
//!
//! Tests:
//! - Basic dependency extraction
//! - Dev-dependencies extraction
//! - Build-dependencies extraction
//! - Feature extraction
//! - Conflict detection (already exists)
//! - Filtering specific dependencies
//! - Workspace dependencies ({ workspace = true })
//! - Path dependencies
//! - Git dependencies
//! - Dry-run mode

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

#[tokio::test]
async fn test_extract_basic_dependencies() {
    // Test extracting basic dependencies from source to target
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create source Cargo.toml
    workspace.create_file(
        "source/Cargo.toml",
        r#"[package]
name = "source-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
"#,
    );

    // Create target Cargo.toml (empty dependencies)
    workspace.create_file(
        "target/Cargo.toml",
        r#"[package]
name = "target-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
    );

    let source_path = workspace.absolute_path("source/Cargo.toml");
    let target_path = workspace.absolute_path("target/Cargo.toml");

    // Extract specific dependencies
    let result = client
        .call_tool(
            "workspace.extract_dependencies",
            json!({
                "source_manifest": source_path.to_string_lossy(),
                "target_manifest": target_path.to_string_lossy(),
                "dependencies": ["tokio", "serde"],
                "options": {
                    "dry_run": false,
                    "preserve_versions": true,
                    "preserve_features": true,
                    "section": "dependencies"
                }
            }),
        )
        .await
        .expect("workspace.extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");

    // Verify extraction succeeded
    assert_eq!(
        content
            .get("dependencies_extracted")
            .and_then(|v| v.as_u64()),
        Some(2),
        "Should extract 2 dependencies"
    );

    assert_eq!(
        content
            .get("target_manifest_updated")
            .and_then(|v| v.as_bool()),
        Some(true),
        "Target manifest should be updated"
    );

    // Verify target Cargo.toml was updated
    let target_content = workspace.read_file("target/Cargo.toml");
    assert!(
        target_content.contains("tokio"),
        "Target should contain tokio"
    );
    assert!(
        target_content.contains("serde"),
        "Target should contain serde"
    );
    assert!(
        !target_content.contains("anyhow"),
        "Target should not contain anyhow (not requested)"
    );

    // Verify features were preserved
    assert!(
        target_content.contains("features = [\"full\"]"),
        "Tokio features should be preserved"
    );
    assert!(
        target_content.contains("features = [\"derive\"]"),
        "Serde features should be preserved"
    );
}

#[tokio::test]
async fn test_extract_dev_dependencies() {
    // Test extracting dev-dependencies
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create source with dev-dependencies
    workspace.create_file(
        "source/Cargo.toml",
        r#"[package]
name = "source-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = "1.0"

[dev-dependencies]
tempfile = "3.0"
criterion = "0.5"
"#,
    );

    // Create target
    workspace.create_file(
        "target/Cargo.toml",
        r#"[package]
name = "target-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
    );

    let source_path = workspace.absolute_path("source/Cargo.toml");
    let target_path = workspace.absolute_path("target/Cargo.toml");

    // Extract dev-dependency
    let result = client
        .call_tool(
            "workspace.extract_dependencies",
            json!({
                "source_manifest": source_path.to_string_lossy(),
                "target_manifest": target_path.to_string_lossy(),
                "dependencies": ["tempfile"],
                "options": {
                    "dry_run": false,
                    "section": "dev-dependencies"
                }
            }),
        )
        .await
        .expect("workspace.extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");
    assert_eq!(
        content
            .get("dependencies_extracted")
            .and_then(|v| v.as_u64()),
        Some(1)
    );

    // Verify target has dev-dependencies section
    let target_content = workspace.read_file("target/Cargo.toml");
    assert!(
        target_content.contains("[dev-dependencies]"),
        "Target should have dev-dependencies section"
    );
    assert!(
        target_content.contains("tempfile"),
        "Target should contain tempfile"
    );
}

#[tokio::test]
async fn test_extract_conflict_detection() {
    // Test that existing dependencies are detected and skipped
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create source
    workspace.create_file(
        "source/Cargo.toml",
        r#"[package]
name = "source-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = "1.0"
"#,
    );

    // Create target with existing serde (different version)
    workspace.create_file(
        "target/Cargo.toml",
        r#"[package]
name = "target-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "0.9"
"#,
    );

    let source_path = workspace.absolute_path("source/Cargo.toml");
    let target_path = workspace.absolute_path("target/Cargo.toml");

    // Try to extract both
    let result = client
        .call_tool(
            "workspace.extract_dependencies",
            json!({
                "source_manifest": source_path.to_string_lossy(),
                "target_manifest": target_path.to_string_lossy(),
                "dependencies": ["tokio", "serde"],
                "options": {
                    "dry_run": false
                }
            }),
        )
        .await
        .expect("workspace.extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");

    // Verify warnings about existing dependency
    let warnings = content.get("warnings").and_then(|v| v.as_array());
    assert!(warnings.is_some(), "Should have warnings");
    let warnings = warnings.unwrap();
    assert!(
        warnings
            .iter()
            .any(|w| w.as_str().unwrap().contains("serde")
                && w.as_str().unwrap().contains("already exists")),
        "Should warn about serde already existing"
    );

    // Verify dependencies_added includes both (with already_exists flag for serde)
    let deps_added = content
        .get("dependencies_added")
        .and_then(|v| v.as_array())
        .expect("Should have dependencies_added");

    assert_eq!(deps_added.len(), 2, "Should report both dependencies");

    let serde_info = deps_added
        .iter()
        .find(|d| d.get("name").and_then(|n| n.as_str()) == Some("serde"))
        .expect("Should have serde info");

    assert_eq!(
        serde_info.get("already_exists").and_then(|v| v.as_bool()),
        Some(true),
        "Serde should be marked as already_exists"
    );

    // Verify target still has old serde version (not overwritten)
    let target_content = workspace.read_file("target/Cargo.toml");
    assert!(
        target_content.contains("serde = \"0.9\""),
        "Original serde version should be preserved"
    );
    assert!(target_content.contains("tokio"), "Tokio should be added");
}

#[tokio::test]
async fn test_extract_workspace_dependencies() {
    // Test extracting workspace dependencies ({ workspace = true })
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create source with workspace dependency
    workspace.create_file(
        "source/Cargo.toml",
        r#"[package]
name = "source-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
my-common = { workspace = true }
tokio = "1.0"
"#,
    );

    // Create target
    workspace.create_file(
        "target/Cargo.toml",
        r#"[package]
name = "target-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
    );

    let source_path = workspace.absolute_path("source/Cargo.toml");
    let target_path = workspace.absolute_path("target/Cargo.toml");

    // Extract workspace dependency
    let result = client
        .call_tool(
            "workspace.extract_dependencies",
            json!({
                "source_manifest": source_path.to_string_lossy(),
                "target_manifest": target_path.to_string_lossy(),
                "dependencies": ["my-common"],
                "options": {
                    "dry_run": false
                }
            }),
        )
        .await
        .expect("workspace.extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");
    assert_eq!(
        content
            .get("dependencies_extracted")
            .and_then(|v| v.as_u64()),
        Some(1)
    );

    // Verify workspace dependency was copied
    let target_content = workspace.read_file("target/Cargo.toml");
    assert!(
        target_content.contains("my-common"),
        "Target should contain my-common"
    );
    assert!(
        target_content.contains("workspace = true"),
        "Workspace reference should be preserved"
    );
}

#[tokio::test]
async fn test_extract_path_dependencies() {
    // Test extracting path dependencies
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create source with path dependency
    workspace.create_file(
        "source/Cargo.toml",
        r#"[package]
name = "source-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
my-local = { path = "../my-local" }
"#,
    );

    // Create target
    workspace.create_file(
        "target/Cargo.toml",
        r#"[package]
name = "target-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
    );

    let source_path = workspace.absolute_path("source/Cargo.toml");
    let target_path = workspace.absolute_path("target/Cargo.toml");

    // Extract path dependency
    let result = client
        .call_tool(
            "workspace.extract_dependencies",
            json!({
                "source_manifest": source_path.to_string_lossy(),
                "target_manifest": target_path.to_string_lossy(),
                "dependencies": ["my-local"],
                "options": {
                    "dry_run": false
                }
            }),
        )
        .await
        .expect("workspace.extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");
    assert_eq!(
        content
            .get("dependencies_extracted")
            .and_then(|v| v.as_u64()),
        Some(1)
    );

    // Verify path dependency was copied
    let target_content = workspace.read_file("target/Cargo.toml");
    assert!(
        target_content.contains("my-local"),
        "Target should contain my-local"
    );
    assert!(
        target_content.contains("path = \"../my-local\""),
        "Path should be preserved"
    );
}

#[tokio::test]
async fn test_extract_dependency_not_found() {
    // Test error when dependency not found in source
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create source without requested dependency
    workspace.create_file(
        "source/Cargo.toml",
        r#"[package]
name = "source-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = "1.0"
"#,
    );

    // Create target
    workspace.create_file(
        "target/Cargo.toml",
        r#"[package]
name = "target-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
    );

    let source_path = workspace.absolute_path("source/Cargo.toml");
    let target_path = workspace.absolute_path("target/Cargo.toml");

    // Try to extract non-existent dependency
    let result = client
        .call_tool(
            "workspace.extract_dependencies",
            json!({
                "source_manifest": source_path.to_string_lossy(),
                "target_manifest": target_path.to_string_lossy(),
                "dependencies": ["nonexistent"],
                "options": {
                    "dry_run": false
                }
            }),
        )
        .await
        .expect("workspace.extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");

    // Should succeed but with warning
    assert_eq!(
        content
            .get("dependencies_extracted")
            .and_then(|v| v.as_u64()),
        Some(0),
        "Should extract 0 dependencies"
    );

    let warnings = content.get("warnings").and_then(|v| v.as_array());
    assert!(warnings.is_some(), "Should have warnings");
    let warnings = warnings.unwrap();
    assert!(
        warnings
            .iter()
            .any(|w| w.as_str().unwrap().contains("nonexistent")
                && w.as_str().unwrap().contains("not found")),
        "Should warn about dependency not found"
    );
}

#[tokio::test]
async fn test_extract_dry_run() {
    // Test dry-run mode doesn't modify files
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create source
    workspace.create_file(
        "source/Cargo.toml",
        r#"[package]
name = "source-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = "1.0"
"#,
    );

    // Create target
    workspace.create_file(
        "target/Cargo.toml",
        r#"[package]
name = "target-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
    );

    let source_path = workspace.absolute_path("source/Cargo.toml");
    let target_path = workspace.absolute_path("target/Cargo.toml");
    let original_target_content = workspace.read_file("target/Cargo.toml");

    // Extract with dry_run
    let result = client
        .call_tool(
            "workspace.extract_dependencies",
            json!({
                "source_manifest": source_path.to_string_lossy(),
                "target_manifest": target_path.to_string_lossy(),
                "dependencies": ["tokio"],
                "options": {
                    "dry_run": true
                }
            }),
        )
        .await
        .expect("workspace.extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");

    // Verify dry_run flag in result
    assert_eq!(
        content.get("dry_run").and_then(|v| v.as_bool()),
        Some(true),
        "dry_run should be true"
    );

    // Verify target_manifest_updated is false
    assert_eq!(
        content
            .get("target_manifest_updated")
            .and_then(|v| v.as_bool()),
        Some(false),
        "Target should not be updated in dry_run"
    );

    // Verify dependencies analysis still happened
    assert_eq!(
        content
            .get("dependencies_extracted")
            .and_then(|v| v.as_u64()),
        Some(1),
        "Should still analyze dependencies"
    );

    // Verify file wasn't modified
    let target_content = workspace.read_file("target/Cargo.toml");
    assert_eq!(
        target_content, original_target_content,
        "Target file should not be modified in dry_run"
    );
}

#[tokio::test]
async fn test_extract_optional_dependencies() {
    // Test extracting optional dependencies
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create source with optional dependency
    workspace.create_file(
        "source/Cargo.toml",
        r#"[package]
name = "source-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
feature-dep = { version = "1.0", optional = true }
"#,
    );

    // Create target
    workspace.create_file(
        "target/Cargo.toml",
        r#"[package]
name = "target-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
    );

    let source_path = workspace.absolute_path("source/Cargo.toml");
    let target_path = workspace.absolute_path("target/Cargo.toml");

    // Extract optional dependency
    let result = client
        .call_tool(
            "workspace.extract_dependencies",
            json!({
                "source_manifest": source_path.to_string_lossy(),
                "target_manifest": target_path.to_string_lossy(),
                "dependencies": ["feature-dep"],
                "options": {
                    "dry_run": false
                }
            }),
        )
        .await
        .expect("workspace.extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");
    assert_eq!(
        content
            .get("dependencies_extracted")
            .and_then(|v| v.as_u64()),
        Some(1)
    );

    // Verify optional flag in result
    let deps_added = content
        .get("dependencies_added")
        .and_then(|v| v.as_array())
        .expect("Should have dependencies_added");

    let dep_info = &deps_added[0];
    assert_eq!(
        dep_info.get("optional").and_then(|v| v.as_bool()),
        Some(true),
        "Dependency should be marked as optional"
    );

    // Verify target has optional flag
    let target_content = workspace.read_file("target/Cargo.toml");
    assert!(
        target_content.contains("optional = true"),
        "Optional flag should be preserved"
    );
}

#[tokio::test]
async fn test_extract_build_dependencies() {
    // Test extracting build-dependencies
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create source with build-dependencies
    workspace.create_file(
        "source/Cargo.toml",
        r#"[package]
name = "source-crate"
version = "0.1.0"
edition = "2021"

[build-dependencies]
cc = "1.0"
"#,
    );

    // Create target
    workspace.create_file(
        "target/Cargo.toml",
        r#"[package]
name = "target-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
    );

    let source_path = workspace.absolute_path("source/Cargo.toml");
    let target_path = workspace.absolute_path("target/Cargo.toml");

    // Extract build-dependency
    let result = client
        .call_tool(
            "workspace.extract_dependencies",
            json!({
                "source_manifest": source_path.to_string_lossy(),
                "target_manifest": target_path.to_string_lossy(),
                "dependencies": ["cc"],
                "options": {
                    "dry_run": false,
                    "section": "build-dependencies"
                }
            }),
        )
        .await
        .expect("workspace.extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");
    assert_eq!(
        content
            .get("dependencies_extracted")
            .and_then(|v| v.as_u64()),
        Some(1)
    );

    // Verify target has build-dependencies section
    let target_content = workspace.read_file("target/Cargo.toml");
    assert!(
        target_content.contains("[build-dependencies]"),
        "Target should have build-dependencies section"
    );
    assert!(target_content.contains("cc"), "Target should contain cc");
}
