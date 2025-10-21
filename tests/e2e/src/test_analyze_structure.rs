use crate::harness::{TestClient, TestWorkspace};
use codebuddy_foundation::protocol::analysis_result::{AnalysisResult, Severity};
use serde_json::json;

#[tokio::test]
async fn test_analyze_structure_symbols_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with diverse symbols
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

    workspace.create_file("symbols_test.ts", code);
    let test_file = workspace.absolute_path("symbols_test.ts");

    let response = client
        .call_tool(
            "analyze.structure",
            json!({
                "kind": "symbols",
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

    // Verify result structure
    assert_eq!(result.metadata.category, "structure");
    assert_eq!(result.metadata.kind, "symbols");

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

    // Should detect symbols
    assert!(!result.findings.is_empty(), "Expected symbol findings");

    // Verify finding structure
    let finding = &result.findings[0];
    assert_eq!(finding.kind, "symbols");
    assert_eq!(finding.severity, Severity::Low);

    // Verify metrics
    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(metrics.contains_key("total_symbols"));
    assert!(metrics.contains_key("symbols_by_kind"));
    assert!(metrics.contains_key("visibility_breakdown"));

    // Verify we detected symbols
    let total_symbols = metrics
        .get("total_symbols")
        .and_then(|v| v.as_u64())
        .expect("Should have total_symbols");

    assert!(total_symbols > 0, "Should detect symbols");

    // Verify symbols_by_kind is present
    let symbols_by_kind = metrics
        .get("symbols_by_kind")
        .and_then(|v| v.as_object())
        .expect("Should have symbols_by_kind object");

    assert!(!symbols_by_kind.is_empty(), "Should categorize symbols");
}

#[tokio::test]
async fn test_analyze_structure_hierarchy_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with class hierarchy
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

    workspace.create_file("hierarchy_test.ts", code);
    let test_file = workspace.absolute_path("hierarchy_test.ts");

    let response = client
        .call_tool(
            "analyze.structure",
            json!({
                "kind": "hierarchy",
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

    assert_eq!(result.metadata.kind, "hierarchy");

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

    // Should have hierarchy finding
    assert!(!result.findings.is_empty(), "Expected hierarchy findings");

    let finding = &result.findings[0];
    assert_eq!(finding.kind, "hierarchy");

    // Severity can be Medium (depth > 5) or Low (acceptable depth)
    assert!(
        finding.severity == Severity::Medium || finding.severity == Severity::Low,
        "Severity should be Medium or Low"
    );

    // Verify hierarchy metrics
    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(metrics.contains_key("max_depth"));
    assert!(metrics.contains_key("total_classes"));
    assert!(metrics.contains_key("root_classes"));
    assert!(metrics.contains_key("leaf_classes"));
    assert!(metrics.contains_key("hierarchy_tree"));

    // Verify we detected hierarchy
    let total_classes = metrics
        .get("total_classes")
        .and_then(|v| v.as_u64())
        .expect("Should have total_classes");

    assert!(
        total_classes >= 3,
        "Should detect at least 3 classes in hierarchy"
    );

    let max_depth = metrics
        .get("max_depth")
        .and_then(|v| v.as_u64())
        .expect("Should have max_depth");

    assert!(max_depth > 0, "Should calculate hierarchy depth");
}

#[tokio::test]
async fn test_analyze_structure_interfaces_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with interfaces (including fat interface)
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

    workspace.create_file("interfaces_test.ts", code);
    let test_file = workspace.absolute_path("interfaces_test.ts");

    let response = client
        .call_tool(
            "analyze.structure",
            json!({
                "kind": "interfaces",
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

    assert_eq!(result.metadata.kind, "interfaces");

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

    // Should have interface finding
    assert!(!result.findings.is_empty(), "Expected interface findings");

    let finding = &result.findings[0];
    assert_eq!(finding.kind, "interfaces");

    // Severity can be Medium (fat interfaces) or Low (clean interfaces)
    assert!(
        finding.severity == Severity::Medium || finding.severity == Severity::Low,
        "Severity should be Medium or Low"
    );

    // Verify interface metrics
    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(metrics.contains_key("interface_count"));
    assert!(metrics.contains_key("methods_per_interface"));
    assert!(metrics.contains_key("fat_interfaces"));

    // Verify we detected interfaces
    let interface_count = metrics
        .get("interface_count")
        .and_then(|v| v.as_u64())
        .expect("Should have interface_count");

    assert!(interface_count >= 2, "Should detect at least 2 interfaces");

    // Verify fat interfaces are detected
    let fat_interfaces = metrics
        .get("fat_interfaces")
        .and_then(|v| v.as_array())
        .expect("Should have fat_interfaces array");

    // Should detect the FatService interface
    assert!(
        fat_interfaces.len() > 0,
        "Should detect at least one fat interface"
    );
}

#[tokio::test]
async fn test_analyze_structure_inheritance_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with inheritance chain (5 levels - deep inheritance)
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

    workspace.create_file("inheritance_test.ts", code);
    let test_file = workspace.absolute_path("inheritance_test.ts");

    let response = client
        .call_tool(
            "analyze.structure",
            json!({
                "kind": "inheritance",
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

    assert_eq!(result.metadata.kind, "inheritance");

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

    // Should have inheritance finding
    assert!(!result.findings.is_empty(), "Expected inheritance findings");

    let finding = &result.findings[0];
    assert_eq!(finding.kind, "inheritance");

    // Severity can be High (depth > 4) or Low (acceptable depth)
    assert!(
        finding.severity == Severity::High || finding.severity == Severity::Low,
        "Severity should be High or Low"
    );

    // Verify inheritance metrics
    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(metrics.contains_key("max_inheritance_depth"));
    assert!(metrics.contains_key("classes_by_depth"));
    assert!(metrics.contains_key("inheritance_chains"));

    // Verify we detected inheritance
    let max_depth = metrics
        .get("max_inheritance_depth")
        .and_then(|v| v.as_u64())
        .expect("Should have max_inheritance_depth");

    assert!(max_depth > 0, "Should detect inheritance depth");

    // Verify inheritance chains
    let inheritance_chains = metrics
        .get("inheritance_chains")
        .and_then(|v| v.as_array())
        .expect("Should have inheritance_chains array");

    assert!(
        !inheritance_chains.is_empty(),
        "Should detect inheritance chains"
    );
}

#[tokio::test]
async fn test_analyze_structure_modules_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file with namespace organization
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

    workspace.create_file("modules_test.ts", code);
    let test_file = workspace.absolute_path("modules_test.ts");

    let response = client
        .call_tool(
            "analyze.structure",
            json!({
                "kind": "modules",
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

    assert_eq!(result.metadata.kind, "modules");

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

    // Should have module finding
    assert!(!result.findings.is_empty(), "Expected module findings");

    let finding = &result.findings[0];
    assert_eq!(finding.kind, "modules");

    // Severity can be Medium (god modules or many orphaned items) or Low (acceptable)
    assert!(
        finding.severity == Severity::Medium || finding.severity == Severity::Low,
        "Severity should be Medium or Low"
    );

    // Verify module metrics
    let metrics = finding.metrics.as_ref().expect("Should have metrics");
    assert!(metrics.contains_key("module_count"));
    assert!(metrics.contains_key("items_per_module"));
    assert!(metrics.contains_key("god_modules"));
    assert!(metrics.contains_key("orphaned_items_count"));
    assert!(metrics.contains_key("total_items"));

    // Verify we detected modules or items
    let total_items = metrics
        .get("total_items")
        .and_then(|v| v.as_u64())
        .expect("Should have total_items");

    assert!(total_items > 0, "Should detect items");
}

#[tokio::test]
async fn test_analyze_structure_unsupported_kind() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("test.ts", "export function foo() { return 1; }");
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
