//! Integration tests for Rust same-crate file and directory moves (IMPORT REWRITING)
//!
//! These tests verify the bug fix where same-crate file moves (e.g.,
//! `common/src/utils.rs` → `common/src/helpers.rs`) were NOT updating imports
//! because the code only checked if crate names differed:
//!
//! **OLD BUGGY CODE:**
//! ```rust
//! if old_name != new_name {  // Only triggered for cross-crate moves
//!     // Rewrite imports...
//! }
//! ```
//!
//! **NEW FIXED CODE:**
//! ```rust
//! let old_module_path = compute_module_path_from_file(_old_path, old_name, &canonical_project);
//! let new_module_path = compute_module_path_from_file(_new_path, new_name, &canonical_project);
//! if old_module_path != new_module_path {  // Triggers for same-crate moves too!
//!     // Rewrite imports...
//! }
//! ```
//!
//! **SCOPE**: These tests ONLY verify use statement updates (import rewriting).
//! Mod declaration updates (e.g., `pub mod utils;` → `pub mod helpers;`) are a
//! separate feature that's still TODO and tested in test_rust_mod_declarations.rs.
//!
//! Tests cover:
//! - Same-crate file move: use statement updates across files
//! - Same-crate directory move: use statement updates for nested modules
//! - Multiple importers: all files using the moved module get updated

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

// =============================================================================
// Test 1: Same-Crate File Move
// =============================================================================

#[tokio::test]
async fn test_same_crate_file_move_updates_use_statements() {
    println!("\n🧪 Test: Same-crate file move updates use statements");

    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Setup: Create a Rust workspace with a single crate
    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = ["common"]
resolver = "2"
"#,
    );

    // Create the common crate
    workspace.create_directory("common");
    workspace.create_directory("common/src");

    workspace.create_file(
        "common/Cargo.toml",
        r#"[package]
name = "common"
version = "0.1.0"
edition = "2021"
"#,
    );

    // Create lib.rs (WITHOUT mod declaration to avoid that test)
    workspace.create_file("common/src/lib.rs", "// Library root\n");

    // Create utils.rs with a function
    workspace.create_file(
        "common/src/utils.rs",
        "pub fn calculate(x: i32) -> i32 {\n    x * 2\n}\n",
    );

    // Create processor.rs that IMPORTS from utils
    workspace.create_file(
        "common/src/processor.rs",
        "use common::utils::calculate;\n\npub fn process(x: i32) -> i32 {\n    calculate(x)\n}\n",
    );

    // Create another file that also imports from utils
    workspace.create_file(
        "common/src/main.rs",
        "use common::utils::calculate;\n\nfn main() {\n    println!(\"{}\", calculate(21));\n}\n",
    );

    // Action: Rename utils.rs → helpers.rs
    let old_path = workspace.absolute_path("common/src/utils.rs");
    let new_path = workspace.absolute_path("common/src/helpers.rs");

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

    // Verify: File renamed
    assert!(
        !workspace.file_exists("common/src/utils.rs"),
        "Old file should be deleted"
    );
    assert!(
        workspace.file_exists("common/src/helpers.rs"),
        "New file should exist"
    );

    // Verify: processor.rs use statement updated
    let processor_content = workspace.read_file("common/src/processor.rs");
    assert!(
        processor_content.contains("use common::helpers::calculate;"),
        "processor.rs should contain 'use common::helpers::calculate;'\nActual content:\n{}",
        processor_content
    );
    assert!(
        !processor_content.contains("use common::utils::calculate;"),
        "processor.rs should NOT contain old 'use common::utils::calculate;'\nActual content:\n{}",
        processor_content
    );

    // Verify: main.rs use statement also updated
    let main_content = workspace.read_file("common/src/main.rs");
    assert!(
        main_content.contains("use common::helpers::calculate;"),
        "main.rs should contain 'use common::helpers::calculate;'\nActual content:\n{}",
        main_content
    );
    assert!(
        !main_content.contains("use common::utils::calculate;"),
        "main.rs should NOT contain old 'use common::utils::calculate;'\nActual content:\n{}",
        main_content
    );

    println!("✅ Verified: Same-crate file move updated use statements in all importers");
    println!("✅ Test passed");
}

// =============================================================================
// Test 2: Same-Crate Directory Move (nested module)
// =============================================================================

#[tokio::test]
async fn test_same_crate_directory_move_updates_use_statements() {
    println!("\n🧪 Test: Same-crate directory move updates nested module use statements");

    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Setup: Create a Rust workspace with a single crate
    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = ["common"]
resolver = "2"
"#,
    );

    // Create the common crate
    workspace.create_directory("common");
    workspace.create_directory("common/src");

    workspace.create_file(
        "common/Cargo.toml",
        r#"[package]
name = "common"
version = "0.1.0"
edition = "2021"
"#,
    );

    // Create lib.rs (WITHOUT mod declaration)
    workspace.create_file("common/src/lib.rs", "// Library root\n");

    // Create old_dir module directory with a helper function
    workspace.create_directory("common/src/old_dir");
    workspace.create_file(
        "common/src/old_dir/mod.rs",
        "pub fn helper() -> &'static str {\n    \"helper function\"\n}\n",
    );

    // Create files that IMPORT from old_dir
    workspace.create_file(
        "common/src/main.rs",
        "use common::old_dir::helper;\n\nfn main() {\n    println!(\"{}\", helper());\n}\n",
    );

    workspace.create_file(
        "common/src/processor.rs",
        "use common::old_dir::helper;\n\npub fn process() -> &'static str {\n    helper()\n}\n",
    );

    // Action: Rename old_dir → new_dir
    let old_path = workspace.absolute_path("common/src/old_dir");
    let new_path = workspace.absolute_path("common/src/new_dir");

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
                    "dry_run": false
                }
            }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    // Verify: Directory renamed
    assert!(
        !workspace.file_exists("common/src/old_dir"),
        "Old directory should be deleted"
    );
    assert!(
        workspace.file_exists("common/src/new_dir"),
        "New directory should exist"
    );
    assert!(
        workspace.file_exists("common/src/new_dir/mod.rs"),
        "Files should be preserved in new directory"
    );

    // Verify: main.rs use statement updated
    let main_content = workspace.read_file("common/src/main.rs");
    assert!(
        main_content.contains("use common::new_dir::helper;"),
        "main.rs should contain 'use common::new_dir::helper;'\nActual content:\n{}",
        main_content
    );
    assert!(
        !main_content.contains("use common::old_dir::helper;"),
        "main.rs should NOT contain old 'use common::old_dir::helper;'\nActual content:\n{}",
        main_content
    );

    // Verify: processor.rs use statement updated
    let processor_content = workspace.read_file("common/src/processor.rs");
    assert!(
        processor_content.contains("use common::new_dir::helper;"),
        "processor.rs should contain 'use common::new_dir::helper;'\nActual content:\n{}",
        processor_content
    );
    assert!(
        !processor_content.contains("use common::old_dir::helper;"),
        "processor.rs should NOT contain old 'use common::old_dir::helper;'\nActual content:\n{}",
        processor_content
    );

    println!("✅ Verified: Same-crate directory move updated use statements in all importers");
    println!("✅ Test passed");
}

// =============================================================================
// Test 3: Same-Crate Nested File Move with Multiple Importers
// =============================================================================

#[tokio::test]
async fn test_same_crate_nested_file_move_multiple_importers() {
    println!("\n🧪 Test: Same-crate nested file move with multiple importers");

    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Setup: Create a more complex workspace structure
    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = ["mylib"]
resolver = "2"
"#,
    );

    workspace.create_directory("mylib");
    workspace.create_directory("mylib/src");
    workspace.create_directory("mylib/src/core");

    workspace.create_file(
        "mylib/Cargo.toml",
        r#"[package]
name = "mylib"
version = "0.1.0"
edition = "2021"
"#,
    );

    // Create lib.rs that IMPORTS from core::types (WITHOUT mod declarations)
    workspace.create_file(
        "mylib/src/lib.rs",
        "use crate::core::types::Entity;\n\npub fn lib_fn() -> Entity {\n    Entity::new()\n}\n",
    );

    // Create core module directory
    workspace.create_directory("mylib/src/core");
    workspace.create_file("mylib/src/core/mod.rs", "// Core module\n");

    // Create types.rs with a struct
    workspace.create_file(
        "mylib/src/core/types.rs",
        "pub struct Entity {\n    id: u32,\n}\n\nimpl Entity {\n    pub fn new() -> Self {\n        Self { id: 0 }\n    }\n}\n",
    );

    // Create services.rs that uses core::types
    workspace.create_file(
        "mylib/src/services.rs",
        "use crate::core::types::Entity;\n\npub fn create_entity() -> Entity {\n    Entity::new()\n}\n",
    );

    // Create another file that also imports
    workspace.create_file(
        "mylib/src/main.rs",
        "use mylib::core::types::Entity;\n\nfn main() {\n    let _ = Entity::new();\n}\n",
    );

    // Action: Rename core/types.rs → core/models.rs
    let old_path = workspace.absolute_path("mylib/src/core/types.rs");
    let new_path = workspace.absolute_path("mylib/src/core/models.rs");

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

    // Verify: File renamed
    assert!(
        !workspace.file_exists("mylib/src/core/types.rs"),
        "Old file should be deleted"
    );
    assert!(
        workspace.file_exists("mylib/src/core/models.rs"),
        "New file should exist"
    );

    // Verify: lib.rs use statement updated
    let lib_content = workspace.read_file("mylib/src/lib.rs");
    assert!(
        lib_content.contains("use crate::core::models::Entity;"),
        "lib.rs should contain 'use crate::core::models::Entity;'\nActual content:\n{}",
        lib_content
    );
    assert!(
        !lib_content.contains("use crate::core::types::Entity;"),
        "lib.rs should NOT contain old 'use crate::core::types::Entity;'\nActual content:\n{}",
        lib_content
    );

    // Verify: services.rs use statement updated
    let services_content = workspace.read_file("mylib/src/services.rs");
    assert!(
        services_content.contains("use crate::core::models::Entity;"),
        "services.rs should contain 'use crate::core::models::Entity;'\nActual content:\n{}",
        services_content
    );
    assert!(
        !services_content.contains("use crate::core::types::Entity;"),
        "services.rs should NOT contain old 'use crate::core::types::Entity;'\nActual content:\n{}",
        services_content
    );

    // Verify: main.rs use statement updated (absolute path format)
    let main_content = workspace.read_file("mylib/src/main.rs");
    assert!(
        main_content.contains("use mylib::core::models::Entity;"),
        "main.rs should contain 'use mylib::core::models::Entity;'\nActual content:\n{}",
        main_content
    );
    assert!(
        !main_content.contains("use mylib::core::types::Entity;"),
        "main.rs should NOT contain old 'use mylib::core::types::Entity;'\nActual content:\n{}",
        main_content
    );

    println!("✅ Verified: Same-crate nested file move updated all importers (3 files)");
    println!("✅ Test passed");
}
