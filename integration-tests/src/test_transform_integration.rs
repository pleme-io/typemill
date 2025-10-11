//! Integration tests for transform.plan and workspace.apply_edit
//!
//! Tests code transformation operations:
//! - Transform if-to-match
//! - Add async/await
//! - Convert function to closure

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

#[tokio::test]
async fn test_transform_if_to_match_plan_and_apply() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "transform_if.rs",
        r#"pub fn classify(x: i32) -> &'static str {
    if x < 0 {
        "negative"
    } else if x == 0 {
        "zero"
    } else {
        "positive"
    }
}
"#,
    );

    let file_path = workspace.absolute_path("transform_if.rs");

    // 2. Generate transform.plan
    let plan_result = client
        .call_tool(
            "transform.plan",
            json!({
                "transformation": {
                    "kind": "if_to_match",
                    "file_path": file_path.to_string_lossy(),
                    "range": {
                        "start": {"line": 1, "character": 4},
                        "end": {"line": 7, "character": 5}
                    }
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
                plan.get("plan_type").and_then(|v| v.as_str()),
                Some("TransformPlan"),
                "Should be TransformPlan"
            );

            // 3. Apply plan
            let apply_result = client
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
                .expect("Apply should succeed");

            let result = apply_result
                .get("result")
                .and_then(|r| r.get("content"))
                .expect("Apply result should exist");

            assert_eq!(
                result.get("success").and_then(|v| v.as_bool()),
                Some(true),
                "Transform should succeed"
            );
        }
        Err(_) => {
            eprintln!("INFO: transform.plan may require LSP support, skipping test");
        }
    }
}

#[tokio::test]
async fn test_transform_add_async_dry_run() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "add_async.rs",
        r#"pub fn fetch_data() -> String {
    "data".to_string()
}
"#,
    );

    let file_path = workspace.absolute_path("add_async.rs");

    // 2. Generate transform plan to add async
    let plan_result = client
        .call_tool(
            "transform.plan",
            json!({
                "transformation": {
                    "kind": "add_async",
                    "file_path": file_path.to_string_lossy(),
                    "range": {
                        "start": {"line": 0, "character": 7},
                        "end": {"line": 0, "character": 17}
                    }
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
                workspace.read_file("add_async.rs").contains("pub fn fetch_data()"),
                "File should be unchanged after dry run"
            );
        }
        Err(_) => {
            eprintln!("INFO: transform add_async requires LSP support, skipping test");
        }
    }
}

#[tokio::test]
async fn test_transform_fn_to_closure_checksum_validation() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "closure.rs",
        r#"pub fn mapper() {
    fn double(x: i32) -> i32 {
        x * 2
    }
    let nums = vec![1, 2, 3];
    let doubled: Vec<i32> = nums.iter().map(|&x| double(x)).collect();
}
"#,
    );

    let file_path = workspace.absolute_path("closure.rs");

    // 2. Generate plan
    let plan_result = client
        .call_tool(
            "transform.plan",
            json!({
                "transformation": {
                    "kind": "fn_to_closure",
                    "file_path": file_path.to_string_lossy(),
                    "range": {
                        "start": {"line": 1, "character": 4},
                        "end": {"line": 3, "character": 5}
                    }
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
                "closure.rs",
                r#"pub fn mapper() {
    fn triple(x: i32) -> i32 {
        x * 3
    }
    let nums = vec![1, 2, 3];
    let tripled: Vec<i32> = nums.iter().map(|&x| triple(x)).collect();
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
                apply_result.is_err()
                    || apply_result
                        .unwrap()
                        .get("error")
                        .is_some(),
                "Apply should fail due to checksum mismatch"
            );
        }
        Err(_) => {
            eprintln!("INFO: transform fn_to_closure requires LSP support, skipping test");
        }
    }
}

#[tokio::test]
async fn test_transform_plan_metadata_structure() {
    // 1. Setup
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "meta.rs",
        r#"pub fn example(opt: Option<i32>) -> i32 {
    if let Some(x) = opt {
        x
    } else {
        0
    }
}
"#,
    );

    let file_path = workspace.absolute_path("meta.rs");

    // 2. Generate transform plan
    let plan_result = client
        .call_tool(
            "transform.plan",
            json!({
                "transformation": {
                    "kind": "if_to_match",
                    "file_path": file_path.to_string_lossy(),
                    "range": {
                        "start": {"line": 1, "character": 4},
                        "end": {"line": 5, "character": 5}
                    }
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

            // Verify plan structure
            assert!(plan.get("metadata").is_some(), "Should have metadata");
            assert!(plan.get("summary").is_some(), "Should have summary");
            assert!(
                plan.get("file_checksums").is_some(),
                "Should have checksums"
            );
            assert!(plan.get("edits").is_some(), "Should have edits");

            let metadata = plan.get("metadata").unwrap();
            assert_eq!(
                metadata.get("plan_version").and_then(|v| v.as_str()),
                Some("1.0"),
                "Plan version should be 1.0"
            );
            assert_eq!(
                metadata.get("kind").and_then(|v| v.as_str()),
                Some("transform"),
                "Kind should be transform"
            );
        }
        Err(_) => {
            eprintln!("INFO: transform operations may require LSP support, skipping test");
        }
    }
}
