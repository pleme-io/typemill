//! Integration tests for Rust file renames requiring `mod` declaration updates (V2 - CONSOLIDATED)
//!
//! Migrated to use closure-based helper pattern for reduced boilerplate.

use crate::test_helpers::*;
use anyhow::Result;

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
        "rename.plan",
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
        "rename.plan",
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
        "rename.plan",
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
        "rename.plan",
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
        "rename.plan",
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
