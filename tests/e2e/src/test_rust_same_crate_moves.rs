//! Integration tests for Rust same-crate file and directory moves (V2 - CONSOLIDATED)
//!
//! Migrated to use closure-based helper pattern for reduced boilerplate.

use crate::test_helpers::*;
use anyhow::Result;

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
        "rename.plan",
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
        "rename.plan",
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
        "rename.plan",
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
