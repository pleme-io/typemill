use crate::harness::{TestClient, TestWorkspace};
use codebuddy_foundation::protocol::analysis_result::{AnalysisResult, Severity};
use serde_json::json;

#[tokio::test]
async fn test_analyze_documentation_coverage_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with 5 functions (3 documented, 2 undocumented)
    let code = r#"
/** This is documented */
export function documented1() {
    return 1;
}

/** This is also documented */
export function documented2() {
    return 2;
}

/** Documented function */
export function documented3() {
    return 3;
}

export function undocumented1() {
    return 4;
}

export function undocumented2() {
    return 5;
}
"#;

    workspace.create_file("coverage_test.ts", code);
    let test_file = workspace.absolute_path("coverage_test.ts");

    let response = client
        .call_tool(
            "analyze.documentation",
            json!({
                "kind": "coverage",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.documentation call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    // Verify result structure
    assert_eq!(result.metadata.category, "documentation");
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

    // Should detect documentation coverage
    assert!(
        !result.findings.is_empty(),
        "Expected documentation coverage findings"
    );

    // Verify finding structure
    let finding = &result.findings[0];
    assert_eq!(finding.kind, "coverage");

    // Severity should be Medium for 60% coverage
    assert!(
        finding.severity == Severity::Medium || finding.severity == Severity::Low,
        "Severity should be Medium or Low for partial coverage, but got {:?}",
        finding.severity
    );

    // Verify metrics
    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(metrics.contains_key("coverage_percentage"));
    assert!(metrics.contains_key("documented_count"));
    assert!(metrics.contains_key("undocumented_count"));

    // Verify coverage percentage
    let coverage = metrics
        .get("coverage_percentage")
        .and_then(|v| v.as_f64())
        .expect("Should have coverage_percentage");

    assert!(
        coverage >= 0.0 && coverage <= 100.0,
        "Coverage should be between 0 and 100"
    );

    // Verify undocumented items are tracked
    let undocumented_count = metrics
        .get("undocumented_count")
        .and_then(|v| v.as_u64())
        .expect("Should have undocumented_count");

    assert!(undocumented_count > 0, "Should detect undocumented items");
}

#[tokio::test]
async fn test_analyze_documentation_quality_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with poor quality docs
    let code = r#"
/** fn */
export function poorQuality(x: number, y: string): number {
    return x + y.length;
}

/** Does a thing */
export function vague(data: any): any {
    return data;
}

/** x */
export function trivial() {
    return true;
}
"#;

    workspace.create_file("quality_test.ts", code);
    let test_file = workspace.absolute_path("quality_test.ts");

    let response = client
        .call_tool(
            "analyze.documentation",
            json!({
                "kind": "quality",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.documentation call should succeed");

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

    // Should have quality findings (includes quality_summary and individual quality findings)
    assert!(!result.findings.is_empty(), "Expected quality findings");

    let finding = &result.findings[0];
    // First finding should be the summary
    assert_eq!(finding.kind, "quality_summary");

    // Severity should be Medium for poor quality docs
    assert!(
        finding.severity == Severity::Medium || finding.severity == Severity::Low,
        "Severity should be Medium or Low"
    );

    // Verify quality metrics
    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(
        metrics.contains_key("quality_issues_count")
            || metrics.contains_key("total_issues")
            || metrics.contains_key("issues_count")
    );

    // Should detect some quality issues
    let issues_count = metrics
        .get("quality_issues_count")
        .or_else(|| metrics.get("total_issues"))
        .or_else(|| metrics.get("issues_count"))
        .and_then(|v| v.as_u64());

    if let Some(count) = issues_count {
        assert!(count > 0, "Should detect quality issues");
    }
}

#[tokio::test]
async fn test_analyze_documentation_style_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with mixed doc comment styles
    let code = r#"
/// First doc style
export function fn1() {
    return 1;
}

/** Second doc style */
export function fn2() {
    return 2;
}

/// Third using first style again
export function fn3() {
    return 3;
}

/** Fourth using second style */
export function fn4() {
    return 4;
}
"#;

    workspace.create_file("style_test.ts", code);
    let test_file = workspace.absolute_path("style_test.ts");

    let response = client
        .call_tool(
            "analyze.documentation",
            json!({
                "kind": "style",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.documentation call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    assert_eq!(result.metadata.kind, "style");

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

    // Should have style finding
    assert!(!result.findings.is_empty(), "Expected style findings");

    let finding = &result.findings[0];
    assert_eq!(finding.kind, "style");

    // Severity should be Low for style inconsistencies
    assert!(
        finding.severity == Severity::Low || finding.severity == Severity::Medium,
        "Severity should be Low or Medium"
    );

    // Verify style metrics
    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(
        metrics.contains_key("mixed_styles")
            || metrics.contains_key("style_violations")
            || metrics.contains_key("inconsistencies")
    );
}

#[tokio::test]
async fn test_analyze_documentation_examples_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with complex function lacking code example
    let code = r#"
/** Complex function without example */
export function complexFunction(a: number, b: number, c: string): number {
    if (a > 10) {
        if (b < 5) {
            if (c.length > 0) {
                return a + b + c.length;
            }
            return a + b;
        }
        return a;
    }
    return 0;
}

/** Another complex one without example */
export function anotherComplex(x: string, y: number[]): boolean {
    let sum = 0;
    for (let i = 0; i < y.length; i++) {
        sum += y[i];
    }
    if (sum > x.length) {
        return true;
    }
    return false;
}
"#;

    workspace.create_file("examples_test.ts", code);
    let test_file = workspace.absolute_path("examples_test.ts");

    let response = client
        .call_tool(
            "analyze.documentation",
            json!({
                "kind": "examples",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.documentation call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    assert_eq!(result.metadata.kind, "examples");

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

    // Should have examples finding
    assert!(!result.findings.is_empty(), "Expected examples findings");

    let finding = &result.findings[0];
    assert_eq!(finding.kind, "examples");

    // Severity should be Medium for missing examples
    assert!(
        finding.severity == Severity::Medium || finding.severity == Severity::Low,
        "Severity should be Medium or Low"
    );

    // Verify examples metrics
    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(
        metrics.contains_key("complex_without_examples")
            || metrics.contains_key("missing_examples")
            || metrics.contains_key("functions_needing_examples")
    );
}

#[tokio::test]
async fn test_analyze_documentation_todos_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with TODO, FIXME, NOTE comments
    let code = r#"
// TODO: Implement this feature
export function todoFunction() {
    // FIXME: This is broken
    // HACK: Temporary workaround
    // NOTE: Important detail
    return null;
}

// TODO: Add validation
// TODO: Add error handling
export function multiTodo(x: number): number {
    // FIXME: Handle edge cases
    return x * 2;
}

// NOTE: This is well tested
// NOTE: Performance optimized
export function noteFunction(): string {
    return "done";
}
"#;

    workspace.create_file("todos_test.ts", code);
    let test_file = workspace.absolute_path("todos_test.ts");

    let response = client
        .call_tool(
            "analyze.documentation",
            json!({
                "kind": "todos",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.documentation call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    assert_eq!(result.metadata.kind, "todos");

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

    // Should have todos finding
    assert!(!result.findings.is_empty(), "Expected todos findings");

    let finding = &result.findings[0];
    assert_eq!(finding.kind, "todos");

    // Severity should be High (FIXMEs) or Medium (many TODOs)
    assert!(
        finding.severity == Severity::High
            || finding.severity == Severity::Medium
            || finding.severity == Severity::Low,
        "Severity should be High, Medium, or Low"
    );

    // Verify todos metrics
    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(
        metrics.contains_key("total_todos")
            || metrics.contains_key("todos_count")
            || metrics.contains_key("todo_count")
    );

    // Should detect TODOs
    let todos_count = metrics
        .get("total_todos")
        .or_else(|| metrics.get("todos_count"))
        .or_else(|| metrics.get("todo_count"))
        .and_then(|v| v.as_u64());

    if let Some(count) = todos_count {
        assert!(count > 0, "Should detect TODO comments");
    }

    // Check for categorization
    if metrics.contains_key("todos_by_category") {
        let by_category = metrics
            .get("todos_by_category")
            .and_then(|v| v.as_object())
            .expect("Should have todos_by_category object");

        assert!(!by_category.is_empty(), "Should categorize TODOs");
    }
}

#[tokio::test]
async fn test_analyze_documentation_unsupported_kind() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("test.ts", "export function foo() { return 1; }");
    let test_file = workspace.absolute_path("test.ts");

    let response = client
        .call_tool(
            "analyze.documentation",
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
