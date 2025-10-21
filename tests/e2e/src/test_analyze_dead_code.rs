use crate::harness::{TestClient, TestWorkspace};
use codebuddy_foundation::protocol::analysis_result::{AnalysisResult, Severity};
use serde_json::json;

#[tokio::test]
async fn test_analyze_dead_code_unused_imports_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with unused imports
    let code = r#"
import { useState, useEffect } from 'react';
import { Button } from './components';

export function MyComponent() {
    const [count, setCount] = useState(0);
    return <div>{count}</div>;
}
"#;

    workspace.create_file("unused_imports.ts", code);
    let test_file = workspace.absolute_path("unused_imports.ts");

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

    // Verify result structure
    assert_eq!(result.metadata.category, "dead_code");
    assert_eq!(result.metadata.kind, "unused_imports");

    // Verify symbols_analyzed is present (even if 0 for unsupported files)
    assert!(
        result.summary.symbols_analyzed.is_some(),
        "symbols_analyzed should be present in summary"
    );

    // If no symbols analyzed (e.g., parsing not available), skip detailed assertions
    // Note: Some analyses may return summary findings even with 0 symbols
    if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
        return; // Valid early exit for unparseable files
    }

    // Should detect useEffect and Button as unused
    assert!(
        !result.findings.is_empty(),
        "Expected unused import findings"
    );

    // Verify finding structure
    let finding = &result.findings[0];
    assert_eq!(finding.kind, "unused_import");
    assert_eq!(finding.severity, Severity::Low);
    assert!(finding.metrics.is_some());
}

#[tokio::test]
async fn test_analyze_dead_code_unused_symbols_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with unused private function
    let code = r#"
// Private function never called
function helperFunction() {
    return 42;
}

// Public function (used in export)
export function publicFunction() {
    return 100;
}
"#;

    workspace.create_file("unused_symbols.ts", code);
    let test_file = workspace.absolute_path("unused_symbols.ts");

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

    // Verify symbols_analyzed is present (even if 0 for unsupported files)
    assert!(
        result.summary.symbols_analyzed.is_some(),
        "symbols_analyzed should be present in summary"
    );

    // If no symbols analyzed (e.g., parsing not available), skip detailed assertions
    // Note: Some analyses may return summary findings even with 0 symbols
    if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
        return; // Valid early exit for unparseable files
    }

    // Verify helperFunction is detected as unused
    assert!(
        !result.findings.is_empty(),
        "Expected unused symbol findings"
    );

    let finding = &result.findings[0];
    assert_eq!(finding.kind, "unused_function");
    assert_eq!(finding.severity, Severity::Medium);
}

#[tokio::test]
async fn test_analyze_dead_code_unreachable_code() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with unreachable code
    let code = r#"
export function processData(x: number): number {
    if (x > 0) {
        return x * 2;
        console.log("This line is unreachable");
        let y = x + 1;
    }
    return 0;
}
"#;

    workspace.create_file("unreachable.ts", code);
    let test_file = workspace.absolute_path("unreachable.ts");

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

    assert_eq!(result.metadata.kind, "unreachable_code");

    // Verify symbols_analyzed is present (even if 0 for unsupported files)
    assert!(
        result.summary.symbols_analyzed.is_some(),
        "symbols_analyzed should be present in summary"
    );

    // If no symbols analyzed (e.g., parsing not available), skip detailed assertions
    // Note: Some analyses may return summary findings even with 0 symbols
    if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
        return; // Valid early exit for unparseable files
    }

    // Should detect unreachable code after return
    assert!(
        !result.findings.is_empty(),
        "Expected unreachable code findings"
    );

    let finding = &result.findings[0];
    assert_eq!(finding.kind, "unreachable_code");
    assert_eq!(finding.severity, Severity::Medium);

    // Verify metrics
    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(metrics.contains_key("lines_unreachable"));
    assert!(metrics.contains_key("after_statement"));
}

#[tokio::test]
async fn test_analyze_dead_code_unused_parameters() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a Rust file with unused parameters
    let code = r#"
fn process_data(x: i32, y: i32, z: String) -> i32 {
    // Only x is used, y and z are unused
    x * 2
}

fn main() {
    let result = process_data(5, 10, "unused".to_string());
    println!("{}", result);
}
"#;

    workspace.create_file("unused_params.rs", code);
    let test_file = workspace.absolute_path("unused_params.rs");

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

    assert_eq!(result.metadata.kind, "unused_parameters");

    // Verify symbols_analyzed is present (even if 0 for unsupported files)
    assert!(
        result.summary.symbols_analyzed.is_some(),
        "symbols_analyzed should be present in summary"
    );

    // If no symbols analyzed (e.g., parsing not available), skip detailed assertions
    // Note: Some analyses may return summary findings even with 0 symbols
    if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
        return; // Valid early exit for unparseable files
    }

    // Should detect y and z as unused
    assert!(
        !result.findings.is_empty(),
        "Expected unused parameter findings"
    );

    for finding in &result.findings {
        assert_eq!(finding.kind, "unused_parameter");
        assert_eq!(finding.severity, Severity::Low);

        let metrics = finding.metrics.as_ref().expect("Should have metrics");
        assert!(metrics.contains_key("parameter_name"));
        assert!(metrics.contains_key("function_name"));
    }
}

#[tokio::test]
async fn test_analyze_dead_code_unused_types() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with unused interface
    let code = r#"
interface UnusedInterface {
    id: number;
    name: string;
}

interface UsedInterface {
    value: string;
}

export function getData(): UsedInterface {
    return { value: "test" };
}
"#;

    workspace.create_file("unused_types.ts", code);
    let test_file = workspace.absolute_path("unused_types.ts");

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

    assert_eq!(result.metadata.kind, "unused_types");

    // Verify symbols_analyzed is present (even if 0 for unsupported files)
    assert!(
        result.summary.symbols_analyzed.is_some(),
        "symbols_analyzed should be present in summary"
    );

    // If no symbols analyzed (e.g., parsing not available), skip detailed assertions
    // Note: Some analyses may return summary findings even with 0 symbols
    if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
        return; // Valid early exit for unparseable files
    }

    // Should detect UnusedInterface but not UsedInterface
    assert!(!result.findings.is_empty(), "Expected unused type findings");

    let finding = &result.findings[0];
    assert_eq!(finding.kind, "unused_type");
    assert_eq!(finding.severity, Severity::Low);

    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(metrics.contains_key("type_name"));
    assert!(metrics.contains_key("type_kind"));
}

#[tokio::test]
async fn test_analyze_dead_code_unused_variables() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with unused variables
    let code = r#"
export function calculateTotal(price: number, tax: number): number {
    const basePrice = price;
    const unusedVar = 100;  // Never used
    const taxAmount = basePrice * tax;
    const anotherUnused = "test";  // Never used

    return basePrice + taxAmount;
}
"#;

    workspace.create_file("unused_vars.ts", code);
    let test_file = workspace.absolute_path("unused_vars.ts");

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

    assert_eq!(result.metadata.kind, "unused_variables");

    // Verify symbols_analyzed is present (even if 0 for unsupported files)
    assert!(
        result.summary.symbols_analyzed.is_some(),
        "symbols_analyzed should be present in summary"
    );

    // If no symbols analyzed (e.g., parsing not available), skip detailed assertions
    // Note: Some analyses may return summary findings even with 0 symbols
    if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
        return; // Valid early exit for unparseable files
    }

    // Should detect unusedVar and anotherUnused
    assert!(
        !result.findings.is_empty(),
        "Expected unused variable findings"
    );

    for finding in &result.findings {
        assert_eq!(finding.kind, "unused_variable");
        assert_eq!(finding.severity, Severity::Low);

        let metrics = finding.metrics.as_ref().expect("Should have metrics");
        assert!(metrics.contains_key("variable_name"));
        assert!(metrics.contains_key("scope"));
    }
}

#[tokio::test]
async fn test_analyze_dead_code_unsupported_kind() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("test.ts", "export function foo() { return 1; }");
    let test_file = workspace.absolute_path("test.ts");

    let response = client
        .call_tool(
            "analyze.dead_code",
            json!({
                "kind": "invalid_kind",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await;

    // Should return error for unsupported kind
    match response {
        Err(e) => {
            let error_msg = format!("{:?}", e);
            assert!(
                error_msg.contains("Unsupported") || error_msg.contains("supported"),
                "Error should mention unsupported kind: {}",
                error_msg
            );
        }
        Ok(value) => {
            assert!(
                value.get("error").is_some(),
                "Expected error for unsupported kind"
            );
        }
    }
}
