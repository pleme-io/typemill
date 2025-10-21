//! Edge case tests for Cargo manifest updates during directory renames
//!
//! These tests cover less common but important scenarios:
//! - dev-dependencies and build-dependencies updates
//! - [workspace.dependencies] section updates
//! - Inline table vs regular table dependency formats
//! - Mixed dependency types in same Cargo.toml

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

// =============================================================================
// Test 1: dev-dependencies and build-dependencies Updates
// =============================================================================

#[tokio::test]
async fn test_rust_dev_and_build_dependencies_update() {
    println!("\n🧪 Test: Rename updates dev-dependencies and build-dependencies");

    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create workspace root
    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = ["crates/utils", "crates/app"]
resolver = "2"
"#,
    );

    workspace.create_directory("crates");

    // Create utils crate
    workspace.create_directory("crates/utils");
    workspace.create_directory("crates/utils/src");

    workspace.create_file(
        "crates/utils/Cargo.toml",
        r#"[package]
name = "utils"
version = "0.1.0"
edition = "2021"
"#,
    );

    workspace.create_file(
        "crates/utils/src/lib.rs",
        "pub fn utility() -> i32 { 42 }\n",
    );

    // Create app crate with dev-dependencies and build-dependencies
    workspace.create_directory("crates/app");
    workspace.create_directory("crates/app/src");

    workspace.create_file(
        "crates/app/Cargo.toml",
        r#"[package]
name = "app"
version = "0.1.0"
edition = "2021"

[dependencies]
utils = { path = "../utils" }

[dev-dependencies]
utils = { path = "../utils" }

[build-dependencies]
utils = { path = "../utils" }
"#,
    );

    workspace.create_file(
        "crates/app/src/main.rs",
        "use utils::utility;\n\nfn main() {\n    println!(\"{}\", utility());\n}\n",
    );

    // Action: Rename utils → helpers
    let old_path = workspace.absolute_path("crates/utils");
    let new_path = workspace.absolute_path("crates/helpers");

    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "directory",
                    "path": old_path.to_string_lossy()
                },
                "new_name": new_path.to_string_lossy()
            }),
        )
        .await
        .expect("rename.plan should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dry_run": false,
                    "validate_checksums": true
                }
            }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    // Verify: ALL dependency sections updated
    let app_cargo = workspace.read_file("crates/app/Cargo.toml");

    assert!(
        app_cargo.contains("[dependencies]")
            && app_cargo.contains("helpers = { path = \"../helpers\" }"),
        "Regular dependencies should be updated. Content:\n{}",
        app_cargo
    );

    assert!(
        app_cargo.contains("[dev-dependencies]")
            && app_cargo.contains("helpers = { path = \"../helpers\" }"),
        "dev-dependencies should be updated. Content:\n{}",
        app_cargo
    );

    assert!(
        app_cargo.contains("[build-dependencies]")
            && app_cargo.contains("helpers = { path = \"../helpers\" }"),
        "build-dependencies should be updated. Content:\n{}",
        app_cargo
    );

    // Verify: Old package name removed from all sections
    assert!(
        !app_cargo.contains("utils = { path"),
        "Old dependency name should be removed from all sections. Content:\n{}",
        app_cargo
    );

    println!("✅ Verified: All dependency sections (dependencies, dev-dependencies, build-dependencies) updated");
}

// =============================================================================
// Test 2: Workspace Dependencies Section
// =============================================================================

#[tokio::test]
async fn test_rust_workspace_dependencies_update() {
    println!("\n🧪 Test: Rename updates [workspace.dependencies] section");

    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create workspace root with [workspace.dependencies]
    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = ["crates/utils", "crates/app"]
resolver = "2"

[workspace.dependencies]
utils = { path = "crates/utils" }
"#,
    );

    workspace.create_directory("crates");

    // Create utils crate
    workspace.create_directory("crates/utils");
    workspace.create_directory("crates/utils/src");

    workspace.create_file(
        "crates/utils/Cargo.toml",
        r#"[package]
name = "utils"
version = "0.1.0"
edition = "2021"
"#,
    );

    workspace.create_file(
        "crates/utils/src/lib.rs",
        "pub fn utility() -> i32 { 42 }\n",
    );

    // Create app crate using workspace dependency
    workspace.create_directory("crates/app");
    workspace.create_directory("crates/app/src");

    workspace.create_file(
        "crates/app/Cargo.toml",
        r#"[package]
name = "app"
version = "0.1.0"
edition = "2021"

[dependencies]
utils = { workspace = true }
"#,
    );

    workspace.create_file(
        "crates/app/src/main.rs",
        "use utils::utility;\n\nfn main() {\n    println!(\"{}\", utility());\n}\n",
    );

    // Action: Rename utils → helpers
    let old_path = workspace.absolute_path("crates/utils");
    let new_path = workspace.absolute_path("crates/helpers");

    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "directory",
                    "path": old_path.to_string_lossy()
                },
                "new_name": new_path.to_string_lossy()
            }),
        )
        .await
        .expect("rename.plan should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dry_run": false,
                    "validate_checksums": true
                }
            }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    // Verify: [workspace.dependencies] section updated
    let root_cargo = workspace.read_file("Cargo.toml");

    assert!(
        root_cargo.contains("[workspace.dependencies]"),
        "Workspace dependencies section should exist"
    );

    assert!(
        root_cargo.contains("helpers = { path = \"crates/helpers\" }"),
        "Workspace dependency should be updated. Content:\n{}",
        root_cargo
    );

    assert!(
        !root_cargo.contains("utils = { path"),
        "Old workspace dependency should be removed. Content:\n{}",
        root_cargo
    );

    // Verify: App crate still uses workspace dependency (name updated)
    let app_cargo = workspace.read_file("crates/app/Cargo.toml");

    assert!(
        app_cargo.contains("helpers = { workspace = true }"),
        "App should reference new workspace dependency. Content:\n{}",
        app_cargo
    );

    println!("✅ Verified: [workspace.dependencies] section updated correctly");
}

// =============================================================================
// Test 3: Inline Table Dependencies
// =============================================================================

#[tokio::test]
async fn test_rust_inline_table_dependencies() {
    println!("\n🧪 Test: Rename updates inline table dependency format");

    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create workspace
    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = ["crates/utils", "crates/app"]
resolver = "2"
"#,
    );

    workspace.create_directory("crates");

    // Utils crate
    workspace.create_directory("crates/utils");
    workspace.create_directory("crates/utils/src");

    workspace.create_file(
        "crates/utils/Cargo.toml",
        r#"[package]
name = "utils"
version = "0.1.0"
edition = "2021"
"#,
    );

    workspace.create_file("crates/utils/src/lib.rs", "pub fn helper() {}\n");

    // App crate with inline table dependency (common format)
    workspace.create_directory("crates/app");
    workspace.create_directory("crates/app/src");

    workspace.create_file(
        "crates/app/Cargo.toml",
        r#"[package]
name = "app"
version = "0.1.0"
edition = "2021"

[dependencies]
utils = { path = "../utils", version = "0.1.0" }
"#,
    );

    workspace.create_file(
        "crates/app/src/main.rs",
        "use utils::helper;\n\nfn main() {\n    helper();\n}\n",
    );

    // Action: Rename
    let old_path = workspace.absolute_path("crates/utils");
    let new_path = workspace.absolute_path("crates/core");

    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "directory",
                    "path": old_path.to_string_lossy()
                },
                "new_name": new_path.to_string_lossy()
            }),
        )
        .await
        .expect("rename.plan should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dry_run": false,
                    "validate_checksums": true
                }
            }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    // Verify: Inline table dependency updated (name AND path)
    let app_cargo = workspace.read_file("crates/app/Cargo.toml");

    // Should have new dependency name with updated path
    assert!(
        app_cargo.contains("core = { path = \"../core\""),
        "Dependency name and path should be updated in inline table. Content:\n{}",
        app_cargo
    );

    // Should NOT have old dependency
    assert!(
        !app_cargo.contains("utils = { path"),
        "Old dependency should be removed. Content:\n{}",
        app_cargo
    );

    // Verify: Use statement updated
    let main_content = workspace.read_file("crates/app/src/main.rs");
    assert!(
        main_content.contains("use core::helper;"),
        "Use statement should be updated. Content:\n{}",
        main_content
    );

    println!("✅ Verified: Inline table dependencies handled correctly");
}

// =============================================================================
// Test 4: Mixed Dependency Formats
// =============================================================================

#[tokio::test]
async fn test_rust_mixed_dependency_formats() {
    println!("\n🧪 Test: Rename updates mixed inline and table dependencies");

    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Workspace with multiple crates
    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = ["crates/common", "crates/app"]
resolver = "2"
"#,
    );

    workspace.create_directory("crates");

    // Common crate
    workspace.create_directory("crates/common");
    workspace.create_directory("crates/common/src");

    workspace.create_file(
        "crates/common/Cargo.toml",
        r#"[package]
name = "common"
version = "0.1.0"
edition = "2021"
"#,
    );

    workspace.create_file("crates/common/src/lib.rs", "pub fn func() {}\n");

    // App crate with MIXED formats (inline + table)
    workspace.create_directory("crates/app");
    workspace.create_directory("crates/app/src");

    workspace.create_file(
        "crates/app/Cargo.toml",
        r#"[package]
name = "app"
version = "0.1.0"
edition = "2021"

[dependencies]
common = { path = "../common" }

[dev-dependencies.common]
path = "../common"
"#,
    );

    workspace.create_file(
        "crates/app/src/main.rs",
        "use common::func;\n\nfn main() {\n    func();\n}\n",
    );

    // Action: Rename common → shared
    let old_path = workspace.absolute_path("crates/common");
    let new_path = workspace.absolute_path("crates/shared");

    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "directory",
                    "path": old_path.to_string_lossy()
                },
                "new_name": new_path.to_string_lossy()
            }),
        )
        .await
        .expect("rename.plan should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dry_run": false,
                    "validate_checksums": true
                }
            }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    // Verify: Both inline and table format dependencies updated
    let app_cargo = workspace.read_file("crates/app/Cargo.toml");

    // Inline format in [dependencies]
    assert!(
        app_cargo.contains("shared = { path = \"../shared\" }"),
        "Inline dependency should be updated. Content:\n{}",
        app_cargo
    );

    // Table format in [dev-dependencies]
    assert!(
        app_cargo.contains("[dev-dependencies.shared]")
            && app_cargo.contains("path = \"../shared\""),
        "Table-format dev-dependency should be updated. Content:\n{}",
        app_cargo
    );

    // Old name removed
    assert!(
        !app_cargo.contains("common = { path") && !app_cargo.contains("[dev-dependencies.common]"),
        "Old dependency names should be removed. Content:\n{}",
        app_cargo
    );

    // Use statements updated
    let main_content = workspace.read_file("crates/app/src/main.rs");
    assert!(
        main_content.contains("use shared::func;"),
        "Use statement should be updated. Content:\n{}",
        main_content
    );

    println!("✅ Verified: Mixed dependency formats (inline + table) handled correctly");
}
