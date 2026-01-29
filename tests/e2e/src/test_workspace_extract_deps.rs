//! workspace extract_dependencies tests migrated to closure-based helpers (v2)
//!
//! BEFORE: 735 lines with repetitive workspace setup
//! AFTER: Focused dependency extraction verification
//!
//! Tests dependency extraction between Cargo.toml files.

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

/// Helper: Create source/target Cargo.toml files
fn setup_extract_test(
    workspace: &TestWorkspace,
    source_deps: &str,
    target_deps: &str,
) -> (std::path::PathBuf, std::path::PathBuf) {
    workspace.create_file(
        "source/Cargo.toml",
        &format!(
            r#"[package]
name = "source-crate"
version = "0.1.0"
edition = "2021"

{}
"#,
            source_deps
        ),
    );

    workspace.create_file(
        "target/Cargo.toml",
        &format!(
            r#"[package]
name = "target-crate"
version = "0.1.0"
edition = "2021"

{}
"#,
            target_deps
        ),
    );

    (
        workspace.absolute_path("source/Cargo.toml"),
        workspace.absolute_path("target/Cargo.toml"),
    )
}

#[tokio::test]
async fn test_extract_basic_dependencies() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let (source_path, target_path) = setup_extract_test(
        &workspace,
        r#"[dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
"#,
        "[dependencies]\n",
    );

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "extract_dependencies",
                "params": {
                    "sourceManifest": source_path.to_string_lossy(),
                    "targetManifest": target_path.to_string_lossy(),
                    "dependencies": ["tokio", "serde"]
                },
                "options": {
                    "dryRun": false,
                    "preserveVersions": true,
                    "preserveFeatures": true,
                    "section": "dependencies"
                }
            }),
        )
        .await
        .expect("workspace extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");

    // M7 response: status at top level, action-specific data in nested result
    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success")
    );
    let changes = content.get("changes").expect("Changes should exist");

    assert_eq!(
        changes
            .get("dependenciesExtracted")
            .and_then(|v| v.as_u64()),
        Some(2)
    );
    assert_eq!(
        changes
            .get("targetManifestUpdated")
            .and_then(|v| v.as_bool()),
        Some(true)
    );

    // Verify target updated
    let target_content = workspace.read_file("target/Cargo.toml");
    assert!(target_content.contains("tokio"));
    assert!(target_content.contains("serde"));
    assert!(!target_content.contains("anyhow"));
    assert!(target_content.contains("features = [\"full\"]"));
    assert!(target_content.contains("features = [\"derive\"]"));
}

#[tokio::test]
async fn test_extract_dev_dependencies() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let (source_path, target_path) = setup_extract_test(
        &workspace,
        r#"[dependencies]
tokio = "1.0"

[dev-dependencies]
tempfile = "3.0"
criterion = "0.5"
"#,
        "[dependencies]\n",
    );

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "extract_dependencies",
                "params": {
                    "sourceManifest": source_path.to_string_lossy(),
                    "targetManifest": target_path.to_string_lossy(),
                    "dependencies": ["tempfile"]
                },
                "options": {
                    "dryRun": false,
                    "section": "dev-dependencies"
                }
            }),
        )
        .await
        .expect("workspace extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");

    // M7 response: status at top level, action-specific data in nested result
    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success")
    );
    let changes = content.get("changes").expect("Changes should exist");

    assert_eq!(
        changes
            .get("dependenciesExtracted")
            .and_then(|v| v.as_u64()),
        Some(1)
    );

    let target_content = workspace.read_file("target/Cargo.toml");
    assert!(target_content.contains("[dev-dependencies]"));
    assert!(target_content.contains("tempfile"));
}

#[tokio::test]
async fn test_extract_conflict_detection() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let (source_path, target_path) = setup_extract_test(
        &workspace,
        r#"[dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = "1.0"
"#,
        r#"[dependencies]
serde = "0.9"
"#,
    );

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "extract_dependencies",
                "params": {
                    "sourceManifest": source_path.to_string_lossy(),
                    "targetManifest": target_path.to_string_lossy(),
                    "dependencies": ["tokio", "serde"]
                },
                "options": {
                    "dryRun": false
                }
            }),
        )
        .await
        .expect("workspace extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");

    // M7 response: status at top level, action-specific data in nested result
    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success")
    );
    let changes = content.get("changes").expect("Changes should exist");

    // Verify warnings
    let warnings = changes.get("warnings").and_then(|v| v.as_array()).unwrap();
    assert!(warnings
        .iter()
        .any(|w| w.as_str().unwrap().contains("serde")
            && w.as_str().unwrap().contains("already exists")));

    // Verify dependencies_added
    let deps_added = changes
        .get("dependenciesAdded")
        .and_then(|v| v.as_array())
        .unwrap();
    assert_eq!(deps_added.len(), 2);

    let serde_info = deps_added
        .iter()
        .find(|d| d.get("name").and_then(|n| n.as_str()) == Some("serde"))
        .unwrap();
    assert_eq!(
        serde_info.get("alreadyExists").and_then(|v| v.as_bool()),
        Some(true)
    );

    // Verify original version preserved
    let target_content = workspace.read_file("target/Cargo.toml");
    assert!(target_content.contains("serde = \"0.9\""));
    assert!(target_content.contains("tokio"));
}

#[tokio::test]
async fn test_extract_workspace_dependencies() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let (source_path, target_path) = setup_extract_test(
        &workspace,
        r#"[dependencies]
my-common = { workspace = true }
tokio = "1.0"
"#,
        "[dependencies]\n",
    );

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "extract_dependencies",
                "params": {
                    "sourceManifest": source_path.to_string_lossy(),
                    "targetManifest": target_path.to_string_lossy(),
                    "dependencies": ["my-common"]
                },
                "options": {
                    "dryRun": false
                }
            }),
        )
        .await
        .expect("workspace extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");

    // M7 response: status at top level, action-specific data in nested result
    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success")
    );
    let changes = content.get("changes").expect("Changes should exist");

    assert_eq!(
        changes
            .get("dependenciesExtracted")
            .and_then(|v| v.as_u64()),
        Some(1)
    );

    let target_content = workspace.read_file("target/Cargo.toml");
    assert!(target_content.contains("my-common"));
    assert!(target_content.contains("workspace = true"));
}

#[tokio::test]
async fn test_extract_path_dependencies() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let (source_path, target_path) = setup_extract_test(
        &workspace,
        r#"[dependencies]
my-local = { path = "../my-local" }
"#,
        "[dependencies]\n",
    );

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "extract_dependencies",
                "params": {
                    "sourceManifest": source_path.to_string_lossy(),
                    "targetManifest": target_path.to_string_lossy(),
                    "dependencies": ["my-local"]
                },
                "options": {
                    "dryRun": false
                }
            }),
        )
        .await
        .expect("workspace extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");

    // M7 response: status at top level, action-specific data in nested result
    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success")
    );
    let changes = content.get("changes").expect("Changes should exist");

    assert_eq!(
        changes
            .get("dependenciesExtracted")
            .and_then(|v| v.as_u64()),
        Some(1)
    );

    let target_content = workspace.read_file("target/Cargo.toml");
    assert!(target_content.contains("my-local"));
    assert!(target_content.contains("path = \"../my-local\""));
}

#[tokio::test]
async fn test_extract_dependency_not_found() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let (source_path, target_path) = setup_extract_test(
        &workspace,
        r#"[dependencies]
tokio = "1.0"
"#,
        "[dependencies]\n",
    );

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "extract_dependencies",
                "params": {
                    "sourceManifest": source_path.to_string_lossy(),
                    "targetManifest": target_path.to_string_lossy(),
                    "dependencies": ["nonexistent"]
                },
                "options": {
                    "dryRun": false
                }
            }),
        )
        .await
        .expect("workspace extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");

    // M7 response: status at top level, action-specific data in nested result
    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success")
    );
    let changes = content.get("changes").expect("Changes should exist");

    assert_eq!(
        changes
            .get("dependenciesExtracted")
            .and_then(|v| v.as_u64()),
        Some(0)
    );

    let warnings = changes.get("warnings").and_then(|v| v.as_array()).unwrap();
    assert!(warnings
        .iter()
        .any(|w| w.as_str().unwrap().contains("nonexistent")
            && w.as_str().unwrap().contains("not found")));
}

#[tokio::test]
async fn test_extract_dry_run() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let (source_path, target_path) = setup_extract_test(
        &workspace,
        r#"[dependencies]
tokio = "1.0"
"#,
        "[dependencies]\n",
    );

    let original_target_content = workspace.read_file("target/Cargo.toml");

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "extract_dependencies",
                "params": {
                    "sourceManifest": source_path.to_string_lossy(),
                    "targetManifest": target_path.to_string_lossy(),
                    "dependencies": ["tokio"]
                },
                "options": {
                    "dryRun": true
                }
            }),
        )
        .await
        .expect("workspace extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");

    // M7 response: status at top level (preview for dry run), action-specific data in nested result
    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("preview")
    );
    let changes = content.get("changes").expect("Changes should exist");

    assert_eq!(changes.get("dryRun").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        changes
            .get("targetManifestUpdated")
            .and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        changes
            .get("dependenciesExtracted")
            .and_then(|v| v.as_u64()),
        Some(1)
    );

    // Verify file unchanged
    let target_content = workspace.read_file("target/Cargo.toml");
    assert_eq!(target_content, original_target_content);
}

#[tokio::test]
async fn test_extract_optional_dependencies() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let (source_path, target_path) = setup_extract_test(
        &workspace,
        r#"[dependencies]
feature-dep = { version = "1.0", optional = true }
"#,
        "[dependencies]\n",
    );

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "extract_dependencies",
                "params": {
                    "sourceManifest": source_path.to_string_lossy(),
                    "targetManifest": target_path.to_string_lossy(),
                    "dependencies": ["feature-dep"]
                },
                "options": {
                    "dryRun": false
                }
            }),
        )
        .await
        .expect("workspace extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");

    // M7 response: status at top level, action-specific data in nested result
    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success")
    );
    let changes = content.get("changes").expect("Changes should exist");

    assert_eq!(
        changes
            .get("dependenciesExtracted")
            .and_then(|v| v.as_u64()),
        Some(1)
    );

    // Verify optional flag
    let deps_added = changes
        .get("dependenciesAdded")
        .and_then(|v| v.as_array())
        .unwrap();
    let dep_info = &deps_added[0];
    assert_eq!(
        dep_info.get("optional").and_then(|v| v.as_bool()),
        Some(true)
    );

    let target_content = workspace.read_file("target/Cargo.toml");
    assert!(target_content.contains("optional = true"));
}

#[tokio::test]
async fn test_extract_build_dependencies() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let (source_path, target_path) = setup_extract_test(
        &workspace,
        r#"[build-dependencies]
cc = "1.0"
"#,
        "[dependencies]\n",
    );

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "extract_dependencies",
                "params": {
                    "sourceManifest": source_path.to_string_lossy(),
                    "targetManifest": target_path.to_string_lossy(),
                    "dependencies": ["cc"]
                },
                "options": {
                    "dryRun": false,
                    "section": "build-dependencies"
                }
            }),
        )
        .await
        .expect("workspace extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");

    // M7 response: status at top level, action-specific data in nested result
    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success")
    );
    let changes = content.get("changes").expect("Changes should exist");

    assert_eq!(
        changes
            .get("dependenciesExtracted")
            .and_then(|v| v.as_u64()),
        Some(1)
    );

    let target_content = workspace.read_file("target/Cargo.toml");
    assert!(target_content.contains("[build-dependencies]"));
    assert!(target_content.contains("cc"));
}
