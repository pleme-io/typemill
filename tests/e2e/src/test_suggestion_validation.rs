use crate::harness::{
    TestClient,
    TestWorkspace,
    TEST_DATA_DIR,
};
use codebuddy_foundation::protocol::analysis_result::{ AnalysisResult , Finding };
use mill_handlers::handlers::tools::analysis::suggestions::validation::validate_suggestion;
use serde_json::json;

#[tokio::test]
async fn test_all_suggestions_pass_validation() {
    let workspace = TestWorkspace::new(TEST_DATA_DIR).await;
    let client = TestClient::new(&workspace.root).await;

    // Test a batch analysis call that should generate suggestions.
    let response = client
        .call_tool(
            "analyze.batch",
            json!({
                "queries": [
                    {
                        "command": "analyze.quality",
                        "kind": "complexity",
                        "scope": { "type": "file", "path": "complex.ts" }
                    },
                    {
                        "command": "analyze.dead_code",
                        "kind": "unused_imports",
                        "scope": { "type": "file", "path": "complex.ts" }
                    }
                ]
            }),
        )
        .await
        .unwrap();

    // Extract results from the JSON response
    let results: Vec<AnalysisResult> = serde_json::from_value(response["results"].clone()).unwrap();

    // Iterate through the findings and validate each suggestion.
    for result in results {
        for finding in result.findings {
            for suggestion in finding.suggestions {
                validate_suggestion(&suggestion).unwrap_or_else(|e| {
                    panic!(
                        "Invalid suggestion in {} (tool: {}): {:?}, error: {}",
                        finding.location.file_path, result.metadata.category, suggestion, e
                    )
                });
            }
        }
    }
}