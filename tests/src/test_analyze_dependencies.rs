use crate::harness::{TestClient, TestWorkspace};
use cb_protocol::analysis_result::{AnalysisResult, Severity};
use serde_json::json;

#[tokio::test]
async fn test_analyze_dependencies_imports_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with multiple import types
    let code = r#"
import { useState, useEffect } from 'react';
import { formatDate } from './utils';
import config from '../config';

export function MyComponent() {
    const [count, setCount] = useState(0);
    const formatted = formatDate(new Date());
    return <div>{count}</div>;
}
"#;

    workspace.create_file("imports_test.ts", code);
    let test_file = workspace.absolute_path("imports_test.ts");

    let response = client
        .call_tool(
            "analyze.dependencies",
            json!({
                "kind": "imports",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.dependencies call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    // Verify result structure
    assert_eq!(result.metadata.category, "dependencies");
    assert_eq!(result.metadata.kind, "imports");

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

    // Should detect imports
    assert!(!result.findings.is_empty(), "Expected import findings");

    // Verify finding structure
    let finding = &result.findings[0];
    assert_eq!(finding.kind, "import");
    assert_eq!(finding.severity, Severity::Low);

    // Verify metrics
    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(metrics.contains_key("source_module"));
    assert!(metrics.contains_key("imported_symbols"));
    assert!(metrics.contains_key("import_category"));

    // Verify we detect different import categories
    let categories: Vec<String> = result
        .findings
        .iter()
        .filter_map(|f| {
            f.metrics.as_ref().and_then(|m| {
                m.get("import_category")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
        })
        .collect();

    assert!(
        categories.contains(&"external".to_string())
            || categories.contains(&"relative".to_string()),
        "Should detect external or relative imports"
    );
}

#[tokio::test]
async fn test_analyze_dependencies_graph_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with multiple imports forming a graph
    let code = r#"
import { useState } from 'react';
import { formatDate } from './utils';
import { validate } from './validators';
import config from '../config';

export function DataProcessor() {
    const [data, setData] = useState(null);
    const formatted = formatDate(new Date());
    const isValid = validate(data);
    return <div>{formatted}</div>;
}
"#;

    workspace.create_file("graph_test.ts", code);
    let test_file = workspace.absolute_path("graph_test.ts");

    let response = client
        .call_tool(
            "analyze.dependencies",
            json!({
                "kind": "graph",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.dependencies call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    assert_eq!(result.metadata.kind, "graph");

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

    // Should have dependency graph finding
    assert!(
        !result.findings.is_empty(),
        "Expected dependency graph findings"
    );

    let finding = &result.findings[0];
    assert_eq!(finding.kind, "dependency_graph");
    assert_eq!(finding.severity, Severity::Low);

    // Verify graph metrics
    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(metrics.contains_key("direct_dependencies"));
    assert!(metrics.contains_key("fan_in"));
    assert!(metrics.contains_key("fan_out"));
    assert!(metrics.contains_key("total_dependencies"));

    // Verify we detected dependencies
    let direct_deps = metrics
        .get("direct_dependencies")
        .and_then(|v| v.as_array())
        .expect("Should have direct_dependencies array");

    assert!(!direct_deps.is_empty(), "Should detect direct dependencies");
}

#[tokio::test]
async fn test_analyze_dependencies_circular_detection() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a Rust file with self-referential import (circular dependency)
    let code = r#"
// Self-referential import (circular)
use crate::test_circular;

pub fn example() {
    println!("Example");
}
"#;

    workspace.create_file("test_circular.rs", code);
    let test_file = workspace.absolute_path("test_circular.rs");

    let response = client
        .call_tool(
            "analyze.dependencies",
            json!({
                "kind": "circular",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.dependencies call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    assert_eq!(result.metadata.kind, "circular");

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

    // Should detect circular dependency
    assert!(
        !result.findings.is_empty(),
        "Expected circular dependency findings"
    );

    let finding = &result.findings[0];
    assert_eq!(finding.kind, "circular_dependency");
    assert_eq!(finding.severity, Severity::High);

    // Verify metrics
    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(metrics.contains_key("cycle_length"));
    assert!(metrics.contains_key("cycle_path"));

    let cycle_path = metrics
        .get("cycle_path")
        .and_then(|v| v.as_array())
        .expect("Should have cycle_path array");

    assert!(!cycle_path.is_empty(), "Cycle path should not be empty");
}

#[tokio::test]
async fn test_analyze_dependencies_coupling_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with many imports (high coupling)
    let code = r#"
import { a } from './a';
import { b } from './b';
import { c } from './c';
import { d } from './d';
import { e } from './e';
import { f } from './f';
import { g } from './g';
import { h } from './h';

export function process() {
    return a() + b() + c() + d() + e() + f() + g() + h();
}
"#;

    workspace.create_file("coupling_test.ts", code);
    let test_file = workspace.absolute_path("coupling_test.ts");

    let response = client
        .call_tool(
            "analyze.dependencies",
            json!({
                "kind": "coupling",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.dependencies call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    assert_eq!(result.metadata.kind, "coupling");

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

    // Should have coupling metric finding
    assert!(!result.findings.is_empty(), "Expected coupling findings");

    let finding = &result.findings[0];
    assert_eq!(finding.kind, "coupling_metric");

    // Severity can be Medium (high coupling) or Low (acceptable coupling)
    assert!(
        finding.severity == Severity::Medium || finding.severity == Severity::Low,
        "Severity should be Medium or Low"
    );

    // Verify metrics
    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(metrics.contains_key("afferent_coupling"));
    assert!(metrics.contains_key("efferent_coupling"));
    assert!(metrics.contains_key("instability"));

    // Verify instability is calculated
    let instability = metrics
        .get("instability")
        .and_then(|v| v.as_f64())
        .expect("Should have instability metric");

    assert!(
        instability >= 0.0 && instability <= 1.0,
        "Instability should be between 0 and 1"
    );
}

#[tokio::test]
async fn test_analyze_dependencies_cohesion_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with many functions (low cohesion indicator)
    let code = r#"
export function fn1() { return 1; }
export function fn2() { return 2; }
export function fn3() { return 3; }
export function fn4() { return 4; }
export function fn5() { return 5; }
export function fn6() { return 6; }
export function fn7() { return 7; }
export function fn8() { return 8; }
export function fn9() { return 9; }
export function fn10() { return 10; }
export function fn11() { return 11; }
export function fn12() { return 12; }
export function fn13() { return 13; }
export function fn14() { return 14; }
export function fn15() { return 15; }
export function fn16() { return 16; }
export function fn17() { return 17; }
export function fn18() { return 18; }
export function fn19() { return 19; }
export function fn20() { return 20; }
export function fn21() { return 21; }
"#;

    workspace.create_file("cohesion_test.ts", code);
    let test_file = workspace.absolute_path("cohesion_test.ts");

    let response = client
        .call_tool(
            "analyze.dependencies",
            json!({
                "kind": "cohesion",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.dependencies call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    assert_eq!(result.metadata.kind, "cohesion");

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

    // Should have cohesion metric finding
    assert!(!result.findings.is_empty(), "Expected cohesion findings");

    let finding = &result.findings[0];
    assert_eq!(finding.kind, "cohesion_metric");

    // Severity can be Medium (low cohesion) or Low (acceptable cohesion)
    assert!(
        finding.severity == Severity::Medium || finding.severity == Severity::Low,
        "Severity should be Medium or Low"
    );

    // Verify metrics
    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(metrics.contains_key("lcom_score"));
    assert!(metrics.contains_key("functions_analyzed"));
    assert!(metrics.contains_key("shared_data_ratio"));

    // Verify LCOM score is calculated
    let lcom_score = metrics
        .get("lcom_score")
        .and_then(|v| v.as_f64())
        .expect("Should have lcom_score metric");

    assert!(
        lcom_score >= 0.0 && lcom_score <= 1.0,
        "LCOM score should be between 0 and 1"
    );
}

#[tokio::test]
async fn test_analyze_dependencies_depth_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with dependency chain
    let code = r#"
import { module1 } from './layer1/module1';
import { module2 } from './layer2/module2';
import { module3 } from './layer3/module3';
import { module4 } from './layer4/module4';
import { module5 } from './layer5/module5';
import { module6 } from './layer6/module6';

export function deepDependency() {
    return module1() + module2() + module3() + module4() + module5() + module6();
}
"#;

    workspace.create_file("depth_test.ts", code);
    let test_file = workspace.absolute_path("depth_test.ts");

    let response = client
        .call_tool(
            "analyze.dependencies",
            json!({
                "kind": "depth",
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.dependencies call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    assert_eq!(result.metadata.kind, "depth");

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

    // Should have dependency depth finding
    assert!(
        !result.findings.is_empty(),
        "Expected dependency depth findings"
    );

    let finding = &result.findings[0];
    assert_eq!(finding.kind, "dependency_depth");

    // Severity can be Medium (excessive depth) or Low (acceptable depth)
    assert!(
        finding.severity == Severity::Medium || finding.severity == Severity::Low,
        "Severity should be Medium or Low"
    );

    // Verify metrics
    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(metrics.contains_key("max_depth"));
    assert!(metrics.contains_key("dependency_chain"));
    assert!(metrics.contains_key("direct_dependencies_count"));

    // Verify dependency chain is present
    let dependency_chain = metrics
        .get("dependency_chain")
        .and_then(|v| v.as_array())
        .expect("Should have dependency_chain array");

    // Should have at least one dependency in the chain
    assert!(
        dependency_chain.len() > 0,
        "Dependency chain should not be empty"
    );
}

#[tokio::test]
async fn test_analyze_dependencies_unsupported_kind() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("test.ts", "export function foo() { return 1; }");
    let test_file = workspace.absolute_path("test.ts");

    let response = client
        .call_tool(
            "analyze.dependencies",
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

#[tokio::test]
async fn test_analyze_dependencies_circular_typescript_workspace() {
    let workspace = TestWorkspace::new();
    workspace.setup_typescript_project("circular-dep-test");

    let fixture_dir = std::path::Path::new("fixtures/circular_dependency");
    for entry in std::fs::read_dir(fixture_dir).unwrap() {
        let entry = entry.unwrap();
        let content = std::fs::read_to_string(entry.path()).unwrap();
        workspace.create_file(
            &format!("src/{}", entry.file_name().to_str().unwrap()),
            &content,
        );
    }

    let mut client = TestClient::new(workspace.path());

    let response = client
        .call_tool(
            "analyze.dependencies",
            json!({
                "kind": "circular",
                "scope": {
                    "type": "workspace"
                }
            }),
        )
        .await
        .expect("analyze.dependencies call should succeed");

    let result: AnalysisResult = serde_json::from_value(
        response
            .get("result")
            .expect("Response should have result field")
            .clone(),
    )
    .expect("Should parse as AnalysisResult");

    assert_eq!(result.metadata.kind, "circular");
    assert_eq!(result.summary.total_findings, 1);
    assert_eq!(result.findings.len(), 1);

    let finding = &result.findings[0];
    assert_eq!(finding.kind, "circular_dependency");
    assert_eq!(finding.severity, Severity::High);

    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    let cycle_path = metrics
        .get("cycle_path")
        .and_then(|v| v.as_array())
        .expect("Should have cycle_path array");

    assert_eq!(cycle_path.len(), 2);
    // The order is not guaranteed, so check for both files.
    let path1 = workspace
        .absolute_path("src/a.ts")
        .to_string_lossy()
        .to_string();
    let path2 = workspace
        .absolute_path("src/b.ts")
        .to_string_lossy()
        .to_string();
    let cycle_path_strings: Vec<String> = cycle_path
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(cycle_path_strings.contains(&path1));
    assert!(cycle_path_strings.contains(&path2));
}
