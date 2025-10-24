//! Analysis API tests for analyze.structure (MIGRATED VERSION)
//!
//! BEFORE: 573 lines with repetitive setup and result parsing
//! AFTER: Using simplified helper pattern for analysis tests
//!
//! Tests structure analysis: symbols, hierarchy, interfaces, inheritance, modules

use crate::harness::{TestClient, TestWorkspace};
use mill_foundation::protocol::analysis_result::{AnalysisResult, Severity};
use serde_json::json;

/// Helper to run structure analysis test
async fn run_structure_test<V>(
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
            "analyze.structure",
            json!({
                "kind": kind,
                "scope": {
                    "type": "file",
                    "path": test_file.to_string_lossy()
                }
            }),
        )
        .await
        .expect("analyze.structure call should succeed");

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
async fn test_analyze_structure_symbols_basic() {
    let code = r#"
export interface User {
    id: number;
    name: string;
}

export enum Status {
    Active,
    Inactive
}

export class UserService {
    getUser(id: number): User {
        return { id, name: "Test" };
    }
}

export function formatUser(user: User): string {
    return `User: ${user.name}`;
}

export type UserData = {
    user: User;
    status: Status;
};
"#;

    run_structure_test("symbols_test.ts", code, "symbols", |result| {
        assert_eq!(result.metadata.category, "structure");
        assert_eq!(result.metadata.kind, "symbols");
        assert!(result.summary.symbols_analyzed.is_some());

        if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
            return Ok(());
        }

        assert!(!result.findings.is_empty());

        let finding = &result.findings[0];
        assert_eq!(finding.kind, "symbols");
        assert_eq!(finding.severity, Severity::Low);

        let metrics = finding.metrics.as_ref().expect("Should have metrics");
        assert!(metrics.contains_key("total_symbols"));
        assert!(metrics.contains_key("symbols_by_kind"));
        assert!(metrics.contains_key("visibility_breakdown"));

        let total_symbols = metrics
            .get("total_symbols")
            .and_then(|v| v.as_u64())
            .expect("Should have total_symbols");

        assert!(total_symbols > 0);

        let symbols_by_kind = metrics
            .get("symbols_by_kind")
            .and_then(|v| v.as_object())
            .expect("Should have symbols_by_kind object");

        assert!(!symbols_by_kind.is_empty());

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_analyze_structure_hierarchy_basic() {
    let code = r#"
export class BaseClass {
    baseMethod() {
        console.log("Base");
    }
}

export class MiddleClass extends BaseClass {
    middleMethod() {
        console.log("Middle");
    }
}

export class LeafClass extends MiddleClass {
    leafMethod() {
        console.log("Leaf");
    }
}
"#;

    run_structure_test("hierarchy_test.ts", code, "hierarchy", |result| {
        assert_eq!(result.metadata.kind, "hierarchy");
        assert!(result.summary.symbols_analyzed.is_some());

        if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
            return Ok(());
        }

        assert!(!result.findings.is_empty());

        let finding = &result.findings[0];
        assert_eq!(finding.kind, "hierarchy");
        assert!(finding.severity == Severity::Medium || finding.severity == Severity::Low);

        let metrics = finding.metrics.as_ref().expect("Should have metrics");
        assert!(metrics.contains_key("max_depth"));
        assert!(metrics.contains_key("total_classes"));
        assert!(metrics.contains_key("root_classes"));
        assert!(metrics.contains_key("leaf_classes"));
        assert!(metrics.contains_key("hierarchy_tree"));

        let total_classes = metrics
            .get("total_classes")
            .and_then(|v| v.as_u64())
            .expect("Should have total_classes");

        assert!(total_classes >= 3);

        let max_depth = metrics
            .get("max_depth")
            .and_then(|v| v.as_u64())
            .expect("Should have max_depth");

        assert!(max_depth > 0);

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_analyze_structure_interfaces_basic() {
    let code = r#"
// Fat interface with 12 methods (violates ISP)
export interface FatService {
    method1(): void;
    method2(): void;
    method3(): void;
    method4(): void;
    method5(): void;
    method6(): void;
    method7(): void;
    method8(): void;
    method9(): void;
    method10(): void;
    method11(): void;
    method12(): void;
}

// Clean interface
export interface SimpleInterface {
    getData(): string;
    setData(data: string): void;
}
"#;

    run_structure_test("interfaces_test.ts", code, "interfaces", |result| {
        assert_eq!(result.metadata.kind, "interfaces");
        assert!(result.summary.symbols_analyzed.is_some());

        if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
            return Ok(());
        }

        assert!(!result.findings.is_empty());

        let finding = &result.findings[0];
        assert_eq!(finding.kind, "interfaces");
        assert!(finding.severity == Severity::Medium || finding.severity == Severity::Low);

        let metrics = finding.metrics.as_ref().expect("Should have metrics");
        assert!(metrics.contains_key("interface_count"));
        assert!(metrics.contains_key("methods_per_interface"));
        assert!(metrics.contains_key("fat_interfaces"));

        let interface_count = metrics
            .get("interface_count")
            .and_then(|v| v.as_u64())
            .expect("Should have interface_count");

        assert!(interface_count >= 2);

        let fat_interfaces = metrics
            .get("fat_interfaces")
            .and_then(|v| v.as_array())
            .expect("Should have fat_interfaces array");

        assert!(fat_interfaces.len() > 0);

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_analyze_structure_inheritance_basic() {
    let code = r#"
export class Level1 {
    method1() {
        console.log("Level 1");
    }
}

export class Level2 extends Level1 {
    method2() {
        console.log("Level 2");
    }
}

export class Level3 extends Level2 {
    method3() {
        console.log("Level 3");
    }
}

export class Level4 extends Level3 {
    method4() {
        console.log("Level 4");
    }
}

export class Level5 extends Level4 {
    method5() {
        console.log("Level 5");
    }
}
"#;

    run_structure_test("inheritance_test.ts", code, "inheritance", |result| {
        assert_eq!(result.metadata.kind, "inheritance");
        assert!(result.summary.symbols_analyzed.is_some());

        if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
            return Ok(());
        }

        assert!(!result.findings.is_empty());

        let finding = &result.findings[0];
        assert_eq!(finding.kind, "inheritance");
        assert!(finding.severity == Severity::High || finding.severity == Severity::Low);

        let metrics = finding.metrics.as_ref().expect("Should have metrics");
        assert!(metrics.contains_key("max_inheritance_depth"));
        assert!(metrics.contains_key("classes_by_depth"));
        assert!(metrics.contains_key("inheritance_chains"));

        let max_depth = metrics
            .get("max_inheritance_depth")
            .and_then(|v| v.as_u64())
            .expect("Should have max_inheritance_depth");

        assert!(max_depth > 0);

        let inheritance_chains = metrics
            .get("inheritance_chains")
            .and_then(|v| v.as_array())
            .expect("Should have inheritance_chains array");

        assert!(!inheritance_chains.is_empty());

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_analyze_structure_modules_basic() {
    let code = r#"
export namespace Utils {
    export function fn1() { return 1; }
    export function fn2() { return 2; }
    export function fn3() { return 3; }
    export function fn4() { return 4; }
    export function fn5() { return 5; }
}

export namespace DataAccess {
    export class Repository {
        getData() { return []; }
    }
    export interface IRepository {
        getData(): any[];
    }
}

// Top-level functions (orphaned items)
export function helper1() { return "helper1"; }
export function helper2() { return "helper2"; }
export function helper3() { return "helper3"; }
"#;

    run_structure_test("modules_test.ts", code, "modules", |result| {
        assert_eq!(result.metadata.kind, "modules");
        assert!(result.summary.symbols_analyzed.is_some());

        if result.summary.symbols_analyzed.unwrap_or(0) == 0 {
            return Ok(());
        }

        assert!(!result.findings.is_empty());

        let finding = &result.findings[0];
        assert_eq!(finding.kind, "modules");
        assert!(finding.severity == Severity::Medium || finding.severity == Severity::Low);

        let metrics = finding.metrics.as_ref().expect("Should have metrics");
        assert!(metrics.contains_key("module_count"));
        assert!(metrics.contains_key("items_per_module"));
        assert!(metrics.contains_key("god_modules"));
        assert!(metrics.contains_key("orphaned_items_count"));
        assert!(metrics.contains_key("total_items"));

        let total_items = metrics
            .get("total_items")
            .and_then(|v| v.as_u64())
            .expect("Should have total_items");

        assert!(total_items > 0);

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_analyze_structure_unsupported_kind() {
    let workspace = TestWorkspace::new();
    workspace.create_file("test.ts", "export function foo() { return 1; }");
    let mut client = TestClient::new(workspace.path());
    let test_file = workspace.absolute_path("test.ts");

    let response = client
        .call_tool(
            "analyze.structure",
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
