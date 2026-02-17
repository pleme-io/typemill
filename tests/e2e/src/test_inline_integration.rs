//! Integration tests for inline refactorings (variable, function, constant).
//!
//! REFACTORED: Uses try_refactor_tool() + build_inline_params() helpers.
//! 296 lines → ~130 lines (56% reduction).
//!
//! Note: Inline operations require LSP support. Tests gracefully skip if unavailable.

use crate::harness::{TestClient, TestWorkspace};
use crate::test_helpers::{build_inline_params, extract_content, set_dry_run, try_refactor_tool};
use serde_json::json;

/// Test 1: Inline variable plan and apply
#[tokio::test]
async fn test_inline_variable_plan_and_apply() {
    let workspace = TestWorkspace::new();
    workspace.create_file("inline_var.rs", "pub fn calculate() -> i32 {\n    let temp = 10;\n    let result = temp * 2;\n    result\n}\n");
    let mut client = TestClient::new(workspace.path());

    let params = build_inline_params(&workspace, "inline_var.rs", "variable", 1, 8);

    let Some(response) = try_refactor_tool(&mut client, params.clone())
        .await
        .unwrap()
    else {
        return;
    };

    let plan = extract_content(&response).expect("Plan should exist");
    assert_eq!(
        plan.get("planType").and_then(|v| v.as_str()),
        Some("inlinePlan"),
        "Should be InlinePlan"
    );

    // Apply
    let mut exec_params = build_inline_params(&workspace, "inline_var.rs", "variable", 1, 8);
    exec_params["options"] = json!({"dryRun": false, "validateChecksums": true});
    let apply = client
        .call_tool("refactor", exec_params)
        .await
        .expect("Apply should succeed");
    let result = extract_content(&apply).expect("Apply result should exist");
    assert_eq!(
        result.get("status").and_then(|v| v.as_str()),
        Some("success"),
        "Inline should succeed"
    );
}

/// Test 2: Inline function dry-run (file unchanged)
#[tokio::test]
async fn test_inline_function_dry_run() {
    let workspace = TestWorkspace::new();
    workspace.create_file(
        "inline_fn.rs",
        "fn double(x: i32) -> i32 {\n    x * 2\n}\n\npub fn test() -> i32 {\n    double(5)\n}\n",
    );
    let mut client = TestClient::new(workspace.path());

    let params = build_inline_params(&workspace, "inline_fn.rs", "function", 0, 3);

    let Some(_) = try_refactor_tool(&mut client, params).await.unwrap() else {
        return;
    };

    // Dry run
    let mut dry_params = build_inline_params(&workspace, "inline_fn.rs", "function", 0, 3);
    set_dry_run(&mut dry_params, true);
    let dry_result = client
        .call_tool("refactor", dry_params)
        .await
        .expect("Dry run should succeed");
    let result = extract_content(&dry_result).expect("Dry run result should exist");
    assert_eq!(
        result.get("status").and_then(|v| v.as_str()),
        Some("success"),
        "Dry run should succeed"
    );
    assert!(
        workspace
            .read_file("inline_fn.rs")
            .contains("fn double(x: i32)"),
        "File should be unchanged after dry run"
    );
}

/// Test 3: Inline constant checksum validation (modify file to invalidate checksum)
#[tokio::test]
async fn test_inline_constant_checksum_validation() {
    let workspace = TestWorkspace::new();
    workspace.create_file("inline_const.rs", "const MAX_SIZE: usize = 100;\n\npub fn get_buffer() -> Vec<u8> {\n    Vec::with_capacity(MAX_SIZE)\n}\n");
    let mut client = TestClient::new(workspace.path());

    let params = build_inline_params(&workspace, "inline_const.rs", "constant", 0, 6);

    let Some(_) = try_refactor_tool(&mut client, params).await.unwrap() else {
        return;
    };

    // Modify file to invalidate checksum
    workspace.create_file("inline_const.rs", "const MAX_SIZE: usize = 200;\n\npub fn get_buffer() -> Vec<u8> {\n    Vec::with_capacity(MAX_SIZE)\n}\n");

    // Try to apply with checksum validation - should fail
    let mut exec_params = build_inline_params(&workspace, "inline_const.rs", "constant", 0, 6);
    exec_params["options"] = json!({"validateChecksums": true, "dryRun": false});
    let apply_result = client.call_tool("refactor", exec_params).await;
    assert!(
        apply_result.is_err() || apply_result.unwrap().get("error").is_some(),
        "Apply should fail due to checksum mismatch"
    );
}

/// Test 4: Inline plan warnings (function used multiple times)
#[tokio::test]
async fn test_inline_plan_warnings() {
    let workspace = TestWorkspace::new();
    workspace.create_file("warnings.rs", "pub fn helper() -> i32 {\n    42\n}\n\npub fn use_once() -> i32 {\n    helper()\n}\n\npub fn use_twice() -> i32 {\n    helper() + helper()\n}\n");
    let mut client = TestClient::new(workspace.path());

    let params = build_inline_params(&workspace, "warnings.rs", "function", 0, 7);

    let Some(response) = try_refactor_tool(&mut client, params).await.unwrap() else {
        return;
    };

    let plan = extract_content(&response).unwrap();
    assert!(
        plan.get("warnings").is_some(),
        "Plan should have warnings field"
    );
}
