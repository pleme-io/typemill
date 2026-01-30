//! Real-world operations testing on Zod repository
//!
//! Tests move (relocate) and extract operations on a fresh Zod clone
//! to verify they work correctly and don't break the codebase.

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;
use std::process::Command;
use std::time::Duration;

const LARGE_PROJECT_TIMEOUT: Duration = Duration::from_secs(120);

/// Setup a fresh Zod workspace for testing
fn setup_zod_workspace() -> TestWorkspace {
    let workspace = TestWorkspace::new();

    // Clone Zod
    let status = Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "https://github.com/colinhacks/zod.git",
            ".",
        ])
        .current_dir(workspace.path())
        .status()
        .expect("Failed to clone zod");

    assert!(status.success(), "git clone failed");

    // Run mill setup
    let mill_path = std::env::var("MILL_PATH")
        .unwrap_or_else(|_| "/home/user/typemill/target/debug/mill".to_string());

    let status = Command::new(&mill_path)
        .args(["setup"])
        .current_dir(workspace.path())
        .status()
        .expect("Failed to run mill setup");

    assert!(status.success(), "mill setup failed");

    workspace
}

// ============================================================================
// File Move (Relocate) Tests
// ============================================================================

#[tokio::test]
async fn test_zod_relocate_file_dry_run() {
    let workspace = setup_zod_workspace();
    let mut client = TestClient::new(workspace.path());

    // Create a test file to move
    workspace.create_file(
        "packages/zod/src/test-source.ts",
        "export const testValue = 42;\n",
    );

    let source_path = workspace.absolute_path("packages/zod/src/test-source.ts");
    let dest_path = workspace.absolute_path("packages/zod/src/test-dest.ts");

    let result = client
        .call_tool_with_timeout(
            "relocate",
            json!({
                "target": {
                    "kind": "file",
                    "filePath": source_path.to_string_lossy()
                },
                "destination": dest_path.to_string_lossy(),
                "options": {
                    "dryRun": true
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("relocate dry-run should succeed");

    // Dry run should not modify files
    assert!(source_path.exists(), "Source should still exist after dry run");
    assert!(
        !dest_path.exists(),
        "Destination should not exist after dry run"
    );

    // Result should exist
    assert!(result.get("result").is_some(), "Should have result: {:?}", result);
}

#[tokio::test]
async fn test_zod_relocate_file_execute() {
    let workspace = setup_zod_workspace();
    let mut client = TestClient::new(workspace.path());

    // Create a test file to move
    workspace.create_file(
        "packages/zod/src/test-move-source.ts",
        r#"export const testValue = 42;
export function testFunc() { return testValue; }
"#,
    );

    let source_path = workspace.absolute_path("packages/zod/src/test-move-source.ts");
    let dest_path = workspace.absolute_path("packages/zod/src/v3/helpers/test-move-dest.ts");

    let _result = client
        .call_tool_with_timeout(
            "relocate",
            json!({
                "target": {
                    "kind": "file",
                    "filePath": source_path.to_string_lossy()
                },
                "destination": dest_path.to_string_lossy(),
                "options": {
                    "dryRun": false
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("relocate execute should succeed");

    // Verify file was moved
    assert!(
        !source_path.exists(),
        "Source should no longer exist after move"
    );
    assert!(dest_path.exists(), "Destination should exist after move");

    // Verify content is preserved
    let content = workspace.read_file("packages/zod/src/v3/helpers/test-move-dest.ts");
    assert!(content.contains("testValue = 42"));
    assert!(content.contains("testFunc"));
}

// ============================================================================
// Rename Tests
// ============================================================================

#[tokio::test]
async fn test_zod_rename_file_dry_run() {
    let workspace = setup_zod_workspace();
    let mut client = TestClient::new(workspace.path());

    // Create a test file
    workspace.create_file(
        "packages/zod/src/test-rename.ts",
        "export const value = 'original';",
    );

    let source_path = workspace.absolute_path("packages/zod/src/test-rename.ts");
    let new_name = workspace.absolute_path("packages/zod/src/test-renamed.ts");

    let _result = client
        .call_tool_with_timeout(
            "rename_all",
            json!({
                "target": {
                    "kind": "file",
                    "filePath": source_path.to_string_lossy()
                },
                "newName": new_name.to_string_lossy(),
                "options": {
                    "dryRun": true
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("rename_all dry-run should succeed");

    // Original should still exist (dry run)
    assert!(source_path.exists());
}

// ============================================================================
// Extract Dependencies Tests (npm)
// ============================================================================

#[tokio::test]
async fn test_zod_extract_dependencies_npm() {
    let workspace = setup_zod_workspace();
    let mut client = TestClient::new(workspace.path());

    // Create source and target packages
    workspace.create_file(
        "packages/source-pkg/package.json",
        r#"{
  "name": "source-pkg",
  "version": "1.0.0",
  "dependencies": {
    "lodash": "^4.17.0",
    "axios": "^1.0.0"
  },
  "devDependencies": {
    "typescript": "^5.0.0"
  }
}"#,
    );

    workspace.create_file(
        "packages/target-pkg/package.json",
        r#"{
  "name": "target-pkg",
  "version": "1.0.0",
  "dependencies": {}
}"#,
    );

    let source_path = workspace.absolute_path("packages/source-pkg/package.json");
    let target_path = workspace.absolute_path("packages/target-pkg/package.json");

    // Extract lodash dependency
    let _result = client
        .call_tool_with_timeout(
            "workspace",
            json!({
                "action": "extract_dependencies",
                "params": {
                    "sourceManifest": source_path.to_string_lossy(),
                    "targetManifest": target_path.to_string_lossy(),
                    "dependencies": ["lodash"]
                },
                "options": {
                    "dryRun": false,
                    "section": "dependencies"
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("extract_dependencies execute should succeed");

    // Verify target was updated
    let target_content = workspace.read_file("packages/target-pkg/package.json");
    assert!(target_content.contains("lodash"), "Should have lodash: {}", target_content);
    assert!(target_content.contains("^4.17.0"), "Should have version: {}", target_content);
    assert!(!target_content.contains("axios"), "Should not have axios"); // Not extracted
}

#[tokio::test]
async fn test_zod_extract_dependencies_dry_run() {
    let workspace = setup_zod_workspace();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "packages/src-pkg/package.json",
        r#"{
  "name": "src-pkg",
  "version": "1.0.0",
  "dependencies": {
    "react": "^18.0.0"
  }
}"#,
    );

    workspace.create_file(
        "packages/tgt-pkg/package.json",
        r#"{
  "name": "tgt-pkg",
  "version": "1.0.0",
  "dependencies": {}
}"#,
    );

    let source_path = workspace.absolute_path("packages/src-pkg/package.json");
    let target_path = workspace.absolute_path("packages/tgt-pkg/package.json");

    let _result = client
        .call_tool_with_timeout(
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
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("extract_dependencies dry-run should succeed");

    // Target should NOT be modified (dry run)
    let target_content = workspace.read_file("packages/tgt-pkg/package.json");
    assert!(!target_content.contains("react"), "Should not have react in dry run");
}

// ============================================================================
// Combined Workflow Test: Extract + Move
// ============================================================================

#[tokio::test]
async fn test_zod_workflow_extract_and_move() {
    let workspace = setup_zod_workspace();
    let mut client = TestClient::new(workspace.path());

    // Step 1: Create a source package with dependencies
    workspace.create_file(
        "packages/old-utils/package.json",
        r#"{
  "name": "old-utils",
  "version": "1.0.0",
  "dependencies": {
    "lodash": "^4.17.0"
  }
}"#,
    );
    workspace.create_file(
        "packages/old-utils/src/helpers.ts",
        r#"export function formatName(name: string): string {
  return name.trim().toLowerCase();
}
"#,
    );

    // Create target package manually (since create_package needs Rust workspace)
    workspace.create_file(
        "packages/new-utils/package.json",
        r#"{
  "name": "new-utils",
  "version": "1.0.0",
  "dependencies": {}
}"#,
    );
    std::fs::create_dir_all(workspace.path().join("packages/new-utils/src"))
        .expect("Failed to create src dir");

    // Step 2: Extract dependencies
    let source_manifest = workspace.absolute_path("packages/old-utils/package.json");
    let target_manifest = workspace.absolute_path("packages/new-utils/package.json");

    let _result = client
        .call_tool_with_timeout(
            "workspace",
            json!({
                "action": "extract_dependencies",
                "params": {
                    "sourceManifest": source_manifest.to_string_lossy(),
                    "targetManifest": target_manifest.to_string_lossy(),
                    "dependencies": ["lodash"]
                },
                "options": {
                    "dryRun": false,
                    "section": "dependencies"
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("extract_dependencies should succeed");

    // Verify lodash was added
    let new_pkg = workspace.read_file("packages/new-utils/package.json");
    assert!(new_pkg.contains("lodash"), "Should have lodash dependency");

    // Step 3: Move a file to the new package
    let source_file = workspace.absolute_path("packages/old-utils/src/helpers.ts");
    let dest_file = workspace.absolute_path("packages/new-utils/src/helpers.ts");

    let _result = client
        .call_tool_with_timeout(
            "relocate",
            json!({
                "target": {
                    "kind": "file",
                    "filePath": source_file.to_string_lossy()
                },
                "destination": dest_file.to_string_lossy(),
                "options": {
                    "dryRun": false
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("relocate should succeed");

    // Verify file was moved
    assert!(!source_file.exists(), "Source file should be gone");
    assert!(dest_file.exists(), "Dest file should exist");

    // Verify content is preserved
    let helpers_content = workspace.read_file("packages/new-utils/src/helpers.ts");
    assert!(helpers_content.contains("formatName"));
    assert!(helpers_content.contains("trim()"));
}
