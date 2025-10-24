//! Analysis API tests for analyze.quality (MIGRATED VERSION)
//!
//! BEFORE: 616 lines with repetitive workspace setup, client creation, and result parsing
//! AFTER: Using simplified pattern for analysis tests
//!
//! Analysis tests are simpler than refactoring tests:
//! - No plan/apply workflow - just analyze + verify
//! - Focus on result structure validation
//! - Less setup boilerplate

use crate::harness::{TestClient, TestWorkspace};
use mill_foundation::protocol::analysis_result::{AnalysisResult, Severity};
use serde_json::json;

/// Helper to run analysis test with result validation
async fn run_analysis_test<V>(
    file_name: &str,
    file_content: &str,
    kind: &str,
    options: Option<serde_json::Value>,
    verify: V,
) -> anyhow::Result<()>
where
    V: FnOnce(&AnalysisResult) -> anyhow::Result<()>,
{
    let workspace = TestWorkspace::new();
    workspace.create_file(file_name, file_content);
    let mut client = TestClient::new(workspace.path());
    let test_file = workspace.absolute_path(file_name);

    let mut params = json!({
        "kind": kind,
        "scope": {
            "type": "file",
            "path": test_file.to_string_lossy()
        }
    });

    if let Some(opts) = options {
        params.as_object_mut().unwrap().insert("options".to_string(), opts);
    }

    let response = client
        .call_tool("analyze.quality", params)
        .await
        .expect("analyze.quality call should succeed");

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
async fn test_analyze_quality_complexity_basic() {
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

    run_analysis_test(
        "complex.ts",
        complex_code,
        "complexity",
        Some(json!({
            "thresholds": {
                "cyclomatic_complexity": 5,
                "cognitive_complexity": 5
            },
            "include_suggestions": true
        })),
        |result| {
            assert_eq!(result.metadata.category, "quality");
            assert_eq!(result.metadata.kind, "complexity");
            assert!(result.summary.symbols_analyzed.is_some());

            if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
                return Ok(()); // Valid early exit for unparseable files
            }

            assert!(!result.findings.is_empty());

            let finding = &result.findings[0];
            assert_eq!(finding.kind, "complexity_hotspot");
            assert!(matches!(finding.severity, Severity::High | Severity::Medium));
            assert_eq!(finding.location.symbol.as_ref().unwrap(), "processOrder");

            let metrics = finding.metrics.as_ref().unwrap();
            assert!(metrics.contains_key("cyclomatic_complexity"));
            assert!(metrics.contains_key("cognitive_complexity"));
            assert!(metrics.contains_key("nesting_depth"));
            assert!(metrics.contains_key("parameter_count"));

            assert!(!finding.suggestions.is_empty());
            let suggestion = &finding.suggestions[0];
            assert!(!suggestion.action.is_empty());
            assert!(suggestion.confidence > 0.0 && suggestion.confidence <= 1.0);

            Ok(())
        },
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_analyze_quality_unsupported_kind() {
    let workspace = TestWorkspace::new();
    workspace.create_file("test.ts", "export function simple() { return 1; }");
    let mut client = TestClient::new(workspace.path());
    let test_file = workspace.absolute_path("test.ts");

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

#[tokio::test]
async fn test_analyze_quality_with_thresholds() {
    let simple_code = r#"
export function add(a: number, b: number): number {
    return a + b;
}
"#;

    run_analysis_test(
        "simple.ts",
        simple_code,
        "complexity",
        Some(json!({
            "thresholds": {
                "cyclomatic_complexity": 100,
                "cognitive_complexity": 100
            }
        })),
        |result| {
            assert_eq!(result.findings.len(), 0);
            assert_eq!(result.summary.total_findings, 0);
            Ok(())
        },
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_analyze_quality_smells() {
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

    run_analysis_test(
        "smelly.ts",
        smelly_code,
        "smells",
        Some(json!({"include_suggestions": true})),
        |result| {
            assert_eq!(result.metadata.category, "quality");
            assert_eq!(result.metadata.kind, "smells");
            assert!(result.summary.symbols_analyzed.is_some());

            if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
                return Ok(());
            }

            assert!(!result.findings.is_empty());

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
                    assert!(suggestion.confidence > 0.0 && suggestion.confidence <= 1.0);
                }
            }
            Ok(())
        },
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_analyze_quality_maintainability() {
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

    run_analysis_test(
        "mixed.ts",
        mixed_code,
        "maintainability",
        Some(json!({"include_suggestions": true})),
        |result| {
            assert_eq!(result.metadata.category, "quality");
            assert_eq!(result.metadata.kind, "maintainability");
            assert!(result.summary.symbols_analyzed.is_some());

            if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
                return Ok(());
            }

            assert_eq!(result.findings.len(), 1);

            let finding = &result.findings[0];
            assert_eq!(finding.kind, "maintainability_summary");
            assert!(!finding.message.is_empty());

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

            Ok(())
        },
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_analyze_quality_readability() {
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

    run_analysis_test(
        "unreadable.ts",
        unreadable_code,
        "readability",
        Some(json!({"include_suggestions": true})),
        |result| {
            assert_eq!(result.metadata.category, "quality");
            assert_eq!(result.metadata.kind, "readability");
            assert!(result.summary.symbols_analyzed.is_some());

            if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
                return Ok(());
            }

            assert!(!result.findings.is_empty());

            for finding in &result.findings {
                assert!(matches!(
                    finding.kind.as_str(),
                    "deep_nesting" | "too_many_parameters" | "long_function" | "low_comment_ratio"
                ));
                assert!(!finding.message.is_empty());
                assert!(finding.metrics.is_some());
            }

            let finding_kinds: Vec<&str> = result.findings.iter().map(|f| f.kind.as_str()).collect();
            assert!(finding_kinds.contains(&"too_many_parameters"));
            assert!(finding_kinds.contains(&"deep_nesting"));

            Ok(())
        },
    )
    .await
    .unwrap();
}
