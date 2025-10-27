//! Closed-loop workflow test: Analysis → Suggestion → Apply → Verify
//!
//! Tests the complete end-to-end workflow of analyzing code, getting suggestions,
//! applying the suggested fix, and verifying the issue is resolved.

use crate::harness::{TestClient, TestWorkspace};
use mill_foundation::protocol::analysis_result::{AnalysisResult, SafetyLevel};
use serde_json::json;

#[tokio::test]
async fn test_closed_loop_workflow_dead_code_removal() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Step 1: Create a file with unused import
    let test_code = r#"
import { unusedFunction, usedFunction } from './anotherFile';

function main() {
    usedFunction();
}
"#;
    workspace.create_file("test_file.ts", test_code);
    workspace.create_file(
        "anotherFile.ts",
        "export function unusedFunction() {}; export function usedFunction() {};",
    );

    // Step 2: Analyze and get findings
    let test_file = workspace.absolute_path("test_file.ts");
    let analysis_response = client
        .call_tool(
            "analyze.dead_code",
            json!({
                "kind": "unused_imports",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.dead_code call should succeed");

    let analysis_result: AnalysisResult = serde_json::from_value(
        analysis_response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    assert!(
        !analysis_result.findings.is_empty(),
        "Should have findings for unused import"
    );

    // Step 3: Find safe suggestion
    let finding = &analysis_result.findings[0];
    let safe_suggestion = finding
        .suggestions
        .iter()
        .find(|s| s.safety == SafetyLevel::Safe && s.confidence > 0.7)
        .expect("Should have a safe suggestion");

    assert!(
        safe_suggestion.refactor_call.is_some(),
        "Safe suggestion should have refactor_call"
    );

    let refactor_call = safe_suggestion.refactor_call.as_ref().unwrap();

    // Step 4: Apply the suggestion via refactor_call (unified dryRun API)
    let mut apply_params = refactor_call.arguments.clone();
    if let Some(options) = apply_params.get_mut("options") {
        options["dryRun"] = json!(false);
    } else {
        apply_params["options"] = json!({ "dryRun": false });
    }

    let apply_response = client
        .call_tool(&refactor_call.command, apply_params)
        .await
        .expect("Refactor call should succeed");

    // Verify the operation succeeded
    let apply_result = apply_response
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Apply result should exist");

    assert_eq!(
        apply_result.get("success").and_then(|v| v.as_bool()),
        Some(true),
        "Refactor operation should succeed"
    );

    // Step 5: Re-analyze to verify the fix
    let reanalysis_response = client
        .call_tool(
            "analyze.dead_code",
            json!({
                "kind": "unused_imports",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("Re-analysis should succeed");

    let reanalysis_result: AnalysisResult = serde_json::from_value(
        reanalysis_response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    // Issue should be fixed - either no findings or fewer findings
    assert!(
        reanalysis_result.findings.is_empty() || reanalysis_result.findings.len() < analysis_result.findings.len(),
        "Issue should be fixed after applying suggestion"
    );
}
