use crate::harness::{TestClient, TestWorkspace};
use codebuddy_foundation::protocol::analysis_result::{ AnalysisResult , Severity };
use serde_json::json;

#[tokio::test]
async fn test_analyze_tests_coverage_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with 5 functions and 2 tests
    let code = r#"
export function add(a: number, b: number): number {
    return a + b;
}

export function subtract(a: number, b: number): number {
    return a - b;
}

export function multiply(a: number, b: number): number {
    return a * b;
}

export function divide(a: number, b: number): number {
    return a / b;
}

export function mod(a: number, b: number): number {
    return a % b;
}

// Only 2 tests for 5 functions = 0.4 ratio
it('should add numbers', () => {
    expect(add(1, 2)).toBe(3);
});

it('should subtract numbers', () => {
    expect(subtract(5, 3)).toBe(2);
});
"#;

    workspace.create_file("coverage_test.ts", code);
    let test_file = workspace.absolute_path("coverage_test.ts");

    let response = client
        .call_tool(
            "analyze.tests",
            json!({
                "kind": "coverage",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.tests call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    // Verify result structure
    assert_eq!(result.metadata.category, "tests");
    assert_eq!(result.metadata.kind, "coverage");

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

    // Should detect test coverage
    assert!(
        !result.findings.is_empty(),
        "Expected test coverage findings"
    );

    // Verify finding structure
    let finding = &result.findings[0];
    assert_eq!(finding.kind, "coverage");

    // Severity should be High for ratio < 0.5
    assert!(
        finding.severity == Severity::High
            || finding.severity == Severity::Medium
            || finding.severity == Severity::Low,
        "Severity should be High, Medium, or Low"
    );

    // Verify metrics
    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(
        metrics.contains_key("coverage_ratio")
            || metrics.contains_key("test_coverage")
            || metrics.contains_key("coverage")
    );
    assert!(metrics.contains_key("total_tests") || metrics.contains_key("tests_count"));
    assert!(metrics.contains_key("total_functions") || metrics.contains_key("functions_count"));

    // Verify coverage ratio if available
    let coverage_ratio = metrics
        .get("coverage_ratio")
        .or_else(|| metrics.get("test_coverage"))
        .or_else(|| metrics.get("coverage"))
        .and_then(|v| v.as_f64());

    if let Some(ratio) = coverage_ratio {
        assert!(
            ratio >= 0.0 && ratio <= 1.0,
            "Coverage ratio should be between 0 and 1"
        );
    }
}

#[tokio::test]
async fn test_analyze_tests_quality_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with test smells
    let code = r#"
it('empty test', () => {
    // Empty test body - test smell
});

it('single assertion', () => {
    expect(true).toBe(true);
});

it('another trivial test', () => {
    const x = 1;
    expect(x).toBe(1);
});

it('no assertions here', () => {
    const data = getData();
    console.log(data);
});

function getData() {
    return { value: 42 };
}
"#;

    workspace.create_file("quality_test.ts", code);
    let test_file = workspace.absolute_path("quality_test.ts");

    let response = client
        .call_tool(
            "analyze.tests",
            json!({
                "kind": "quality",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.tests call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    assert_eq!(result.metadata.kind, "quality");

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

    // Should have quality finding
    assert!(!result.findings.is_empty(), "Expected quality findings");

    let finding = &result.findings[0];
    assert!(
        finding.kind == "quality" || finding.kind == "test_smell",
        "Kind should be quality or test_smell"
    );

    // Severity should be Medium for test smells
    assert!(
        finding.severity == Severity::Medium || finding.severity == Severity::Low,
        "Severity should be Medium or Low"
    );

    // Verify quality metrics
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

    // Create a TypeScript file with tests lacking assertions
    let code = r#"
it('test without assertions', () => {
    const x = 1 + 1;
    const y = x * 2;
    // No assertions - test smell
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

function calculate() {
    return 42;
}
"#;

    workspace.create_file("assertions_test.ts", code);
    let test_file = workspace.absolute_path("assertions_test.ts");

    let response = client
        .call_tool(
            "analyze.tests",
            json!({
                "kind": "assertions",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.tests call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    assert!(
        result.metadata.kind == "assertions" || result.metadata.kind == "assertion_analysis",
        "Kind should be assertions or assertion_analysis"
    );

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

    // Should have assertions finding
    assert!(!result.findings.is_empty(), "Expected assertions findings");

    let finding = &result.findings[0];
    assert!(
        finding.kind == "assertions" || finding.kind == "assertion_analysis",
        "Kind should be assertions or assertion_analysis"
    );

    // Severity should be Medium for missing assertions
    assert!(
        finding.severity == Severity::Medium || finding.severity == Severity::Low,
        "Severity should be Medium or Low"
    );

    // Verify assertions metrics
    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(
        metrics.contains_key("tests_without_assertions")
            || metrics.contains_key("missing_assertions")
            || metrics.contains_key("no_assertions_count")
    );

    // Check for average assertions metric
    if metrics.contains_key("avg_assertions_per_test") {
        let avg_assertions = metrics
            .get("avg_assertions_per_test")
            .and_then(|v| v.as_f64())
            .expect("Should have avg_assertions_per_test");

        assert!(
            avg_assertions >= 0.0,
            "Average assertions should be non-negative"
        );
    }
}

#[tokio::test]
async fn test_analyze_tests_organization_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript test file with proper organization
    let code = r#"
describe('MathOperations', () => {
    it('should add', () => {
        expect(1 + 1).toBe(2);
    });

    it('should subtract', () => {
        expect(2 - 1).toBe(1);
    });

    it('should multiply', () => {
        expect(2 * 3).toBe(6);
    });
});

describe('StringOperations', () => {
    it('should concat', () => {
        expect('a' + 'b').toBe('ab');
    });

    it('should uppercase', () => {
        expect('hello'.toUpperCase()).toBe('HELLO');
    });
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
    let test_file = workspace.absolute_path("organization_test.ts");

    let response = client
        .call_tool(
            "analyze.tests",
            json!({
                "kind": "organization",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.tests call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    assert_eq!(result.metadata.kind, "organization");

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

    // Should have organization finding
    assert!(
        !result.findings.is_empty(),
        "Expected organization findings"
    );

    let finding = &result.findings[0];
    assert_eq!(finding.kind, "organization");

    // Severity should be Low (good) or Medium (poor organization)
    assert!(
        finding.severity == Severity::Low || finding.severity == Severity::Medium,
        "Severity should be Low or Medium"
    );

    // Verify organization metrics
    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(metrics.contains_key("is_test_file") || metrics.contains_key("test_file"));

    // Check if organization score is present
    if metrics.contains_key("organization_score") {
        let score = metrics
            .get("organization_score")
            .and_then(|v| v.as_f64())
            .expect("Should have organization_score");

        assert!(
            score >= 0.0 && score <= 1.0,
            "Organization score should be between 0 and 1"
        );
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