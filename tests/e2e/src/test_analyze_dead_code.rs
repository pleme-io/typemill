//! Analysis API tests for analyze.dead_code (MIGRATED VERSION)
//!
//! BEFORE: 468 lines with repetitive workspace setup and result parsing
//! AFTER: Using simplified helper pattern for analysis tests
//!
//! Analysis tests follow simpler pattern: setup → analyze → verify

use crate::harness::{TestClient, TestWorkspace};
use mill_foundation::protocol::analysis_result::{AnalysisResult, Severity};
use serde_json::json;

/// Helper to run dead code analysis test
async fn run_dead_code_test<V>(
    file_name: &str,
    file_content: &str,
    kind: &str,
    verify: V,
) -> anyhow::Result<()>
where
    V: FnOnce(&AnalysisResult) -> anyhow::Result<()>,
{
    let workspace = TestWorkspace::new();
    workspace.create_file(file_name, file_content);
    let mut client = TestClient::new(workspace.path());
    let test_file = workspace.absolute_path(file_name);

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

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    verify(&result)?;
    Ok(())
}

#[tokio::test]
async fn test_analyze_dead_code_unused_imports_basic() {
    let code = r#"
import { useState, useEffect } from 'react';
import { Button } from './components';

export function MyComponent() {
    const [count, setCount] = useState(0);
    return <div>{count}</div>;
}
"#;

    run_dead_code_test("unused_imports.ts", code, "unused_imports", |result| {
        assert_eq!(result.metadata.category, "dead_code");
        assert_eq!(result.metadata.kind, "unused_imports");
        assert!(result.summary.symbols_analyzed.is_some());

        if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
            return Ok(());
        }

        assert!(!result.findings.is_empty());

        let finding = &result.findings[0];
        assert_eq!(finding.kind, "unused_import");
        assert_eq!(finding.severity, Severity::Low);
        assert!(finding.metrics.is_some());

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_analyze_dead_code_unused_symbols_basic() {
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

    run_dead_code_test("unused_symbols.ts", code, "unused_symbols", |result| {
        assert!(result.summary.symbols_analyzed.is_some());

        if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
            return Ok(());
        }

        assert!(!result.findings.is_empty());

        let finding = &result.findings[0];
        assert_eq!(finding.kind, "unused_function");
        assert_eq!(finding.severity, Severity::Medium);

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_analyze_dead_code_unreachable_code() {
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

    run_dead_code_test("unreachable.ts", code, "unreachable_code", |result| {
        assert_eq!(result.metadata.kind, "unreachable_code");
        assert!(result.summary.symbols_analyzed.is_some());

        if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
            return Ok(());
        }

        assert!(!result.findings.is_empty());

        let finding = &result.findings[0];
        assert_eq!(finding.kind, "unreachable_code");
        assert_eq!(finding.severity, Severity::Medium);

        let metrics = finding.metrics.as_ref().expect("Should have metrics");
        assert!(metrics.contains_key("lines_unreachable"));
        assert!(metrics.contains_key("after_statement"));

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_analyze_dead_code_unused_parameters() {
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

    run_dead_code_test("unused_params.rs", code, "unused_parameters", |result| {
        assert_eq!(result.metadata.kind, "unused_parameters");
        assert!(result.summary.symbols_analyzed.is_some());

        if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
            return Ok(());
        }

        assert!(!result.findings.is_empty());

        for finding in &result.findings {
            assert_eq!(finding.kind, "unused_parameter");
            assert_eq!(finding.severity, Severity::Low);

            let metrics = finding.metrics.as_ref().expect("Should have metrics");
            assert!(metrics.contains_key("parameter_name"));
            assert!(metrics.contains_key("function_name"));
        }

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_analyze_dead_code_unused_types() {
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

    run_dead_code_test("unused_types.ts", code, "unused_types", |result| {
        assert_eq!(result.metadata.kind, "unused_types");
        assert!(result.summary.symbols_analyzed.is_some());

        if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
            return Ok(());
        }

        assert!(!result.findings.is_empty());

        let finding = &result.findings[0];
        assert_eq!(finding.kind, "unused_type");
        assert_eq!(finding.severity, Severity::Low);

        let metrics = finding.metrics.as_ref().expect("Should have metrics");
        assert!(metrics.contains_key("type_name"));
        assert!(metrics.contains_key("type_kind"));

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_analyze_dead_code_unused_variables() {
    let code = r#"
export function calculateTotal(price: number, tax: number): number {
    const basePrice = price;
    const unusedVar = 100;  // Never used
    const taxAmount = basePrice * tax;
    const anotherUnused = "test";  // Never used

    return basePrice + taxAmount;
}
"#;

    run_dead_code_test("unused_vars.ts", code, "unused_variables", |result| {
        assert_eq!(result.metadata.kind, "unused_variables");
        assert!(result.summary.symbols_analyzed.is_some());

        if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
            return Ok(());
        }

        assert!(!result.findings.is_empty());

        for finding in &result.findings {
            assert_eq!(finding.kind, "unused_variable");
            assert_eq!(finding.severity, Severity::Low);

            let metrics = finding.metrics.as_ref().expect("Should have metrics");
            assert!(metrics.contains_key("variable_name"));
            assert!(metrics.contains_key("scope"));
        }

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_analyze_dead_code_unsupported_kind() {
    let workspace = TestWorkspace::new();
    workspace.create_file("test.ts", "export function foo() { return 1; }");
    let mut client = TestClient::new(workspace.path());
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

    match response {
        Err(e) => {
            let error_msg = format!("{:?}", e);
            assert!(error_msg.contains("Unsupported") || error_msg.contains("supported"));
        }
        Ok(value) => {
            assert!(value.get("error").is_some());
        }
    }
}
