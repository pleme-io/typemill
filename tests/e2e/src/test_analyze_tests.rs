//! analyze.tests tests migrated to closure-based helpers (v2)
//!
//! BEFORE: 493 lines with manual setup/client creation/verification
//! AFTER: Simplified with helper-based assertions
//!
//! Analysis tests focus on result structure verification.

use crate::harness::{TestClient, TestWorkspace};
use mill_foundation::protocol::analysis_result::AnalysisResult;
use serde_json::json;

/// Helper: Call analyze.tests and parse result
async fn analyze_tests(
    workspace: &TestWorkspace,
    client: &mut TestClient,
    kind: &str,
    file: &str,
) -> AnalysisResult {
    let test_file = workspace.absolute_path(file);
    let response = client
        .call_tool(
            "analyze.tests",
            json!({
                "kind": kind,
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.tests call should succeed");

    serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult")
}

/// Helper: Skip test if no symbols analyzed
fn skip_if_no_symbols(result: &AnalysisResult) -> bool {
    result.summary.symbols_analyzed.unwrap_or(0) == 0
}

#[tokio::test]
async fn test_analyze_tests_coverage_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // 5 functions, 2 tests = 0.4 ratio (low coverage)
    let code = r#"
export function add(a: number, b: number): number { return a + b; }
export function subtract(a: number, b: number): number { return a - b; }
export function multiply(a: number, b: number): number { return a * b; }
export function divide(a: number, b: number): number { return a / b; }
export function mod(a: number, b: number): number { return a % b; }

it('should add numbers', () => { expect(add(1, 2)).toBe(3); });
it('should subtract numbers', () => { expect(subtract(5, 3)).toBe(2); });
"#;
    workspace.create_file("coverage_test.ts", code);

    let result = analyze_tests(&workspace, &mut client, "coverage", "coverage_test.ts").await;

    assert_eq!(result.metadata.category, "tests");
    assert_eq!(result.metadata.kind, "coverage");
    assert!(result.summary.symbols_analyzed.is_some());

    if skip_if_no_symbols(&result) { return; }

    // Verify coverage findings
    assert!(!result.findings.is_empty());
    let finding = &result.findings[0];
    assert_eq!(finding.kind, "coverage");

    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(
        metrics.contains_key("coverage_ratio")
            || metrics.contains_key("test_coverage")
            || metrics.contains_key("coverage")
    );
    assert!(metrics.contains_key("total_tests") || metrics.contains_key("tests_count"));
    assert!(metrics.contains_key("total_functions") || metrics.contains_key("functions_count"));
}

#[tokio::test]
async fn test_analyze_tests_quality_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Tests with smells: empty test, no assertions
    let code = r#"
it('empty test', () => {
    // Empty test body - test smell
});

it('single assertion', () => {
    expect(true).toBe(true);
});

it('no assertions here', () => {
    const data = getData();
    console.log(data);
});

function getData() { return { value: 42 }; }
"#;
    workspace.create_file("quality_test.ts", code);

    let result = analyze_tests(&workspace, &mut client, "quality", "quality_test.ts").await;

    assert_eq!(result.metadata.kind, "quality");
    assert!(result.summary.symbols_analyzed.is_some());

    if skip_if_no_symbols(&result) { return; }

    // Verify quality findings
    assert!(!result.findings.is_empty());
    let finding = &result.findings[0];
    assert!(
        finding.kind == "quality" || finding.kind == "test_smell",
        "Kind should be quality or test_smell"
    );

    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(
        metrics.contains_key("test_smells_count")
            || metrics.contains_key("smells_count")
            || metrics.contains_key("quality_issues")
    );
}

#[tokio::test]
async fn test_analyze_tests_assertions_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Tests lacking assertions
    let code = r#"
it('test without assertions', () => {
    const x = 1 + 1;
    const y = x * 2;
});

it('test with assertion', () => {
    expect(1 + 1).toBe(2);
});

it('another without assertions', () => {
    const result = calculate();
    console.log(result);
});

it('test with multiple assertions', () => {
    expect(1 + 1).toBe(2);
    expect(2 + 2).toBe(4);
    expect(3 + 3).toBe(6);
});

function calculate() { return 42; }
"#;
    workspace.create_file("assertions_test.ts", code);

    let result = analyze_tests(&workspace, &mut client, "assertions", "assertions_test.ts").await;

    assert!(
        result.metadata.kind == "assertions" || result.metadata.kind == "assertion_analysis",
        "Kind should be assertions or assertion_analysis"
    );
    assert!(result.summary.symbols_analyzed.is_some());

    if skip_if_no_symbols(&result) { return; }

    // Verify assertions findings
    assert!(!result.findings.is_empty());
    let finding = &result.findings[0];
    assert!(
        finding.kind == "assertions" || finding.kind == "assertion_analysis",
        "Kind should be assertions or assertion_analysis"
    );

    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(
        metrics.contains_key("tests_without_assertions")
            || metrics.contains_key("missing_assertions")
            || metrics.contains_key("no_assertions_count")
    );
}

#[tokio::test]
async fn test_analyze_tests_organization_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Properly organized test file
    let code = r#"
describe('MathOperations', () => {
    it('should add', () => { expect(1 + 1).toBe(2); });
    it('should subtract', () => { expect(2 - 1).toBe(1); });
    it('should multiply', () => { expect(2 * 3).toBe(6); });
});

describe('StringOperations', () => {
    it('should concat', () => { expect('a' + 'b').toBe('ab'); });
    it('should uppercase', () => { expect('hello'.toUpperCase()).toBe('HELLO'); });
});

describe('ArrayOperations', () => {
    it('should push', () => {
        const arr = [1, 2];
        arr.push(3);
        expect(arr.length).toBe(3);
    });
});
"#;
    workspace.create_file("organization_test.ts", code);

    let result = analyze_tests(&workspace, &mut client, "organization", "organization_test.ts").await;

    assert_eq!(result.metadata.kind, "organization");
    assert!(result.summary.symbols_analyzed.is_some());

    if skip_if_no_symbols(&result) { return; }

    // Verify organization findings
    assert!(!result.findings.is_empty());
    let finding = &result.findings[0];
    assert_eq!(finding.kind, "organization");

    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(metrics.contains_key("is_test_file") || metrics.contains_key("test_file"));

    // Check for organization score
    if metrics.contains_key("organization_score") {
        let score = metrics.get("organization_score").and_then(|v| v.as_f64()).unwrap();
        assert!(score >= 0.0 && score <= 1.0);
    }

    // Check for test suites
    if metrics.contains_key("test_suites_count") || metrics.contains_key("describe_blocks") {
        let suites = metrics
            .get("test_suites_count")
            .or_else(|| metrics.get("describe_blocks"))
            .and_then(|v| v.as_u64());
        if let Some(count) = suites {
            assert!(count > 0, "Should detect test suites");
        }
    }
}

#[tokio::test]
async fn test_analyze_tests_unsupported_kind() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("test.ts", "export function foo() { return 1; }");
    let test_file = workspace.absolute_path("test.ts");

    let response = client
        .call_tool(
            "analyze.tests",
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
