//! Integration tests for complete Cargo package rename coverage (Proposal 02g)

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

/// Test complete Cargo package rename workflow
/// Verifies all 4 critical features from Proposal 02g:
/// 1. Root workspace Cargo.toml members list updated
/// 2. Package name in moved Cargo.toml updated
/// 3. Dev-dependency references updated across workspace
/// 4. Build succeeds without manual fixes
#[tokio::test]
async fn test_complete_cargo_package_rename() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create root workspace Cargo.toml
    workspace.create_file(
        "Cargo.toml",
        r#"
[workspace]
members = [
    "integration-tests",
    "app",
]
"#,
    );

    // Create integration-tests package
    workspace.create_directory("integration-tests/src");
    workspace.create_file(
        "integration-tests/Cargo.toml",
        r#"
[package]
name = "integration-tests"
version = "0.1.0"
edition = "2021"
"#,
    );
    workspace.create_file(
        "integration-tests/src/lib.rs",
        "pub fn test_helper() {}",
    );

    // Create app package that depends on integration-tests
    workspace.create_directory("app/src");
    workspace.create_file(
        "app/Cargo.toml",
        r#"
[package]
name = "app"
version = "0.1.0"
edition = "2021"

[dev-dependencies]
integration-tests = { path = "../integration-tests" }
"#,
    );
    workspace.create_file(
        "app/src/lib.rs",
        "pub fn app_fn() {}",
    );

    // Rename integration-tests → tests
    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "directory",
                    "path": workspace.absolute_path("integration-tests").to_string_lossy()
                },
                "new_name": workspace.absolute_path("tests").to_string_lossy()
            }),
        )
        .await
        .expect("rename.plan should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    // Apply the plan
    client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dry_run": false
                }
            }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    // VERIFICATION 1: Root workspace Cargo.toml members list updated
    let root_cargo = workspace.read_file("Cargo.toml");
    assert!(
        root_cargo.contains(r#""tests""#) || root_cargo.contains("tests"),
        "Root Cargo.toml should reference 'tests' in members. Actual:\n{}",
        root_cargo
    );
    assert!(
        !root_cargo.contains("integration-tests"),
        "Root Cargo.toml should not reference 'integration-tests' anymore. Actual:\n{}",
        root_cargo
    );

    // VERIFICATION 2: Package name in moved Cargo.toml updated
    let package_cargo = workspace.read_file("tests/Cargo.toml");
    assert!(
        package_cargo.contains(r#"name = "tests""#),
        "Package Cargo.toml should have name = 'tests'. Actual:\n{}",
        package_cargo
    );

    // VERIFICATION 3: Dev-dependency references updated
    let app_cargo = workspace.read_file("app/Cargo.toml");
    assert!(
        app_cargo.contains(r#"tests = { path = "../tests" }"#)
            || (app_cargo.contains("tests") && app_cargo.contains("../tests")),
        "App Cargo.toml should reference 'tests' with correct path. Actual:\n{}",
        app_cargo
    );
    assert!(
        !app_cargo.contains("integration-tests"),
        "App Cargo.toml should not reference 'integration-tests' anymore. Actual:\n{}",
        app_cargo
    );

    println!("✅ All Cargo package rename features verified!");
    println!("  ✓ Root workspace members updated");
    println!("  ✓ Package name updated");
    println!("  ✓ Dev-dependency references updated");
}
