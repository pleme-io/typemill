//! Test helper functions to reduce boilerplate across E2E tests
//!
//! These helpers implement the standard test pattern: setup → plan → apply → verify
//! Each helper creates FRESH TestWorkspace and TestClient instances to ensure test isolation.
//!
//! Design principles:
//! - Fresh instances per test (no state bleed)
//! - Automatic cleanup via Drop
//! - Closure-based custom assertions
//! - Follows established pattern from all 204 existing tests

use crate::harness::{TestClient, TestWorkspace};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::Path;

/// Standard test helper: setup → plan → apply → verify (CLOSURE-BASED PARAMS)
///
/// Creates a fresh workspace and client, runs the tool workflow, and executes custom verifications.
/// **NEW:** Accepts closure to build params, solving the workspace dependency issue.
///
/// # Arguments
/// * `files` - Initial files to create in workspace (path, content)
/// * `tool` - Tool name (e.g., "rename.plan", "move.plan")
/// * `params_fn` - Closure that builds params given workspace (for absolute paths)
/// * `verify` - Closure for custom assertions on the workspace after operation
///
/// # Example
/// ```no_run
/// run_tool_test(
///     &[("old.rs", "pub fn test() {}")],
///     "rename.plan",
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

    // Generate plan
    let plan_result = client
        .call_tool(tool, params)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to call tool '{}': {}", tool, e))?;

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Plan should have result.content"))?;

    // Apply plan
    client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dryRun": false,
                    "validateChecksums": true
                }
            }),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to apply plan: {}", e))?;

    // Run custom verifications
    verify(&workspace)?;

    // Cleanup automatic: workspace.drop() deletes temp dir, client.drop() kills server

    Ok(())
}

/// Test helper with plan validation: setup → plan → validate plan → apply → verify (CLOSURE-BASED PARAMS)
///
/// Same as `run_tool_test` but allows inspection/assertion on the plan before applying.
///
/// # Arguments
/// * `files` - Initial files to create
/// * `tool` - Tool name
/// * `params_fn` - Closure that builds params given workspace (for absolute paths)
/// * `plan_validator` - Closure to assert on plan structure/metadata
/// * `result_validator` - Closure to assert on final workspace state
///
/// # Example
/// ```no_run
/// run_tool_test_with_plan_validation(
///     &[("file.rs", "content")],
///     "rename.plan",
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

    // Generate plan
    let plan_result = client
        .call_tool(tool, params)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to call tool '{}': {}", tool, e))?;

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Plan should have result.content"))?;

    // VALIDATE PLAN BEFORE APPLYING
    plan_validator(&plan).map_err(|e| anyhow::anyhow!("Plan validation failed: {}", e))?;

    // Apply plan
    client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dryRun": false,
                    "validateChecksums": true
                }
            }),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to apply plan: {}", e))?;

    // Validate result
    result_validator(&workspace).map_err(|e| anyhow::anyhow!("Result validation failed: {}", e))?;

    Ok(())
}

/// Test helper expecting failure: setup → plan/apply → assert error (CLOSURE-BASED PARAMS)
///
/// Verifies that the operation fails with expected error message.
///
/// # Arguments
/// * `files` - Initial files to create
/// * `tool` - Tool name
/// * `params_fn` - Closure that builds params given workspace (for absolute paths)
/// * `error_contains` - Optional substring that error message should contain
///
/// # Example
/// ```no_run
/// run_tool_test_expecting_failure(
///     &[("file.rs", "content")],
///     "rename.plan",
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

/// Helper for dry-run tests: setup → plan → apply with dryRun=true → verify no changes (CLOSURE-BASED PARAMS)
///
/// # Arguments
/// * `files` - Initial files to create
/// * `tool` - Tool name
/// * `params_fn` - Closure that builds params given workspace (for absolute paths)
/// * `verify_no_changes` - Closure to assert workspace is unchanged
///
/// # Example
/// ```no_run
/// run_dry_run_test(
///     &[("original.rs", "content")],
///     "rename.plan",
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
    let params = params_fn(&workspace);

    // Generate plan
    let plan_result = client
        .call_tool(tool, params)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to call tool '{}': {}", tool, e))?;

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Plan should exist"))?;

    // Apply with DRY RUN
    client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dryRun": true  // Critical: no actual changes
                }
            }),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Dry run failed: {}", e))?;

    // Verify nothing changed
    verify_no_changes(&workspace).map_err(|e| anyhow::anyhow!("Dry run should not modify workspace: {}", e))?;

    Ok(())
}

/// Helper for tests with mutation between plan and apply (CLOSURE-BASED PARAMS)
///
/// Useful for checksum validation tests that modify files after plan generation
/// but before application to test that checksum guards work.
///
/// # Arguments
/// * `files` - Initial files to create
/// * `tool` - Tool name
/// * `params_fn` - Closure that builds params given workspace (for absolute paths)
/// * `mutate_fn` - Closure to mutate workspace BETWEEN plan generation and application
/// * `verify` - Closure for custom assertions on the workspace after operation
///
/// # Example
/// ```no_run
/// run_tool_test_with_mutation(
///     &[("source/file.rs", "original content")],
///     "move.plan",
///     |ws| build_move_params(ws, "source/file.rs", "target/file.rs", "file"),
///     |ws, _plan| {
///         // Mutate file after plan generated to trigger checksum validation
///         ws.create_file("source/file.rs", "MODIFIED CONTENT");
///     },
///     |ws| {
///         // Should fail checksum validation, file stays in original location
///         assert!(ws.file_exists("source/file.rs"));
///         assert!(!ws.file_exists("target/file.rs"));
///         Ok(())
///     }
/// ).await?;
/// ```
pub async fn run_tool_test_with_mutation<P, M, V>(
    files: &[(&str, &str)],
    tool: &str,
    params_fn: P,
    mutate_fn: M,
    verify: V,
) -> Result<()>
where
    P: FnOnce(&TestWorkspace) -> Value,
    M: FnOnce(&TestWorkspace, &Value),
    V: FnOnce(&TestWorkspace) -> Result<()>,
{
    let workspace = TestWorkspace::new();

    // Setup initial files
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

    // Generate plan
    let plan_result = client
        .call_tool(tool, params)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to call tool '{}': {}", tool, e))?;

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Plan should have result.content"))?;

    // MUTATE workspace between plan and apply
    mutate_fn(&workspace, &plan);

    // Apply plan (may fail if checksums invalid)
    client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dryRun": false,
                    "validateChecksums": true
                }
            }),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to apply plan: {}", e))?;

    // Run custom verifications
    verify(&workspace)?;

    Ok(())
}

/// Helper to build tool parameters with absolute paths
///
/// Converts relative paths to absolute paths for the workspace.
/// Useful for building rename/move parameters.
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
            "path": workspace.absolute_path(old_path).to_string_lossy().to_string()
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
            "path": workspace.absolute_path(source).to_string_lossy().to_string()
        },
        "destination": workspace.absolute_path(destination).to_string_lossy().to_string()
    })
}

/// Helper to build delete parameters with absolute paths
pub fn build_delete_params(
    workspace: &TestWorkspace,
    path: &str,
    kind: &str,
) -> Value {
    json!({
        "target": {
            "kind": kind,
            "path": workspace.absolute_path(path).to_string_lossy().to_string()
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
