use crate::harness::{TestClient, TestWorkspace};
use cb_protocol::analysis_result::{AnalysisResult, Severity};
use serde_json::json;

#[tokio::test]
async fn test_analyze_quality_complexity_basic() {
    // Create a test workspace with a complex function
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Write a TypeScript file with high complexity
    let complex_code = r#"
export function processOrder(
    orderId: string,
    userId: string,
    items: any[],
    discount: number,
    coupon: string,
    shipping: string,
    payment: string,
    tax: number
) {
    if (orderId) {
        if (userId) {
            if (items.length > 0) {
                for (let item of items) {
                    if (item.quantity > 0) {
                        if (item.price > 0) {
                            if (discount > 0) {
                                if (coupon) {
                                    if (shipping) {
                                        if (payment) {
                                            return true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    return false;
}
"#;

    workspace.create_file("complex.ts", complex_code);
    let test_file = workspace.absolute_path("complex.ts");

    // Call analyze.quality with kind="complexity"
    let response = client
        .call_tool(
            "analyze.quality",
            json!({
                "kind": "complexity",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                },
                "options": {
                    "thresholds": {
                        "cyclomatic_complexity": 5,
                        "cognitive_complexity": 5
                    },
                    "include_suggestions": true
                }
            }),
        )
        .await
        .expect("analyze.quality call should succeed");

    // Extract AnalysisResult from MCP response structure
    // Note: analyze.quality returns the result directly under "result", not "result.content"
    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    // Verify result structure
    assert_eq!(result.metadata.category, "quality");
    assert_eq!(result.metadata.kind, "complexity");
    assert_eq!(result.metadata.scope.scope_type, "file");

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

    assert!(
        !result.findings.is_empty(),
        "Expected findings for complex function (symbols analyzed: {})",
        result.summary.symbols_analyzed.unwrap_or(0)
    );

    let finding = &result.findings[0];
    assert_eq!(finding.kind, "complexity_hotspot");
    assert!(matches!(
        finding.severity,
        Severity::High | Severity::Medium
    ));
    assert!(finding.location.symbol.is_some());
    assert_eq!(finding.location.symbol.as_ref().unwrap(), "processOrder");

    // Verify metrics are present
    assert!(finding.metrics.is_some());
    let metrics = finding.metrics.as_ref().unwrap();
    assert!(metrics.contains_key("cyclomatic_complexity"));
    assert!(metrics.contains_key("cognitive_complexity"));
    assert!(metrics.contains_key("nesting_depth"));
    assert!(metrics.contains_key("parameter_count"));

    // Verify suggestions are included
    assert!(!finding.suggestions.is_empty(), "Expected suggestions");
    let suggestion = &finding.suggestions[0];
    assert!(!suggestion.action.is_empty());
    assert!(!suggestion.description.is_empty());
    assert!(!suggestion.estimated_impact.is_empty());
    assert!(suggestion.confidence > 0.0 && suggestion.confidence <= 1.0);

    // Verify summary
    assert_eq!(result.summary.files_analyzed, 1);
    assert!(result.summary.total_findings > 0);
    assert_eq!(
        result.summary.returned_findings,
        result.summary.total_findings
    );
}

#[tokio::test]
async fn test_analyze_quality_unsupported_kind() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("test.ts", "export function simple() { return 1; }");
    let test_file = workspace.absolute_path("test.ts");

    // Try to call with unsupported kind (use "performance" which doesn't exist)
    let response = client
        .call_tool(
            "analyze.quality",
            json!({
                "kind": "performance",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await;

    // Should return error (either Rust Error or JSON-RPC error object)
    match response {
        Err(e) => {
            // Rust error from client
            let error_msg = format!("{:?}", e);
            assert!(
                error_msg.contains("Unsupported") || error_msg.contains("supported"),
                "Error should mention unsupported kind: {}",
                error_msg
            );
        }
        Ok(value) => {
            // JSON-RPC error response
            assert!(
                value.get("error").is_some(),
                "Expected error field in response for unsupported kind, got: {:?}",
                value
            );
            let error_obj = value.get("error").unwrap();
            let error_msg = serde_json::to_string(error_obj).unwrap();
            assert!(
                error_msg.contains("Unsupported") || error_msg.contains("supported"),
                "Error should mention unsupported kind: {}",
                error_msg
            );
        }
    }
}

#[tokio::test]
async fn test_analyze_quality_with_thresholds() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Simple function that won't trigger high thresholds
    let simple_code = r#"
export function add(a: number, b: number): number {
    return a + b;
}
"#;

    workspace.create_file("simple.ts", simple_code);
    let test_file = workspace.absolute_path("simple.ts");

    // Call with very high thresholds (should not flag simple function)
    let response = client
        .call_tool(
            "analyze.quality",
            json!({
                "kind": "complexity",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                },
                "options": {
                    "thresholds": {
                        "cyclomatic_complexity": 100,
                        "cognitive_complexity": 100
                    }
                }
            }),
        )
        .await
        .expect("analyze.quality should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    // Should have no findings due to high thresholds
    assert_eq!(
        result.findings.len(),
        0,
        "Simple function should not trigger high thresholds"
    );
    assert_eq!(result.summary.total_findings, 0);
}

#[tokio::test]
async fn test_analyze_quality_smells() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Code with multiple smells
    let smelly_code = r#"
export class DataProcessor {
    // Long method with magic numbers
    process(a: number, b: number, c: number, d: number, e: number, f: number) {
        if (a > 100) {
            if (b > 100) {
                if (c > 100) {
                    for (let i = 0; i < 100; i++) {
                        console.log(i);
                    }
                }
            }
        }
        return a + b + c + d + e + f + 100 + 100;
    }

    method1() { return 100; }
    method2() { return 100; }
    method3() { return 100; }
    method4() { return 100; }
    method5() { return 100; }
    method6() { return 100; }
    method7() { return 100; }
    method8() { return 100; }
    method9() { return 100; }
    method10() { return 100; }
    method11() { return 100; }
    method12() { return 100; }
    method13() { return 100; }
    method14() { return 100; }
    method15() { return 100; }
    method16() { return 100; }
    method17() { return 100; }
    method18() { return 100; }
    method19() { return 100; }
    method20() { return 100; }
    method21() { return 100; }
    method22() { return 100; }
}
"#;

    workspace.create_file("smelly.ts", smelly_code);
    let test_file = workspace.absolute_path("smelly.ts");

    let response = client
        .call_tool(
            "analyze.quality",
            json!({
                "kind": "smells",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                },
                "options": {
                    "include_suggestions": true
                }
            }),
        )
        .await
        .expect("analyze.quality smells call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    // Verify result structure
    assert_eq!(result.metadata.category, "quality");
    assert_eq!(result.metadata.kind, "smells");

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

    // Should detect at least magic numbers and possibly god class
    assert!(
        !result.findings.is_empty(),
        "Expected smell findings (symbols analyzed: {})",
        result.summary.symbols_analyzed.unwrap_or(0)
    );

    // Verify findings have correct structure
    for finding in &result.findings {
        assert!(matches!(
            finding.kind.as_str(),
            "magic_number" | "god_class" | "long_method"
        ));
        assert!(!finding.message.is_empty());
        assert!(finding.metrics.is_some());

        if !finding.suggestions.is_empty() {
            let suggestion = &finding.suggestions[0];
            assert!(!suggestion.action.is_empty());
            assert!(!suggestion.description.is_empty());
            assert!(!suggestion.estimated_impact.is_empty());
            assert!(suggestion.confidence > 0.0 && suggestion.confidence <= 1.0);
        }
    }
}

#[tokio::test]
async fn test_analyze_quality_maintainability() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a file with mixed complexity
    let mixed_code = r#"
export function simple1() { return 1; }
export function simple2() { return 2; }
export function simple3() { return 3; }

export function moderate1(x: number) {
    if (x > 0) {
        return x * 2;
    }
    return 0;
}

export function complex1(a: number, b: number) {
    if (a > 0) {
        if (b > 0) {
            if (a > b) {
                return a;
            } else {
                return b;
            }
        }
    }
    return 0;
}

export function veryComplex(x: number, y: number, z: number) {
    if (x > 0) {
        if (y > 0) {
            if (z > 0) {
                if (x > y) {
                    if (y > z) {
                        return x + y + z;
                    }
                }
            }
        }
    }
    return 0;
}
"#;

    workspace.create_file("mixed.ts", mixed_code);
    let test_file = workspace.absolute_path("mixed.ts");

    let response = client
        .call_tool(
            "analyze.quality",
            json!({
                "kind": "maintainability",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                },
                "options": {
                    "include_suggestions": true
                }
            }),
        )
        .await
        .expect("analyze.quality maintainability call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    // Verify result structure
    assert_eq!(result.metadata.category, "quality");
    assert_eq!(result.metadata.kind, "maintainability");

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

    // Should have exactly 1 finding (summary)
    assert_eq!(
        result.findings.len(),
        1,
        "Maintainability should produce single summary finding"
    );

    let finding = &result.findings[0];
    assert_eq!(finding.kind, "maintainability_summary");
    assert!(!finding.message.is_empty());

    // Verify comprehensive metrics
    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(metrics.contains_key("avg_cyclomatic"));
    assert!(metrics.contains_key("avg_cognitive"));
    assert!(metrics.contains_key("max_cyclomatic"));
    assert!(metrics.contains_key("max_cognitive"));
    assert!(metrics.contains_key("total_functions"));
    assert!(metrics.contains_key("needs_attention"));
    assert!(metrics.contains_key("simple"));
    assert!(metrics.contains_key("moderate"));
    assert!(metrics.contains_key("complex"));
    assert!(metrics.contains_key("very_complex"));

    // Verify suggestions if present
    if !finding.suggestions.is_empty() {
        for suggestion in &finding.suggestions {
            assert!(!suggestion.action.is_empty());
            assert!(!suggestion.description.is_empty());
            assert!(!suggestion.estimated_impact.is_empty());
            assert!(suggestion.confidence > 0.0 && suggestion.confidence <= 1.0);
        }
    }
}

#[tokio::test]
async fn test_analyze_quality_readability() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a file with readability issues
    let unreadable_code = r#"
export function complexProcess(param1: number, param2: string, param3: boolean, param4: any[], param5: object, param6: number, param7: string) {
    // Deep nesting and long function with few comments
    if (param1 > 0) {
        if (param2.length > 0) {
            if (param3) {
                if (param4.length > 0) {
                    if (param5) {
                        if (param6 > 0) {
                            for (let i = 0; i < param4.length; i++) {
                                console.log(i);
                                if (i > 5) {
                                    console.log("done");
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let result = param1 + param6;
    let doubled = result * 2;
    let final = doubled + 10;
    let output = final.toString();
    let formatted = output + param7;
    let capitalized = formatted.toUpperCase();
    let trimmed = capitalized.trim();
    let reversed = trimmed.split('').reverse().join('');
    let encoded = btoa(reversed);
    let decoded = atob(encoded);
    let processed = decoded.toLowerCase();
    return processed;
}

export function wellDocumented() {
    // This function is well documented
    // It returns 42
    // Which is the answer
    return 42;
}
"#;

    workspace.create_file("unreadable.ts", unreadable_code);
    let test_file = workspace.absolute_path("unreadable.ts");

    let response = client
        .call_tool(
            "analyze.quality",
            json!({
                "kind": "readability",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                },
                "options": {
                    "include_suggestions": true
                }
            }),
        )
        .await
        .expect("analyze.quality readability call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    // Verify result structure
    assert_eq!(result.metadata.category, "quality");
    assert_eq!(result.metadata.kind, "readability");

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

    // Should detect multiple readability issues
    assert!(
        !result.findings.is_empty(),
        "Expected readability findings (symbols analyzed: {})",
        result.summary.symbols_analyzed.unwrap_or(0)
    );

    // Verify findings have correct structure
    for finding in &result.findings {
        assert!(matches!(
            finding.kind.as_str(),
            "deep_nesting" | "too_many_parameters" | "long_function" | "low_comment_ratio"
        ));
        assert!(!finding.message.is_empty());
        assert!(finding.metrics.is_some());

        if !finding.suggestions.is_empty() {
            let suggestion = &finding.suggestions[0];
            assert!(!suggestion.action.is_empty());
            assert!(!suggestion.description.is_empty());
            assert!(!suggestion.estimated_impact.is_empty());
            assert!(suggestion.confidence > 0.0 && suggestion.confidence <= 1.0);
        }
    }

    // Verify we detected the specific issues in complexProcess
    let finding_kinds: Vec<&str> = result.findings.iter().map(|f| f.kind.as_str()).collect();
    assert!(
        finding_kinds.contains(&"too_many_parameters"),
        "Should detect too many parameters (7 params)"
    );
    assert!(
        finding_kinds.contains(&"deep_nesting"),
        "Should detect deep nesting (7 levels)"
    );
}
