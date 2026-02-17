//! Integration tests for extract refactorings (extract function, variable, constant).
//!
//! REFACTORED: Uses try_refactor_tool() + build_extract_params() helpers.
//! 280 lines → ~120 lines (57% reduction).
//!
//! Note: Extract operations require LSP support. Tests gracefully skip if unavailable.

use crate::harness::{TestClient, TestWorkspace};
use crate::test_helpers::{build_extract_params, extract_content, set_dry_run, try_refactor_tool};

/// Test 1: Extract function basic workflow (plan + apply)
#[tokio::test]
async fn test_extract_function_plan_basic_workflow() {
    let workspace = TestWorkspace::new();
    workspace.create_file("calc.rs", "pub fn calculate(x: i32, y: i32) -> i32 {\n    let sum = x + y;\n    let doubled = sum * 2;\n    doubled\n}\n");
    let mut client = TestClient::new(workspace.path());

    let params = build_extract_params(
        &workspace,
        "calc.rs",
        "function",
        1,
        4,
        2,
        26,
        "compute_sum_doubled",
    );

    let Some(response) = try_refactor_tool(&mut client, params.clone())
        .await
        .unwrap()
    else {
        return;
    };

    let plan = extract_content(&response).expect("Plan should exist");
    assert_eq!(
        plan.get("planType").and_then(|v| v.as_str()),
        Some("extractPlan"),
        "Should be ExtractPlan"
    );

    // Apply
    let mut exec_params = params;
    set_dry_run(&mut exec_params, false);
    let apply = client
        .call_tool("refactor", exec_params)
        .await
        .expect("Apply should succeed");
    let result = extract_content(&apply).expect("Apply result should exist");
    assert_eq!(
        result.get("status").and_then(|v| v.as_str()),
        Some("success")
    );
}

/// Test 2: Extract variable dry run (file unchanged after preview)
#[tokio::test]
async fn test_extract_variable_dry_run() {
    let workspace = TestWorkspace::new();
    workspace.create_file(
        "vars.rs",
        "pub fn process() -> i32 {\n    let result = (10 + 5) * 2;\n    result\n}\n",
    );
    let mut client = TestClient::new(workspace.path());

    let params = build_extract_params(
        &workspace,
        "vars.rs",
        "variable",
        1,
        17,
        1,
        27,
        "base_value",
    );

    let Some(_) = try_refactor_tool(&mut client, params.clone())
        .await
        .unwrap()
    else {
        return;
    };

    // Dry run with explicit dryRun: true
    let mut dry_params = params;
    set_dry_run(&mut dry_params, true);
    let dry_result = client
        .call_tool("refactor", dry_params)
        .await
        .expect("Dry run should succeed");
    let plan = extract_content(&dry_result).expect("Dry run should return plan");
    assert!(
        plan.get("planType").is_some(),
        "Dry run should return plan structure"
    );
    assert!(
        workspace
            .read_file("vars.rs")
            .contains("let result = (10 + 5) * 2;"),
        "File should be unchanged after dry run"
    );
}

/// Test 3: Extract plan metadata structure
#[tokio::test]
async fn test_extract_plan_metadata_structure() {
    let workspace = TestWorkspace::new();
    workspace.create_file(
        "meta.rs",
        "pub fn test() -> i32 {\n    let x = 5;\n    x * 2\n}\n",
    );
    let mut client = TestClient::new(workspace.path());

    let mut params =
        build_extract_params(&workspace, "meta.rs", "variable", 2, 4, 2, 9, "multiplier");
    set_dry_run(&mut params, true);

    let Some(response) = try_refactor_tool(&mut client, params).await.unwrap() else {
        return;
    };

    let plan = extract_content(&response).expect("Plan should exist");
    assert!(plan.get("metadata").is_some(), "Plan should have metadata");
    assert!(plan.get("summary").is_some(), "Plan should have summary");
    assert!(
        plan.get("fileChecksums").is_some(),
        "Plan should have checksums"
    );
    assert!(plan.get("edits").is_some(), "Plan should have edits");

    let metadata = plan.get("metadata").unwrap();
    assert_eq!(
        metadata.get("planVersion").and_then(|v| v.as_str()),
        Some("1.0")
    );
    assert_eq!(
        metadata.get("kind").and_then(|v| v.as_str()),
        Some("extract")
    );
}
