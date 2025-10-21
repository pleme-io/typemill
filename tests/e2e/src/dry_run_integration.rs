//! Dry run integration tests for Unified Refactoring API
//!
//! This test suite ensures that workspace.apply_edit with dry_run=true does not
//! modify the file system. This is critical for safety and user trust.
//!
//! NOTE: These tests use the Unified Refactoring API pattern:
//! 1. Generate a plan with *.plan() command
//! 2. Apply with workspace.apply_edit(plan, { dry_run: true })
//! 3. Verify no file system modifications occurred

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

#[tokio::test]
async fn test_rename_file_dry_run_does_not_modify_disk() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a test file
    workspace.create_file("original.txt", "content");
    let old_file_path = workspace.path().join("original.txt");
    let new_file_path = workspace.path().join("renamed.txt");

    // 2. Generate rename plan using unified API
    let plan_response = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": old_file_path.to_str().unwrap()
                },
                "new_name": new_file_path.to_str().unwrap()
            }),
        )
        .await
        .unwrap();

    // Debug: Print the full response to understand what's happening
    eprintln!(
        "DEBUG plan_response: {}",
        serde_json::to_string_pretty(&plan_response).unwrap()
    );

    // Check if there's an error instead of a result
    if !plan_response["error"].is_null() {
        panic!("rename.plan failed with error: {}", plan_response["error"]);
    }

    let plan = &plan_response["result"]["content"];
    assert_eq!(
        plan["plan_type"], "RenamePlan",
        "Should generate a RenamePlan"
    );

    // 3. Apply plan with dry_run=true
    let apply_response = client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dry_run": true
                }
            }),
        )
        .await
        .unwrap();

    let result = &apply_response["result"]["content"];
    assert_eq!(result["success"], true, "Dry run should succeed");

    // 4. CRITICAL: Verify file system is unchanged
    assert!(
        workspace.file_exists("original.txt"),
        "Original file should still exist after dry run"
    );
    assert!(
        !workspace.file_exists("renamed.txt"),
        "New file should NOT exist after dry run"
    );
    assert_eq!(
        workspace.read_file("original.txt"),
        "content",
        "Original file content should be unchanged"
    );
}

#[tokio::test]
async fn test_create_file_dry_run_does_not_create_file() {
    // NOTE: File creation is typically part of extract/move operations in unified API
    // This test demonstrates using delete.plan (which can represent file operations)
    // For creating files outside refactoring context, use FileService directly or write_file utility

    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a source file to extract from
    workspace.create_file("source.rs", "const VALUE: i32 = 42;");
    let source_path = workspace.path().join("source.rs");
    let new_file_path = workspace.path().join("extracted.rs");

    // 2. Generate extract plan that creates a new file
    let plan_response = client
        .call_tool(
            "extract.plan",
            json!({
                "kind": "constant",
                "source": {
                    "file_path": source_path.to_str().unwrap(),
                    "range": {
                        "start": { "line": 0, "character": 0 },
                        "end": { "line": 0, "character": 23 }
                    },
                    "name": "VALUE",
                    "destination": new_file_path.to_str().unwrap()
                }
            }),
        )
        .await
        .unwrap();

    let plan = &plan_response["result"]["content"];
    assert_eq!(
        plan["plan_type"], "ExtractPlan",
        "Should generate an ExtractPlan"
    );

    // 3. Apply plan with dry_run=true
    let apply_response = client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dry_run": true
                }
            }),
        )
        .await
        .unwrap();

    let result = &apply_response["result"]["content"];
    assert_eq!(result["success"], true, "Dry run should succeed");

    // 4. CRITICAL: Verify no new file was created
    assert!(
        !workspace.file_exists("extracted.rs"),
        "New file should NOT be created after dry run"
    );
    assert!(
        workspace.file_exists("source.rs"),
        "Source file should still exist"
    );
}

#[tokio::test]
async fn test_delete_file_dry_run_does_not_delete_file() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a file to delete
    workspace.create_file("to_delete.txt", "important content");
    let file_path = workspace.path().join("to_delete.txt");

    // 2. Generate delete plan using unified API
    let plan_response = client
        .call_tool(
            "delete.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": file_path.to_str().unwrap()
                }
            }),
        )
        .await
        .unwrap();

    let plan = &plan_response["result"]["content"];
    assert_eq!(
        plan["plan_type"], "DeletePlan",
        "Should generate a DeletePlan"
    );

    // 3. Apply plan with dry_run=true
    let apply_response = client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dry_run": true
                }
            }),
        )
        .await
        .unwrap();

    let result = &apply_response["result"]["content"];
    assert_eq!(result["success"], true, "Dry run should succeed");

    // 4. CRITICAL: Verify file still exists
    assert!(
        workspace.file_exists("to_delete.txt"),
        "File should still exist after dry run delete"
    );
    assert_eq!(
        workspace.read_file("to_delete.txt"),
        "important content",
        "File content should be unchanged"
    );
}

#[tokio::test]
async fn test_rename_directory_dry_run_does_not_modify_disk() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a directory with a file
    workspace.create_directory("old_dir");
    workspace.create_file("old_dir/file.txt", "content in directory");
    let old_dir = workspace.path().join("old_dir");
    let new_dir = workspace.path().join("new_dir");

    // 2. Generate rename plan for directory using unified API
    let plan_response = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "directory",
                    "path": old_dir.to_str().unwrap()
                },
                "new_name": new_dir.to_str().unwrap()
            }),
        )
        .await
        .unwrap();

    let plan = &plan_response["result"]["content"];
    assert_eq!(
        plan["plan_type"], "RenamePlan",
        "Should generate a RenamePlan"
    );

    // 3. Apply plan with dry_run=true
    let apply_response = client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dry_run": true
                }
            }),
        )
        .await
        .unwrap();

    let result = &apply_response["result"]["content"];
    assert_eq!(result["success"], true, "Dry run should succeed");

    // 4. CRITICAL: Verify directory is unchanged
    assert!(
        workspace.file_exists("old_dir"),
        "Original directory should still exist after dry run"
    );
    assert!(
        !workspace.file_exists("new_dir"),
        "New directory should NOT exist after dry run"
    );
    assert!(
        workspace.file_exists("old_dir/file.txt"),
        "Files in original directory should still exist"
    );
}

// =============================================================================
// Dry-Run Preview Accuracy Tests
// =============================================================================

#[tokio::test]
async fn test_dry_run_rename_file_shows_accurate_files_to_modify() {
    // Setup: TypeScript file with import from another file
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create directory structure
    workspace.create_directory("src");
    workspace.create_file("src/utils.ts", "export function helper() { return 42; }");
    workspace.create_file("src/app.ts", "import { helper } from './utils';");

    let old_path = workspace.absolute_path("src/utils.ts");
    let new_path = workspace.absolute_path("src/helpers.ts");

    // Action: Dry-run rename via rename.plan
    let plan_response = client
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
        .unwrap();

    let plan = &plan_response["result"]["content"];

    // Verify: Plan contains accurate preview data
    assert_eq!(
        plan["plan_type"], "RenamePlan",
        "Should generate a RenamePlan"
    );

    // Check summary fields
    // For file renames: affected_files counts all files involved (rename + import updates)
    let affected_files = plan["summary"]["affected_files"]
        .as_u64()
        .expect("affected_files should be a number");
    let created_files = plan["summary"]["created_files"]
        .as_u64()
        .expect("created_files should be a number");
    let deleted_files = plan["summary"]["deleted_files"]
        .as_u64()
        .expect("deleted_files should be a number");

    assert_eq!(created_files, 1, "Should create 1 file (helpers.ts)");
    assert_eq!(deleted_files, 1, "Should delete 1 file (utils.ts)");
    assert!(
        affected_files >= 1,
        "Should show at least 1 affected file (app.ts with import update), got {}",
        affected_files
    );

    // Now apply with dry_run to verify consistency
    let apply_response = client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dry_run": true
                }
            }),
        )
        .await
        .unwrap();

    let result = &apply_response["result"]["content"];
    assert_eq!(result["success"], true, "Dry run should succeed");

    // Verify filesystem is unchanged
    assert!(
        workspace.file_exists("src/utils.ts"),
        "Original file should still exist"
    );
    assert!(
        !workspace.file_exists("src/helpers.ts"),
        "New file should NOT exist after dry run"
    );
    assert_eq!(
        workspace.read_file("src/app.ts"),
        "import { helper } from './utils';",
        "Importer should be unchanged"
    );
}

#[tokio::test]
async fn test_dry_run_rename_file_rust_mod_declarations() {
    // Setup: Rust file with mod declaration in parent
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create directory structure
    workspace.create_directory("src");
    workspace.create_file("src/utils.rs", "pub fn helper() -> i32 { 42 }");
    workspace.create_file("src/lib.rs", "pub mod utils;\n\nuse utils::helper;");

    let old_path = workspace.absolute_path("src/utils.rs");
    let new_path = workspace.absolute_path("src/helpers.rs");

    // Action: Dry-run rename
    let plan_response = client
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
        .unwrap();

    let plan = &plan_response["result"]["content"];

    // Verify: Preview shows accurate file counts
    // For file renames: affected_files = files with content edits (imports updated)
    // created_files/deleted_files track the rename operation itself
    let affected_files = plan["summary"]["affected_files"]
        .as_u64()
        .expect("affected_files should be a number");
    let created_files = plan["summary"]["created_files"]
        .as_u64()
        .expect("created_files should be a number");
    let deleted_files = plan["summary"]["deleted_files"]
        .as_u64()
        .expect("deleted_files should be a number");

    // File rename: created=1, deleted=1
    assert_eq!(created_files, 1, "Should create 1 file (the renamed file)");
    assert_eq!(deleted_files, 1, "Should delete 1 file (the original file)");

    // lib.rs has mod declaration that needs updating
    assert!(
        affected_files >= 1,
        "Should show at least 1 affected file (lib.rs with mod declaration), got {}",
        affected_files
    );

    // Apply with dry_run
    let apply_response = client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dry_run": true
                }
            }),
        )
        .await
        .unwrap();

    let result = &apply_response["result"]["content"];
    assert_eq!(result["success"], true, "Dry run should succeed");

    // Verify filesystem is unchanged
    assert!(
        workspace.file_exists("src/utils.rs"),
        "Original file should still exist"
    );
    assert!(
        !workspace.file_exists("src/helpers.rs"),
        "New file should NOT exist"
    );
    let lib_content = workspace.read_file("src/lib.rs");
    assert!(
        lib_content.contains("pub mod utils;"),
        "Module declaration should be unchanged"
    );
}

#[tokio::test]
async fn test_dry_run_rename_directory_shows_files_list() {
    // Setup: Directory with multiple files
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create directory structure with 3 files
    workspace.create_directory("src/components");
    workspace.create_file("src/components/Button.ts", "export class Button {}");
    workspace.create_file("src/components/Input.ts", "export class Input {}");
    workspace.create_file("src/components/Modal.ts", "export class Modal {}");

    let old_dir = workspace.absolute_path("src/components");
    let new_dir = workspace.absolute_path("src/ui");

    // Action: Dry-run directory rename
    let plan_response = client
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
        .unwrap();

    let plan = &plan_response["result"]["content"];
    assert_eq!(
        plan["plan_type"], "RenamePlan",
        "Should generate a RenamePlan"
    );

    // Verify: Summary shows correct file count
    // For directory renames, files being moved count as created/deleted
    let created_files = plan["summary"]["created_files"]
        .as_u64()
        .expect("created_files should be a number");
    let deleted_files = plan["summary"]["deleted_files"]
        .as_u64()
        .expect("deleted_files should be a number");

    // Directory has 3 files being moved
    assert_eq!(created_files, 3, "Should create 3 files in new location");
    assert_eq!(deleted_files, 3, "Should delete 3 files from old location");
    // Note: affected_files tracks files with edits (imports), which may vary based on external imports

    // Apply with dry_run
    let apply_response = client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dry_run": true
                }
            }),
        )
        .await
        .unwrap();

    let result = &apply_response["result"]["content"];
    assert_eq!(result["success"], true, "Dry run should succeed");

    // Verify filesystem is unchanged
    assert!(
        workspace.file_exists("src/components"),
        "Original directory should still exist"
    );
    assert!(
        !workspace.file_exists("src/ui"),
        "New directory should NOT exist"
    );
    assert_eq!(
        workspace.file_exists("src/components/Button.ts")
            && workspace.file_exists("src/components/Input.ts")
            && workspace.file_exists("src/components/Modal.ts"),
        true,
        "All files should still exist in original location"
    );
}

#[tokio::test]
async fn test_dry_run_rename_directory_shows_import_updates() {
    // Setup: Directory with files imported by external files
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create directory structure
    workspace.create_directory("src/utils");
    workspace.create_file("src/utils/helpers.ts", "export function log() {}");
    workspace.create_file("src/app.ts", "import { log } from './utils/helpers';");

    let old_dir = workspace.absolute_path("src/utils");
    let new_dir = workspace.absolute_path("src/core");

    // Action: Dry-run directory rename
    let plan_response = client
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
        .unwrap();

    let plan = &plan_response["result"]["content"];

    // Verify: Preview shows files that will be updated
    let affected_files = plan["summary"]["affected_files"]
        .as_u64()
        .expect("affected_files should be a number");
    let created_files = plan["summary"]["created_files"]
        .as_u64()
        .expect("created_files should be a number");
    let deleted_files = plan["summary"]["deleted_files"]
        .as_u64()
        .expect("deleted_files should be a number");

    // Directory rename creates and deletes files
    assert_eq!(created_files, 1, "Should create 1 file in new location");
    assert_eq!(deleted_files, 1, "Should delete 1 file from old location");

    // app.ts has import that needs updating
    assert!(
        affected_files >= 1,
        "Should show at least 1 affected file (app.ts importing the renamed directory)"
    );

    // Apply with dry_run
    let apply_response = client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dry_run": true
                }
            }),
        )
        .await
        .unwrap();

    let result = &apply_response["result"]["content"];
    assert_eq!(result["success"], true, "Dry run should succeed");

    // Verify filesystem is unchanged
    assert!(
        workspace.file_exists("src/utils"),
        "Original directory should still exist"
    );
    assert!(
        !workspace.file_exists("src/core"),
        "New directory should NOT exist"
    );
    let app_content = workspace.read_file("src/app.ts");
    assert!(
        app_content.contains("from './utils/helpers'"),
        "Import path should be unchanged in dry run"
    );
}

#[tokio::test]
async fn test_dry_run_vs_execution_consistency() {
    // Setup: Same scenario - file rename with import updates
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create directory structure
    workspace.create_directory("src");
    workspace.create_file("src/utils.ts", "export function helper() { return 42; }");
    workspace.create_file("src/app.ts", "import { helper } from './utils';");
    workspace.create_file("src/index.ts", "import { helper } from './utils';");

    let old_path = workspace.absolute_path("src/utils.ts");
    let new_path = workspace.absolute_path("src/helpers.ts");

    // Step 1: Generate plan
    let plan_response = client
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
        .unwrap();

    let plan = &plan_response["result"]["content"];

    // Capture dry-run predictions
    let dry_run_affected_files = plan["summary"]["affected_files"]
        .as_u64()
        .expect("affected_files should be a number");
    let dry_run_created_files = plan["summary"]["created_files"]
        .as_u64()
        .expect("created_files should be a number");
    let dry_run_deleted_files = plan["summary"]["deleted_files"]
        .as_u64()
        .expect("deleted_files should be a number");

    // Step 2: Apply with dry_run to get preview
    let dry_run_response = client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dry_run": true
                }
            }),
        )
        .await
        .unwrap();

    let dry_run_result = &dry_run_response["result"]["content"];
    assert_eq!(dry_run_result["success"], true, "Dry run should succeed");

    // Step 3: Apply for real
    let real_response = client
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
        .unwrap();

    let real_result = &real_response["result"]["content"];
    assert_eq!(real_result["success"], true, "Real apply should succeed");

    // Step 4: Verify predictions matched reality
    // The key verification: affected_files from plan should be accurate
    // Check that the actual number of files changed matches the prediction
    assert_eq!(
        dry_run_created_files, 1,
        "Should have predicted 1 file created"
    );
    assert_eq!(
        dry_run_deleted_files, 1,
        "Should have predicted 1 file deleted"
    );
    assert!(
        dry_run_affected_files >= 2,
        "Should have predicted at least 2 affected files (imports updated in app.ts and index.ts)"
    );

    // Verify actual file changes occurred
    assert!(
        !workspace.file_exists("src/utils.ts"),
        "Original file should be deleted"
    );
    assert!(
        workspace.file_exists("src/helpers.ts"),
        "New file should exist"
    );

    // Verify imports were actually updated
    let app_updated = workspace
        .read_file("src/app.ts")
        .contains("from './helpers'");
    let index_updated = workspace
        .read_file("src/index.ts")
        .contains("from './helpers'");
    assert!(app_updated, "app.ts import should be updated");
    assert!(index_updated, "index.ts import should be updated");
}
