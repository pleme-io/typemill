//! Integration tests for file rename with import updates
//!
//! These tests verify that when a file is renamed, all imports pointing to it
//! are automatically updated with the new path. This is critical for maintaining
//! code correctness across the workspace.
//!
//! Tests cover:
//! - Same directory renames (./file → ./renamed)
//! - Cross-directory renames (../../old → ../../new)
//! - Multiple importers updated in single operation
//! - Moving files to subdirectories
//! - Complex nested directory structures

use crate::harness::{TestClient, TestWorkspace};
use cb_test_support::harness::mcp_fixtures::RENAME_FILE_TESTS;
use serde_json::json;

// =============================================================================
// Phase 1: Activate Existing Fixtures
// =============================================================================

#[tokio::test]
async fn test_rename_file_updates_imports_from_fixtures() {
    for case in RENAME_FILE_TESTS {
        println!("\n🧪 Running test case: {}", case.test_name);

        // 1. Setup workspace from fixture
        let workspace = TestWorkspace::new();
        let mut client = TestClient::new(workspace.path());

        // Create all initial files (with parent directories)
        for (file_path, content) in case.initial_files {
            // Ensure parent directories exist
            if let Some(parent) = std::path::Path::new(file_path).parent() {
                if parent != std::path::Path::new("") {
                    workspace.create_directory(parent.to_str().unwrap());
                }
            }
            workspace.create_file(file_path, content);
        }

        let old_path = workspace.absolute_path(case.old_file_path);
        let new_path = workspace.absolute_path(case.new_file_path);

        // 2. Generate rename plan
        let plan_result = client
            .call_tool(
                "rename.plan",
                json!({
                    "target": {
                        "kind": "file",
                        "path": old_path.to_string_lossy()
                    },
                    "new_name": new_path.to_string_lossy()
                }),
            )
            .await
            .expect("rename.plan should succeed");

        let plan = plan_result
            .get("result")
            .and_then(|r| r.get("content"))
            .expect("Plan should have result.content");

        // 3. Apply plan via workspace.apply_edit
        let apply_result = client
            .call_tool(
                "workspace.apply_edit",
                json!({
                    "plan": plan,
                    "options": {
                        "dry_run": false,
                        "validate_checksums": true
                    }
                }),
            )
            .await;

        if case.expect_success {
            let apply_response = apply_result.expect("workspace.apply_edit should succeed");
            let result = apply_response
                .get("result")
                .and_then(|r| r.get("content"))
                .expect("Apply should have result.content");

            assert_eq!(
                result.get("success").and_then(|v| v.as_bool()),
                Some(true),
                "Apply should succeed for test case: {}",
                case.test_name
            );

            // 4. Verify the rename occurred
            assert!(
                !workspace.file_exists(case.old_file_path),
                "Old file '{}' should be deleted in test case: {}",
                case.old_file_path,
                case.test_name
            );
            assert!(
                workspace.file_exists(case.new_file_path),
                "New file '{}' should exist in test case: {}",
                case.new_file_path,
                case.test_name
            );

            // 5. CRITICAL: Verify imports were updated in dependent files
            for (importer_path, expected_substring) in case.expected_import_updates {
                let content = workspace.read_file(importer_path);
                assert!(
                    content.contains(expected_substring),
                    "❌ Import in '{}' was not updated correctly in test case: '{}'.\n\
                     Expected to find: '{}'\n\
                     Actual file content:\n{}",
                    importer_path,
                    case.test_name,
                    expected_substring,
                    content
                );

                // Also verify old import path is gone (prevent regressions where both coexist)
                let old_file_name = std::path::Path::new(case.old_file_path)
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap();
                let new_file_name = std::path::Path::new(case.new_file_path)
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap();

                if old_file_name != new_file_name {
                    assert!(
                        !content.contains(&format!("from './{}'", old_file_name))
                            && !content.contains(&format!("from '../{}'", old_file_name)),
                        "❌ Old import path still exists in '{}' for test case: '{}'.\n\
                         This indicates both old and new imports coexist!\n\
                         File content:\n{}",
                        importer_path,
                        case.test_name,
                        content
                    );
                }

                println!(
                    "  ✅ Verified import updated in '{}': contains '{}'",
                    importer_path, expected_substring
                );
            }
        } else {
            assert!(
                apply_result.is_err() || apply_result.unwrap().get("error").is_some(),
                "Operation should fail for test case: {}",
                case.test_name
            );
        }

        println!("✅ Test case '{}' passed", case.test_name);
    }
}

// =============================================================================
// Phase 2: Targeted Edge Case Tests
// =============================================================================

#[tokio::test]
async fn test_rename_file_updates_parent_directory_importer() {
    // Setup: src/index.ts imports from src/components/Button.ts
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create directory structure first
    workspace.create_directory("src");
    workspace.create_directory("src/components");

    workspace.create_file("src/components/Button.ts", "export class Button {}");
    workspace.create_file(
        "src/index.ts",
        "import { Button } from './components/Button';",
    );

    // Action: Rename the component
    let old_path = workspace.absolute_path("src/components/Button.ts");
    let new_path = workspace.absolute_path("src/components/FancyButton.ts");

    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": old_path.to_string_lossy()
                },
                "new_name": new_path.to_string_lossy()
            }),
        )
        .await
        .expect("rename.plan should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dry_run": false
                }
            }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    // Assert: Check the import path in the parent file
    let content = workspace.read_file("src/index.ts");
    assert!(
        content.contains("from './components/FancyButton'"),
        "Import should be updated to new path. Actual content:\n{}",
        content
    );
    // Verify old import is gone
    assert!(
        !content.contains("from './components/Button'"),
        "Old import path should be removed. Actual content:\n{}",
        content
    );
}

#[tokio::test]
async fn test_rename_file_updates_sibling_directory_importer() {
    // Setup: src/components/Button.ts imports from src/utils/helpers.ts
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create directory structure first
    workspace.create_directory("src");
    workspace.create_directory("src/utils");
    workspace.create_directory("src/components");

    workspace.create_file("src/utils/helpers.ts", "export function log() {}");
    workspace.create_file(
        "src/components/Button.ts",
        "import { log } from '../utils/helpers';",
    );

    // Action: Rename the utility file
    let old_path = workspace.absolute_path("src/utils/helpers.ts");
    let new_path = workspace.absolute_path("src/utils/core.ts");

    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": old_path.to_string_lossy()
                },
                "new_name": new_path.to_string_lossy()
            }),
        )
        .await
        .expect("rename.plan should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dry_run": false
                }
            }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    // Assert: Check the relative import path in the sibling's file
    let content = workspace.read_file("src/components/Button.ts");
    assert!(
        content.contains("from '../utils/core'"),
        "Import should be updated to new path. Actual content:\n{}",
        content
    );
    // Verify old import is gone
    assert!(
        !content.contains("from '../utils/helpers'"),
        "Old import path should be removed. Actual content:\n{}",
        content
    );
}

#[tokio::test]
async fn test_directory_rename_updates_all_imports() {
    // Setup: A multi-file structure inside a directory to be renamed
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create directory structure first
    workspace.create_directory("src");
    workspace.create_directory("src/core");

    workspace.create_file("src/core/utils.ts", "export function util() {}");
    workspace.create_file(
        "src/core/api.ts",
        "import { util } from './utils'; export function api() {}",
    );
    workspace.create_file("src/app.ts", "import { api } from './core/api';");

    // Action: Rename the 'core' directory to 'legacy'
    let old_dir = workspace.absolute_path("src/core");
    let new_dir = workspace.absolute_path("src/legacy");

    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "directory",
                    "path": old_dir.to_string_lossy()
                },
                "new_name": new_dir.to_string_lossy()
            }),
        )
        .await
        .expect("rename.plan should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dry_run": false
                }
            }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    // Assert: Check imports both inside and outside the renamed directory
    let api_content = workspace.read_file("src/legacy/api.ts");
    assert!(
        api_content.contains("from './utils'"),
        "Internal import should be preserved. Actual content:\n{}",
        api_content
    );

    let app_content = workspace.read_file("src/app.ts");
    assert!(
        app_content.contains("from './legacy/api'"),
        "External import should be updated. Actual content:\n{}",
        app_content
    );
    // Verify old import is gone (prevent regression where both paths coexist)
    assert!(
        !app_content.contains("from './core/api'"),
        "Old directory path should be removed from imports. Actual content:\n{}",
        app_content
    );
}
