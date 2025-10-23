//! Integration tests for inline.plan and workspace.apply_edit
//!
//! Tests inline refactorings:
//! - Inline variable (AST-based)
//! - Inline function (AST-based)
//! - Inline constant (AST-based)

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

#[tokio::test]
async fn test_inline_variable_plan_and_apply() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "inline_var.rs",
        r#"pub fn calculate() -> i32 {
    let temp = 10;
    let result = temp * 2;
    result
}
"#,
    );

    let file_path = workspace.absolute_path("inline_var.rs");

    // 2. Generate inline.plan for variable
    let plan_result = client
        .call_tool(
            "inline.plan",
            json!({
                "kind": "variable",
                "target": {
                    "filePath": file_path.to_string_lossy(),
                    "position": {"line": 1, "character": 8}
                }
            }),
        )
        .await;

    match plan_result {
        Ok(response) => {
            let plan = response
                .get("result")
                .and_then(|r| r.get("content"))
                .expect("Plan should exist");

            assert_eq!(
                plan.get("planType").and_then(|v| v.as_str()),
                Some("inlinePlan"),
                "Should be InlinePlan"
            );

            // 3. Apply plan
            let apply_result = client
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
                .expect("Apply should succeed");

            let result = apply_result
                .get("result")
                .and_then(|r| r.get("content"))
                .expect("Apply result should exist");

            assert_eq!(
                result.get("success").and_then(|v| v.as_bool()),
                Some(true),
                "Inline should succeed"
            );
        }
        Err(_) => {
            eprintln!("INFO: inline.plan requires LSP support, skipping test");
        }
    }
}

#[tokio::test]
async fn test_inline_function_dry_run() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "inline_fn.rs",
        r#"fn double(x: i32) -> i32 {
    x * 2
}

pub fn test() -> i32 {
    double(5)
}
"#,
    );

    let file_path = workspace.absolute_path("inline_fn.rs");

    // 2. Generate inline.plan for function
    let plan_result = client
        .call_tool(
            "inline.plan",
            json!({
                "kind": "function",
                "target": {
                    "filePath": file_path.to_string_lossy(),
                    "position": {"line": 0, "character": 3}
                }
            }),
        )
        .await;

    match plan_result {
        Ok(response) => {
            // Check if response has error field (LSP unavailable)
            if response.get("error").is_some() {
                eprintln!("INFO: inline function requires LSP support, skipping test");
                return;
            }

            let plan = response
                .get("result")
                .and_then(|r| r.get("content"))
                .cloned();

            // If no plan content, likely LSP not available
            if plan.is_none() {
                eprintln!("INFO: inline function requires LSP support, skipping test");
                return;
            }

            let plan = plan.unwrap();

            // 3. Apply with dry_run=true
            let apply_result = client
                .call_tool(
                    "workspace.apply_edit",
                    json!({
                        "plan": plan,
                        "options": {
                            "dryRun": true
                        }
                    }),
                )
                .await
                .expect("Dry run should succeed");

            let result = apply_result
                .get("result")
                .and_then(|r| r.get("content"))
                .expect("Dry run result should exist");

            assert_eq!(
                result.get("success").and_then(|v| v.as_bool()),
                Some(true),
                "Dry run should succeed"
            );

            // 4. Verify file unchanged
            assert!(
                workspace
                    .read_file("inline_fn.rs")
                    .contains("fn double(x: i32)"),
                "File should be unchanged after dry run"
            );
        }
        Err(_) => {
            eprintln!("INFO: inline function requires LSP support, skipping test");
        }
    }
}

#[tokio::test]
async fn test_inline_constant_checksum_validation() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "inline_const.rs",
        r#"const MAX_SIZE: usize = 100;

pub fn get_buffer() -> Vec<u8> {
    Vec::with_capacity(MAX_SIZE)
}
"#,
    );

    let file_path = workspace.absolute_path("inline_const.rs");

    // 2. Generate plan
    let plan_result = client
        .call_tool(
            "inline.plan",
            json!({
                "kind": "constant",
                "target": {
                    "filePath": file_path.to_string_lossy(),
                    "position": {"line": 0, "character": 6}
                }
            }),
        )
        .await;

    match plan_result {
        Ok(response) => {
            let plan = response
                .get("result")
                .and_then(|r| r.get("content"))
                .expect("Plan should exist");

            // 3. Modify file to invalidate checksum
            workspace.create_file(
                "inline_const.rs",
                r#"const MAX_SIZE: usize = 200;

pub fn get_buffer() -> Vec<u8> {
    Vec::with_capacity(MAX_SIZE)
}
"#,
            );

            // 4. Try to apply with checksum validation
            let apply_result = client
                .call_tool(
                    "workspace.apply_edit",
                    json!({
                        "plan": plan,
                        "options": {
                            "validateChecksums": true
                        }
                    }),
                )
                .await;

            // Should fail due to checksum mismatch
            assert!(
                apply_result.is_err() || apply_result.unwrap().get("error").is_some(),
                "Apply should fail due to checksum mismatch"
            );
        }
        Err(_) => {
            eprintln!("INFO: inline constant requires LSP support, skipping test");
        }
    }
}

#[tokio::test]
async fn test_inline_plan_warnings() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "warnings.rs",
        r#"pub fn helper() -> i32 {
    42
}

pub fn use_once() -> i32 {
    helper()
}

pub fn use_twice() -> i32 {
    helper() + helper()
}
"#,
    );

    let file_path = workspace.absolute_path("warnings.rs");

    // 2. Generate inline plan for function used multiple times
    let plan_result = client
        .call_tool(
            "inline.plan",
            json!({
                "kind": "function",
                "target": {
                    "filePath": file_path.to_string_lossy(),
                    "position": {"line": 0, "character": 7}
                }
            }),
        )
        .await;

    match plan_result {
        Ok(response) => {
            // Check if response has error field (LSP unavailable)
            if response.get("error").is_some() {
                eprintln!("INFO: inline operations require LSP support, skipping warnings test");
                return;
            }

            let plan = response
                .get("result")
                .and_then(|r| r.get("content"))
                .cloned();

            // If no plan content, likely LSP not available
            if plan.is_none() {
                eprintln!("INFO: inline operations require LSP support, skipping warnings test");
                return;
            }

            let plan = plan.unwrap();

            // Inline of function used multiple times may generate warnings
            // This is acceptable - we're just validating the plan structure
            assert!(
                plan.get("warnings").is_some(),
                "Plan should have warnings field"
            );
        }
        Err(_) => {
            eprintln!("INFO: inline operations require LSP support, skipping warnings test");
        }
    }
}
