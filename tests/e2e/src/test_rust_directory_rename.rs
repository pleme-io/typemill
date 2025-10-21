//! Integration tests for Rust workspace member directory renames
//!
//! These tests verify that when a Rust workspace member directory is renamed,
//! all related updates occur correctly:
//! - Cargo.toml workspace members array updates
//! - Package Cargo.toml name updates
//! - Path dependency updates in dependent crates
//! - Use statements across crates update correctly
//!
//! Tests cover:
//! - Simple workspace member rename
//! - Path dependency updates
//! - Cross-crate use statement updates
//! - Nested module directory renames
//! - Comprehensive multi-crate workspace scenarios

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

// =============================================================================
// Test 1: Basic Workspace Member Rename
// =============================================================================

#[tokio::test]
async fn test_rust_workspace_member_rename() {
    // Setup: Root Cargo.toml with workspace member
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create workspace root Cargo.toml
    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = ["crates/my-crate"]
resolver = "2"
"#,
    );

    // Create workspace member
    workspace.create_directory("crates");
    workspace.create_directory("crates/my-crate");
    workspace.create_directory("crates/my-crate/src");

    workspace.create_file(
        "crates/my-crate/Cargo.toml",
        r#"[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
    );

    workspace.create_file(
        "crates/my-crate/src/lib.rs",
        "pub fn hello() -> String {\n    String::from(\"Hello\")\n}\n",
    );

    // Action: Rename crates/my-crate → crates/my-renamed-crate
    let old_path = workspace.absolute_path("crates/my-crate");
    let new_path = workspace.absolute_path("crates/my-renamed-crate");

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
        .expect("Plan should have result.content");

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

    // Verify: Directory renamed
    assert!(
        !workspace.file_exists("crates/my-crate"),
        "Old directory should be deleted"
    );
    assert!(
        workspace.file_exists("crates/my-renamed-crate"),
        "New directory should exist"
    );
    assert!(
        workspace.file_exists("crates/my-renamed-crate/src/lib.rs"),
        "Files should be preserved"
    );

    // Verify: Root Cargo.toml members array updated
    let root_cargo = workspace.read_file("Cargo.toml");
    assert!(
        root_cargo.contains("crates/my-renamed-crate"),
        "Root Cargo.toml should contain new member path. Content:\n{}",
        root_cargo
    );
    assert!(
        !root_cargo.contains("crates/my-crate\"") || !root_cargo.contains("crates/my-crate]"),
        "Root Cargo.toml should not contain old member path. Content:\n{}",
        root_cargo
    );

    // Verify: Package Cargo.toml name updated
    let package_cargo = workspace.read_file("crates/my-renamed-crate/Cargo.toml");
    assert!(
        package_cargo.contains("name = \"my-renamed-crate\""),
        "Package name should be updated. Content:\n{}",
        package_cargo
    );
    assert!(
        !package_cargo.contains("name = \"my-crate\""),
        "Old package name should be removed. Content:\n{}",
        package_cargo
    );
}

// =============================================================================
// Test 2: Path Dependency Updates
// =============================================================================

#[tokio::test]
async fn test_rust_path_dependency_updates() {
    // Setup: Two crates, one depends on the other via path
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create workspace root
    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = ["crates/common", "crates/app"]
resolver = "2"
"#,
    );

    // Create common crate
    workspace.create_directory("crates");
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

    workspace.create_file(
        "crates/common/src/lib.rs",
        "pub fn utility() -> i32 { 42 }\n",
    );

    // Create app crate that depends on common
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
"#,
    );

    workspace.create_file(
        "crates/app/src/main.rs",
        "use common::utility;\n\nfn main() {\n    println!(\"{}\", utility());\n}\n",
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

    // Verify: Dependent crate's Cargo.toml path updated
    let app_cargo = workspace.read_file("crates/app/Cargo.toml");
    assert!(
        app_cargo.contains("path = \"../shared\""),
        "Path dependency should be updated. Content:\n{}",
        app_cargo
    );
    assert!(
        !app_cargo.contains("path = \"../common\""),
        "Old path should be removed. Content:\n{}",
        app_cargo
    );

    // Verify: Dependency name updated (package name changed)
    assert!(
        app_cargo.contains("shared = { path = \"../shared\" }"),
        "Dependency name should be updated. Content:\n{}",
        app_cargo
    );
}

// =============================================================================
// Test 3: Cross-Crate Use Statements Update
// =============================================================================

#[tokio::test]
async fn test_rust_cross_crate_use_statements_update() {
    // Setup: common/src/lib.rs with public function, app uses it
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Root Cargo.toml
    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = ["crates/common", "crates/app"]
resolver = "2"
"#,
    );

    // Common crate
    workspace.create_directory("crates");
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

    workspace.create_file(
        "crates/common/src/lib.rs",
        "pub fn util() -> &'static str {\n    \"utility\"\n}\n",
    );

    // App crate
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
"#,
    );

    workspace.create_file(
        "crates/app/src/main.rs",
        "use common::util;\n\nfn main() {\n    println!(\"{}\", util());\n}\n",
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

    // Verify: use common::util → use shared::util
    let main_content = workspace.read_file("crates/app/src/main.rs");
    assert!(
        main_content.contains("use shared::util;"),
        "Use statement should be updated. Content:\n{}",
        main_content
    );
    assert!(
        !main_content.contains("use common::util;"),
        "Old use statement should be removed. Content:\n{}",
        main_content
    );

    // Verify: Cargo.toml also updated
    let app_cargo = workspace.read_file("crates/app/Cargo.toml");
    assert!(
        app_cargo.contains("shared = { path = \"../shared\" }"),
        "Cargo.toml dependency should be updated. Content:\n{}",
        app_cargo
    );
}

// =============================================================================
// Test 4: Nested Module Directory Rename
// =============================================================================

#[tokio::test]
async fn test_rust_nested_module_directory_rename() {
    // Setup: src/utils/ directory with multiple files importing each other
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Root package
    workspace.create_file(
        "Cargo.toml",
        r#"[package]
name = "myproject"
version = "0.1.0"
edition = "2021"
"#,
    );

    // Create utils module
    workspace.create_directory("src");
    workspace.create_directory("src/utils");

    workspace.create_file("src/utils/mod.rs", "pub mod strings;\npub mod numbers;\n");

    workspace.create_file(
        "src/utils/strings.rs",
        "pub fn uppercase(s: &str) -> String {\n    s.to_uppercase()\n}\n",
    );

    workspace.create_file(
        "src/utils/numbers.rs",
        "pub fn double(n: i32) -> i32 {\n    n * 2\n}\n",
    );

    // Main file uses utils
    workspace.create_file(
        "src/main.rs",
        "mod utils;\n\nuse utils::strings::uppercase;\nuse utils::numbers::double;\n\nfn main() {\n    println!(\"{}\", uppercase(\"hello\"));\n    println!(\"{}\", double(21));\n}\n",
    );

    // Action: Rename src/utils → src/helpers
    let old_path = workspace.absolute_path("src/utils");
    let new_path = workspace.absolute_path("src/helpers");

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

    // Verify: Directory renamed
    assert!(
        !workspace.file_exists("src/utils"),
        "Old directory should be deleted"
    );
    assert!(
        workspace.file_exists("src/helpers"),
        "New directory should exist"
    );

    // Verify: External imports updated in main.rs
    let main_content = workspace.read_file("src/main.rs");
    assert!(
        main_content.contains("mod helpers;"),
        "Module declaration should be updated. Content:\n{}",
        main_content
    );
    assert!(
        main_content.contains("use helpers::strings::uppercase;"),
        "Use statement should be updated. Content:\n{}",
        main_content
    );
    assert!(
        main_content.contains("use helpers::numbers::double;"),
        "Use statement should be updated. Content:\n{}",
        main_content
    );
    assert!(
        !main_content.contains("use utils::"),
        "Old use statements should be removed. Content:\n{}",
        main_content
    );

    // Verify: Internal module structure preserved
    let mod_content = workspace.read_file("src/helpers/mod.rs");
    assert!(
        mod_content.contains("pub mod strings;"),
        "Internal module declarations should remain. Content:\n{}",
        mod_content
    );
    assert!(
        mod_content.contains("pub mod numbers;"),
        "Internal module declarations should remain. Content:\n{}",
        mod_content
    );
}

// =============================================================================
// Test 5: Comprehensive Workspace Rename
// =============================================================================

#[tokio::test]
async fn test_rust_comprehensive_workspace_rename() {
    // Setup: Full workspace with multiple crates, dependencies, use statements
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Root workspace
    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = [
    "crates/core",
    "crates/utils",
    "crates/app"
]
resolver = "2"
"#,
    );

    workspace.create_directory("crates");

    // Core crate
    workspace.create_directory("crates/core");
    workspace.create_directory("crates/core/src");

    workspace.create_file(
        "crates/core/Cargo.toml",
        r#"[package]
name = "core"
version = "0.1.0"
edition = "2021"
"#,
    );

    workspace.create_file(
        "crates/core/src/lib.rs",
        "pub struct Engine {\n    pub power: u32,\n}\n\nimpl Engine {\n    pub fn new(power: u32) -> Self {\n        Self { power }\n    }\n}\n",
    );

    // Utils crate (depends on core)
    workspace.create_directory("crates/utils");
    workspace.create_directory("crates/utils/src");

    workspace.create_file(
        "crates/utils/Cargo.toml",
        r#"[package]
name = "utils"
version = "0.1.0"
edition = "2021"

[dependencies]
core = { path = "../core" }
"#,
    );

    workspace.create_file(
        "crates/utils/src/lib.rs",
        "use core::Engine;\n\npub fn create_engine() -> Engine {\n    Engine::new(100)\n}\n",
    );

    // App crate (depends on both core and utils)
    workspace.create_directory("crates/app");
    workspace.create_directory("crates/app/src");

    workspace.create_file(
        "crates/app/Cargo.toml",
        r#"[package]
name = "app"
version = "0.1.0"
edition = "2021"

[dependencies]
core = { path = "../core" }
utils = { path = "../utils" }
"#,
    );

    workspace.create_file(
        "crates/app/src/main.rs",
        "use core::Engine;\nuse utils::create_engine;\n\nfn main() {\n    let engine = create_engine();\n    println!(\"Engine power: {}\", engine.power);\n}\n",
    );

    // Action: Rename core → engine
    let old_path = workspace.absolute_path("crates/core");
    let new_path = workspace.absolute_path("crates/engine");

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

    // Verify ALL updates:

    // 1. Workspace members updated
    let root_cargo = workspace.read_file("Cargo.toml");
    assert!(
        root_cargo.contains("\"crates/engine\""),
        "Workspace members should contain new path. Content:\n{}",
        root_cargo
    );
    assert!(
        !root_cargo.contains("\"crates/core\""),
        "Workspace members should not contain old path. Content:\n{}",
        root_cargo
    );

    // 2. Package name updated
    let engine_cargo = workspace.read_file("crates/engine/Cargo.toml");
    assert!(
        engine_cargo.contains("name = \"engine\""),
        "Package name should be updated. Content:\n{}",
        engine_cargo
    );

    // 3. Path dependencies updated in utils
    let utils_cargo = workspace.read_file("crates/utils/Cargo.toml");
    assert!(
        utils_cargo.contains("engine = { path = \"../engine\" }"),
        "Utils dependency should be updated. Content:\n{}",
        utils_cargo
    );
    assert!(
        !utils_cargo.contains("core = { path"),
        "Old dependency should be removed. Content:\n{}",
        utils_cargo
    );

    // 4. Use statements updated in utils
    let utils_lib = workspace.read_file("crates/utils/src/lib.rs");
    assert!(
        utils_lib.contains("use engine::Engine;"),
        "Use statement in utils should be updated. Content:\n{}",
        utils_lib
    );
    assert!(
        !utils_lib.contains("use core::Engine;"),
        "Old use statement should be removed. Content:\n{}",
        utils_lib
    );

    // 5. Path dependencies updated in app
    let app_cargo = workspace.read_file("crates/app/Cargo.toml");
    assert!(
        app_cargo.contains("engine = { path = \"../engine\" }"),
        "App dependency should be updated. Content:\n{}",
        app_cargo
    );

    // 6. Use statements updated in app
    let app_main = workspace.read_file("crates/app/src/main.rs");
    assert!(
        app_main.contains("use engine::Engine;"),
        "Use statement in app should be updated. Content:\n{}",
        app_main
    );
    assert!(
        !app_main.contains("use core::Engine;"),
        "Old use statement should be removed. Content:\n{}",
        app_main
    );

    println!("✅ Comprehensive workspace rename verified successfully");
}
