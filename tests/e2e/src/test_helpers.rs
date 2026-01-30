//! Test helper functions to reduce boilerplate across E2E tests
//!
//! These helpers implement the standard test pattern: setup → execute → verify
//! Each helper creates FRESH TestWorkspace and TestClient instances to ensure test isolation.
//!
//! Design principles:
//! - Fresh instances per test (no state bleed)
//! - Automatic cleanup via Drop
//! - Closure-based custom assertions
//! - Uses unified refactoring API with dryRun option

use crate::harness::{TestClient, TestWorkspace};
use anyhow::Result;
use serde_json::{json, Value};
use std::path::Path;

/// Standard test helper: setup → execute → verify (CLOSURE-BASED PARAMS)
///
/// Creates a fresh workspace and client, executes the tool with dryRun: false, and runs verifications.
/// Uses unified refactoring API (single call with dryRun option instead of plan + apply).
///
/// # Arguments
/// * `files` - Initial files to create in workspace (path, content)
/// * `tool` - Tool name (e.g., "rename_all", "relocate", "refactor")
/// * `params_fn` - Closure that builds params given workspace (for absolute paths)
/// * `verify` - Closure for custom assertions on the workspace after operation
///
/// # Example
/// ```no_run
/// run_tool_test(
///     &[("old.rs", "pub fn test() {}")],
///     "rename_all",
///     |ws| build_rename_params(ws, "old.rs", "new.rs", "file"),
///     |ws| {
///         assert!(ws.file_exists("new.rs"));
///         assert!(!ws.file_exists("old.rs"));
///         Ok(())
///     }
/// ).await?;
/// ```
pub async fn run_tool_test<P, V>(
    files: &[(&str, &str)],
    tool: &str,
    params_fn: P,
    verify: V,
) -> Result<()>
where
    P: FnOnce(&TestWorkspace) -> Value,
    V: FnOnce(&TestWorkspace) -> Result<()>,
{
    // Create FRESH workspace (new temp directory)
    let workspace = TestWorkspace::new();

    // Setup initial files
    for (file_path, content) in files {
        // Create parent directories if needed
        if let Some(parent) = Path::new(file_path).parent() {
            if parent != Path::new("") {
                workspace.create_directory(parent.to_str().unwrap());
            }
        }
        workspace.create_file(file_path, content);
    }

    // Create FRESH client (new server process)
    let mut client = TestClient::new(workspace.path());

    // BUILD PARAMS with workspace access
    let params = params_fn(&workspace);

    // Ensure dryRun: false is set (execute mode)
    let mut params = params;
    if let Some(obj) = params.as_object_mut() {
        obj.entry("options").or_insert_with(|| json!({}));
        if let Some(options) = obj.get_mut("options").and_then(|v| v.as_object_mut()) {
            options.insert("dryRun".to_string(), json!(false));
        }
    }

    // Execute operation (plan + apply atomically)
    client
        .call_tool(tool, params)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to execute tool '{}': {}", tool, e))?;

    // Run custom verifications
    verify(&workspace)?;

    // Cleanup automatic: workspace.drop() deletes temp dir, client.drop() kills server

    Ok(())
}

/// Test helper with plan validation: setup → preview → validate plan → execute → verify (CLOSURE-BASED PARAMS)
///
/// Uses unified API: calls tool with dryRun: true to get plan, validates it, then calls with dryRun: false to execute.
///
/// # Arguments
/// * `files` - Initial files to create
/// * `tool` - Tool name (e.g., "rename_all", "refactor")
/// * `params_fn` - Closure that builds params given workspace (for absolute paths)
/// * `plan_validator` - Closure to assert on plan structure/metadata
/// * `result_validator` - Closure to assert on final workspace state
///
/// # Example
/// ```no_run
/// run_tool_test_with_plan_validation(
///     &[("file.rs", "content")],
///     "rename_all",
///     |ws| build_rename_params(ws, "file.rs", "renamed.rs", "file"),
///     |plan| {
///         assert_eq!(plan.get("planType").and_then(|v| v.as_str()), Some("renamePlan"));
///         assert!(plan.get("metadata").is_some());
///         Ok(())
///     },
///     |ws| {
///         assert!(ws.file_exists("renamed.rs"));
///         Ok(())
///     }
/// ).await?;
/// ```
pub async fn run_tool_test_with_plan_validation<P, F, V>(
    files: &[(&str, &str)],
    tool: &str,
    params_fn: P,
    plan_validator: F,
    result_validator: V,
) -> Result<()>
where
    P: FnOnce(&TestWorkspace) -> Value,
    F: FnOnce(&Value) -> Result<()>,
    V: FnOnce(&TestWorkspace) -> Result<()>,
{
    let workspace = TestWorkspace::new();

    // Setup files
    for (file_path, content) in files {
        if let Some(parent) = Path::new(file_path).parent() {
            if parent != Path::new("") {
                workspace.create_directory(parent.to_str().unwrap());
            }
        }
        workspace.create_file(file_path, content);
    }

    let mut client = TestClient::new(workspace.path());

    // BUILD PARAMS with workspace access
    let params = params_fn(&workspace);

    // Step 1: Preview mode (dryRun: true) - get plan
    let mut preview_params = params.clone();
    if let Some(obj) = preview_params.as_object_mut() {
        obj.entry("options").or_insert_with(|| json!({}));
        if let Some(options) = obj.get_mut("options").and_then(|v| v.as_object_mut()) {
            options.insert("dryRun".to_string(), json!(true));
        }
    }

    let plan_result = client
        .call_tool(tool, preview_params)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to preview with '{}': {}", tool, e))?;

    // M7 response: Tool returns {"content": WriteResponse}, and WriteResponse has status, summary, filesChanged, diagnostics, changes
    let result = plan_result
        .get("result")
        .ok_or_else(|| anyhow::anyhow!("Response should have result"))?;

    let response = result
        .get("content")
        .ok_or_else(|| anyhow::anyhow!("Response should have result.content (M7 format)"))?;

    let plan = response
        .get("changes")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Plan should have result.content.changes (M7 format)"))?;

    // VALIDATE PLAN BEFORE EXECUTING
    plan_validator(&plan).map_err(|e| anyhow::anyhow!("Plan validation failed: {}", e))?;

    // Step 2: Execute mode (dryRun: false) - apply changes
    let mut execute_params = params;
    if let Some(obj) = execute_params.as_object_mut() {
        obj.entry("options").or_insert_with(|| json!({}));
        if let Some(options) = obj.get_mut("options").and_then(|v| v.as_object_mut()) {
            options.insert("dryRun".to_string(), json!(false));
        }
    }

    client
        .call_tool(tool, execute_params)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to execute '{}': {}", tool, e))?;

    // Validate result
    result_validator(&workspace).map_err(|e| anyhow::anyhow!("Result validation failed: {}", e))?;

    Ok(())
}

/// Test helper expecting failure: setup → execute → assert error (CLOSURE-BASED PARAMS)
///
/// Verifies that the operation fails with expected error message.
/// Note: Add dryRun: false to params_fn if you want to test execution failure.
///
/// # Arguments
/// * `files` - Initial files to create
/// * `tool` - Tool name (e.g., "rename_all", "refactor")
/// * `params_fn` - Closure that builds params given workspace (for absolute paths)
/// * `error_contains` - Optional substring that error message should contain
///
/// # Example
/// ```no_run
/// run_tool_test_expecting_failure(
///     &[("file.rs", "content")],
///     "rename_all",
///     |ws| build_rename_params(ws, "nonexistent.rs", "new.rs", "file"),
///     Some("file not found")
/// ).await?;
/// ```
pub async fn run_tool_test_expecting_failure<P>(
    files: &[(&str, &str)],
    tool: &str,
    params_fn: P,
    error_contains: Option<&str>,
) -> Result<()>
where
    P: FnOnce(&TestWorkspace) -> Value,
{
    let workspace = TestWorkspace::new();

    // Setup files
    for (file_path, content) in files {
        if let Some(parent) = Path::new(file_path).parent() {
            if parent != Path::new("") {
                workspace.create_directory(parent.to_str().unwrap());
            }
        }
        workspace.create_file(file_path, content);
    }

    let mut client = TestClient::new(workspace.path());

    // BUILD PARAMS with workspace access
    let params = params_fn(&workspace);

    // Call tool - should fail
    let result = client.call_tool(tool, params).await;

    // Verify it failed
    match result {
        Err(e) => {
            // Error case - check message if needed
            if let Some(expected_msg) = error_contains {
                let error_msg = e.to_string();
                assert!(
                    error_msg.contains(expected_msg),
                    "Error message should contain '{}', got: {}",
                    expected_msg,
                    error_msg
                );
            }
            Ok(())
        }
        Ok(response) => {
            // Response might have error field
            if let Some(error) = response.get("error") {
                if let Some(expected_msg) = error_contains {
                    let error_str = error.to_string();
                    assert!(
                        error_str.contains(expected_msg),
                        "Error should contain '{}', got: {}",
                        expected_msg,
                        error_str
                    );
                }
                Ok(())
            } else {
                anyhow::bail!(
                    "Tool call succeeded but should have failed. Response: {:?}",
                    response
                );
            }
        }
    }
}

/// Helper for dry-run tests: setup → preview with dryRun=true → verify no changes (CLOSURE-BASED PARAMS)
///
/// Uses unified API with dryRun: true (default) to verify preview mode doesn't modify files.
///
/// # Arguments
/// * `files` - Initial files to create
/// * `tool` - Tool name (e.g., "rename_all", "refactor")
/// * `params_fn` - Closure that builds params given workspace (for absolute paths)
/// * `verify_no_changes` - Closure to assert workspace is unchanged
///
/// # Example
/// ```no_run
/// run_dry_run_test(
///     &[("original.rs", "content")],
///     "rename_all",
///     |ws| build_rename_params(ws, "original.rs", "renamed.rs", "file"),
///     |ws| {
///         assert!(ws.file_exists("original.rs"), "Original file should still exist");
///         assert!(!ws.file_exists("renamed.rs"), "New file should NOT exist");
///         Ok(())
///     }
/// ).await?;
/// ```
pub async fn run_dry_run_test<P, V>(
    files: &[(&str, &str)],
    tool: &str,
    params_fn: P,
    verify_no_changes: V,
) -> Result<()>
where
    P: FnOnce(&TestWorkspace) -> Value,
    V: FnOnce(&TestWorkspace) -> Result<()>,
{
    let workspace = TestWorkspace::new();

    // Setup files
    for (file_path, content) in files {
        if let Some(parent) = Path::new(file_path).parent() {
            if parent != Path::new("") {
                workspace.create_directory(parent.to_str().unwrap());
            }
        }
        workspace.create_file(file_path, content);
    }

    let mut client = TestClient::new(workspace.path());

    // BUILD PARAMS with workspace access
    let mut params = params_fn(&workspace);

    // Ensure dryRun: true is set (preview mode - default, but explicit here)
    if let Some(obj) = params.as_object_mut() {
        obj.entry("options").or_insert_with(|| json!({}));
        if let Some(options) = obj.get_mut("options").and_then(|v| v.as_object_mut()) {
            options.insert("dryRun".to_string(), json!(true));
        }
    }

    // Call tool in preview mode (should not modify workspace)
    client
        .call_tool(tool, params)
        .await
        .map_err(|e| anyhow::anyhow!("Preview mode failed for '{}': {}", tool, e))?;

    // Verify nothing changed
    verify_no_changes(&workspace)
        .map_err(|e| anyhow::anyhow!("Preview mode should not modify workspace: {}", e))?;

    Ok(())
}

/// Helper to build tool parameters with absolute paths
///
/// Converts relative paths to absolute paths for the workspace.
/// Useful for building rename_all/relocate parameters.
///
/// # Example
/// ```no_run
/// let workspace = TestWorkspace::new();
/// let params = build_rename_params(&workspace, "old.rs", "new.rs", "file");
/// // Returns: {"target": {"kind": "file", "path": "/tmp/xyz/old.rs"}, "newName": "/tmp/xyz/new.rs"}
/// ```
pub fn build_rename_params(
    workspace: &TestWorkspace,
    old_path: &str,
    new_path: &str,
    kind: &str,
) -> Value {
    json!({
        "target": {
            "kind": kind,
            "filePath": workspace.absolute_path(old_path).to_string_lossy().to_string()
        },
        "newName": workspace.absolute_path(new_path).to_string_lossy().to_string()
    })
}

/// Helper to build move parameters with absolute paths
pub fn build_move_params(
    workspace: &TestWorkspace,
    source: &str,
    destination: &str,
    kind: &str,
) -> Value {
    json!({
        "target": {
            "kind": kind,
            "filePath": workspace.absolute_path(source).to_string_lossy().to_string()
        },
        "destination": workspace.absolute_path(destination).to_string_lossy().to_string()
    })
}

/// Helper to build delete parameters with absolute paths
pub fn build_delete_params(workspace: &TestWorkspace, path: &str, kind: &str) -> Value {
    json!({
        "target": {
            "kind": kind,
            "filePath": workspace.absolute_path(path).to_string_lossy().to_string()
        }
    })
}

/// Helper to create workspace from fixture with directory structure
pub fn setup_workspace_from_fixture(workspace: &TestWorkspace, files: &[(&str, &str)]) {
    for (file_path, content) in files {
        // Ensure parent directories exist
        if let Some(parent) = std::path::Path::new(file_path).parent() {
            if parent != std::path::Path::new("") {
                workspace.create_directory(parent.to_str().unwrap());
            }
        }
        workspace.create_file(file_path, content);
    }
}

/// Recursive copy directory helper
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), &dst_path)?;
        }
    }
    Ok(())
}

/// Helper to find the mill binary in the build target directory
pub fn find_mill_binary() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("MILL_PATH") {
        return std::path::PathBuf::from(path);
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let mut root = std::path::PathBuf::from(manifest_dir);
    root.pop(); // e2e
    root.pop(); // tests

    // Check debug first
    let debug_path = root.join("target/debug/mill");
    if debug_path.exists() {
        return debug_path;
    }

    // Check release
    let release_path = root.join("target/release/mill");
    if release_path.exists() {
        return release_path;
    }

    // Fallback to debug (default expected)
    debug_path
}
