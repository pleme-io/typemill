//! Edge case tests for Cargo manifest updates during directory renames (V2 - CONSOLIDATED)
//!
//! Migrated to use closure-based helper pattern for reduced boilerplate.

use crate::test_helpers::*;
use anyhow::Result;

#[tokio::test]
async fn test_rust_dev_and_build_dependencies_update() -> Result<()> {
    run_tool_test(
        &[
            (
                "Cargo.toml",
                r#"[workspace]
members = ["crates/utils", "crates/app"]
resolver = "2"
"#,
            ),
            (
                "crates/utils/Cargo.toml",
                r#"[package]
name = "utils"
version = "0.1.0"
edition = "2021"
"#,
            ),
            (
                "crates/utils/src/lib.rs",
                "pub fn utility() -> i32 { 42 }\n",
            ),
            (
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
            ),
            (
                "crates/app/src/main.rs",
                "use utils::utility;\n\nfn main() {\n    println!(\"{}\", utility());\n}\n",
            ),
        ],
        "rename",
        |ws| build_rename_params(ws, "crates/utils", "crates/helpers", "directory"),
        |ws| {
            let app_cargo = ws.read_file("crates/app/Cargo.toml");

            assert!(
                app_cargo.contains("[dependencies]")
                    && app_cargo.contains("helpers = { path = \"../helpers\" }"),
                "Regular dependencies should be updated\nActual:\n{}",
                app_cargo
            );

            assert!(
                app_cargo.contains("[dev-dependencies]")
                    && app_cargo.contains("helpers = { path = \"../helpers\" }"),
                "dev-dependencies should be updated\nActual:\n{}",
                app_cargo
            );

            assert!(
                app_cargo.contains("[build-dependencies]")
                    && app_cargo.contains("helpers = { path = \"../helpers\" }"),
                "build-dependencies should be updated\nActual:\n{}",
                app_cargo
            );

            assert!(
                !app_cargo.contains("utils = { path"),
                "Old dependency name should be removed\nActual:\n{}",
                app_cargo
            );
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_rust_workspace_dependencies_update() -> Result<()> {
    run_tool_test(
        &[
            (
                "Cargo.toml",
                r#"[workspace]
members = ["crates/utils", "crates/app"]
resolver = "2"

[workspace.dependencies]
utils = { path = "crates/utils" }
"#,
            ),
            (
                "crates/utils/Cargo.toml",
                r#"[package]
name = "utils"
version = "0.1.0"
edition = "2021"
"#,
            ),
            (
                "crates/utils/src/lib.rs",
                "pub fn utility() -> i32 { 42 }\n",
            ),
            (
                "crates/app/Cargo.toml",
                r#"[package]
name = "app"
version = "0.1.0"
edition = "2021"

[dependencies]
utils = { workspace = true }
"#,
            ),
            (
                "crates/app/src/main.rs",
                "use utils::utility;\n\nfn main() {\n    println!(\"{}\", utility());\n}\n",
            ),
        ],
        "rename",
        |ws| build_rename_params(ws, "crates/utils", "crates/helpers", "directory"),
        |ws| {
            let root_cargo = ws.read_file("Cargo.toml");
            assert!(
                root_cargo.contains("[workspace.dependencies]"),
                "Workspace dependencies section should exist"
            );
            assert!(
                root_cargo.contains("helpers = { path = \"crates/helpers\" }"),
                "Workspace dependency should be updated\nActual:\n{}",
                root_cargo
            );
            assert!(
                !root_cargo.contains("utils = { path"),
                "Old workspace dependency should be removed\nActual:\n{}",
                root_cargo
            );

            let app_cargo = ws.read_file("crates/app/Cargo.toml");
            assert!(
                app_cargo.contains("helpers = { workspace = true }"),
                "App should reference new workspace dependency\nActual:\n{}",
                app_cargo
            );
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_rust_inline_table_dependencies() -> Result<()> {
    run_tool_test(
        &[
            (
                "Cargo.toml",
                r#"[workspace]
members = ["crates/utils", "crates/app"]
resolver = "2"
"#,
            ),
            (
                "crates/utils/Cargo.toml",
                r#"[package]
name = "utils"
version = "0.1.0"
edition = "2021"
"#,
            ),
            ("crates/utils/src/lib.rs", "pub fn helper() {}\n"),
            (
                "crates/app/Cargo.toml",
                r#"[package]
name = "app"
version = "0.1.0"
edition = "2021"

[dependencies]
utils = { path = "../utils", version = "0.1.0" }
"#,
            ),
            (
                "crates/app/src/main.rs",
                "use utils::helper;\n\nfn main() {\n    helper();\n}\n",
            ),
        ],
        "rename",
        |ws| build_rename_params(ws, "crates/utils", "crates/core", "directory"),
        |ws| {
            let app_cargo = ws.read_file("crates/app/Cargo.toml");
            assert!(
                app_cargo.contains("core = { path = \"../core\""),
                "Inline table dependency should be updated\nActual:\n{}",
                app_cargo
            );
            assert!(
                !app_cargo.contains("utils = { path"),
                "Old dependency should be removed\nActual:\n{}",
                app_cargo
            );

            let main_content = ws.read_file("crates/app/src/main.rs");
            assert!(
                main_content.contains("use core::helper;"),
                "Use statement should be updated\nActual:\n{}",
                main_content
            );
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_rust_mixed_dependency_formats() -> Result<()> {
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
            ("crates/common/src/lib.rs", "pub fn func() {}\n"),
            (
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
            ),
            (
                "crates/app/src/main.rs",
                "use common::func;\n\nfn main() {\n    func();\n}\n",
            ),
        ],
        "rename",
        |ws| build_rename_params(ws, "crates/common", "crates/shared", "directory"),
        |ws| {
            let app_cargo = ws.read_file("crates/app/Cargo.toml");

            assert!(
                app_cargo.contains("shared = { path = \"../shared\" }"),
                "Inline dependency should be updated\nActual:\n{}",
                app_cargo
            );

            assert!(
                app_cargo.contains("[dev-dependencies.shared]")
                    && app_cargo.contains("path = \"../shared\""),
                "Table-format dev-dependency should be updated\nActual:\n{}",
                app_cargo
            );

            assert!(
                !app_cargo.contains("common = { path")
                    && !app_cargo.contains("[dev-dependencies.common]"),
                "Old dependency names should be removed\nActual:\n{}",
                app_cargo
            );

            let main_content = ws.read_file("crates/app/src/main.rs");
            assert!(
                main_content.contains("use shared::func;"),
                "Use statement should be updated\nActual:\n{}",
                main_content
            );
            Ok(())
        },
    )
    .await
}
