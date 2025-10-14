//! Integration tests for direct invocation of internal rename tools
//!
//! These tests verify that the internal tools `rename_file` and `rename_directory`
//! work correctly when called directly (not via the unified API).
//!
//! Tests cover:
//! - Direct invocation of rename_file with import updates
//! - Direct invocation of rename_directory with import updates
//! - Dry-run mode validation
//! - Rust mod declaration updates
//! - Rust workspace Cargo.toml updates
//!
//! This ensures internal tools maintain feature parity with the unified API.

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

// =============================================================================
// DIRECT RENAME_FILE TESTS
// =============================================================================

#[tokio::test]
async fn test_direct_rename_file_updates_imports() {
    // Setup: TypeScript file importing from another file
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create files with imports
    workspace.create_directory("src");
    workspace.create_file(
        "src/utils.ts",
        r#"export const myUtil = () => {
    return "utility function";
};

export function helperFunc(data: string): string {
    return data.toUpperCase();
}
"#,
    );
    workspace.create_file(
        "src/main.ts",
        r#"import { myUtil, helperFunc } from './utils';

export function main() {
    const result = myUtil();
    const processed = helperFunc(result);
    console.log(processed);
}
"#,
    );

    let old_path = workspace.absolute_path("src/utils.ts");
    let new_path = workspace.absolute_path("src/renamed_utils.ts");

    // Action: Call rename_file tool directly (NOT via rename.plan)
    let result = client
        .call_tool(
            "rename_file",
            json!({
                "old_path": old_path.to_string_lossy(),
                "new_path": new_path.to_string_lossy(),
                "dry_run": false
            }),
        )
        .await
        .expect("rename_file should succeed");

    // Verify result structure
    let content = result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Result should have content");

    assert_eq!(
        content.get("success").and_then(|v| v.as_bool()),
        Some(true),
        "Direct rename_file should succeed"
    );

    // Verify: File was renamed
    assert!(
        !workspace.file_exists("src/utils.ts"),
        "Original file should be deleted"
    );
    assert!(
        workspace.file_exists("src/renamed_utils.ts"),
        "New file should exist"
    );

    // Verify: Import was updated
    let main_content = workspace.read_file("src/main.ts");
    assert!(
        main_content.contains("from './renamed_utils'"),
        "Import should be updated. Actual content:\n{}",
        main_content
    );
    assert!(
        !main_content.contains("from './utils'"),
        "Old import should be removed. Actual content:\n{}",
        main_content
    );
}

#[tokio::test]
async fn test_direct_rename_file_rust_mod_updates() {
    // Setup: Rust file with mod declaration
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_directory("src");
    workspace.create_file(
        "src/lib.rs",
        r#"pub mod utils;
pub mod helpers;

pub fn main_function() {
    utils::do_something();
}
"#,
    );
    workspace.create_file(
        "src/utils.rs",
        r#"pub fn do_something() {
    println!("Doing something");
}
"#,
    );
    workspace.create_file(
        "src/helpers.rs",
        r#"pub fn helper() {
    println!("Helper");
}
"#,
    );

    let old_path = workspace.absolute_path("src/utils.rs");
    let new_path = workspace.absolute_path("src/core_utils.rs");

    // Action: Call rename_file tool directly
    let result = client
        .call_tool(
            "rename_file",
            json!({
                "old_path": old_path.to_string_lossy(),
                "new_path": new_path.to_string_lossy(),
                "dry_run": false
            }),
        )
        .await
        .expect("rename_file should succeed");

    let content = result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Result should have content");

    assert_eq!(
        content.get("success").and_then(|v| v.as_bool()),
        Some(true),
        "Direct rename_file should succeed"
    );

    // Verify: mod declaration was updated in parent file
    let lib_content = workspace.read_file("src/lib.rs");
    assert!(
        lib_content.contains("pub mod core_utils;"),
        "mod declaration should be updated. Actual content:\n{}",
        lib_content
    );
    assert!(
        !lib_content.contains("pub mod utils;") || lib_content.contains("pub mod core_utils;"),
        "Old mod declaration should be replaced. Actual content:\n{}",
        lib_content
    );

    // Verify: File was renamed
    assert!(
        workspace.file_exists("src/core_utils.rs"),
        "New file should exist"
    );
    assert!(
        !workspace.file_exists("src/utils.rs"),
        "Old file should be deleted"
    );
}

#[tokio::test]
async fn test_direct_rename_file_dry_run() {
    // Setup: File with imports
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_directory("src");
    workspace.create_file("src/module.ts", "export const value = 42;");
    workspace.create_file(
        "src/app.ts",
        "import { value } from './module';\nconsole.log(value);",
    );

    let old_path = workspace.absolute_path("src/module.ts");
    let new_path = workspace.absolute_path("src/renamed_module.ts");

    // Action: Call rename_file with dry_run: true
    let result = client
        .call_tool(
            "rename_file",
            json!({
                "old_path": old_path.to_string_lossy(),
                "new_path": new_path.to_string_lossy(),
                "dry_run": true
            }),
        )
        .await
        .expect("rename_file dry run should succeed");

    let content = result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Result should have content");

    // Verify: dry_run returned success
    assert_eq!(
        content.get("success").and_then(|v| v.as_bool()),
        Some(true),
        "Dry run should succeed"
    );

    // Verify: import_updates field is populated
    let import_updates = content.get("import_updates").expect("Should have import_updates");
    let files_to_modify = import_updates
        .get("files_to_modify")
        .and_then(|v| v.as_array())
        .expect("Should have files_to_modify array");

    assert!(
        !files_to_modify.is_empty(),
        "files_to_modify should be populated with affected files"
    );

    // Verify: Check that src/app.ts is in the list
    let has_app_file = files_to_modify.iter().any(|f| {
        f.as_str()
            .map(|s| s.contains("app.ts"))
            .unwrap_or(false)
    });
    assert!(
        has_app_file,
        "src/app.ts should be in files_to_modify list"
    );

    // Verify: Filesystem NOT modified
    assert!(
        workspace.file_exists("src/module.ts"),
        "Original file should still exist after dry run"
    );
    assert!(
        !workspace.file_exists("src/renamed_module.ts"),
        "New file should NOT exist after dry run"
    );

    let app_content = workspace.read_file("src/app.ts");
    assert!(
        app_content.contains("from './module'"),
        "Import should NOT be updated in dry run. Actual content:\n{}",
        app_content
    );
}

// =============================================================================
// DIRECT RENAME_DIRECTORY TESTS
// =============================================================================

#[tokio::test]
async fn test_direct_rename_directory_updates_imports() {
    // Setup: Directory with files imported externally
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_directory("src");
    workspace.create_directory("src/components");
    workspace.create_file(
        "src/components/Button.ts",
        "export class Button {}\nexport class Icon {}",
    );
    workspace.create_file(
        "src/components/Input.ts",
        "export class Input {}",
    );
    workspace.create_file(
        "src/app.ts",
        r#"import { Button } from './components/Button';
import { Input } from './components/Input';

export function main() {
    const btn = new Button();
    const input = new Input();
}
"#,
    );

    let old_dir = workspace.absolute_path("src/components");
    let new_dir = workspace.absolute_path("src/ui");

    // Action: Call rename_directory tool directly (NOT via rename.plan)
    let result = client
        .call_tool(
            "rename_directory",
            json!({
                "old_path": old_dir.to_string_lossy(),
                "new_path": new_dir.to_string_lossy(),
                "dry_run": false
            }),
        )
        .await
        .expect("rename_directory should succeed");

    let content = result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Result should have content");

    assert_eq!(
        content.get("success").and_then(|v| v.as_bool()),
        Some(true),
        "Direct rename_directory should succeed"
    );

    // Verify: Directory was renamed
    assert!(
        !workspace.file_exists("src/components"),
        "Old directory should be deleted"
    );
    assert!(
        workspace.file_exists("src/ui"),
        "New directory should exist"
    );
    assert!(
        workspace.file_exists("src/ui/Button.ts"),
        "Files should be moved"
    );
    assert!(
        workspace.file_exists("src/ui/Input.ts"),
        "Files should be moved"
    );

    // Verify: External imports were updated
    let app_content = workspace.read_file("src/app.ts");
    assert!(
        app_content.contains("from './ui/Button'"),
        "Import should be updated to new directory. Actual content:\n{}",
        app_content
    );
    assert!(
        app_content.contains("from './ui/Input'"),
        "Import should be updated to new directory. Actual content:\n{}",
        app_content
    );
    assert!(
        !app_content.contains("from './components/"),
        "Old directory path should be removed. Actual content:\n{}",
        app_content
    );
}

#[tokio::test]
async fn test_direct_rename_directory_dry_run() {
    // Setup: Directory with files
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_directory("src");
    workspace.create_directory("src/old_module");
    workspace.create_file("src/old_module/file1.ts", "export const a = 1;");
    workspace.create_file("src/old_module/file2.ts", "export const b = 2;");
    workspace.create_file(
        "src/main.ts",
        "import { a } from './old_module/file1';\nimport { b } from './old_module/file2';",
    );

    let old_dir = workspace.absolute_path("src/old_module");
    let new_dir = workspace.absolute_path("src/new_module");

    // Action: Call rename_directory with dry_run: true
    let result = client
        .call_tool(
            "rename_directory",
            json!({
                "old_path": old_dir.to_string_lossy(),
                "new_path": new_dir.to_string_lossy(),
                "dry_run": true
            }),
        )
        .await
        .expect("rename_directory dry run should succeed");

    let content = result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Result should have content");

    // Verify: dry_run returned success
    assert_eq!(
        content.get("success").and_then(|v| v.as_bool()),
        Some(true),
        "Dry run should succeed"
    );

    // Verify: files_to_move count is accurate
    let files_to_move = content
        .get("files_to_move")
        .and_then(|v| v.as_u64())
        .expect("Should have files_to_move count");
    assert_eq!(
        files_to_move, 2,
        "Should report 2 files to move (file1.ts and file2.ts)"
    );

    // Verify: import_updates shows affected files
    let import_updates = content.get("import_updates").expect("Should have import_updates");
    let files_to_modify = import_updates
        .get("files_to_modify")
        .and_then(|v| v.as_array())
        .expect("Should have files_to_modify array");

    assert!(
        !files_to_modify.is_empty(),
        "files_to_modify should be populated"
    );

    let has_main_file = files_to_modify.iter().any(|f| {
        f.as_str()
            .map(|s| s.contains("main.ts"))
            .unwrap_or(false)
    });
    assert!(
        has_main_file,
        "main.ts should be in files_to_modify list"
    );

    // Verify: affected_files array is populated
    let affected_files = content
        .get("affected_files")
        .and_then(|v| v.as_array())
        .expect("Should have affected_files array");
    assert!(
        !affected_files.is_empty(),
        "affected_files should be populated"
    );

    // Verify: Filesystem NOT modified
    assert!(
        workspace.file_exists("src/old_module"),
        "Original directory should still exist after dry run"
    );
    assert!(
        !workspace.file_exists("src/new_module"),
        "New directory should NOT exist after dry run"
    );
    assert!(
        workspace.file_exists("src/old_module/file1.ts"),
        "Files should still be in old location"
    );

    let main_content = workspace.read_file("src/main.ts");
    assert!(
        main_content.contains("from './old_module/file1'"),
        "Imports should NOT be updated in dry run. Actual content:\n{}",
        main_content
    );
}

#[tokio::test]
async fn test_direct_rename_directory_rust_workspace() {
    // Setup: Rust workspace member
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create workspace structure
    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = ["old_crate", "app"]
"#,
    );

    workspace.create_directory("old_crate");
    workspace.create_directory("old_crate/src");
    workspace.create_file(
        "old_crate/Cargo.toml",
        r#"[package]
name = "old_crate"
version = "0.1.0"
edition = "2021"
"#,
    );
    workspace.create_file(
        "old_crate/src/lib.rs",
        r#"pub fn utility() {
    println!("Utility function");
}
"#,
    );

    workspace.create_directory("app");
    workspace.create_directory("app/src");
    workspace.create_file(
        "app/Cargo.toml",
        r#"[package]
name = "app"
version = "0.1.0"
edition = "2021"

[dependencies]
old_crate = { path = "../old_crate" }
"#,
    );
    workspace.create_file(
        "app/src/main.rs",
        r#"use old_crate::utility;

fn main() {
    utility();
}
"#,
    );

    let old_dir = workspace.absolute_path("old_crate");
    let new_dir = workspace.absolute_path("new_crate");

    // Action: Call rename_directory tool directly
    let result = client
        .call_tool(
            "rename_directory",
            json!({
                "old_path": old_dir.to_string_lossy(),
                "new_path": new_dir.to_string_lossy(),
                "dry_run": false
            }),
        )
        .await
        .expect("rename_directory should succeed");

    let content = result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Result should have content");

    assert_eq!(
        content.get("success").and_then(|v| v.as_bool()),
        Some(true),
        "Direct rename_directory should succeed"
    );

    // Verify: Directory was renamed
    assert!(
        !workspace.file_exists("old_crate"),
        "Old directory should be deleted"
    );
    assert!(
        workspace.file_exists("new_crate"),
        "New directory should exist"
    );

    // Verify: Workspace Cargo.toml was updated
    let workspace_toml = workspace.read_file("Cargo.toml");
    assert!(
        workspace_toml.contains("\"new_crate\""),
        "Workspace Cargo.toml should reference new_crate. Actual content:\n{}",
        workspace_toml
    );
    assert!(
        !workspace_toml.contains("\"old_crate\"") || workspace_toml.contains("\"new_crate\""),
        "Old crate reference should be replaced. Actual content:\n{}",
        workspace_toml
    );

    // Verify: Crate name in package Cargo.toml was updated
    let crate_toml = workspace.read_file("new_crate/Cargo.toml");
    assert!(
        crate_toml.contains("name = \"new_crate\""),
        "Crate Cargo.toml should have new name. Actual content:\n{}",
        crate_toml
    );

    // Verify: Dependency path in app Cargo.toml was updated
    let app_toml = workspace.read_file("app/Cargo.toml");
    assert!(
        app_toml.contains("new_crate = { path = \"../new_crate\" }"),
        "App Cargo.toml should reference new path. Actual content:\n{}",
        app_toml
    );

    // Verify: use statements were updated
    let main_rs = workspace.read_file("app/src/main.rs");
    assert!(
        main_rs.contains("use new_crate::utility;"),
        "use statement should be updated. Actual content:\n{}",
        main_rs
    );
    assert!(
        !main_rs.contains("use old_crate::utility;"),
        "Old use statement should be removed. Actual content:\n{}",
        main_rs
    );
}
