//! Rust-specific refactoring integration tests (CONSOLIDATED VERSION)
//!
//! BEFORE: 3 files with 881 total lines and significant overlap
//! AFTER: Single organized file with clear sections (~500 lines)
//!
//! This file consolidates all Rust-specific rename/move tests:
//! - test_rust_directory_rename.rs (417 lines) - Workspace & Cargo.toml updates
//! - test_rust_mod_declarations.rs (215 lines) - Module declaration updates
//! - test_rust_same_crate_moves.rs (249 lines) - Use statement updates
//!
//! Organization:
//! - Section 1: Workspace-Level Changes (5 tests)
//! - Section 2: Module Declarations (5 tests)
//! - Section 3: Same-Crate Moves (3 tests)

use crate::test_helpers::*;
use anyhow::Result;

// ============================================================================
// Section 1: Workspace-Level Changes
// ============================================================================
// Tests for renaming crates/packages that require:
// - Workspace member list updates in root Cargo.toml
// - Package name updates in crate's Cargo.toml
// - Path dependency updates across workspace
// - Cross-crate use statement updates

#[tokio::test]
async fn test_rust_workspace_member_rename() -> Result<()> {
    run_tool_test(
        &[
            (
                "Cargo.toml",
                r#"[workspace]
members = ["crates/my-crate"]
resolver = "2"
"#,
            ),
            (
                "crates/my-crate/Cargo.toml",
                r#"[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
            ),
            (
                "crates/my-crate/src/lib.rs",
                "pub fn hello() -> String {\n    String::from(\"Hello\")\n}\n",
            ),
        ],
        "rename_all",
        |ws| {
            build_rename_params(
                ws,
                "crates/my-crate",
                "crates/my-renamed-crate",
                "directory",
            )
        },
        |ws| {
            // Directory renamed
            assert!(
                !ws.file_exists("crates/my-crate"),
                "Old directory should be deleted"
            );
            assert!(
                ws.file_exists("crates/my-renamed-crate"),
                "New directory should exist"
            );
            assert!(
                ws.file_exists("crates/my-renamed-crate/src/lib.rs"),
                "Files should be preserved"
            );

            // Root Cargo.toml updated
            let root_cargo = ws.read_file("Cargo.toml");
            assert!(
                root_cargo.contains("crates/my-renamed-crate"),
                "Root Cargo.toml should contain new member\nActual:\n{}",
                root_cargo
            );
            assert!(
                !root_cargo.contains("crates/my-crate\"")
                    && !root_cargo.contains("crates/my-crate]"),
                "Root Cargo.toml should not contain old member\nActual:\n{}",
                root_cargo
            );

            // Package Cargo.toml updated
            let package_cargo = ws.read_file("crates/my-renamed-crate/Cargo.toml");
            assert!(
                package_cargo.contains("name = \"my-renamed-crate\""),
                "Package name should be updated\nActual:\n{}",
                package_cargo
            );
            assert!(
                !package_cargo.contains("name = \"my-crate\""),
                "Old package name should be removed\nActual:\n{}",
                package_cargo
            );
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_rust_path_dependency_updates() -> Result<()> {
    run_tool_test(
        &[
            (
                "Cargo.toml",
                r#"[workspace]
members = ["crates/common", "crates/app"]
resolver = "2"
"#,
            ),
            (
                "crates/common/Cargo.toml",
                r#"[package]
name = "common"
version = "0.1.0"
edition = "2021"
"#,
            ),
            (
                "crates/common/src/lib.rs",
                "pub fn utility() -> i32 { 42 }\n",
            ),
            (
                "crates/app/Cargo.toml",
                r#"[package]
name = "app"
version = "0.1.0"
edition = "2021"

[dependencies]
common = { path = "../common" }
"#,
            ),
            (
                "crates/app/src/main.rs",
                "use common::utility;\n\nfn main() {\n    println!(\"{}\", utility());\n}\n",
            ),
        ],
        "rename_all",
        |ws| build_rename_params(ws, "crates/common", "crates/shared", "directory"),
        |ws| {
            let app_cargo = ws.read_file("crates/app/Cargo.toml");
            assert!(
                app_cargo.contains("path = \"../shared\""),
                "Path dependency should be updated\nActual:\n{}",
                app_cargo
            );
            assert!(
                !app_cargo.contains("path = \"../common\""),
                "Old path should be removed\nActual:\n{}",
                app_cargo
            );
            assert!(
                app_cargo.contains("shared = { path = \"../shared\" }"),
                "Dependency name should be updated\nActual:\n{}",
                app_cargo
            );
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_rust_cross_crate_use_statements_update() -> Result<()> {
    run_tool_test(
        &[
            (
                "Cargo.toml",
                r#"[workspace]
members = ["crates/common", "crates/app"]
resolver = "2"
"#,
            ),
            (
                "crates/common/Cargo.toml",
                r#"[package]
name = "common"
version = "0.1.0"
edition = "2021"
"#,
            ),
            (
                "crates/common/src/lib.rs",
                "pub fn util() -> &'static str {\n    \"utility\"\n}\n",
            ),
            (
                "crates/app/Cargo.toml",
                r#"[package]
name = "app"
version = "0.1.0"
edition = "2021"

[dependencies]
common = { path = "../common" }
"#,
            ),
            (
                "crates/app/src/main.rs",
                "use common::util;\n\nfn main() {\n    println!(\"{}\", util());\n}\n",
            ),
        ],
        "rename_all",
        |ws| build_rename_params(ws, "crates/common", "crates/shared", "directory"),
        |ws| {
            let main_content = ws.read_file("crates/app/src/main.rs");
            assert!(
                main_content.contains("use shared::util;"),
                "Use statement should be updated\nActual:\n{}",
                main_content
            );
            assert!(
                !main_content.contains("use common::util;"),
                "Old use statement should be removed\nActual:\n{}",
                main_content
            );

            let app_cargo = ws.read_file("crates/app/Cargo.toml");
            assert!(
                app_cargo.contains("shared = { path = \"../shared\" }"),
                "Cargo.toml dependency should be updated\nActual:\n{}",
                app_cargo
            );
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_rust_nested_module_directory_rename() -> Result<()> {
    run_tool_test(
        &[
            (
                "Cargo.toml",
                r#"[package]
name = "myproject"
version = "0.1.0"
edition = "2021"
"#,
            ),
            ("src/utils/mod.rs", "pub mod strings;\npub mod numbers;\n"),
            (
                "src/utils/strings.rs",
                "pub fn uppercase(s: &str) -> String {\n    s.to_uppercase()\n}\n",
            ),
            (
                "src/utils/numbers.rs",
                "pub fn double(n: i32) -> i32 {\n    n * 2\n}\n",
            ),
            (
                "src/main.rs",
                "mod utils;\n\nuse utils::strings::uppercase;\nuse utils::numbers::double;\n\nfn main() {\n    println!(\"{}\", uppercase(\"hello\"));\n    println!(\"{}\", double(21));\n}\n",
            ),
        ],
        "rename_all",
        |ws| build_rename_params(ws, "src/utils", "src/helpers", "directory"),
        |ws| {
            assert!(
                !ws.file_exists("src/utils"),
                "Old directory should be deleted"
            );
            assert!(ws.file_exists("src/helpers"), "New directory should exist");

            let main_content = ws.read_file("src/main.rs");
            assert!(
                main_content.contains("mod helpers;"),
                "Module declaration should be updated\nActual:\n{}",
                main_content
            );
            assert!(
                main_content.contains("use helpers::strings::uppercase;"),
                "Use statement should be updated\nActual:\n{}",
                main_content
            );
            assert!(
                main_content.contains("use helpers::numbers::double;"),
                "Use statement should be updated\nActual:\n{}",
                main_content
            );
            assert!(
                !main_content.contains("use utils::"),
                "Old use statements should be removed\nActual:\n{}",
                main_content
            );

            let mod_content = ws.read_file("src/helpers/mod.rs");
            assert!(
                mod_content.contains("pub mod strings;"),
                "Internal module declarations preserved\nActual:\n{}",
                mod_content
            );
            assert!(
                mod_content.contains("pub mod numbers;"),
                "Internal module declarations preserved\nActual:\n{}",
                mod_content
            );
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_rust_comprehensive_workspace_rename() -> Result<()> {
    run_tool_test(
        &[
            (
                "Cargo.toml",
                r#"[workspace]
members = [
    "crates/core",
    "crates/utils",
    "crates/app"
]
resolver = "2"
"#,
            ),
            (
                "crates/core/Cargo.toml",
                r#"[package]
name = "core"
version = "0.1.0"
edition = "2021"
"#,
            ),
            (
                "crates/core/src/lib.rs",
                "pub struct Engine {\n    pub power: u32,\n}\n\nimpl Engine {\n    pub fn new(power: u32) -> Self {\n        Self { power }\n    }\n}\n",
            ),
            (
                "crates/utils/Cargo.toml",
                r#"[package]
name = "utils"
version = "0.1.0"
edition = "2021"

[dependencies]
core = { path = "../core" }
"#,
            ),
            (
                "crates/utils/src/lib.rs",
                "use core::Engine;\n\npub fn create_engine() -> Engine {\n    Engine::new(100)\n}\n",
            ),
            (
                "crates/app/Cargo.toml",
                r#"[package]
name = "app"
version = "0.1.0"
edition = "2021"

[dependencies]
core = { path = "../core" }
utils = { path = "../utils" }
"#,
            ),
            (
                "crates/app/src/main.rs",
                "use core::Engine;\nuse utils::create_engine;\n\nfn main() {\n    let engine = create_engine();\n    println!(\"Engine power: {}\", engine.power);\n}\n",
            ),
        ],
        "rename_all",
        |ws| build_rename_params(ws, "crates/core", "crates/engine", "directory"),
        |ws| {
            // Workspace members updated
            let root_cargo = ws.read_file("Cargo.toml");
            assert!(
                root_cargo.contains("\"crates/engine\""),
                "Workspace should contain new path\nActual:\n{}",
                root_cargo
            );
            assert!(
                !root_cargo.contains("\"crates/core\""),
                "Workspace should not contain old path\nActual:\n{}",
                root_cargo
            );

            // Package name updated
            let engine_cargo = ws.read_file("crates/engine/Cargo.toml");
            assert!(
                engine_cargo.contains("name = \"engine\""),
                "Package name should be updated\nActual:\n{}",
                engine_cargo
            );

            // Path dependencies in utils
            let utils_cargo = ws.read_file("crates/utils/Cargo.toml");
            assert!(
                utils_cargo.contains("engine = { path = \"../engine\" }"),
                "Utils dependency should be updated\nActual:\n{}",
                utils_cargo
            );
            assert!(
                !utils_cargo.contains("core = { path"),
                "Old dependency should be removed\nActual:\n{}",
                utils_cargo
            );

            // Use statements in utils
            let utils_lib = ws.read_file("crates/utils/src/lib.rs");
            assert!(
                utils_lib.contains("use engine::Engine;"),
                "Use statement in utils should be updated\nActual:\n{}",
                utils_lib
            );
            assert!(
                !utils_lib.contains("use core::Engine;"),
                "Old use statement should be removed\nActual:\n{}",
                utils_lib
            );

            // Path dependencies in app
            let app_cargo = ws.read_file("crates/app/Cargo.toml");
            assert!(
                app_cargo.contains("engine = { path = \"../engine\" }"),
                "App dependency should be updated\nActual:\n{}",
                app_cargo
            );

            // Use statements in app
            let app_main = ws.read_file("crates/app/src/main.rs");
            assert!(
                app_main.contains("use engine::Engine;"),
                "Use statement in app should be updated\nActual:\n{}",
                app_main
            );
            assert!(
                !app_main.contains("use core::Engine;"),
                "Old use statement should be removed\nActual:\n{}",
                app_main
            );
            Ok(())
        },
    )
    .await
}

// ============================================================================
// Section 2: Module Declarations
// ============================================================================
// Tests for file renames that require module declaration updates in parent
// files (lib.rs or mod.rs). Tests verify both mod declarations and use
// statement updates within the same file.

#[tokio::test]
async fn test_rust_rename_updates_mod_in_parent_mod_rs() -> Result<()> {
    run_tool_test(
        &[
            (
                "src/mod.rs",
                "pub mod utils;\n\npub fn main_fn() {\n    utils::helper();\n}\n",
            ),
            ("src/utils.rs", "pub fn helper() {}\n"),
        ],
        "rename_all",
        |ws| build_rename_params(ws, "src/utils.rs", "src/helpers.rs", "file"),
        |ws| {
            let mod_content = ws.read_file("src/mod.rs");
            assert!(
                mod_content.contains("pub mod helpers;"),
                "src/mod.rs should contain 'pub mod helpers;'\nActual:\n{}",
                mod_content
            );
            assert!(
                !mod_content.contains("pub mod utils;"),
                "src/mod.rs should NOT contain 'pub mod utils;'\nActual:\n{}",
                mod_content
            );
            assert!(
                mod_content.contains("helpers::helper()"),
                "Use statement should be updated\nActual:\n{}",
                mod_content
            );
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_rust_rename_updates_mod_in_lib_rs() -> Result<()> {
    run_tool_test(
        &[
            (
                "src/lib.rs",
                "pub mod utils;\n\npub fn lib_fn() {\n    utils::helper();\n}\n",
            ),
            ("src/utils.rs", "pub fn helper() {}\n"),
        ],
        "rename_all",
        |ws| build_rename_params(ws, "src/utils.rs", "src/helpers.rs", "file"),
        |ws| {
            let lib_content = ws.read_file("src/lib.rs");
            assert!(
                lib_content.contains("pub mod helpers;"),
                "src/lib.rs should contain 'pub mod helpers;'\nActual:\n{}",
                lib_content
            );
            assert!(
                !lib_content.contains("pub mod utils;"),
                "src/lib.rs should NOT contain old 'pub mod utils;'\nActual:\n{}",
                lib_content
            );
            assert!(
                lib_content.contains("helpers::helper()"),
                "Use statement should be updated\nActual:\n{}",
                lib_content
            );
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_rust_rename_updates_sibling_mod_rs() -> Result<()> {
    run_tool_test(
        &[
            (
                "src/utils/mod.rs",
                "pub mod helpers;\n\npub fn utils_fn() {\n    helpers::do_work();\n}\n",
            ),
            ("src/utils/helpers.rs", "pub fn do_work() {}\n"),
        ],
        "rename_all",
        |ws| build_rename_params(ws, "src/utils/helpers.rs", "src/utils/support.rs", "file"),
        |ws| {
            let mod_content = ws.read_file("src/utils/mod.rs");
            assert!(
                mod_content.contains("pub mod support;"),
                "src/utils/mod.rs should contain 'pub mod support;'\nActual:\n{}",
                mod_content
            );
            assert!(
                !mod_content.contains("pub mod helpers;"),
                "src/utils/mod.rs should NOT contain old 'pub mod helpers;'\nActual:\n{}",
                mod_content
            );
            assert!(
                mod_content.contains("support::do_work()"),
                "Use statement should be updated\nActual:\n{}",
                mod_content
            );
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_rust_rename_nested_mod_tree() -> Result<()> {
    run_tool_test(
        &[
            (
                "Cargo.toml",
                r#"[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#,
            ),
            (
                "src/lib.rs",
                "pub mod utils;\n\nuse utils::helpers::process;\n\npub fn lib_fn() {\n    process();\n}\n",
            ),
            (
                "src/utils/mod.rs",
                "pub mod helpers;\n\npub fn utils_fn() {\n    helpers::process();\n}\n",
            ),
            ("src/utils/helpers.rs", "pub fn process() {}\n"),
        ],
        "rename_all",
        |ws| build_rename_params(ws, "src/utils/helpers.rs", "src/utils/support.rs", "file"),
        |ws| {
            let utils_mod = ws.read_file("src/utils/mod.rs");
            assert!(
                utils_mod.contains("pub mod support;"),
                "src/utils/mod.rs should contain 'pub mod support;'\nActual:\n{}",
                utils_mod
            );
            assert!(
                !utils_mod.contains("pub mod helpers;"),
                "src/utils/mod.rs should NOT contain old declaration\nActual:\n{}",
                utils_mod
            );
            assert!(
                utils_mod.contains("support::process()"),
                "Local use should be updated\nActual:\n{}",
                utils_mod
            );

            let lib_content = ws.read_file("src/lib.rs");
            assert!(
                lib_content.contains("use utils::support::process;"),
                "src/lib.rs should contain updated use\nActual:\n{}",
                lib_content
            );
            assert!(
                !lib_content.contains("use utils::helpers::process;"),
                "src/lib.rs should NOT contain old use\nActual:\n{}",
                lib_content
            );
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_rust_rename_affects_both_mod_and_use() -> Result<()> {
    run_tool_test(
        &[
            (
                "src/lib.rs",
                "pub mod utils;\n\nuse utils::helper;\n\npub fn lib_fn() {\n    helper();\n}\n",
            ),
            ("src/utils.rs", "pub fn helper() {}\n"),
        ],
        "rename_all",
        |ws| build_rename_params(ws, "src/utils.rs", "src/helpers.rs", "file"),
        |ws| {
            let lib_content = ws.read_file("src/lib.rs");

            // Check mod declaration
            assert!(
                lib_content.contains("pub mod helpers;"),
                "Should contain 'pub mod helpers;'\nActual:\n{}",
                lib_content
            );
            assert!(
                !lib_content.contains("pub mod utils;"),
                "Should NOT contain old 'pub mod utils;'\nActual:\n{}",
                lib_content
            );

            // Check use statement
            assert!(
                lib_content.contains("use helpers::helper;"),
                "Should contain 'use helpers::helper;'\nActual:\n{}",
                lib_content
            );
            assert!(
                !lib_content.contains("use utils::helper;"),
                "Should NOT contain old 'use utils::helper;'\nActual:\n{}",
                lib_content
            );
            Ok(())
        },
    )
    .await
}

// ============================================================================
// Section 3: Same-Crate Moves
// ============================================================================
// Tests for moving files within the same crate. Verifies that use statements
// within the crate are updated when files are moved to different locations.

#[tokio::test]
async fn test_same_crate_file_move_updates_use_statements() -> Result<()> {
    run_tool_test(
        &[
            (
                "Cargo.toml",
                r#"[workspace]
members = ["common"]
resolver = "2"
"#,
            ),
            (
                "common/Cargo.toml",
                r#"[package]
name = "common"
version = "0.1.0"
edition = "2021"
"#,
            ),
            ("common/src/lib.rs", "// Library root\n"),
            (
                "common/src/utils.rs",
                "pub fn calculate(x: i32) -> i32 {\n    x * 2\n}\n",
            ),
            (
                "common/src/processor.rs",
                "use common::utils::calculate;\n\npub fn process(x: i32) -> i32 {\n    calculate(x)\n}\n",
            ),
            (
                "common/src/main.rs",
                "use common::utils::calculate;\n\nfn main() {\n    println!(\"{}\", calculate(21));\n}\n",
            ),
        ],
        "rename_all",
        |ws| build_rename_params(ws, "common/src/utils.rs", "common/src/helpers.rs", "file"),
        |ws| {
            assert!(
                !ws.file_exists("common/src/utils.rs"),
                "Old file should be deleted"
            );
            assert!(
                ws.file_exists("common/src/helpers.rs"),
                "New file should exist"
            );

            let processor = ws.read_file("common/src/processor.rs");
            assert!(
                processor.contains("use common::helpers::calculate;"),
                "processor.rs should be updated\nActual:\n{}",
                processor
            );
            assert!(
                !processor.contains("use common::utils::calculate;"),
                "processor.rs should not contain old import\nActual:\n{}",
                processor
            );

            let main_content = ws.read_file("common/src/main.rs");
            assert!(
                main_content.contains("use common::helpers::calculate;"),
                "main.rs should be updated\nActual:\n{}",
                main_content
            );
            assert!(
                !main_content.contains("use common::utils::calculate;"),
                "main.rs should not contain old import\nActual:\n{}",
                main_content
            );
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_same_crate_directory_move_updates_use_statements() -> Result<()> {
    run_tool_test(
        &[
            (
                "Cargo.toml",
                r#"[workspace]
members = ["common"]
resolver = "2"
"#,
            ),
            (
                "common/Cargo.toml",
                r#"[package]
name = "common"
version = "0.1.0"
edition = "2021"
"#,
            ),
            ("common/src/lib.rs", "// Library root\n"),
            (
                "common/src/old_dir/mod.rs",
                "pub fn helper() -> &'static str {\n    \"helper function\"\n}\n",
            ),
            (
                "common/src/main.rs",
                "use common::old_dir::helper;\n\nfn main() {\n    println!(\"{}\", helper());\n}\n",
            ),
            (
                "common/src/processor.rs",
                "use common::old_dir::helper;\n\npub fn process() -> &'static str {\n    helper()\n}\n",
            ),
        ],
        "rename_all",
        |ws| build_rename_params(ws, "common/src/old_dir", "common/src/new_dir", "directory"),
        |ws| {
            assert!(
                !ws.file_exists("common/src/old_dir"),
                "Old directory should be deleted"
            );
            assert!(
                ws.file_exists("common/src/new_dir"),
                "New directory should exist"
            );
            assert!(
                ws.file_exists("common/src/new_dir/mod.rs"),
                "Files should be preserved"
            );

            let main_content = ws.read_file("common/src/main.rs");
            assert!(
                main_content.contains("use common::new_dir::helper;"),
                "main.rs should be updated\nActual:\n{}",
                main_content
            );
            assert!(
                !main_content.contains("use common::old_dir::helper;"),
                "main.rs should not contain old import\nActual:\n{}",
                main_content
            );

            let processor = ws.read_file("common/src/processor.rs");
            assert!(
                processor.contains("use common::new_dir::helper;"),
                "processor.rs should be updated\nActual:\n{}",
                processor
            );
            assert!(
                !processor.contains("use common::old_dir::helper;"),
                "processor.rs should not contain old import\nActual:\n{}",
                processor
            );
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_same_crate_nested_file_move_multiple_importers() -> Result<()> {
    run_tool_test(
        &[
            (
                "Cargo.toml",
                r#"[workspace]
members = ["mylib"]
resolver = "2"
"#,
            ),
            (
                "mylib/Cargo.toml",
                r#"[package]
name = "mylib"
version = "0.1.0"
edition = "2021"
"#,
            ),
            (
                "mylib/src/lib.rs",
                "use crate::core::types::Entity;\n\npub fn lib_fn() -> Entity {\n    Entity::new()\n}\n",
            ),
            ("mylib/src/core/mod.rs", "// Core module\n"),
            (
                "mylib/src/core/types.rs",
                "pub struct Entity {\n    id: u32,\n}\n\nimpl Entity {\n    pub fn new() -> Self {\n        Self { id: 0 }\n    }\n}\n",
            ),
            (
                "mylib/src/services.rs",
                "use crate::core::types::Entity;\n\npub fn create_entity() -> Entity {\n    Entity::new()\n}\n",
            ),
            (
                "mylib/src/main.rs",
                "use mylib::core::types::Entity;\n\nfn main() {\n    let _ = Entity::new();\n}\n",
            ),
        ],
        "rename_all",
        |ws| build_rename_params(ws, "mylib/src/core/types.rs", "mylib/src/core/models.rs", "file"),
        |ws| {
            assert!(
                !ws.file_exists("mylib/src/core/types.rs"),
                "Old file should be deleted"
            );
            assert!(
                ws.file_exists("mylib/src/core/models.rs"),
                "New file should exist"
            );

            let lib_content = ws.read_file("mylib/src/lib.rs");
            assert!(
                lib_content.contains("use crate::core::models::Entity;"),
                "lib.rs should be updated\nActual:\n{}",
                lib_content
            );
            assert!(
                !lib_content.contains("use crate::core::types::Entity;"),
                "lib.rs should not contain old import\nActual:\n{}",
                lib_content
            );

            let services = ws.read_file("mylib/src/services.rs");
            assert!(
                services.contains("use crate::core::models::Entity;"),
                "services.rs should be updated\nActual:\n{}",
                services
            );
            assert!(
                !services.contains("use crate::core::types::Entity;"),
                "services.rs should not contain old import\nActual:\n{}",
                services
            );

            let main_content = ws.read_file("mylib/src/main.rs");
            assert!(
                main_content.contains("use mylib::core::models::Entity;"),
                "main.rs should be updated\nActual:\n{}",
                main_content
            );
            assert!(
                !main_content.contains("use mylib::core::types::Entity;"),
                "main.rs should not contain old import\nActual:\n{}",
                main_content
            );
            Ok(())
        },
    )
    .await
}
