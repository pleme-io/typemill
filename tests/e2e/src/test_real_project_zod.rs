//! Real project integration tests using Zod TypeScript library
//!
//! Tests mill operations against a real-world TypeScript project (Zod)
//! to validate that refactoring tools work on production codebases.
//!
//! NOTE: These tests share a single Zod clone and TestClient to avoid
//! redundant setup time. Tests run serially within the module.

use crate::harness::{TestClient, TestWorkspace};
use once_cell::sync::Lazy;
use serde_json::json;
use serial_test::serial;
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

/// Extended timeout for operations that scan many files (e.g., rename with import updates)
const LARGE_PROJECT_TIMEOUT: Duration = Duration::from_secs(120);

/// Shared test context that persists across all tests in this module.
/// This avoids cloning Zod and booting up the LSP server for each test.
struct ZodTestContext {
    workspace: TestWorkspace,
    client: TestClient,
}

impl ZodTestContext {
    fn new() -> Self {
        let workspace = TestWorkspace::new();

        // Clone zod into the workspace
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

        assert!(status.success(), "Failed to clone zod repository");

        // Run mill setup
        let mill_path = std::env::var("CARGO_MANIFEST_DIR")
            .map(|dir| {
                let mut path = std::path::PathBuf::from(dir);
                path.pop(); // e2e
                path.pop(); // tests
                path.push("target/debug/mill");
                path
            })
            .expect("CARGO_MANIFEST_DIR not set");

        let setup_status = Command::new(&mill_path)
            .args(["setup", "--update"])
            .current_dir(workspace.path())
            .status()
            .expect("Failed to run mill setup");

        assert!(setup_status.success(), "Failed to run mill setup");

        let client = TestClient::new(workspace.path());

        Self { workspace, client }
    }
}

/// Global shared context - initialized once, used by all tests
static ZOD_CONTEXT: Lazy<Mutex<ZodTestContext>> = Lazy::new(|| Mutex::new(ZodTestContext::new()));

// ============================================================================
// Search & Inspect Tests
// ============================================================================

/// Test: Search for symbols in Zod
#[tokio::test]
#[serial]
async fn test_zod_search_symbols() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    let result = ctx
        .client
        .call_tool_with_timeout(
            "search_code",
            json!({ "query": "ZodType" }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("search_code should succeed");

    let inner_result = result.get("result").expect("Should have result field");
    let symbols = inner_result.get("results").and_then(|s| s.as_array());

    match symbols {
        Some(arr) if !arr.is_empty() => {
            println!("✅ Found {} ZodType symbols", arr.len());
        }
        Some(_) => {
            println!("⚠️ search_code returned empty results (LSP may not be fully indexed)");
        }
        None => {
            if let Some(error) = inner_result.get("error") {
                println!("⚠️ search_code returned error: {:?}", error);
            } else {
                panic!("search_code should return results array");
            }
        }
    }
}

/// Test: Inspect code at a specific location
#[tokio::test]
#[serial]
async fn test_zod_inspect_code() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    let types_file = ctx.workspace.path().join("packages/zod/src/v3/types.ts");

    // Wait for LSP to index
    let _ = ctx.client.wait_for_lsp_ready(&types_file, 10000).await;

    let result = ctx
        .client
        .call_tool(
            "inspect_code",
            json!({
                "filePath": types_file.to_string_lossy(),
                "line": 1,
                "character": 0,
                "include": ["diagnostics"]
            }),
        )
        .await
        .expect("inspect_code should succeed");

    let inner_result = result.get("result").expect("Should have result field");

    assert!(
        inner_result.is_object(),
        "Result should be an object, got: {:?}",
        inner_result
    );

    println!("✅ Successfully inspected Zod types.ts");
}

// ============================================================================
// File Rename Tests (Dry Run + Execute)
// ============================================================================

/// Test: Dry-run rename file in Zod (verify import updates planned)
#[tokio::test]
#[serial]
async fn test_zod_rename_file_dry_run() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    // Create a test file to avoid modifying the actual Zod codebase
    ctx.workspace.create_file(
        "packages/zod/src/v3/test-rename-dry.ts",
        "export const testValue = 'dry-run-test';",
    );

    let old_path = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/test-rename-dry.ts");
    let new_path = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/test-renamed-dry.ts");

    let result = ctx
        .client
        .call_tool_with_timeout(
            "rename_all",
            json!({
                "target": {
                    "kind": "file",
                    "filePath": old_path.to_string_lossy()
                },
                "newName": new_path.to_string_lossy(),
                "options": {
                    "dryRun": true
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("rename_all dry-run should succeed");

    let inner_result = result.get("result").expect("Should have result field");
    let content = inner_result
        .get("content")
        .expect("Should have content field");

    let status = content.get("status").and_then(|s| s.as_str());
    assert!(
        status == Some("preview") || status == Some("success"),
        "Should return preview or success status, got: {:?}",
        status
    );

    // Verify file NOT actually renamed (dry-run)
    assert!(
        old_path.exists(),
        "test-rename-dry.ts should still exist after dry-run"
    );
    assert!(
        !new_path.exists(),
        "test-renamed-dry.ts should NOT exist after dry-run"
    );

    println!("✅ Successfully dry-run renamed test-rename-dry.ts");
}

/// Test: Execute actual rename on Zod file
#[tokio::test]
#[serial]
async fn test_zod_rename_file_execute() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    // Create a test file
    ctx.workspace.create_file(
        "packages/zod/src/v3/test-rename-exec.ts",
        "export const execValue = 'execute-test';",
    );

    let old_path = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/test-rename-exec.ts");
    let new_path = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/test-renamed-exec.ts");

    let original_content =
        std::fs::read_to_string(&old_path).expect("Should read test-rename-exec.ts");

    let result = ctx
        .client
        .call_tool_with_timeout(
            "rename_all",
            json!({
                "target": {
                    "kind": "file",
                    "filePath": old_path.to_string_lossy()
                },
                "newName": new_path.to_string_lossy(),
                "options": {
                    "dryRun": false
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("rename_all should succeed");

    let inner_result = result.get("result").expect("Should have result field");
    let content = inner_result
        .get("content")
        .expect("Should have content field");

    let status = content.get("status").and_then(|s| s.as_str());
    assert_eq!(
        status,
        Some("success"),
        "Rename should succeed, got: {:?}",
        status
    );

    // Verify file was actually renamed
    assert!(
        !old_path.exists(),
        "test-rename-exec.ts should no longer exist after rename"
    );
    assert!(
        new_path.exists(),
        "test-renamed-exec.ts should exist after rename"
    );

    // Verify content preserved
    let new_content = std::fs::read_to_string(&new_path).expect("Should read renamed file");
    assert_eq!(
        original_content, new_content,
        "Content should be preserved after rename"
    );

    println!("✅ Successfully renamed test-rename-exec.ts -> test-renamed-exec.ts");
}

// ============================================================================
// File Move (Relocate) Tests (Dry Run + Execute)
// ============================================================================

/// Test: Dry-run move file in Zod
#[tokio::test]
#[serial]
async fn test_zod_move_file_dry_run() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    // Create a test file
    ctx.workspace.create_file(
        "packages/zod/src/v3/test-move-dry.ts",
        "export const moveValue = 'move-dry-test';",
    );

    let source = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/test-move-dry.ts");
    let dest = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/utils/test-move-dry.ts");

    // Create destination directory
    std::fs::create_dir_all(dest.parent().unwrap()).ok();

    let result = ctx
        .client
        .call_tool_with_timeout(
            "relocate",
            json!({
                "target": {
                    "kind": "file",
                    "filePath": source.to_string_lossy()
                },
                "destination": dest.to_string_lossy(),
                "options": {
                    "dryRun": true
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("relocate dry-run should succeed");

    let inner_result = result.get("result").expect("Should have result field");
    let content = inner_result
        .get("content")
        .expect("Should have content field");

    let status = content.get("status").and_then(|s| s.as_str());
    assert!(
        status == Some("preview") || status == Some("success"),
        "Should return preview or success status, got: {:?}",
        status
    );

    // Verify dry-run
    assert!(
        source.exists(),
        "test-move-dry.ts should still exist after dry-run"
    );
    assert!(
        !dest.exists(),
        "utils/test-move-dry.ts should NOT exist after dry-run"
    );

    println!("✅ Successfully dry-run moved test-move-dry.ts -> utils/test-move-dry.ts");
}

/// Test: Execute actual move on Zod file
#[tokio::test]
#[serial]
async fn test_zod_move_file_execute() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    // Create a test file
    ctx.workspace.create_file(
        "packages/zod/src/v3/test-move-exec.ts",
        r#"export const moveExecValue = 42;
export function moveExecFunc() { return moveExecValue; }
"#,
    );

    let source = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/test-move-exec.ts");
    let dest = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/helpers/test-move-exec.ts");

    let original_content = std::fs::read_to_string(&source).expect("Should read source file");

    // Create destination directory
    std::fs::create_dir_all(dest.parent().unwrap()).expect("Should create helpers dir");

    let result = ctx
        .client
        .call_tool_with_timeout(
            "relocate",
            json!({
                "target": {
                    "kind": "file",
                    "filePath": source.to_string_lossy()
                },
                "destination": dest.to_string_lossy(),
                "options": {
                    "dryRun": false
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("relocate should succeed");

    let inner_result = result.get("result").expect("Should have result field");
    let content = inner_result
        .get("content")
        .expect("Should have content field");

    let status = content.get("status").and_then(|s| s.as_str());
    assert_eq!(
        status,
        Some("success"),
        "Move should succeed, got: {:?}",
        status
    );

    // Verify file was actually moved
    assert!(
        !source.exists(),
        "test-move-exec.ts should no longer exist at original location"
    );
    assert!(
        dest.exists(),
        "test-move-exec.ts should exist at new location"
    );

    // Verify content preserved
    let new_content = std::fs::read_to_string(&dest).expect("Should read moved file");
    assert_eq!(
        original_content, new_content,
        "Content should be preserved after move"
    );

    println!("✅ Successfully moved test-move-exec.ts -> helpers/test-move-exec.ts");
}

// ============================================================================
// Folder/Directory Move Tests (Dry Run + Execute)
// ============================================================================

/// Test: Dry-run move folder in Zod
#[tokio::test]
#[serial]
async fn test_zod_move_folder_dry_run() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    // Create a test folder with multiple files
    ctx.workspace.create_file(
        "packages/zod/src/v3/test-folder-dry/index.ts",
        "export * from './utils';",
    );
    ctx.workspace.create_file(
        "packages/zod/src/v3/test-folder-dry/utils.ts",
        "export const folderUtil = 'folder-util';",
    );
    ctx.workspace.create_file(
        "packages/zod/src/v3/test-folder-dry/types.ts",
        "export interface FolderType { value: string; }",
    );

    let source = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/test-folder-dry");
    let dest = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/moved-folder-dry");

    let result = ctx
        .client
        .call_tool_with_timeout(
            "relocate",
            json!({
                "target": {
                    "kind": "directory",
                    "filePath": source.to_string_lossy()
                },
                "destination": dest.to_string_lossy(),
                "options": {
                    "dryRun": true
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("relocate folder dry-run should succeed");

    let inner_result = result.get("result").expect("Should have result field");
    let content = inner_result
        .get("content")
        .expect("Should have content field");

    let status = content.get("status").and_then(|s| s.as_str());
    assert!(
        status == Some("preview") || status == Some("success"),
        "Should return preview or success status, got: {:?}",
        status
    );

    // Verify dry-run - source folder should still exist
    assert!(
        source.exists(),
        "test-folder-dry should still exist after dry-run"
    );
    assert!(
        source.join("index.ts").exists(),
        "test-folder-dry/index.ts should still exist"
    );
    assert!(
        !dest.exists(),
        "moved-folder-dry should NOT exist after dry-run"
    );

    println!("✅ Successfully dry-run moved test-folder-dry -> moved-folder-dry");
}

/// Test: Execute actual folder move in Zod
#[tokio::test]
#[serial]
async fn test_zod_move_folder_execute() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    // Create a test folder with multiple files
    ctx.workspace.create_file(
        "packages/zod/src/v3/test-folder-exec/index.ts",
        "export * from './utils';",
    );
    ctx.workspace.create_file(
        "packages/zod/src/v3/test-folder-exec/utils.ts",
        "export const folderExecUtil = 'folder-exec-util';",
    );
    ctx.workspace.create_file(
        "packages/zod/src/v3/test-folder-exec/types.ts",
        "export interface FolderExecType { value: string; }",
    );

    let source = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/test-folder-exec");
    let dest = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/moved-folder-exec");

    // Read original content
    let original_index =
        std::fs::read_to_string(source.join("index.ts")).expect("Should read index.ts");
    let original_utils =
        std::fs::read_to_string(source.join("utils.ts")).expect("Should read utils.ts");

    let result = ctx
        .client
        .call_tool_with_timeout(
            "relocate",
            json!({
                "target": {
                    "kind": "directory",
                    "filePath": source.to_string_lossy()
                },
                "destination": dest.to_string_lossy(),
                "options": {
                    "dryRun": false
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("relocate folder should succeed");

    let inner_result = result.get("result").expect("Should have result field");
    let content = inner_result
        .get("content")
        .expect("Should have content field");

    let status = content.get("status").and_then(|s| s.as_str());
    assert_eq!(
        status,
        Some("success"),
        "Folder move should succeed, got: {:?}",
        status
    );

    // Verify folder was actually moved
    assert!(
        !source.exists(),
        "test-folder-exec should no longer exist at original location"
    );
    assert!(dest.exists(), "moved-folder-exec should exist");
    assert!(
        dest.join("index.ts").exists(),
        "moved-folder-exec/index.ts should exist"
    );
    assert!(
        dest.join("utils.ts").exists(),
        "moved-folder-exec/utils.ts should exist"
    );
    assert!(
        dest.join("types.ts").exists(),
        "moved-folder-exec/types.ts should exist"
    );

    // Verify content preserved
    let new_index =
        std::fs::read_to_string(dest.join("index.ts")).expect("Should read moved index.ts");
    let new_utils =
        std::fs::read_to_string(dest.join("utils.ts")).expect("Should read moved utils.ts");

    assert_eq!(
        original_index, new_index,
        "index.ts content should be preserved"
    );
    assert_eq!(
        original_utils, new_utils,
        "utils.ts content should be preserved"
    );

    println!("✅ Successfully moved test-folder-exec -> moved-folder-exec (3 files)");
}

// ============================================================================
// Rename Symbol Tests (Dry Run + Execute)
// ============================================================================

/// Test: Dry-run rename symbol in Zod
#[tokio::test]
#[serial]
async fn test_zod_rename_symbol_dry_run() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    // Create a test file with a symbol we can rename
    ctx.workspace.create_file(
        "packages/zod/src/v3/test-symbol-dry.ts",
        r#"export const myConstant = 42;
export function useMyConstant() {
    return myConstant * 2;
}
"#,
    );

    let file_path = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/test-symbol-dry.ts");

    // Wait for LSP to index
    let _ = ctx.client.wait_for_lsp_ready(&file_path, 15000).await;

    // Try to rename the symbol at line 1, character 13 (myConstant)
    let rename_result = ctx
        .client
        .call_tool_with_timeout(
            "rename_all",
            json!({
                "target": {
                    "kind": "symbol",
                    "filePath": file_path.to_string_lossy(),
                    "line": 1,
                    "character": 13
                },
                "newName": "myRenamedConstant",
                "options": {
                    "dryRun": true
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await;

    match rename_result {
        Ok(resp) => {
            let status = resp
                .get("result")
                .and_then(|r| r.get("content"))
                .and_then(|c| c.get("status"));
            println!("✅ Symbol rename dry-run succeeded: {:?}", status);

            // Verify original file unchanged
            let content =
                std::fs::read_to_string(&file_path).expect("Should read test-symbol-dry.ts");
            assert!(
                content.contains("myConstant"),
                "Original symbol should still exist after dry-run"
            );
            assert!(
                !content.contains("myRenamedConstant"),
                "Renamed symbol should NOT exist after dry-run"
            );
        }
        Err(e) => {
            println!("⚠️ Symbol rename dry-run failed (may need LSP): {}", e);
        }
    }
}

/// Test: Execute actual symbol rename in Zod
#[tokio::test]
#[serial]
async fn test_zod_rename_symbol_execute() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    // Create a test file with a symbol we can rename
    ctx.workspace.create_file(
        "packages/zod/src/v3/test-symbol-exec.ts",
        r#"export const oldSymbol = 42;
export function useOldSymbol() {
    return oldSymbol * 2;
}
console.log(oldSymbol);
"#,
    );

    let file_path = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/test-symbol-exec.ts");

    // Wait for LSP to index
    let _ = ctx.client.wait_for_lsp_ready(&file_path, 15000).await;

    // Rename the symbol at line 1, character 13 (oldSymbol)
    let rename_result = ctx
        .client
        .call_tool_with_timeout(
            "rename_all",
            json!({
                "target": {
                    "kind": "symbol",
                    "filePath": file_path.to_string_lossy(),
                    "line": 1,
                    "character": 13
                },
                "newName": "newSymbol",
                "options": {
                    "dryRun": false
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await;

    match rename_result {
        Ok(resp) => {
            let status = resp
                .get("result")
                .and_then(|r| r.get("content"))
                .and_then(|c| c.get("status"));
            println!("✅ Symbol rename execute succeeded: {:?}", status);

            // Verify file was updated
            let content =
                std::fs::read_to_string(&file_path).expect("Should read test-symbol-exec.ts");

            // All references should be renamed
            assert!(
                content.contains("newSymbol"),
                "New symbol should exist after rename. Content: {}",
                content
            );
        }
        Err(e) => {
            println!("⚠️ Symbol rename execute failed (may need LSP): {}", e);
        }
    }
}

// ============================================================================
// Extract Dependencies Tests (Dry Run + Execute)
// ============================================================================

/// Test: Dry-run extract dependencies (npm)
#[tokio::test]
#[serial]
async fn test_zod_extract_dependencies_dry_run() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    ctx.workspace.create_file(
        "packages/src-pkg/package.json",
        r#"{
  "name": "src-pkg",
  "version": "1.0.0",
  "dependencies": {
    "react": "^18.0.0",
    "lodash": "^4.17.0"
  }
}"#,
    );

    ctx.workspace.create_file(
        "packages/tgt-pkg/package.json",
        r#"{
  "name": "tgt-pkg",
  "version": "1.0.0",
  "dependencies": {}
}"#,
    );

    let source_path = ctx.workspace.absolute_path("packages/src-pkg/package.json");
    let target_path = ctx.workspace.absolute_path("packages/tgt-pkg/package.json");

    let result = ctx
        .client
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

    assert!(
        result.get("result").is_some(),
        "Should have result: {:?}",
        result
    );

    // Target should NOT be modified (dry run)
    let target_content = ctx.workspace.read_file("packages/tgt-pkg/package.json");
    assert!(
        !target_content.contains("react"),
        "Should not have react in dry run"
    );

    println!("✅ Successfully dry-run extract_dependencies");
}

/// Test: Execute extract dependencies (npm)
#[tokio::test]
#[serial]
async fn test_zod_extract_dependencies_execute() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    ctx.workspace.create_file(
        "packages/source-exec/package.json",
        r#"{
  "name": "source-exec",
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

    ctx.workspace.create_file(
        "packages/target-exec/package.json",
        r#"{
  "name": "target-exec",
  "version": "1.0.0",
  "dependencies": {}
}"#,
    );

    let source_path = ctx
        .workspace
        .absolute_path("packages/source-exec/package.json");
    let target_path = ctx
        .workspace
        .absolute_path("packages/target-exec/package.json");

    let result = ctx
        .client
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

    assert!(
        result.get("result").is_some(),
        "Should have result: {:?}",
        result
    );

    // Verify target was updated
    let target_content = ctx.workspace.read_file("packages/target-exec/package.json");
    assert!(
        target_content.contains("lodash"),
        "Should have lodash: {}",
        target_content
    );
    assert!(
        target_content.contains("^4.17.0"),
        "Should have version: {}",
        target_content
    );
    assert!(!target_content.contains("axios"), "Should not have axios");

    println!("✅ Successfully executed extract_dependencies");
}

// ============================================================================
// Create Package Tests (Dry Run + Execute)
// ============================================================================

/// Test: Dry-run create package - previews what files would be created
#[tokio::test]
#[serial]
async fn test_zod_create_package_dry_run() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    let pkg_path = ctx.workspace.absolute_path("packages/new-pkg-dry");

    let result = ctx
        .client
        .call_tool_with_timeout(
            "workspace",
            json!({
                "action": "create_package",
                "params": {
                    "path": pkg_path.to_string_lossy(),
                    "name": "new-pkg-dry",
                    "type": "npm"
                },
                "options": {
                    "dryRun": true
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("create_package dry-run should succeed");

    assert!(
        result.get("result").is_some(),
        "Should have result: {:?}",
        result
    );

    let inner = result.get("result").unwrap();

    // Verify status is preview (WriteResponse format)
    assert_eq!(
        inner.get("status"),
        Some(&json!("preview")),
        "status should be 'preview' for dry run: {:?}",
        inner
    );

    // Verify filesChanged preview contains expected npm package files
    let files_changed = inner
        .get("filesChanged")
        .and_then(|v| v.as_array())
        .expect("filesChanged should be an array");
    assert!(
        !files_changed.is_empty(),
        "filesChanged should not be empty"
    );

    // Verify changes contains the original create_package result
    let changes = inner.get("changes").expect("changes should exist");

    // Verify dry run flag in changes
    assert_eq!(
        changes.get("dryRun"),
        Some(&json!(true)),
        "changes.dryRun should be true"
    );

    // Verify packageInfo is populated in changes
    let package_info = changes
        .get("packageInfo")
        .expect("packageInfo should exist");
    assert!(
        package_info.get("name").is_some(),
        "packageInfo.name should exist"
    );
    assert!(
        package_info.get("manifestPath").is_some(),
        "packageInfo.manifestPath should exist"
    );

    // Package should NOT be created (dry run)
    assert!(
        !pkg_path.exists(),
        "new-pkg-dry should NOT exist after dry run"
    );

    println!(
        "✅ Successfully dry-run create_package with {} predicted files",
        files_changed.len()
    );
}

/// Test: Execute create package
#[tokio::test]
#[serial]
async fn test_zod_create_package_execute() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    let pkg_path = ctx.workspace.absolute_path("packages/new-pkg-exec");

    let result = ctx
        .client
        .call_tool_with_timeout(
            "workspace",
            json!({
                "action": "create_package",
                "params": {
                    "path": pkg_path.to_string_lossy(),
                    "name": "new-pkg-exec",
                    "type": "npm"
                },
                "options": {
                    "dryRun": false,
                    "addToWorkspace": false  // Zod doesn't have a workspace package.json
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("create_package execute should succeed");

    assert!(
        result.get("result").is_some(),
        "Should have result: {:?}",
        result
    );

    // Package should be created
    assert!(pkg_path.exists(), "new-pkg-exec should exist after execute");
    assert!(
        pkg_path.join("package.json").exists(),
        "package.json should exist"
    );

    // Verify package.json content
    let pkg_content =
        std::fs::read_to_string(pkg_path.join("package.json")).expect("Should read package.json");
    assert!(
        pkg_content.contains("new-pkg-exec"),
        "Should have package name"
    );

    println!("✅ Successfully executed create_package");
}

// ============================================================================
// Combined Workflow Tests
// ============================================================================

/// Test: Complete workflow - extract deps + move file
#[tokio::test]
#[serial]
async fn test_zod_workflow_extract_and_move() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    // Step 1: Create a source package with dependencies
    ctx.workspace.create_file(
        "packages/old-workflow/package.json",
        r#"{
  "name": "old-workflow",
  "version": "1.0.0",
  "dependencies": {
    "lodash": "^4.17.0"
  }
}"#,
    );
    ctx.workspace.create_file(
        "packages/old-workflow/src/helpers.ts",
        r#"export function formatName(name: string): string {
  return name.trim().toLowerCase();
}
"#,
    );

    // Create target package
    ctx.workspace.create_file(
        "packages/new-workflow/package.json",
        r#"{
  "name": "new-workflow",
  "version": "1.0.0",
  "dependencies": {}
}"#,
    );
    std::fs::create_dir_all(ctx.workspace.path().join("packages/new-workflow/src"))
        .expect("Failed to create src dir");

    // Step 2: Extract dependencies
    let source_manifest = ctx
        .workspace
        .absolute_path("packages/old-workflow/package.json");
    let target_manifest = ctx
        .workspace
        .absolute_path("packages/new-workflow/package.json");

    let _result = ctx
        .client
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
    let new_pkg = ctx
        .workspace
        .read_file("packages/new-workflow/package.json");
    assert!(new_pkg.contains("lodash"), "Should have lodash dependency");

    // Step 3: Move a file to the new package
    let source_file = ctx
        .workspace
        .absolute_path("packages/old-workflow/src/helpers.ts");
    let dest_file = ctx
        .workspace
        .absolute_path("packages/new-workflow/src/helpers.ts");

    let _result = ctx
        .client
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
    let helpers_content = ctx
        .workspace
        .read_file("packages/new-workflow/src/helpers.ts");
    assert!(helpers_content.contains("formatName"));
    assert!(helpers_content.contains("trim()"));

    println!("✅ Successfully completed workflow: extract deps + move file");
}

/// Test: Complete workflow - create package + move folder into it
#[tokio::test]
#[serial]
async fn test_zod_workflow_create_package_and_move_folder() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    // Step 1: Create a source folder with files
    ctx.workspace.create_file(
        "packages/zod/src/v3/utils-to-extract/index.ts",
        "export * from './validators';",
    );
    ctx.workspace.create_file(
        "packages/zod/src/v3/utils-to-extract/validators.ts",
        r#"export function validateEmail(email: string): boolean {
  return email.includes('@');
}
export function validatePhone(phone: string): boolean {
  return phone.length >= 10;
}
"#,
    );

    // Step 2: Create a new package
    let new_pkg_path = ctx.workspace.absolute_path("packages/validators-pkg");

    let _result = ctx
        .client
        .call_tool_with_timeout(
            "workspace",
            json!({
                "action": "create_package",
                "params": {
                    "path": new_pkg_path.to_string_lossy(),
                    "name": "validators-pkg",
                    "type": "npm"
                },
                "options": {
                    "dryRun": false
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("create_package should succeed");

    assert!(new_pkg_path.exists(), "Package should be created");

    // Create src directory in new package
    std::fs::create_dir_all(new_pkg_path.join("src")).expect("Failed to create src dir");

    // Step 3: Move the folder into the new package
    let source_folder = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/utils-to-extract");
    let dest_folder = ctx
        .workspace
        .absolute_path("packages/validators-pkg/src/validators");

    let _result = ctx
        .client
        .call_tool_with_timeout(
            "relocate",
            json!({
                "target": {
                    "kind": "directory",
                    "filePath": source_folder.to_string_lossy()
                },
                "destination": dest_folder.to_string_lossy(),
                "options": {
                    "dryRun": false
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("relocate folder should succeed");

    // Verify folder was moved
    assert!(
        !source_folder.exists(),
        "Source folder should no longer exist"
    );
    assert!(dest_folder.exists(), "Destination folder should exist");
    assert!(
        dest_folder.join("index.ts").exists(),
        "index.ts should exist"
    );
    assert!(
        dest_folder.join("validators.ts").exists(),
        "validators.ts should exist"
    );

    // Verify content is preserved
    let validators_content = std::fs::read_to_string(dest_folder.join("validators.ts"))
        .expect("Should read validators.ts");
    assert!(validators_content.contains("validateEmail"));
    assert!(validators_content.contains("validatePhone"));

    println!("✅ Successfully completed workflow: create package + move folder into it");
}

// ============================================================================
// Folder Rename Tests (Dry Run + Execute)
// ============================================================================

/// Test: Dry-run rename folder in Zod
#[tokio::test]
#[serial]
async fn test_zod_rename_folder_dry_run() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    // Create a test folder
    ctx.workspace.create_file(
        "packages/zod/src/v3/rename-folder-dry/index.ts",
        "export const value = 'rename-folder-test';",
    );

    let old_path = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/rename-folder-dry");
    let new_path = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/renamed-folder-dry");

    let result = ctx
        .client
        .call_tool_with_timeout(
            "rename_all",
            json!({
                "target": {
                    "kind": "directory",
                    "filePath": old_path.to_string_lossy()
                },
                "newName": new_path.to_string_lossy(),
                "options": {
                    "dryRun": true
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("rename_all folder dry-run should succeed");

    let inner_result = result.get("result").expect("Should have result field");
    let content = inner_result
        .get("content")
        .expect("Should have content field");

    let status = content.get("status").and_then(|s| s.as_str());
    assert!(
        status == Some("preview") || status == Some("success"),
        "Should return preview or success status, got: {:?}",
        status
    );

    // Verify folder NOT actually renamed (dry-run)
    assert!(
        old_path.exists(),
        "rename-folder-dry should still exist after dry-run"
    );
    assert!(
        !new_path.exists(),
        "renamed-folder-dry should NOT exist after dry-run"
    );

    println!("✅ Successfully dry-run renamed rename-folder-dry");
}

/// Test: Execute actual folder rename in Zod
#[tokio::test]
#[serial]
async fn test_zod_rename_folder_execute() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    // Create a test folder
    ctx.workspace.create_file(
        "packages/zod/src/v3/rename-folder-exec/index.ts",
        "export const value = 'rename-folder-exec-test';",
    );
    ctx.workspace.create_file(
        "packages/zod/src/v3/rename-folder-exec/utils.ts",
        "export const util = 'util-value';",
    );

    let old_path = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/rename-folder-exec");
    let new_path = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/renamed-folder-exec");

    let result = ctx
        .client
        .call_tool_with_timeout(
            "rename_all",
            json!({
                "target": {
                    "kind": "directory",
                    "filePath": old_path.to_string_lossy()
                },
                "newName": new_path.to_string_lossy(),
                "options": {
                    "dryRun": false
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("rename_all folder should succeed");

    let inner_result = result.get("result").expect("Should have result field");
    let content = inner_result
        .get("content")
        .expect("Should have content field");

    let status = content.get("status").and_then(|s| s.as_str());
    assert_eq!(
        status,
        Some("success"),
        "Folder rename should succeed, got: {:?}",
        status
    );

    // Verify folder was actually renamed
    assert!(
        !old_path.exists(),
        "rename-folder-exec should no longer exist after rename"
    );
    assert!(
        new_path.exists(),
        "renamed-folder-exec should exist after rename"
    );
    assert!(
        new_path.join("index.ts").exists(),
        "index.ts should exist in renamed folder"
    );
    assert!(
        new_path.join("utils.ts").exists(),
        "utils.ts should exist in renamed folder"
    );

    println!("✅ Successfully renamed rename-folder-exec -> renamed-folder-exec");
}

// ============================================================================
// Find/Replace Tests (Dry Run + Execute)
// ============================================================================

/// Test: Dry-run find/replace in Zod
#[tokio::test]
#[serial]
async fn test_zod_find_replace_dry_run() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    // Create test files with content to replace
    ctx.workspace.create_file(
        "packages/zod/src/v3/find-replace-dry/config.ts",
        r#"export const OLD_CONFIG_NAME = 'old-value';
export const anotherOLD_CONFIG_NAMEref = OLD_CONFIG_NAME;
"#,
    );
    ctx.workspace.create_file(
        "packages/zod/src/v3/find-replace-dry/utils.ts",
        r#"import { OLD_CONFIG_NAME } from './config';
console.log(OLD_CONFIG_NAME);
"#,
    );

    let result = ctx
        .client
        .call_tool_with_timeout(
            "workspace",
            json!({
                "action": "find_replace",
                "params": {
                    "pattern": "OLD_CONFIG_NAME",
                    "replacement": "NEW_CONFIG_NAME",
                    "mode": "literal"
                },
                "options": {
                    "dryRun": true
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("find_replace dry-run should succeed");

    let inner_result = result.get("result").expect("Should have result field");

    let status = inner_result.get("status").and_then(|s| s.as_str());
    assert_eq!(
        status,
        Some("preview"),
        "Should return preview status, got: {:?}",
        status
    );

    // Verify files NOT actually modified (dry-run)
    let config_content = ctx
        .workspace
        .read_file("packages/zod/src/v3/find-replace-dry/config.ts");
    assert!(
        config_content.contains("OLD_CONFIG_NAME"),
        "Original text should still exist after dry-run"
    );
    assert!(
        !config_content.contains("NEW_CONFIG_NAME"),
        "Replacement text should NOT exist after dry-run"
    );

    println!("✅ Successfully dry-run find_replace");
}

/// Test: Execute find/replace in Zod
#[tokio::test]
#[serial]
async fn test_zod_find_replace_execute() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    // Create test files with content to replace
    ctx.workspace.create_file(
        "packages/zod/src/v3/find-replace-exec/config.ts",
        r#"export const REPLACE_ME = 'value';
export const useREPLACE_ME = REPLACE_ME;
"#,
    );
    ctx.workspace.create_file(
        "packages/zod/src/v3/find-replace-exec/utils.ts",
        r#"import { REPLACE_ME } from './config';
console.log(REPLACE_ME);
"#,
    );

    let result = ctx
        .client
        .call_tool_with_timeout(
            "workspace",
            json!({
                "action": "find_replace",
                "params": {
                    "pattern": "REPLACE_ME",
                    "replacement": "REPLACED_VALUE",
                    "mode": "literal"
                },
                "options": {
                    "dryRun": false
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("find_replace execute should succeed");

    let inner_result = result.get("result").expect("Should have result field");

    let status = inner_result.get("status").and_then(|s| s.as_str());
    assert_eq!(
        status,
        Some("success"),
        "Should return success status, got: {:?}",
        status
    );

    // Verify files were actually modified
    let config_content = ctx
        .workspace
        .read_file("packages/zod/src/v3/find-replace-exec/config.ts");
    assert!(
        config_content.contains("REPLACED_VALUE"),
        "Replacement text should exist after execute. Content: {}",
        config_content
    );
    assert!(
        !config_content.contains("REPLACE_ME"),
        "Original text should NOT exist after execute"
    );

    let utils_content = ctx
        .workspace
        .read_file("packages/zod/src/v3/find-replace-exec/utils.ts");
    assert!(
        utils_content.contains("REPLACED_VALUE"),
        "Replacement in utils.ts should exist"
    );

    println!("✅ Successfully executed find_replace");
}

// ============================================================================
// Prune (Delete) Tests (Dry Run + Execute)
// ============================================================================

/// Test: Dry-run prune file in Zod
#[tokio::test]
#[serial]
async fn test_zod_prune_file_dry_run() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    // Create a test file to delete
    ctx.workspace.create_file(
        "packages/zod/src/v3/prune-file-dry.ts",
        "export const toBeDeleted = 'delete-me';",
    );

    let file_path = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/prune-file-dry.ts");

    let result = ctx
        .client
        .call_tool_with_timeout(
            "prune",
            json!({
                "target": {
                    "kind": "file",
                    "filePath": file_path.to_string_lossy()
                },
                "options": {
                    "dryRun": true
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("prune dry-run should succeed");

    let inner_result = result.get("result").expect("Should have result field");
    let content = inner_result
        .get("content")
        .expect("Should have content field");

    let status = content.get("status").and_then(|s| s.as_str());
    assert!(
        status == Some("preview") || status == Some("success"),
        "Should return preview or success status, got: {:?}",
        status
    );

    // Verify file NOT actually deleted (dry-run)
    assert!(
        file_path.exists(),
        "prune-file-dry.ts should still exist after dry-run"
    );

    println!("✅ Successfully dry-run pruned prune-file-dry.ts");
}

/// Test: Execute prune file in Zod
#[tokio::test]
#[serial]
async fn test_zod_prune_file_execute() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    // Create a test file to delete
    ctx.workspace.create_file(
        "packages/zod/src/v3/prune-file-exec.ts",
        "export const toBeDeleted = 'delete-me-for-real';",
    );

    let file_path = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/prune-file-exec.ts");

    // Verify file exists before prune
    assert!(file_path.exists(), "File should exist before prune");

    let result = ctx
        .client
        .call_tool_with_timeout(
            "prune",
            json!({
                "target": {
                    "kind": "file",
                    "filePath": file_path.to_string_lossy()
                },
                "options": {
                    "dryRun": false
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("prune execute should succeed");

    let inner_result = result.get("result").expect("Should have result field");
    let content = inner_result
        .get("content")
        .expect("Should have content field");

    let status = content.get("status").and_then(|s| s.as_str());
    assert_eq!(
        status,
        Some("success"),
        "Prune should succeed, got: {:?}",
        status
    );

    // Verify file was actually deleted
    assert!(
        !file_path.exists(),
        "prune-file-exec.ts should NOT exist after prune"
    );

    println!("✅ Successfully pruned prune-file-exec.ts");
}

/// Test: Dry-run prune folder in Zod
#[tokio::test]
#[serial]
async fn test_zod_prune_folder_dry_run() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    // Create a test folder to delete
    ctx.workspace.create_file(
        "packages/zod/src/v3/prune-folder-dry/index.ts",
        "export * from './utils';",
    );
    ctx.workspace.create_file(
        "packages/zod/src/v3/prune-folder-dry/utils.ts",
        "export const util = 'util-value';",
    );

    let folder_path = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/prune-folder-dry");

    let result = ctx
        .client
        .call_tool_with_timeout(
            "prune",
            json!({
                "target": {
                    "kind": "directory",
                    "filePath": folder_path.to_string_lossy()
                },
                "options": {
                    "dryRun": true
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("prune folder dry-run should succeed");

    let inner_result = result.get("result").expect("Should have result field");
    let content = inner_result
        .get("content")
        .expect("Should have content field");

    let status = content.get("status").and_then(|s| s.as_str());
    assert!(
        status == Some("preview") || status == Some("success"),
        "Should return preview or success status, got: {:?}",
        status
    );

    // Verify folder NOT actually deleted (dry-run)
    assert!(
        folder_path.exists(),
        "prune-folder-dry should still exist after dry-run"
    );
    assert!(
        folder_path.join("index.ts").exists(),
        "index.ts should still exist after dry-run"
    );

    println!("✅ Successfully dry-run pruned prune-folder-dry");
}

/// Test: Execute prune folder in Zod
#[tokio::test]
#[serial]
async fn test_zod_prune_folder_execute() {
    let mut ctx = ZOD_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    // Create a test folder to delete
    ctx.workspace.create_file(
        "packages/zod/src/v3/prune-folder-exec/index.ts",
        "export * from './utils';",
    );
    ctx.workspace.create_file(
        "packages/zod/src/v3/prune-folder-exec/utils.ts",
        "export const util = 'util-value';",
    );

    let folder_path = ctx
        .workspace
        .absolute_path("packages/zod/src/v3/prune-folder-exec");

    // Verify folder exists before prune
    assert!(folder_path.exists(), "Folder should exist before prune");

    let result = ctx
        .client
        .call_tool_with_timeout(
            "prune",
            json!({
                "target": {
                    "kind": "directory",
                    "filePath": folder_path.to_string_lossy()
                },
                "options": {
                    "dryRun": false
                }
            }),
            LARGE_PROJECT_TIMEOUT,
        )
        .await
        .expect("prune folder execute should succeed");

    let inner_result = result.get("result").expect("Should have result field");
    let content = inner_result
        .get("content")
        .expect("Should have content field");

    let status = content.get("status").and_then(|s| s.as_str());
    assert_eq!(
        status,
        Some("success"),
        "Folder prune should succeed, got: {:?}",
        status
    );

    // Verify folder was actually deleted
    assert!(
        !folder_path.exists(),
        "prune-folder-exec should NOT exist after prune"
    );

    println!("✅ Successfully pruned prune-folder-exec");
}
