use crate::harness::{TestClient, TestWorkspace};
use codebuddy_foundation::protocol::analysis_result::{ AnalysisResult , SafetyLevel };
use serde_json::json;

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
    workspace.create_file("anotherFile.ts", "export function unusedFunction() {}; export function usedFunction() {};");
    let test_file = workspace.absolute_path("test_file.ts");

    let response = client
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

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    assert!(!result.findings.is_empty(), "Should have findings for unused import");

    let finding = &result.findings[0];
    assert!(!finding.suggestions.is_empty(), "Should have suggestions");

    let suggestion = &finding.suggestions[0];
    assert!(matches!(suggestion.safety, SafetyLevel::Safe | SafetyLevel::RequiresReview));
    assert!(suggestion.confidence >= 0.0 && suggestion.confidence <= 1.0);
    assert!(suggestion.refactor_call.is_some(), "Should have refactor_call");

    let refactor_call = suggestion.refactor_call.as_ref().unwrap();
    assert_eq!(refactor_call.command, "delete.plan");
    assert_eq!(refactor_call.arguments["kind"], "import");
    assert_eq!(refactor_call.arguments["target"]["file_path"].as_str().unwrap(), test_file.to_string_lossy());
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
    let test_file = workspace.absolute_path("test_file.ts");

    let response = client
        .call_tool(
            "analyze.dead_code",
            json!({
                "kind": "unused_symbols",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.dead_code call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    assert!(!result.findings.is_empty(), "Should have findings for unused function");

    let finding = &result.findings[0];
    assert_eq!(finding.location.symbol.as_ref().unwrap(), "unusedFunction");
    assert!(!finding.suggestions.is_empty(), "Should have suggestions");

    let suggestion = &finding.suggestions[0];
    assert!(suggestion.refactor_call.is_some(), "Should have refactor_call");

    let refactor_call = suggestion.refactor_call.as_ref().unwrap();
    assert_eq!(refactor_call.command, "delete.plan");
    assert_eq!(refactor_call.arguments["kind"], "function");
    assert_eq!(refactor_call.arguments["target"]["file_path"].as_str().unwrap(), test_file.to_string_lossy());
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
    let test_file = workspace.absolute_path("test_file.ts");

    let response = client
        .call_tool(
            "analyze.dead_code",
            json!({
                "kind": "unreachable_code",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.dead_code call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    assert!(!result.findings.is_empty(), "Should have findings for unreachable code");

    let finding = &result.findings[0];
    assert!(!finding.suggestions.is_empty(), "Should have suggestions");

    let suggestion = &finding.suggestions[0];
    assert!(suggestion.refactor_call.is_some(), "Should have refactor_call");

    let refactor_call = suggestion.refactor_call.as_ref().unwrap();
    assert_eq!(refactor_call.command, "delete.plan");
    assert_eq!(refactor_call.arguments["kind"], "block");
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
    let test_file = workspace.absolute_path("test_file.ts");

    let response = client
        .call_tool(
            "analyze.dead_code",
            json!({
                "kind": "unused_parameters",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.dead_code call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    assert!(!result.findings.is_empty(), "Should have findings for unused_parameters");

    let finding = &result.findings[0];
    assert!(!finding.suggestions.is_empty(), "Should have suggestions");

    let suggestion = &finding.suggestions[0];
    assert!(suggestion.refactor_call.is_some(), "Should have refactor_call");

    let refactor_call = suggestion.refactor_call.as_ref().unwrap();
    assert_eq!(refactor_call.command, "delete.plan");
    assert_eq!(refactor_call.arguments["kind"], "parameter");
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
    let test_file = workspace.absolute_path("test_file.ts");

    let response = client
        .call_tool(
            "analyze.dead_code",
            json!({
                "kind": "unused_types",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.dead_code call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    assert!(!result.findings.is_empty(), "Should have findings for unused_types");

    let finding = &result.findings[0];
    assert!(!finding.suggestions.is_empty(), "Should have suggestions");

    let suggestion = &finding.suggestions[0];
    assert!(suggestion.refactor_call.is_some(), "Should have refactor_call");

    let refactor_call = suggestion.refactor_call.as_ref().unwrap();
    assert_eq!(refactor_call.command, "delete.plan");
    assert_eq!(refactor_call.arguments["kind"], "type");
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
    let test_file = workspace.absolute_path("test_file.ts");

    let response = client
        .call_tool(
            "analyze.dead_code",
            json!({
                "kind": "unused_variables",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.dead_code call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    assert!(!result.findings.is_empty(), "Should have findings for unused_variables");

    let finding = &result.findings[0];
    assert!(!finding.suggestions.is_empty(), "Should have suggestions");

    let suggestion = &finding.suggestions[0];
    assert!(suggestion.refactor_call.is_some(), "Should have refactor_call");

    let refactor_call = suggestion.refactor_call.as_ref().unwrap();
    assert_eq!(refactor_call.command, "delete.plan");
    assert_eq!(refactor_call.arguments["kind"], "variable");
}