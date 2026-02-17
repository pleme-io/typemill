//! E2E tests for scope preset behavior
//!
//! Tests the new scope architecture (code/standard/comments/everything)
//! and verifies backward compatibility with deprecated names (code-only/all).
//!
//! REFACTORED: Uses run_scope_rename_test() helper to eliminate repeated
//! workspace+client+call_tool boilerplate. 408 lines → ~170 lines (58% reduction).

use crate::test_helpers::run_scope_rename_test;

/// Test "code" scope - only updates code files, NOT docs or configs
#[tokio::test]
async fn test_scope_code_only_updates_rust_files() {
    run_scope_rename_test(
        &[
            ("src/old.rs", "pub fn hello() {\n    println!(\"Hello\");\n}\n"),
            ("README.md", "# Documentation\n\nSee [old.rs](src/old.rs) for implementation.\n\nThe file `src/old.rs` contains the code.\n"),
            ("Cargo.toml", "[package]\nname = \"test\"\n\n[dependencies]\nold = { path = \"src/old.rs\" }\n"),
        ],
        Some("code"),
        |ws| {
            let readme = ws.read_file("README.md");
            assert!(readme.contains("old.rs"), "With 'code' scope, markdown should NOT be updated");
            let cargo = ws.read_file("Cargo.toml");
            assert!(cargo.contains("src/old.rs"), "With 'code' scope, Cargo.toml should NOT be updated");
            Ok(())
        },
    )
    .await
    .unwrap();
}

/// Test "standard" scope (default) - updates code + docs + configs
#[tokio::test]
async fn test_scope_standard_updates_code_docs_configs() {
    run_scope_rename_test(
        &[
            ("src/old.rs", "pub fn hello() {}"),
            (
                "README.md",
                "# Readme\n\nSee [old.rs](src/old.rs) for implementation.\n",
            ),
            (
                "docs/guide.md",
                "# Guide\n\nCheck out `src/old.rs` for the source code.\n",
            ),
        ],
        Some("standard"),
        |ws| {
            let readme = ws.read_file("README.md");
            assert!(
                readme.contains("new.rs"),
                "With 'standard' scope, markdown links should be updated"
            );
            let guide = ws.read_file("docs/guide.md");
            assert!(
                guide.contains("src/new.rs") || guide.contains("new.rs"),
                "With 'standard' scope, documentation paths should be updated"
            );
            Ok(())
        },
    )
    .await
    .unwrap();
}

/// Test "comments" scope - includes code comments
#[tokio::test]
async fn test_scope_comments_updates_code_comments() {
    run_scope_rename_test(
        &[
            ("src/old.rs", "pub fn hello() {\n    // This function says hello\n    println!(\"Hello\");\n}\n"),
            ("src/lib.rs", "// Import from old.rs file\npub mod old;\n\n/// Documentation for old.rs module\n/// Located at src/old.rs\npub fn use_old() {\n    old::hello();\n}\n"),
        ],
        Some("comments"),
        |ws| {
            let lib_content = ws.read_file("src/lib.rs");
            assert!(lib_content.contains("// Import from new.rs file"), "Comments scope should update inline comments");
            assert!(lib_content.contains("/// Documentation for new.rs module"), "Comments scope should update doc comments");
            assert!(lib_content.contains("/// Located at src/new.rs"), "Comments scope should update path references in comments");
            Ok(())
        },
    )
    .await
    .unwrap();
}

/// Test "everything" scope - most comprehensive (includes prose)
#[tokio::test]
async fn test_scope_everything_updates_markdown_prose() {
    run_scope_rename_test(
        &[
            ("src/old.rs", "pub fn hello() {}"),
            ("docs/guide.md", "# Guide\n\nThe implementation is in `src/old.rs` file.\n\nYou can find the old.rs source code in the repository.\n\nExample usage from old.rs:\n\n```rust\n// Reference to src/old.rs\nuse old;\n```\n"),
        ],
        Some("everything"),
        |ws| {
            let guide = ws.read_file("docs/guide.md");
            assert!(guide.contains("new.rs"), "Everything scope should at least update file references in markdown");
            Ok(())
        },
    )
    .await
    .unwrap();
}

/// Test backward compatibility - "code-only" deprecated alias
#[tokio::test]
async fn test_deprecated_code_only_alias_still_works() {
    run_scope_rename_test(
        &[
            ("src/old.rs", "pub fn test() {}"),
            ("README.md", "See src/old.rs"),
        ],
        Some("code-only"),
        |ws| {
            let readme = ws.read_file("README.md");
            assert!(
                readme.contains("old.rs"),
                "Deprecated 'code-only' should behave like 'code' scope"
            );
            Ok(())
        },
    )
    .await
    .unwrap();
}

/// Test backward compatibility - "all" deprecated alias
#[tokio::test]
async fn test_deprecated_all_alias_still_works() {
    run_scope_rename_test(
        &[
            ("src/old.rs", "pub fn test() {}"),
            ("README.md", "See [file](src/old.rs)"),
            ("docs/api.md", "# API\n\nSource: `src/old.rs`\n"),
        ],
        Some("all"),
        |ws| {
            let readme = ws.read_file("README.md");
            assert!(
                readme.contains("new.rs"),
                "Deprecated 'all' should behave like 'standard' scope"
            );
            let api = ws.read_file("docs/api.md");
            assert!(
                api.contains("src/new.rs") || api.contains("new.rs"),
                "Deprecated 'all' should update documentation files"
            );
            Ok(())
        },
    )
    .await
    .unwrap();
}

/// Test default scope behavior (no scope specified = "standard")
#[tokio::test]
async fn test_default_scope_is_standard() {
    run_scope_rename_test(
        &[
            ("src/old.rs", "pub fn test() {}"),
            ("README.md", "See [file](src/old.rs)"),
        ],
        None, // No scope = default ("standard")
        |ws| {
            let readme = ws.read_file("README.md");
            assert!(
                readme.contains("new.rs"),
                "Default scope should update docs (standard scope behavior)"
            );
            Ok(())
        },
    )
    .await
    .unwrap();
}
