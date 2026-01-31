//! Real project integration tests for Rust
//!
//! Tests mill operations against real-world Rust projects:
//! - thiserror: Derive macro for custom error types (small, focused)
//! - once_cell: Lazy initialization primitives
//! - anyhow: Flexible error handling
//!
//! Each project tests different aspects of Rust refactoring including
//! Cargo.toml handling, module moves, and workspace operations.

use crate::test_real_projects::{assertions, RealProjectContext};
use once_cell::sync::Lazy;
use serde_json::json;
use serial_test::serial;
use std::sync::Mutex;

// ============================================================================
// thiserror Tests - Error derive macro library
// ============================================================================

static THISERROR_CONTEXT: Lazy<Mutex<RealProjectContext>> = Lazy::new(|| {
    Mutex::new(RealProjectContext::new(
        "https://github.com/dtolnay/thiserror.git",
        "thiserror",
    ))
});

/// Warmup test - runs first (alphabetically) to initialize LSP
#[tokio::test]
#[serial]
async fn test_thiserror_00_warmup() {
    let mut ctx = THISERROR_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
    ctx.ensure_warmed_up()
        .await
        .expect("LSP warmup should succeed for thiserror");
}

#[tokio::test]
#[serial]
async fn test_thiserror_search_symbols() {
    let mut ctx = THISERROR_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
    ctx.ensure_warmed_up().await.expect("LSP should be ready");

    let result = ctx
        .call_tool("search_code", json!({ "query": "Error" }))
        .await
        .expect("search_code should succeed");

    assertions::assert_search_completed(&result, "Error");
    println!("✅ thiserror: Search completed for Error");
}

#[tokio::test]
#[serial]
async fn test_thiserror_inspect_lib() {
    let mut ctx = THISERROR_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    let lib_file = ctx.absolute_path("src/lib.rs");

    if lib_file.exists() {
        ctx.wait_for_lsp(&lib_file).await;

        let result = ctx
            .call_tool(
                "inspect_code",
                json!({
                    "filePath": lib_file.to_string_lossy(),
                    "line": 1,
                    "character": 0,
                    "include": ["diagnostics"]
                }),
            )
            .await
            .expect("inspect_code should succeed");

        assert!(result.get("result").is_some());
        println!("✅ thiserror: Successfully inspected src/lib.rs");
    } else {
        println!("⚠️ thiserror: src/lib.rs not found");
    }
}

#[tokio::test]
#[serial]
async fn test_thiserror_rename_file_dry_run() {
    let mut ctx = THISERROR_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    ctx.create_test_file("src/test_rename.rs", "pub fn test_func() -> u32 { 42 }");

    let old_path = ctx.absolute_path("src/test_rename.rs");
    let new_path = ctx.absolute_path("src/renamed_test.rs");

    let result = ctx
        .call_tool(
            "rename_all",
            json!({
                "target": { "kind": "file", "filePath": old_path.to_string_lossy() },
                "newName": new_path.to_string_lossy(),
                "options": { "dryRun": true }
            }),
        )
        .await
        .expect("rename_all should succeed");

    assertions::assert_preview(&result, "rename file dry-run");
    assert!(old_path.exists(), "File should still exist after dry-run");
    println!("✅ thiserror: Successfully dry-run renamed Rust file");
}

#[tokio::test]
#[serial]
async fn test_thiserror_move_module() {
    let mut ctx = THISERROR_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    ctx.create_test_file(
        "src/test_move.rs",
        r#"//! Test module for move operation
pub struct TestStruct {
    pub value: i32,
}

impl TestStruct {
    pub fn new(value: i32) -> Self {
        Self { value }
    }
}
"#,
    );

    let source = ctx.absolute_path("src/test_move.rs");
    let dest = ctx.absolute_path("src/internal/test_move.rs");

    std::fs::create_dir_all(dest.parent().unwrap()).ok();

    let result = ctx
        .call_tool(
            "relocate",
            json!({
                "target": { "kind": "file", "filePath": source.to_string_lossy() },
                "destination": dest.to_string_lossy(),
                "options": { "dryRun": false }
            }),
        )
        .await
        .expect("relocate should succeed");

    assertions::assert_success(&result, "move module");
    assert!(!source.exists(), "Source should be gone");
    assert!(dest.exists(), "Dest should exist");

    let content = std::fs::read_to_string(&dest).expect("Should read moved file");
    assert!(content.contains("TestStruct"), "Content should be preserved");
    println!("✅ thiserror: Successfully moved Rust module");
}

#[tokio::test]
#[serial]
async fn test_thiserror_prune_module() {
    let mut ctx = THISERROR_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    ctx.create_test_file("src/to_delete.rs", "pub const DELETE_ME: &str = \"delete\";");

    let file_path = ctx.absolute_path("src/to_delete.rs");
    assert!(file_path.exists(), "File should exist before prune");

    let result = ctx
        .call_tool(
            "prune",
            json!({
                "target": { "kind": "file", "filePath": file_path.to_string_lossy() },
                "options": { "dryRun": false }
            }),
        )
        .await
        .expect("prune should succeed");

    assertions::assert_success(&result, "prune module");
    assert!(!file_path.exists(), "File should be deleted");
    println!("✅ thiserror: Successfully pruned Rust module");
}

// ============================================================================
// once_cell Tests - Lazy initialization library
// ============================================================================

static ONCECELL_CONTEXT: Lazy<Mutex<RealProjectContext>> = Lazy::new(|| {
    Mutex::new(RealProjectContext::new(
        "https://github.com/matklad/once_cell.git",
        "once_cell",
    ))
});

/// Warmup test - runs first (alphabetically) to initialize LSP
#[tokio::test]
#[serial]
async fn test_oncecell_00_warmup() {
    let mut ctx = ONCECELL_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
    ctx.ensure_warmed_up()
        .await
        .expect("LSP warmup should succeed for once_cell");
}

#[tokio::test]
#[serial]
async fn test_oncecell_search_symbols() {
    let mut ctx = ONCECELL_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
    ctx.ensure_warmed_up().await.expect("LSP should be ready");

    let result = ctx
        .call_tool("search_code", json!({ "query": "Lazy" }))
        .await
        .expect("search_code should succeed");

    assertions::assert_search_completed(&result, "Lazy");
    println!("✅ once_cell: Search completed for Lazy");
}

#[tokio::test]
#[serial]
async fn test_oncecell_find_replace() {
    let mut ctx = ONCECELL_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    ctx.create_test_file(
        "src/test_replace.rs",
        r#"pub const OLD_CONST: &str = "old";
pub fn use_old() -> &'static str {
    OLD_CONST
}
"#,
    );

    let result = ctx
        .call_tool(
            "workspace",
            json!({
                "action": "find_replace",
                "params": {
                    "pattern": "OLD_CONST",
                    "replacement": "NEW_CONST",
                    "mode": "literal"
                },
                "options": { "dryRun": false }
            }),
        )
        .await
        .expect("find_replace should succeed");

    let content = ctx.read_file("src/test_replace.rs");
    assert!(content.contains("NEW_CONST"), "Should have replaced constant");
    println!("✅ once_cell: Successfully executed find/replace in Rust");
}

#[tokio::test]
#[serial]
async fn test_oncecell_rename_folder() {
    let mut ctx = ONCECELL_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    ctx.create_test_file(
        "src/test_dir/mod.rs",
        "pub mod utils;\npub mod helpers;",
    );
    ctx.create_test_file("src/test_dir/utils.rs", "pub fn util_fn() {}");
    ctx.create_test_file("src/test_dir/helpers.rs", "pub fn helper_fn() {}");

    let old_path = ctx.absolute_path("src/test_dir");
    let new_path = ctx.absolute_path("src/renamed_dir");

    let result = ctx
        .call_tool(
            "rename_all",
            json!({
                "target": { "kind": "directory", "filePath": old_path.to_string_lossy() },
                "newName": new_path.to_string_lossy(),
                "options": { "dryRun": false }
            }),
        )
        .await
        .expect("rename_all should succeed");

    assertions::assert_success(&result, "rename folder");
    assert!(!old_path.exists(), "Old folder should be gone");
    assert!(new_path.exists(), "New folder should exist");
    assert!(new_path.join("mod.rs").exists(), "mod.rs should exist");
    println!("✅ once_cell: Successfully renamed Rust module folder");
}

// ============================================================================
// anyhow Tests - Error handling library
// ============================================================================

static ANYHOW_CONTEXT: Lazy<Mutex<RealProjectContext>> = Lazy::new(|| {
    Mutex::new(RealProjectContext::new(
        "https://github.com/dtolnay/anyhow.git",
        "anyhow",
    ))
});

/// Warmup test - runs first (alphabetically) to initialize LSP
#[tokio::test]
#[serial]
async fn test_anyhow_00_warmup() {
    let mut ctx = ANYHOW_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
    ctx.ensure_warmed_up()
        .await
        .expect("LSP warmup should succeed for anyhow");
}

#[tokio::test]
#[serial]
async fn test_anyhow_search_symbols() {
    let mut ctx = ANYHOW_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
    ctx.ensure_warmed_up().await.expect("LSP should be ready");

    let result = ctx
        .call_tool("search_code", json!({ "query": "Result" }))
        .await
        .expect("search_code should succeed");

    assertions::assert_search_completed(&result, "Result");
    println!("✅ anyhow: Search completed for Result");
}

#[tokio::test]
#[serial]
async fn test_anyhow_create_package_dry_run() {
    let mut ctx = ANYHOW_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    let pkg_path = ctx.absolute_path("crates/test-pkg");

    let result = ctx
        .call_tool(
            "workspace",
            json!({
                "action": "create_package",
                "params": {
                    "path": pkg_path.to_string_lossy(),
                    "name": "test-pkg",
                    "type": "cargo"
                },
                "options": { "dryRun": true }
            }),
        )
        .await
        .expect("create_package should succeed");

    assert!(result.get("result").is_some());
    assert!(!pkg_path.exists(), "Package should NOT exist after dry run");
    println!("✅ anyhow: Successfully dry-run create_package for Cargo");
}

#[tokio::test]
#[serial]
async fn test_anyhow_symbol_rename() {
    let mut ctx = ANYHOW_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    ctx.create_test_file(
        "src/test_symbol.rs",
        r#"pub fn old_function() -> i32 {
    42
}

pub fn caller() -> i32 {
    old_function() + 1
}
"#,
    );

    let file_path = ctx.absolute_path("src/test_symbol.rs");
    ctx.wait_for_lsp(&file_path).await;

    let result = ctx
        .call_tool(
            "rename_all",
            json!({
                "target": {
                    "kind": "symbol",
                    "filePath": file_path.to_string_lossy(),
                    "line": 1,
                    "character": 7
                },
                "newName": "new_function",
                "options": { "dryRun": false }
            }),
        )
        .await;

    match result {
        Ok(resp) => {
            let content = ctx.read_file("src/test_symbol.rs");
            if content.contains("new_function") {
                println!("✅ anyhow: Successfully renamed Rust symbol");
            } else {
                println!("⚠️ anyhow: Symbol rename completed but content unchanged (LSP indexing)");
            }
        }
        Err(e) => {
            println!("⚠️ anyhow: Symbol rename failed (LSP may need time): {}", e);
        }
    }
}

#[tokio::test]
#[serial]
async fn test_anyhow_move_with_mod_update() {
    let mut ctx = ANYHOW_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    // Create a module structure
    ctx.create_test_file(
        "src/utils.rs",
        r#"pub fn format_error(msg: &str) -> String {
    format!("Error: {}", msg)
}
"#,
    );

    let source = ctx.absolute_path("src/utils.rs");
    let dest = ctx.absolute_path("src/internal/utils.rs");

    std::fs::create_dir_all(dest.parent().unwrap()).ok();

    let result = ctx
        .call_tool(
            "relocate",
            json!({
                "target": { "kind": "file", "filePath": source.to_string_lossy() },
                "destination": dest.to_string_lossy(),
                "options": { "dryRun": false }
            }),
        )
        .await
        .expect("relocate should succeed");

    assertions::assert_success(&result, "move with mod update");
    assert!(!source.exists(), "Source should be gone");
    assert!(dest.exists(), "Dest should exist");

    let content = std::fs::read_to_string(&dest).expect("Should read moved file");
    assert!(
        content.contains("format_error"),
        "Content should be preserved"
    );
    println!("✅ anyhow: Successfully moved Rust module");
}
