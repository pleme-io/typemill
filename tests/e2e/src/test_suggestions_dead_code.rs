//! Suggestions for dead code analysis - migrated to closure-based helpers (v2)
//!
//! BEFORE: 368 lines with repetitive setup
//! AFTER: Helper-based suggestion verification
//!
//! Tests actionable suggestions generated for dead code analysis.

use crate::harness::{TestClient, TestWorkspace};
use mill_foundation::protocol::analysis_result::{AnalysisResult, SafetyLevel};
use serde_json::json;

/// Helper: Call analyze.dead_code and parse result
async fn analyze_dead_code(
    workspace: &TestWorkspace,
    client: &mut TestClient,
    kind: &str,
    file: &str,
) -> AnalysisResult {
    let test_file = workspace.absolute_path(file);
    let response = client
        .call_tool(
            "analyze.dead_code",
            json!({
                "kind": kind,
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.dead_code call should succeed");

    serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult")
}

/// Helper: Verify suggestion structure
fn verify_suggestion(result: &AnalysisResult, expected_command: &str, expected_kind: &str) {
    assert!(!result.findings.is_empty());

    let finding = &result.findings[0];
    assert!(!finding.suggestions.is_empty());

    let suggestion = &finding.suggestions[0];
    assert!(matches!(
        suggestion.safety,
        SafetyLevel::Safe | SafetyLevel::RequiresReview
    ));
    assert!(suggestion.confidence >= 0.0 && suggestion.confidence <= 1.0);
    assert!(suggestion.refactor_call.is_some());

    let refactor_call = suggestion.refactor_call.as_ref().unwrap();
    assert_eq!(refactor_call.command, expected_command);
    assert_eq!(refactor_call.arguments["kind"], expected_kind);
}

#[tokio::test]
async fn test_dead_code_analysis_generates_suggestions_for_unused_import() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let test_code = r#"
import { unusedFunction } from './anotherFile';
import { usedFunction } from './anotherFile';

function main() {
    usedFunction();
}
"#;
    workspace.create_file("test_file.ts", test_code);
    workspace.create_file(
        "anotherFile.ts",
        "export function unusedFunction() {}; export function usedFunction() {};",
    );

    let result = analyze_dead_code(&workspace, &mut client, "unused_imports", "test_file.ts").await;
    verify_suggestion(&result, "delete.plan", "import");

    let test_file = workspace.absolute_path("test_file.ts");
    let refactor_call = &result.findings[0].suggestions[0].refactor_call.as_ref().unwrap();
    assert_eq!(
        refactor_call.arguments["target"]["filePath"].as_str().unwrap(),
        test_file.to_string_lossy()
    );
}

#[tokio::test]
async fn test_dead_code_analysis_generates_suggestions_for_unused_function() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let test_code = r#"
function unusedFunction() {
    return "I am never used.";
}

function usedFunction() {
    return "I am used.";
}

usedFunction();
"#;
    workspace.create_file("test_file.ts", test_code);

    let result = analyze_dead_code(&workspace, &mut client, "unused_symbols", "test_file.ts").await;

    assert!(!result.findings.is_empty());
    let finding = &result.findings[0];
    assert_eq!(finding.location.symbol.as_ref().unwrap(), "unusedFunction");

    verify_suggestion(&result, "delete.plan", "function");
}

#[tokio::test]
async fn test_dead_code_analysis_generates_suggestions_for_unreachable_code() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let test_code = r#"
function unreachable() {
    return "I am reachable.";
    const x = "I am not.";
}
"#;
    workspace.create_file("test_file.ts", test_code);

    let result = analyze_dead_code(&workspace, &mut client, "unreachable_code", "test_file.ts").await;
    verify_suggestion(&result, "delete.plan", "block");
}

#[tokio::test]
async fn test_dead_code_analysis_generates_suggestions_for_unused_parameter() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let test_code = r#"
function unusedParameter(a: number, b: number) {
    return a;
}
unusedParameter(1, 2);
"#;
    workspace.create_file("test_file.ts", test_code);

    let result = analyze_dead_code(&workspace, &mut client, "unused_parameters", "test_file.ts").await;
    verify_suggestion(&result, "delete.plan", "parameter");
}

#[tokio::test]
async fn test_dead_code_analysis_generates_suggestions_for_unused_type() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let test_code = r#"
interface UnusedType {
    a: number;
}
"#;
    workspace.create_file("test_file.ts", test_code);

    let result = analyze_dead_code(&workspace, &mut client, "unused_types", "test_file.ts").await;
    verify_suggestion(&result, "delete.plan", "type");
}

#[tokio::test]
async fn test_dead_code_analysis_generates_suggestions_for_unused_variable() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let test_code = r#"
function unusedVariable() {
    const a = 1;
    return 2;
}
unusedVariable();
"#;
    workspace.create_file("test_file.ts", test_code);

    let result = analyze_dead_code(&workspace, &mut client, "unused_variables", "test_file.ts").await;
    verify_suggestion(&result, "delete.plan", "variable");
}
