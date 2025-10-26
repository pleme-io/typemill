use super::test_helpers::{setup_test_server, TestFixture};
use mill_foundation::protocol::analysis_result::{Finding, SafetyLevel};
use serde_json::json;

#[tokio::test]
async fn test_closed_loop_workflow_dead_code_removal() {
    let server = setup_test_server().await;
    let fixture = TestFixture::new("typescript", "unused-code.ts").await;

    // Step 1: Analyze
    let analysis_result = server
        .call_tool(
            "analyze.dead_code",
            json!({
                "file_path": fixture.file_path,
                "kinds": ["unused_import"],
            }),
        )
        .await
        .unwrap();

    let findings: Vec<Finding> = serde_json::from_value(analysis_result["findings"].clone()).unwrap();
    assert!(!findings.is_empty(), "Should have findings");

    // Step 2: Find safe suggestion
    let safe_suggestion = findings[0]
        .suggestions
        .iter()
        .find(|s| s.safety == SafetyLevel::Safe && s.confidence > 0.9)
        .expect("Should have safe suggestion");

    // Step 3: Apply suggestion via refactor_call (unified dryRun API)
    let refactor_call = safe_suggestion.refactor_call.as_ref().unwrap();

    // Add dryRun: false to execute the refactoring
    let mut arguments = refactor_call.arguments.clone();
    arguments["options"] = json!({ "dryRun": false });

    let apply_result = server
        .call_tool(&refactor_call.command, arguments)
        .await
        .unwrap();

    assert_eq!(apply_result["success"], true);

    // Step 4: Re-analyze to verify fix
    let reanalysis_result = server
        .call_tool(
            "analyze.dead_code",
            json!({
                "file_path": fixture.file_path,
                "kinds": ["unused_import"],
            }),
        )
        .await
        .unwrap();

    let new_findings: Vec<Finding> = serde_json::from_value(reanalysis_result["findings"].clone()).unwrap();

    // Issue should be fixed
    assert!(new_findings.is_empty() || new_findings.len() < findings.len());
}