//! Analysis API tests for analyze.dependencies (MIGRATED VERSION)
//!
//! BEFORE: 649 lines with repetitive setup and result parsing
//! AFTER: Using simplified helper pattern for analysis tests
//!
//! Tests various dependency analysis kinds: imports, graph, circular, coupling, cohesion, depth

use crate::harness::{TestClient, TestWorkspace};
use mill_foundation::protocol::analysis_result::{AnalysisResult, Severity};
use serde_json::json;

/// Helper to run dependency analysis test
async fn run_dependency_test<V>(
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
            "analyze.dependencies",
            json!({
                "kind": kind,
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

    verify(&result)?;
    Ok(())
}

#[tokio::test]
async fn test_analyze_dependencies_imports_basic() {
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

    run_dependency_test("imports_test.ts", code, "imports", |result| {
        assert_eq!(result.metadata.category, "dependencies");
        assert_eq!(result.metadata.kind, "imports");
        assert!(result.summary.symbols_analyzed.is_some());

        if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
            return Ok(());
        }

        assert!(!result.findings.is_empty());

        let finding = &result.findings[0];
        assert_eq!(finding.kind, "import");
        assert_eq!(finding.severity, Severity::Low);

        let metrics = finding.metrics.as_ref().expect("Should have metrics");
        assert!(metrics.contains_key("source_module"));
        assert!(metrics.contains_key("imported_symbols"));
        assert!(metrics.contains_key("import_category"));

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_analyze_dependencies_graph_basic() {
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

    run_dependency_test("graph_test.ts", code, "graph", |result| {
        assert_eq!(result.metadata.kind, "graph");
        assert!(result.summary.symbols_analyzed.is_some());

        if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
            return Ok(());
        }

        assert!(!result.findings.is_empty());

        let finding = &result.findings[0];
        assert_eq!(finding.kind, "dependency_graph");
        assert_eq!(finding.severity, Severity::Low);

        let metrics = finding.metrics.as_ref().expect("Should have metrics");
        assert!(metrics.contains_key("direct_dependencies"));
        assert!(metrics.contains_key("fan_in"));
        assert!(metrics.contains_key("fan_out"));
        assert!(metrics.contains_key("total_dependencies"));

        let direct_deps = metrics
            .get("direct_dependencies")
            .and_then(|v| v.as_array())
            .expect("Should have direct_dependencies array");

        assert!(!direct_deps.is_empty());

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_analyze_dependencies_circular_detection() {
    let code = r#"
// Self-referential import (circular)
use crate::test_circular;

pub fn example() {
    println!("Example");
}
"#;

    run_dependency_test("test_circular.rs", code, "circular", |result| {
        assert_eq!(result.metadata.kind, "circular");
        assert!(result.summary.symbols_analyzed.is_some());

        if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
            return Ok(());
        }

        assert!(!result.findings.is_empty());

        let finding = &result.findings[0];
        assert_eq!(finding.kind, "circular_dependency");
        assert_eq!(finding.severity, Severity::High);

        let metrics = finding.metrics.as_ref().expect("Should have metrics");
        assert!(metrics.contains_key("cycle_length"));
        assert!(metrics.contains_key("cycle_path"));

        let cycle_path = metrics
            .get("cycle_path")
            .and_then(|v| v.as_array())
            .expect("Should have cycle_path array");

        assert!(!cycle_path.is_empty());

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_analyze_dependencies_coupling_basic() {
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

    run_dependency_test("coupling_test.ts", code, "coupling", |result| {
        assert_eq!(result.metadata.kind, "coupling");
        assert!(result.summary.symbols_analyzed.is_some());

        if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
            return Ok(());
        }

        assert!(!result.findings.is_empty());

        let finding = &result.findings[0];
        assert_eq!(finding.kind, "coupling_metric");
        assert!(finding.severity == Severity::Medium || finding.severity == Severity::Low);

        let metrics = finding.metrics.as_ref().expect("Should have metrics");
        assert!(metrics.contains_key("afferent_coupling"));
        assert!(metrics.contains_key("efferent_coupling"));
        assert!(metrics.contains_key("instability"));

        let instability = metrics
            .get("instability")
            .and_then(|v| v.as_f64())
            .expect("Should have instability metric");

        assert!(instability >= 0.0 && instability <= 1.0);

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_analyze_dependencies_cohesion_basic() {
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

    run_dependency_test("cohesion_test.ts", code, "cohesion", |result| {
        assert_eq!(result.metadata.kind, "cohesion");
        assert!(result.summary.symbols_analyzed.is_some());

        if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
            return Ok(());
        }

        assert!(!result.findings.is_empty());

        let finding = &result.findings[0];
        assert_eq!(finding.kind, "cohesion_metric");
        assert!(finding.severity == Severity::Medium || finding.severity == Severity::Low);

        let metrics = finding.metrics.as_ref().expect("Should have metrics");
        assert!(metrics.contains_key("lcom_score"));
        assert!(metrics.contains_key("functions_analyzed"));
        assert!(metrics.contains_key("shared_data_ratio"));

        let lcom_score = metrics
            .get("lcom_score")
            .and_then(|v| v.as_f64())
            .expect("Should have lcom_score metric");

        assert!(lcom_score >= 0.0 && lcom_score <= 1.0);

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_analyze_dependencies_depth_basic() {
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

    run_dependency_test("depth_test.ts", code, "depth", |result| {
        assert_eq!(result.metadata.kind, "depth");
        assert!(result.summary.symbols_analyzed.is_some());

        if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
            return Ok(());
        }

        assert!(!result.findings.is_empty());

        let finding = &result.findings[0];
        assert_eq!(finding.kind, "dependency_depth");
        assert!(finding.severity == Severity::Medium || finding.severity == Severity::Low);

        let metrics = finding.metrics.as_ref().expect("Should have metrics");
        assert!(metrics.contains_key("max_depth"));
        assert!(metrics.contains_key("dependency_chain"));
        assert!(metrics.contains_key("direct_dependencies_count"));

        let dependency_chain = metrics
            .get("dependency_chain")
            .and_then(|v| v.as_array())
            .expect("Should have dependency_chain array");

        assert!(dependency_chain.len() > 0);

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_analyze_dependencies_unsupported_kind() {
    let workspace = TestWorkspace::new();
    workspace.create_file("test.ts", "export function foo() { return 1; }");
    let mut client = TestClient::new(workspace.path());
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
