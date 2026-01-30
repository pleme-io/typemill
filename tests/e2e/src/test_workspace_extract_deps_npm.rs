//! workspace extract_dependencies tests for npm/package.json
//!
//! Tests dependency extraction between package.json files.
//! Mirrors the Cargo.toml tests in test_workspace_extract_deps.rs.

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

/// Helper: Create source/target package.json files
fn setup_extract_test(
    workspace: &TestWorkspace,
    source_deps: &str,
    target_deps: &str,
) -> (std::path::PathBuf, std::path::PathBuf) {
    workspace.create_file("source/package.json", source_deps);
    workspace.create_file("target/package.json", target_deps);

    (
        workspace.absolute_path("source/package.json"),
        workspace.absolute_path("target/package.json"),
    )
}

#[tokio::test]
async fn test_extract_basic_dependencies_npm() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let (source_path, target_path) = setup_extract_test(
        &workspace,
        r#"{
  "name": "source-package",
  "version": "1.0.0",
  "dependencies": {
    "react": "^18.0.0",
    "lodash": "~4.17.0",
    "axios": "^1.0.0"
  }
}"#,
        r#"{
  "name": "target-package",
  "version": "1.0.0",
  "dependencies": {}
}"#,
    );

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "extract_dependencies",
                "params": {
                    "sourceManifest": source_path.to_string_lossy(),
                    "targetManifest": target_path.to_string_lossy(),
                    "dependencies": ["react", "lodash"]
                },
                "options": {
                    "dryRun": false,
                    "section": "dependencies"
                }
            }),
        )
        .await
        .expect("workspace extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");

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
    let target_content = workspace.read_file("target/package.json");
    assert!(target_content.contains("react"));
    assert!(target_content.contains("^18.0.0"));
    assert!(target_content.contains("lodash"));
    assert!(target_content.contains("~4.17.0"));
    assert!(!target_content.contains("axios"));
}

#[tokio::test]
async fn test_extract_dev_dependencies_npm() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let (source_path, target_path) = setup_extract_test(
        &workspace,
        r#"{
  "name": "source-package",
  "version": "1.0.0",
  "dependencies": {
    "react": "^18.0.0"
  },
  "devDependencies": {
    "typescript": "^5.0.0",
    "jest": "^29.0.0"
  }
}"#,
        r#"{
  "name": "target-package",
  "version": "1.0.0",
  "dependencies": {}
}"#,
    );

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "extract_dependencies",
                "params": {
                    "sourceManifest": source_path.to_string_lossy(),
                    "targetManifest": target_path.to_string_lossy(),
                    "dependencies": ["typescript"]
                },
                "options": {
                    "dryRun": false,
                    "section": "devDependencies"
                }
            }),
        )
        .await
        .expect("workspace extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");
    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success")
    );

    // Verify target updated with devDependencies section
    let target_content = workspace.read_file("target/package.json");
    assert!(target_content.contains("devDependencies"));
    assert!(target_content.contains("typescript"));
    assert!(target_content.contains("^5.0.0"));
}

#[tokio::test]
async fn test_extract_peer_dependencies_npm() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let (source_path, target_path) = setup_extract_test(
        &workspace,
        r#"{
  "name": "source-package",
  "version": "1.0.0",
  "peerDependencies": {
    "react": ">=17.0.0"
  }
}"#,
        r#"{
  "name": "target-package",
  "version": "1.0.0"
}"#,
    );

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "extract_dependencies",
                "params": {
                    "sourceManifest": source_path.to_string_lossy(),
                    "targetManifest": target_path.to_string_lossy(),
                    "dependencies": ["react"]
                },
                "options": {
                    "dryRun": false,
                    "section": "peerDependencies"
                }
            }),
        )
        .await
        .expect("workspace extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");
    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success")
    );

    // Verify target updated with peerDependencies section
    let target_content = workspace.read_file("target/package.json");
    assert!(target_content.contains("peerDependencies"));
    assert!(target_content.contains("react"));
    assert!(target_content.contains(">=17.0.0"));
}

#[tokio::test]
async fn test_extract_optional_dependencies_npm() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let (source_path, target_path) = setup_extract_test(
        &workspace,
        r#"{
  "name": "source-package",
  "version": "1.0.0",
  "optionalDependencies": {
    "fsevents": "^2.0.0"
  }
}"#,
        r#"{
  "name": "target-package",
  "version": "1.0.0"
}"#,
    );

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "extract_dependencies",
                "params": {
                    "sourceManifest": source_path.to_string_lossy(),
                    "targetManifest": target_path.to_string_lossy(),
                    "dependencies": ["fsevents"]
                },
                "options": {
                    "dryRun": false,
                    "section": "optionalDependencies"
                }
            }),
        )
        .await
        .expect("workspace extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");
    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success")
    );

    // Verify target updated with optionalDependencies section
    let target_content = workspace.read_file("target/package.json");
    assert!(target_content.contains("optionalDependencies"));
    assert!(target_content.contains("fsevents"));
}

#[tokio::test]
async fn test_extract_dependency_not_found_npm() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let (source_path, target_path) = setup_extract_test(
        &workspace,
        r#"{
  "name": "source-package",
  "version": "1.0.0",
  "dependencies": {
    "react": "^18.0.0"
  }
}"#,
        r#"{
  "name": "target-package",
  "version": "1.0.0"
}"#,
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
                    "dryRun": false,
                    "section": "dependencies"
                }
            }),
        )
        .await
        .expect("workspace extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");
    let changes = content.get("changes").expect("Changes should exist");

    // Should have warning about not found
    let warnings = changes.get("warnings").and_then(|v| v.as_array());
    assert!(warnings.is_some());
    let warnings = warnings.unwrap();
    assert!(!warnings.is_empty());
    assert!(warnings[0].as_str().unwrap_or("").contains("not found"));
}

#[tokio::test]
async fn test_extract_dependency_already_exists_npm() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let (source_path, target_path) = setup_extract_test(
        &workspace,
        r#"{
  "name": "source-package",
  "version": "1.0.0",
  "dependencies": {
    "react": "^18.0.0"
  }
}"#,
        r#"{
  "name": "target-package",
  "version": "1.0.0",
  "dependencies": {
    "react": "^17.0.0"
  }
}"#,
    );

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "extract_dependencies",
                "params": {
                    "sourceManifest": source_path.to_string_lossy(),
                    "targetManifest": target_path.to_string_lossy(),
                    "dependencies": ["react"]
                },
                "options": {
                    "dryRun": false,
                    "section": "dependencies"
                }
            }),
        )
        .await
        .expect("workspace extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");
    let changes = content.get("changes").expect("Changes should exist");

    // Should have warning about already exists
    let warnings = changes.get("warnings").and_then(|v| v.as_array());
    assert!(warnings.is_some());
    let warnings = warnings.unwrap();
    assert!(!warnings.is_empty());
    assert!(warnings[0]
        .as_str()
        .unwrap_or("")
        .contains("already exists"));

    // Target should NOT be modified (keep original version)
    let target_content = workspace.read_file("target/package.json");
    assert!(target_content.contains("^17.0.0"));
}

#[tokio::test]
async fn test_extract_dry_run_npm() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let (source_path, target_path) = setup_extract_test(
        &workspace,
        r#"{
  "name": "source-package",
  "version": "1.0.0",
  "dependencies": {
    "react": "^18.0.0"
  }
}"#,
        r#"{
  "name": "target-package",
  "version": "1.0.0",
  "dependencies": {}
}"#,
    );

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "extract_dependencies",
                "params": {
                    "sourceManifest": source_path.to_string_lossy(),
                    "targetManifest": target_path.to_string_lossy(),
                    "dependencies": ["react"]
                },
                "options": {
                    "dryRun": true,
                    "section": "dependencies"
                }
            }),
        )
        .await
        .expect("workspace extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");
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

    // Target should NOT be modified in dry run
    let target_content = workspace.read_file("target/package.json");
    assert!(!target_content.contains("react"));
}

#[tokio::test]
async fn test_extract_workspace_protocol_npm() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let (source_path, target_path) = setup_extract_test(
        &workspace,
        r#"{
  "name": "source-package",
  "version": "1.0.0",
  "dependencies": {
    "shared-utils": "workspace:*"
  }
}"#,
        r#"{
  "name": "target-package",
  "version": "1.0.0"
}"#,
    );

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "extract_dependencies",
                "params": {
                    "sourceManifest": source_path.to_string_lossy(),
                    "targetManifest": target_path.to_string_lossy(),
                    "dependencies": ["shared-utils"]
                },
                "options": {
                    "dryRun": false,
                    "section": "dependencies"
                }
            }),
        )
        .await
        .expect("workspace extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");
    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success")
    );

    // Verify workspace protocol preserved
    let target_content = workspace.read_file("target/package.json");
    assert!(target_content.contains("workspace:*"));
}

#[tokio::test]
async fn test_extract_git_dependency_npm() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let (source_path, target_path) = setup_extract_test(
        &workspace,
        r#"{
  "name": "source-package",
  "version": "1.0.0",
  "dependencies": {
    "my-lib": "git+https://github.com/user/repo.git#v1.0.0"
  }
}"#,
        r#"{
  "name": "target-package",
  "version": "1.0.0"
}"#,
    );

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "extract_dependencies",
                "params": {
                    "sourceManifest": source_path.to_string_lossy(),
                    "targetManifest": target_path.to_string_lossy(),
                    "dependencies": ["my-lib"]
                },
                "options": {
                    "dryRun": false,
                    "section": "dependencies"
                }
            }),
        )
        .await
        .expect("workspace extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");
    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success")
    );

    // Verify git URL preserved
    let target_content = workspace.read_file("target/package.json");
    assert!(target_content.contains("git+https://github.com/user/repo.git#v1.0.0"));
}

#[tokio::test]
async fn test_extract_local_path_dependency_npm() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let (source_path, target_path) = setup_extract_test(
        &workspace,
        r#"{
  "name": "source-package",
  "version": "1.0.0",
  "dependencies": {
    "local-lib": "file:../local-lib"
  }
}"#,
        r#"{
  "name": "target-package",
  "version": "1.0.0"
}"#,
    );

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "extract_dependencies",
                "params": {
                    "sourceManifest": source_path.to_string_lossy(),
                    "targetManifest": target_path.to_string_lossy(),
                    "dependencies": ["local-lib"]
                },
                "options": {
                    "dryRun": false,
                    "section": "dependencies"
                }
            }),
        )
        .await
        .expect("workspace extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");
    assert_eq!(
        content.get("status").and_then(|v| v.as_str()),
        Some("success")
    );

    // Verify local path preserved
    let target_content = workspace.read_file("target/package.json");
    assert!(target_content.contains("file:../local-lib"));
}

#[tokio::test]
async fn test_extract_multiple_dependencies_npm() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let (source_path, target_path) = setup_extract_test(
        &workspace,
        r#"{
  "name": "source-package",
  "version": "1.0.0",
  "dependencies": {
    "react": "^18.0.0",
    "react-dom": "^18.0.0",
    "lodash": "~4.17.0"
  },
  "devDependencies": {
    "typescript": "^5.0.0"
  }
}"#,
        r#"{
  "name": "target-package",
  "version": "1.0.0"
}"#,
    );

    let result = client
        .call_tool(
            "workspace",
            json!({
                "action": "extract_dependencies",
                "params": {
                    "sourceManifest": source_path.to_string_lossy(),
                    "targetManifest": target_path.to_string_lossy(),
                    "dependencies": ["react", "react-dom", "typescript"]
                },
                "options": {
                    "dryRun": false,
                    "section": "dependencies"
                }
            }),
        )
        .await
        .expect("workspace extract_dependencies should succeed");

    let content = result.get("result").expect("Result should exist");
    let changes = content.get("changes").expect("Changes should exist");

    // Should extract 3 dependencies (react, react-dom from deps, typescript from devDeps)
    assert_eq!(
        changes
            .get("dependenciesExtracted")
            .and_then(|v| v.as_u64()),
        Some(3)
    );

    // Verify all added to target
    let target_content = workspace.read_file("target/package.json");
    assert!(target_content.contains("react"));
    assert!(target_content.contains("react-dom"));
    assert!(target_content.contains("typescript"));
}

#[tokio::test]
async fn test_manifest_type_mismatch() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create mixed manifest types (should fail)
    workspace.create_file(
        "source/Cargo.toml",
        r#"[package]
name = "source-crate"
version = "0.1.0"

[dependencies]
tokio = "1.0"
"#,
    );

    workspace.create_file(
        "target/package.json",
        r#"{
  "name": "target-package",
  "version": "1.0.0"
}"#,
    );

    let source_path = workspace.absolute_path("source/Cargo.toml");
    let target_path = workspace.absolute_path("target/package.json");

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
                    "dryRun": false
                }
            }),
        )
        .await;

    // Should fail with type mismatch error
    assert!(result.is_err() || result.unwrap().get("error").is_some());
}
