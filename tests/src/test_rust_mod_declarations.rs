//! Integration tests for Rust file renames requiring `mod` declaration updates
//!
//! These tests verify that when a Rust file is renamed, all `mod` declarations
//! pointing to it are automatically updated with the new name. This is critical
//! for maintaining module system correctness in Rust codebases.
//!
//! Tests cover:
//! - Updating `mod` declarations in parent `mod.rs` files
//! - Updating `mod` declarations in `lib.rs` files
//! - Updating sibling module declarations
//! - Handling nested module trees with multiple levels
//! - Updating both `mod` and `use` statements together
//!
//! Pattern:
//! 1. Create Rust module structure with mod declarations
//! 2. Call rename.plan to generate refactoring plan
//! 3. Call workspace.apply_edit to execute plan
//! 4. Verify both old declarations are removed AND new ones exist

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

#[tokio::test]
async fn test_rust_rename_updates_mod_in_parent_mod_rs() {
    println!("\n🧪 Test: Rename updates mod declaration in parent mod.rs");

    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Setup: Create src/mod.rs with mod declaration and src/utils.rs
    workspace.create_directory("src");
    workspace.create_file(
        "src/mod.rs",
        "pub mod utils;\n\npub fn main_fn() {\n    utils::helper();\n}\n",
    );
    workspace.create_file("src/utils.rs", "pub fn helper() {}\n");

    let old_path = workspace.absolute_path("src/utils.rs");
    let new_path = workspace.absolute_path("src/helpers.rs");

    // Action: Rename src/utils.rs -> src/helpers.rs
    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "file",
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
                    "dry_run": false
                }
            }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    // Verify: Check src/mod.rs now contains "mod helpers;" and NOT "mod utils;"
    let mod_content = workspace.read_file("src/mod.rs");

    assert!(
        mod_content.contains("pub mod helpers;"),
        "❌ src/mod.rs should contain 'pub mod helpers;'\nActual content:\n{}",
        mod_content
    );

    assert!(
        !mod_content.contains("pub mod utils;"),
        "❌ src/mod.rs should NOT contain 'pub mod utils;' (old declaration should be removed)\nActual content:\n{}",
        mod_content
    );

    // Verify: use statement also updated
    assert!(
        mod_content.contains("helpers::helper()"),
        "❌ Use statement should be updated to helpers::helper()\nActual content:\n{}",
        mod_content
    );

    println!("✅ Verified: mod declaration updated from 'mod utils;' to 'mod helpers;'");
    println!("✅ Test passed");
}

#[tokio::test]
async fn test_rust_rename_updates_mod_in_lib_rs() {
    println!("\n🧪 Test: Rename updates mod declaration in lib.rs");

    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Setup: Create src/lib.rs with mod declaration and src/utils.rs
    workspace.create_directory("src");
    workspace.create_file(
        "src/lib.rs",
        "pub mod utils;\n\npub fn lib_fn() {\n    utils::helper();\n}\n",
    );
    workspace.create_file("src/utils.rs", "pub fn helper() {}\n");

    let old_path = workspace.absolute_path("src/utils.rs");
    let new_path = workspace.absolute_path("src/helpers.rs");

    // Action: Rename src/utils.rs -> src/helpers.rs
    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "file",
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
                    "dry_run": false
                }
            }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    // Verify: Check src/lib.rs now contains "mod helpers;" and NOT "mod utils;"
    let lib_content = workspace.read_file("src/lib.rs");

    assert!(
        lib_content.contains("pub mod helpers;"),
        "❌ src/lib.rs should contain 'pub mod helpers;'\nActual content:\n{}",
        lib_content
    );

    assert!(
        !lib_content.contains("pub mod utils;"),
        "❌ src/lib.rs should NOT contain 'pub mod utils;' (old declaration should be removed)\nActual content:\n{}",
        lib_content
    );

    // Verify: use statement also updated
    assert!(
        lib_content.contains("helpers::helper()"),
        "❌ Use statement should be updated to helpers::helper()\nActual content:\n{}",
        lib_content
    );

    println!("✅ Verified: mod declaration updated from 'mod utils;' to 'mod helpers;'");
    println!("✅ Test passed");
}

#[tokio::test]
async fn test_rust_rename_updates_sibling_mod_rs() {
    println!("\n🧪 Test: Rename updates mod declaration in sibling mod.rs");

    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Setup: Create src/utils/mod.rs with mod declaration and src/utils/helpers.rs
    workspace.create_directory("src");
    workspace.create_directory("src/utils");
    workspace.create_file(
        "src/utils/mod.rs",
        "pub mod helpers;\n\npub fn utils_fn() {\n    helpers::do_work();\n}\n",
    );
    workspace.create_file("src/utils/helpers.rs", "pub fn do_work() {}\n");

    let old_path = workspace.absolute_path("src/utils/helpers.rs");
    let new_path = workspace.absolute_path("src/utils/support.rs");

    // Action: Rename src/utils/helpers.rs -> src/utils/support.rs
    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "file",
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
                    "dry_run": false
                }
            }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    // Verify: Check src/utils/mod.rs now contains "mod support;" and NOT "mod helpers;"
    let mod_content = workspace.read_file("src/utils/mod.rs");

    assert!(
        mod_content.contains("pub mod support;"),
        "❌ src/utils/mod.rs should contain 'pub mod support;'\nActual content:\n{}",
        mod_content
    );

    assert!(
        !mod_content.contains("pub mod helpers;"),
        "❌ src/utils/mod.rs should NOT contain 'pub mod helpers;' (old declaration should be removed)\nActual content:\n{}",
        mod_content
    );

    // Verify: use statement also updated
    assert!(
        mod_content.contains("support::do_work()"),
        "❌ Use statement should be updated to support::do_work()\nActual content:\n{}",
        mod_content
    );

    println!("✅ Verified: mod declaration updated from 'mod helpers;' to 'mod support;'");
    println!("✅ Test passed");
}

#[tokio::test]
async fn test_rust_rename_nested_mod_tree() {
    println!("\n🧪 Test: Rename updates nested module tree with multiple levels");

    let workspace = TestWorkspace::new();

    // IMPORTANT: Create Cargo.toml FIRST so crate name can be inferred during detection
    workspace.create_cargo_toml("test_project");

    let mut client = TestClient::new(workspace.path());

    // Setup: Multi-level module structure
    workspace.create_directory("src");
    workspace.create_directory("src/utils");

    workspace.create_file(
        "src/lib.rs",
        "pub mod utils;\n\nuse utils::helpers::process;\n\npub fn lib_fn() {\n    process();\n}\n",
    );
    workspace.create_file(
        "src/utils/mod.rs",
        "pub mod helpers;\n\npub fn utils_fn() {\n    helpers::process();\n}\n",
    );
    workspace.create_file("src/utils/helpers.rs", "pub fn process() {}\n");

    let old_path = workspace.absolute_path("src/utils/helpers.rs");
    let new_path = workspace.absolute_path("src/utils/support.rs");

    // Action: Rename src/utils/helpers.rs -> src/utils/support.rs
    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "file",
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
                    "dry_run": false
                }
            }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    // Verify: Check src/utils/mod.rs mod declaration
    let utils_mod_content = workspace.read_file("src/utils/mod.rs");
    assert!(
        utils_mod_content.contains("pub mod support;"),
        "❌ src/utils/mod.rs should contain 'pub mod support;'\nActual content:\n{}",
        utils_mod_content
    );
    assert!(
        !utils_mod_content.contains("pub mod helpers;"),
        "❌ src/utils/mod.rs should NOT contain old 'pub mod helpers;'\nActual content:\n{}",
        utils_mod_content
    );
    assert!(
        utils_mod_content.contains("support::process()"),
        "❌ src/utils/mod.rs should use support::process()\nActual content:\n{}",
        utils_mod_content
    );

    // Verify: Check src/lib.rs use statement
    let lib_content = workspace.read_file("src/lib.rs");
    assert!(
        lib_content.contains("use utils::support::process;"),
        "❌ src/lib.rs should contain 'use utils::support::process;'\nActual content:\n{}",
        lib_content
    );
    assert!(
        !lib_content.contains("use utils::helpers::process;"),
        "❌ src/lib.rs should NOT contain old 'use utils::helpers::process;'\nActual content:\n{}",
        lib_content
    );

    println!("✅ Verified: Both mod declarations and use statements updated in nested structure");
    println!("✅ Test passed");
}

#[tokio::test]
async fn test_rust_rename_affects_both_mod_and_use() {
    println!("\n🧪 Test: Rename updates both mod and use statements in same file");

    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Setup: Create file with both mod declaration and use statement
    workspace.create_directory("src");
    workspace.create_file(
        "src/lib.rs",
        "pub mod utils;\n\nuse utils::helper;\n\npub fn lib_fn() {\n    helper();\n}\n",
    );
    workspace.create_file("src/utils.rs", "pub fn helper() {}\n");

    let old_path = workspace.absolute_path("src/utils.rs");
    let new_path = workspace.absolute_path("src/helpers.rs");

    // Action: Rename src/utils.rs -> src/helpers.rs
    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "file",
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
                    "dry_run": false
                }
            }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    // Verify: Both mod and use statements updated
    let lib_content = workspace.read_file("src/lib.rs");

    // Check mod declaration
    assert!(
        lib_content.contains("pub mod helpers;"),
        "❌ src/lib.rs should contain 'pub mod helpers;'\nActual content:\n{}",
        lib_content
    );
    assert!(
        !lib_content.contains("pub mod utils;"),
        "❌ src/lib.rs should NOT contain old 'pub mod utils;'\nActual content:\n{}",
        lib_content
    );

    // Check use statement
    assert!(
        lib_content.contains("use helpers::helper;"),
        "❌ src/lib.rs should contain 'use helpers::helper;'\nActual content:\n{}",
        lib_content
    );
    assert!(
        !lib_content.contains("use utils::helper;"),
        "❌ src/lib.rs should NOT contain old 'use utils::helper;'\nActual content:\n{}",
        lib_content
    );

    println!("✅ Verified: Both mod declaration and use statement updated correctly");
    println!("✅ Test passed");
}
