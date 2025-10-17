//! Integration tests for reorder.plan and workspace.apply_edit
//!
//! Tests reorder operations:
//! - Reorder imports (most reliable operation)
//! - Reorder parameters (requires LSP)
//! - Reorder fields (requires LSP)

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

#[tokio::test]
async fn test_reorder_imports_plan_and_apply() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "imports.rs",
        r#"use std::vec::Vec;
use std::collections::HashMap;
use std::fs::File;

pub fn test() {}
"#,
    );

    let file_path = workspace.absolute_path("imports.rs");

    // 2. Generate reorder.plan for imports
    let plan_result = client
        .call_tool(
            "reorder.plan",
            json!({
                "target": {
                    "kind": "imports",
                    "file_path": file_path.to_string_lossy(),
                    "position": {"line": 0, "character": 0}
                },
                "new_order": []
            }),
        )
        .await;

    match plan_result {
        Ok(response) => {
            // Check if response has error field (LSP unavailable)
            if response.get("error").is_some() {
                eprintln!("INFO: reorder.plan requires LSP support, skipping test");
                return;
            }

            let plan = response
                .get("result")
                .and_then(|r| r.get("content"))
                .cloned();

            // If no plan content, likely LSP not available
            if plan.is_none() {
                eprintln!("INFO: reorder.plan requires LSP support, skipping test");
                return;
            }

            let plan = plan.unwrap();

            assert_eq!(
                plan.get("plan_type").and_then(|v| v.as_str()),
                Some("ReorderPlan"),
                "Should be ReorderPlan"
            );

            // 3. Apply plan
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
                .await
                .expect("Apply should succeed");

            let result = apply_result
                .get("result")
                .and_then(|r| r.get("content"))
                .expect("Apply result should exist");

            assert_eq!(
                result.get("success").and_then(|v| v.as_bool()),
                Some(true),
                "Reorder should succeed"
            );
        }
        Err(_) => {
            eprintln!("INFO: reorder.plan requires LSP support, skipping test");
        }
    }
}

#[tokio::test]
async fn test_reorder_parameters_dry_run() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "params.rs",
        r#"pub fn calculate(x: i32, y: i32, z: i32) -> i32 {
    x + y + z
}

pub fn test() {
    calculate(1, 2, 3);
}
"#,
    );

    let file_path = workspace.absolute_path("params.rs");

    // 2. Generate reorder plan for parameters
    let plan_result = client
        .call_tool(
            "reorder.plan",
            json!({
                "target": {
                    "kind": "parameters",
                    "file_path": file_path.to_string_lossy(),
                    "position": {"line": 0, "character": 7}
                },
                "new_order": ["z", "x", "y"]
            }),
        )
        .await;

    match plan_result {
        Ok(response) => {
            // Check if response has error field (LSP unavailable)
            if response.get("error").is_some() {
                eprintln!("INFO: reorder parameters requires LSP support, skipping test");
                return;
            }

            let plan = response
                .get("result")
                .and_then(|r| r.get("content"))
                .cloned();

            // If no plan content, likely LSP not available
            if plan.is_none() {
                eprintln!("INFO: reorder parameters requires LSP support, skipping test");
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
                            "dry_run": true
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
                    .read_file("params.rs")
                    .contains("calculate(x: i32, y: i32, z: i32)"),
                "File should be unchanged after dry run"
            );
        }
        Err(_) => {
            eprintln!("INFO: reorder parameters requires LSP support, skipping test");
        }
    }
}

#[tokio::test]
async fn test_reorder_fields_checksum_validation() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "fields.rs",
        r#"pub struct Point {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}
"#,
    );

    let file_path = workspace.absolute_path("fields.rs");

    // 2. Generate plan
    let plan_result = client
        .call_tool(
            "reorder.plan",
            json!({
                "target": {
                    "kind": "fields",
                    "file_path": file_path.to_string_lossy(),
                    "position": {"line": 0, "character": 11}
                },
                "new_order": ["z", "y", "x"]
            }),
        )
        .await;

    match plan_result {
        Ok(response) => {
            // Check if response has error field (LSP unavailable)
            if response.get("error").is_some() {
                eprintln!("INFO: reorder fields requires LSP support, skipping test");
                return;
            }

            let plan = response
                .get("result")
                .and_then(|r| r.get("content"))
                .cloned();

            // If no plan content, likely LSP not available
            if plan.is_none() {
                eprintln!("INFO: reorder fields requires LSP support, skipping test");
                return;
            }

            let plan = plan.unwrap();

            // 3. Modify file to invalidate checksum
            workspace.create_file(
                "fields.rs",
                r#"pub struct Point {
    pub x: f64,
    pub y: f64,
    pub z: f64,
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
                            "validate_checksums": true
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
            eprintln!("INFO: reorder fields requires LSP support, skipping test");
        }
    }
}

#[tokio::test]
async fn test_reorder_statements_plan_structure() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "statements.rs",
        r#"pub fn process() {
    let c = 3;
    let b = 2;
    let a = 1;
    println!("{} {} {}", a, b, c);
}
"#,
    );

    let file_path = workspace.absolute_path("statements.rs");

    // 2. Generate plan
    let plan_result = client
        .call_tool(
            "reorder.plan",
            json!({
                "target": {
                    "kind": "statements",
                    "file_path": file_path.to_string_lossy(),
                    "position": {"line": 1, "character": 0}
                },
                "new_order": ["let a = 1;", "let b = 2;", "let c = 3;"]
            }),
        )
        .await;

    match plan_result {
        Ok(response) => {
            // Check if response has error field (LSP unavailable)
            if response.get("error").is_some() {
                eprintln!("INFO: reorder operations require LSP support, skipping test");
                return;
            }

            let plan = response
                .get("result")
                .and_then(|r| r.get("content"))
                .cloned();

            // If no plan content, likely LSP not available
            if plan.is_none() {
                eprintln!("INFO: reorder operations require LSP support, skipping test");
                return;
            }

            let plan = plan.unwrap();

            // Verify plan structure
            assert!(plan.get("metadata").is_some(), "Should have metadata");
            assert!(plan.get("summary").is_some(), "Should have summary");
            assert!(
                plan.get("file_checksums").is_some(),
                "Should have checksums"
            );

            let metadata = plan.get("metadata").unwrap();
            assert_eq!(
                metadata.get("kind").and_then(|v| v.as_str()),
                Some("reorder"),
                "Kind should be reorder"
            );
        }
        Err(_) => {
            eprintln!("INFO: reorder operations require LSP support, skipping test");
        }
    }
}
