use super::test_helpers::{setup_test_server, TestFixture};
use mill_foundation::protocol::analysis_result::{Finding, SafetyLevel};
use serde_json::json;

#[tokio::test]
async fn test_tests_analysis_generates_suggestions() {
    let server = setup_test_server().await;
    let fixture = TestFixture::new("typescript", "untested-code.ts").await;

    let result = server
        .call_tool(
            "analyze.tests",
            json!({
                "file_path": fixture.file_path,
                "kinds": ["coverage"],
            }),
        )
        .await
        .unwrap();

    let findings: Vec<Finding> = serde_json::from_value(result["findings"].clone()).unwrap();

    // Assert suggestions exist
    assert!(!findings.is_empty(), "Should have coverage findings");
    let finding = &findings[0];
    assert!(!finding.suggestions.is_empty(), "Should have suggestions");

    // Assert suggestion has required fields
    let suggestion = &finding.suggestions[0];
    assert!(matches!(
        suggestion.safety,
        SafetyLevel::Safe | SafetyLevel::RequiresReview
    ));
    assert!(suggestion.confidence >= 0.0 && suggestion.confidence <= 1.0);
    assert!(suggestion.refactor_call.is_some(), "Should have refactor_call");
}