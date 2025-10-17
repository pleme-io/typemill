//! Integration tests for file move with import updates
//!
//! These tests verify that when a file is moved, all imports pointing to it
//! are automatically updated with the new path. This is critical for maintaining
//! code correctness across the workspace.
//!
//! Tests cover:
//! - Same directory moves (./file → ./moved)
//! - Cross-directory moves (../../old → ../../new)
//! - Multiple importers updated in single operation
//! - Moving files to subdirectories
//! - Complex nested directory structures

use crate::harness::{TestClient, TestWorkspace};
use cb_test_support::harness::mcp_fixtures::{MOVE_FILE_TESTS, RUST_MOVE_FILE_TESTS};
use serde_json::json;

// =============================================================================
// Phase 1: Activate Existing Fixtures
// =============================================================================

#[tokio::test]
async fn test_move_file_updates_imports_from_fixtures() {
    for case in MOVE_FILE_TESTS {
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

        // 2. Generate move plan
        let plan_result = client
            .call_tool(
                "move.plan",
                json!({
                    "target": {
                        "kind": "file",
                        "path": old_path.to_string_lossy()
                    },
                    "destination": new_path.to_string_lossy()
                }),
            )
            .await
            .expect("move.plan should succeed");

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

            // 4. Verify the move occurred
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

#[tokio::test]
async fn test_rust_move_file_updates_imports_from_fixtures() {
    for case in RUST_MOVE_FILE_TESTS {
        println!("\n🧪 Running rust test case: {}", case.test_name);

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

        // 2. Generate move plan
        let plan_result = client
            .call_tool(
                "move.plan",
                json!({
                    "target": {
                        "kind": "file",
                        "path": old_path.to_string_lossy()
                    },
                    "destination": new_path.to_string_lossy()
                }),
            )
            .await
            .expect("move.plan should succeed");

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

            // 4. Verify the move occurred
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

                // Capture stderr logs for debugging
                let stderr_logs = client.get_stderr_logs();
                if !stderr_logs.is_empty() {
                    eprintln!("\n=== SERVER STDERR LOGS ===");
                    for log in &stderr_logs {
                        eprintln!("{}", log);
                    }
                    eprintln!("=== END STDERR LOGS ===\n");
                }

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

                // Also verify old import path is gone
                assert!(
                    !content.contains("use common::utils::do_stuff;"),
                    "❌ Old import path still exists in '{}' for test case: '{}'.\n\
                     File content:\n{}",
                    importer_path,
                    case.test_name,
                    content
                );

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
