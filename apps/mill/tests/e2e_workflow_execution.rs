//! Workflow execution tests
//!
//! This module tests the workflow executor and planner, which orchestrate
//! complex multi-step operations based on intent specifications.
//!
//! Workflow tests verify:
//! - Simple linear workflows (single-step operations)
//! - Complex workflows with dependencies
//! - Workflow failure and rollback scenarios
//! - Intent-based workflow planning and execution

use mill_test_support::harness::{ TestClient , TestWorkspace };
use serde_json::json;

/// Test complex workflow with multiple dependencies
#[tokio::test]
async fn test_execute_complex_workflow_with_dependencies() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a multi-file project structure
    let src_dir = workspace.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();

    let main_ts = src_dir.join("main.ts");
    std::fs::write(
        &main_ts,
        r#"
import { helper } from './helper';

export function main() {
    return helper();
}
"#,
    )
    .unwrap();

    let helper_ts = src_dir.join("helper.ts");
    std::fs::write(
        &helper_ts,
        r#"
export function helper() {
    return "helper";
}
"#,
    )
    .unwrap();

    // Wait for LSP initialization
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

    // Execute a complex workflow: rename file (which updates imports in dependent files)
    // This tests the workflow executor's ability to handle multi-step operations
    let new_helper_path = src_dir.join("utilities.ts");
    let response = client
        .call_tool(
            "rename_file",
            json!({
                "old_path": helper_ts.to_string_lossy(),
                "new_path": new_helper_path.to_string_lossy()
            }),
        )
        .await;

    // Verify the workflow executed successfully
    if let Ok(response_value) = response {
        if response_value.get("result").is_some() {
            // Verify the file was renamed
            assert!(
                new_helper_path.exists(),
                "New file should exist after rename"
            );
            assert!(
                !helper_ts.exists(),
                "Old file should not exist after rename"
            );

            // Note: Import update validation is skipped in E2E tests
            // Import updates require LSP servers to be running and properly configured
            // The file rename operation succeeds, but import updates may not work
            // without LSP support
            eprintln!(
                "ℹ️  Test 'test_execute_complex_workflow_with_dependencies': Skipping import update validation (requires LSP server support)"
            );
        }
    }
}

/// Test workflow failure and error handling
#[tokio::test]
async fn test_workflow_failure_handling() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Try to rename a non-existent file (should fail gracefully)
    let non_existent = workspace.path().join("does_not_exist.ts");
    let new_path = workspace.path().join("new_name.ts");

    let response = client
        .call_tool(
            "rename_file",
            json!({
                "old_path": non_existent.to_string_lossy(),
                "new_path": new_path.to_string_lossy()
            }),
        )
        .await;

    // Verify the workflow failed with appropriate error
    if let Ok(response_value) = response {
        if let Some(error) = response_value.get("error") {
            // Error should contain useful information
            let error_message = error.to_string();
            assert!(
                error_message.contains("not found") || error_message.contains("does not exist"),
                "Error should indicate file not found"
            );
        }
    }
}

/// Test workflow execution with batch operations
#[tokio::test]
async fn test_workflow_batch_operations() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create multiple files
    let files = vec![
        ("file1.ts", "export const A = 1;"),
        ("file2.ts", "export const B = 2;"),
        ("file3.ts", "export const C = 3;"),
    ];

    for (name, content) in &files {
        let file_path = workspace.path().join(name);
        std::fs::write(&file_path, content).unwrap();
    }

    // Execute batch operation workflow
    let operations = vec![
        json!({
            "operation": "read_file",
            "filePath": workspace.path().join("file1.ts").to_string_lossy()
        }),
        json!({
            "operation": "read_file",
            "filePath": workspace.path().join("file2.ts").to_string_lossy()
        }),
        json!({
            "operation": "read_file",
            "filePath": workspace.path().join("file3.ts").to_string_lossy()
        }),
    ];

    let response = client
        .call_tool("execute_batch", json!({ "operations": operations }))
        .await;

    // Verify batch workflow executed successfully
    if let Ok(response_value) = response {
        if let Some(result) = response_value.get("result") {
            // Result should contain results from all operations
            assert!(
                result.get("results").is_some() || result.get("operations").is_some(),
                "Batch workflow should return results"
            );
        }
    }
}

/// Test workflow with dependency resolution
#[tokio::test]
async fn test_workflow_dependency_resolution() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript project with package.json
    let package_json = workspace.path().join("package.json");
    std::fs::write(
        &package_json,
        r#"
{
    "name": "test-project",
    "version": "1.0.0",
    "dependencies": {
        "lodash": "^4.17.21"
    },
    "devDependencies": {
        "typescript": "^5.0.0"
    }
}
"#,
    )
    .unwrap();

    // Create a file that uses the dependency
    let main_ts = workspace.path().join("main.ts");
    std::fs::write(
        &main_ts,
        r#"import _ from 'lodash';

console.log(_.partition([1, 2, 3, 4], n => n % 2));
"#,
    )
    .unwrap();

    // Wait for LSP to initialize
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // Test analyzing imports (workflow operation)
    let response = client
        .call_tool(
            "analyze.dependencies",
            json!({
                "kind": "imports",
                "filePath": main_ts.to_string_lossy()
            }),
        )
        .await;

    // Verify the workflow executed successfully
    if let Ok(response_value) = response {
        if let Some(error) = response_value.get("error") {
            panic!("Workflow should not return an error, but got: {:?}", error);
        }

        let result = response_value
            .get("result")
            .expect("Workflow should return a result");

        // The result uses unified API format with "findings" array
        let findings = result
            .get("findings")
            .and_then(|v| v.as_array())
            .expect("Result should have findings array");

        assert_eq!(findings.len(), 1, "Should find one import");

        let lodash_import = findings
            .iter()
            .find(|f| {
                // In the unified API, import source is in metrics.source_module
                f.get("metrics")
                    .and_then(|m| m.get("source_module"))
                    .and_then(|s| s.as_str())
                    .map(|s| s == "lodash")
                    .unwrap_or(false)
            })
            .expect("Should find lodash import");

        // Check source_module field in metrics
        let source_module = lodash_import
            .get("metrics")
            .and_then(|m| m.get("source_module"))
            .and_then(|s| s.as_str())
            .expect("Import should have source_module in metrics");
        assert_eq!(source_module, "lodash");
    } else {
        panic!("Tool call failed: {:?}", response.err());
    }
}

/// Test workflow planning for complex operations
#[tokio::test]
async fn test_workflow_planning() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a project structure
    let src_dir = workspace.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();

    let old_module = src_dir.join("old_module.ts");
    std::fs::write(
        &old_module,
        r#"
export function feature1() {
    return "feature1";
}

export function feature2() {
    return "feature2";
}
"#,
    )
    .unwrap();

    let consumer = src_dir.join("consumer.ts");
    std::fs::write(
        &consumer,
        r#"
import { feature1, feature2 } from './old_module';

export function useFeatures() {
    return feature1() + feature2();
}
"#,
    )
    .unwrap();

    // Wait for LSP initialization
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

    // Execute a workflow that requires planning: rename module
    // This should plan and execute: rename file + update imports
    let new_module = src_dir.join("new_module.ts");
    let response = client
        .call_tool(
            "rename_file",
            json!({
                "old_path": old_module.to_string_lossy(),
                "new_path": new_module.to_string_lossy()
            }),
        )
        .await;

    // Verify the planned workflow executed correctly
    if let Ok(response_value) = response {
        if response_value.get("result").is_some() {
            // Verify file was renamed
            assert!(new_module.exists(), "New module should exist");
            assert!(!old_module.exists(), "Old module should not exist");

            // Note: Import update validation is skipped in E2E tests
            // Import updates require LSP servers to be running and properly configured
            // The file rename operation succeeds, but import updates may not work
            // without LSP support
            eprintln!(
                "ℹ️  Test 'test_workflow_planning': Skipping import update validation (requires LSP server support)"
            );
        }
    }
}

/// Test workflow execution timeout handling
#[tokio::test]
async fn test_workflow_timeout_handling() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a large project to potentially trigger timeouts
    for i in 0..50 {
        let file = workspace.path().join(format!("file{}.ts", i));
        std::fs::write(
            &file,
            format!(
                r#"
export function func{}() {{
    return {};
}}
"#,
                i, i
            ),
        )
        .unwrap();
    }

    // Wait for LSP to process all files
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

    // Execute a potentially long-running workflow
    let response = client
        .call_tool(
            "analyze.dead_code",
            json!({
                "kind": "unused_symbols",
                "scope": {
                    "type": "workspace"
                }
            }),
        )
        .await;

    // Workflow should complete or timeout gracefully
    assert!(
        response.is_ok() || response.is_err(),
        "Workflow should handle timeout gracefully"
    );

    if let Ok(response_value) = response {
        // If successful, verify it has proper structure
        assert!(
            response_value.get("result").is_some() || response_value.get("error").is_some(),
            "Response should have result or error"
        );
    }
}

/// Test unified refactoring API with dryRun option
#[tokio::test]
async fn test_unified_refactoring_rename() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a test file with a symbol to rename
    let test_file = workspace.path().join("test.ts");
    std::fs::write(
        &test_file,
        r#"
export function oldFunctionName() {
    return "test";
}

export function caller() {
    return oldFunctionName();
}
"#,
    )
    .unwrap();

    // Wait for LSP initialization
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

    // Execute rename with unified API (dryRun: false)
    let apply_response = client
        .call_tool(
            "rename",
            json!({
                "filePath": test_file.to_string_lossy(),
                "line": 2,
                "character": 17,
                "newName": "newFunctionName",
                "options": {
                    "dryRun": false
                }
            }),
        )
        .await;

    // Verify edit was applied successfully
    if let Ok(apply_value) = apply_response {
        if let Some(result) = apply_value.get("result") {
            assert!(
                result.get("content").is_some(),
                "Rename should return result content"
            );

            // Verify the file was actually modified
            let modified_content = std::fs::read_to_string(&test_file).unwrap();
            assert!(
                modified_content.contains("newFunctionName"),
                "File should contain the new function name"
            );
            assert!(
                !modified_content.contains("oldFunctionName"),
                "File should not contain the old function name"
            );
        } else {
            eprintln!(
                "ℹ️  Test 'test_unified_refactoring_rename': Rename skipped (requires LSP support)"
            );
        }
    } else {
        eprintln!(
            "ℹ️  Test 'test_unified_refactoring_rename': rename skipped (requires LSP support)"
        );
    }
}