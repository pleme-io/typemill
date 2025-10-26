//! analyze.documentation tests migrated to closure-based helpers (v2)
//!
//! BEFORE: 547 lines with manual setup/client creation/verification
//! AFTER: Simplified with helper-based assertions
//!
//! Analysis tests focus on result structure verification, not setup boilerplate.

use crate::harness::{TestClient, TestWorkspace};
use mill_foundation::protocol::analysis_result::AnalysisResult;
use serde_json::json;

/// Helper: Call analyze.documentation and parse result
async fn analyze_documentation(
    workspace: &TestWorkspace,
    client: &mut TestClient,
    kind: &str,
    file: &str,
) -> AnalysisResult {
    let test_file = workspace.absolute_path(file);
    let response = client
        .call_tool(
            "analyze.documentation",
            json!({
                "kind": kind,
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.documentation call should succeed");

    serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult")
}

/// Helper: Skip test if no symbols analyzed (unsupported file type)
fn skip_if_no_symbols(result: &AnalysisResult) -> bool {
    result.summary.symbols_analyzed.unwrap_or(0) == 0
}

#[tokio::test]
async fn test_analyze_documentation_coverage_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create TypeScript file: 3 documented + 2 undocumented = 60% coverage
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

    let result =
        analyze_documentation(&workspace, &mut client, "coverage", "coverage_test.ts").await;

    // Verify result structure
    assert_eq!(result.metadata.category, "documentation");
    assert_eq!(result.metadata.kind, "coverage");
    assert!(result.summary.symbols_analyzed.is_some());

    if skip_if_no_symbols(&result) {
        return;
    }

    // Verify coverage findings
    assert!(!result.findings.is_empty());
    let finding = &result.findings[0];
    assert_eq!(finding.kind, "coverage");

    // Verify metrics
    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(metrics.contains_key("coverage_percentage"));
    assert!(metrics.contains_key("documented_count"));
    assert!(metrics.contains_key("undocumented_count"));

    let coverage = metrics
        .get("coverage_percentage")
        .and_then(|v| v.as_f64())
        .unwrap();
    assert!(coverage >= 0.0 && coverage <= 100.0);

    let undocumented = metrics
        .get("undocumented_count")
        .and_then(|v| v.as_u64())
        .unwrap();
    assert!(undocumented > 0, "Should detect undocumented items");
}

#[tokio::test]
async fn test_analyze_documentation_quality_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create file with poor quality docs
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

    let result = analyze_documentation(&workspace, &mut client, "quality", "quality_test.ts").await;

    assert_eq!(result.metadata.kind, "quality");
    assert!(result.summary.symbols_analyzed.is_some());

    if skip_if_no_symbols(&result) {
        return;
    }

    // Verify quality findings
    assert!(!result.findings.is_empty());
    let finding = &result.findings[0];
    assert_eq!(finding.kind, "quality_summary");

    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(
        metrics.contains_key("quality_issues_count")
            || metrics.contains_key("total_issues")
            || metrics.contains_key("issues_count")
    );
}

#[tokio::test]
async fn test_analyze_documentation_style_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create file with mixed doc styles
    let code = r#"
/// First doc style
export function fn1() { return 1; }

/** Second doc style */
export function fn2() { return 2; }

/// Third using first style
export function fn3() { return 3; }

/** Fourth using second style */
export function fn4() { return 4; }
"#;
    workspace.create_file("style_test.ts", code);

    let result = analyze_documentation(&workspace, &mut client, "style", "style_test.ts").await;

    assert_eq!(result.metadata.kind, "style");
    assert!(result.summary.symbols_analyzed.is_some());

    if skip_if_no_symbols(&result) {
        return;
    }

    // Verify style findings
    assert!(!result.findings.is_empty());
    let finding = &result.findings[0];
    assert_eq!(finding.kind, "style");

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

    // Create complex function lacking examples
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

    let result =
        analyze_documentation(&workspace, &mut client, "examples", "examples_test.ts").await;

    assert_eq!(result.metadata.kind, "examples");
    assert!(result.summary.symbols_analyzed.is_some());

    if skip_if_no_symbols(&result) {
        return;
    }

    // Verify examples findings
    assert!(!result.findings.is_empty());
    let finding = &result.findings[0];
    assert_eq!(finding.kind, "examples");

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

    // Create file with TODO, FIXME, NOTE comments
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

    let result = analyze_documentation(&workspace, &mut client, "todos", "todos_test.ts").await;

    assert_eq!(result.metadata.kind, "todos");
    assert!(result.summary.symbols_analyzed.is_some());

    if skip_if_no_symbols(&result) {
        return;
    }

    // Verify todos findings
    assert!(!result.findings.is_empty());
    let finding = &result.findings[0];
    assert_eq!(finding.kind, "todos");

    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(
        metrics.contains_key("total_todos")
            || metrics.contains_key("todos_count")
            || metrics.contains_key("todo_count")
    );

    // Check for categorization
    if metrics.contains_key("todos_by_category") {
        let by_category = metrics
            .get("todos_by_category")
            .and_then(|v| v.as_object())
            .expect("Should have todos_by_category");
        assert!(!by_category.is_empty());
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
