//! Integration tests for Rust workspace member directory renames (V2 - CONSOLIDATED)
//!
//! Migrated to use closure-based helper pattern for reduced boilerplate.

use crate::test_helpers::*;
use anyhow::Result;

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
        "rename.plan",
        |ws| build_rename_params(ws, "crates/my-crate", "crates/my-renamed-crate", "directory"),
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
                !root_cargo.contains("crates/my-crate\"") && !root_cargo.contains("crates/my-crate]"),
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
            ("crates/common/src/lib.rs", "pub fn utility() -> i32 { 42 }\n"),
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
        "rename.plan",
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
        "rename.plan",
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
        "rename.plan",
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
        "rename.plan",
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
