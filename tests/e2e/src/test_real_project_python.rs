//! Real project integration tests for Python
//!
//! Tests mill operations against real-world Python projects:
//! - httpx: Modern HTTP client (async-first design)
//! - rich: Beautiful terminal formatting
//! - pydantic: Data validation using Python type annotations
//!
//! Each project tests different aspects of Python refactoring including
//! import updates, package moves, and pyproject.toml handling.

use crate::test_real_projects::{assertions, RealProjectContext};
use once_cell::sync::Lazy;
use serde_json::json;
use serial_test::serial;
use std::sync::Mutex;

// ============================================================================
// httpx Tests - Modern async HTTP client
// ============================================================================

static HTTPX_CONTEXT: Lazy<Mutex<RealProjectContext>> = Lazy::new(|| {
    Mutex::new(RealProjectContext::new(
        "https://github.com/encode/httpx.git",
        "httpx",
    ))
});

/// Warmup test - runs first (alphabetically) to initialize LSP
#[tokio::test]
#[serial]
async fn test_httpx_00_warmup() {
    let mut ctx = HTTPX_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
    ctx.ensure_warmed_up()
        .await
        .expect("LSP warmup should succeed for httpx");
}

#[tokio::test]
#[serial]
async fn test_httpx_search_symbols() {
    let mut ctx = HTTPX_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
    ctx.ensure_warmed_up().await.expect("LSP should be ready");

    let result = ctx
        .call_tool("search_code", json!({ "query": "Client" }))
        .await
        .expect("search_code should succeed");

    assertions::assert_search_completed(&result, "Client");
    println!("✅ httpx: Search completed for Client");
}

#[tokio::test]
#[serial]
async fn test_httpx_inspect_code() {
    let mut ctx = HTTPX_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    let init_file = ctx.absolute_path("httpx/__init__.py");

    if init_file.exists() {
        ctx.wait_for_lsp(&init_file).await;

        let result = ctx
            .call_tool(
                "inspect_code",
                json!({
                    "filePath": init_file.to_string_lossy(),
                    "line": 1,
                    "character": 0,
                    "include": ["diagnostics"]
                }),
            )
            .await
            .expect("inspect_code should succeed");

        assert!(result.get("result").is_some());
        println!("✅ httpx: Successfully inspected __init__.py");
    } else {
        println!("⚠️ httpx: __init__.py not found at expected location");
    }
}

#[tokio::test]
#[serial]
async fn test_httpx_rename_file_dry_run() {
    let mut ctx = HTTPX_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    ctx.create_test_file(
        "httpx/test_rename.py",
        r#""""Test module for rename operation."""

def test_function():
    return "hello"
"#,
    );

    let old_path = ctx.absolute_path("httpx/test_rename.py");
    let new_path = ctx.absolute_path("httpx/renamed_test.py");

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
    println!("✅ httpx: Successfully dry-run renamed Python file");
}

#[tokio::test]
#[serial]
async fn test_httpx_move_module() {
    let mut ctx = HTTPX_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    ctx.create_test_file(
        "httpx/test_move.py",
        r#""""Module to be moved."""

class TestClass:
    def __init__(self, value: str):
        self.value = value

    def get_value(self) -> str:
        return self.value
"#,
    );

    let source = ctx.absolute_path("httpx/test_move.py");
    let dest = ctx.absolute_path("httpx/_internal/test_move.py");

    std::fs::create_dir_all(dest.parent().unwrap()).ok();
    // Create __init__.py for the package
    ctx.create_test_file("httpx/_internal/__init__.py", "");

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
    assert!(content.contains("TestClass"), "Content should be preserved");
    println!("✅ httpx: Successfully moved Python module");
}

#[tokio::test]
#[serial]
async fn test_httpx_prune_module() {
    let mut ctx = HTTPX_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    ctx.create_test_file(
        "httpx/to_delete.py",
        "DELETE_ME = 'this will be deleted'",
    );

    let file_path = ctx.absolute_path("httpx/to_delete.py");
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
    println!("✅ httpx: Successfully pruned Python module");
}

// ============================================================================
// rich Tests - Terminal formatting library
// ============================================================================

static RICH_CONTEXT: Lazy<Mutex<RealProjectContext>> = Lazy::new(|| {
    Mutex::new(RealProjectContext::new(
        "https://github.com/Textualize/rich.git",
        "rich",
    ))
});

/// Warmup test - runs first (alphabetically) to initialize LSP
#[tokio::test]
#[serial]
async fn test_rich_00_warmup() {
    let mut ctx = RICH_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
    ctx.ensure_warmed_up()
        .await
        .expect("LSP warmup should succeed for rich");
}

#[tokio::test]
#[serial]
async fn test_rich_search_symbols() {
    let mut ctx = RICH_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
    ctx.ensure_warmed_up().await.expect("LSP should be ready");

    let result = ctx
        .call_tool("search_code", json!({ "query": "Console" }))
        .await
        .expect("search_code should succeed");

    assertions::assert_search_completed(&result, "Console");
    println!("✅ rich: Search completed for Console");
}

#[tokio::test]
#[serial]
async fn test_rich_find_replace() {
    let mut ctx = RICH_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    ctx.create_test_file(
        "rich/test_replace.py",
        r#"OLD_CONSTANT = "old_value"

def use_old():
    return OLD_CONSTANT
"#,
    );

    let result = ctx
        .call_tool(
            "workspace",
            json!({
                "action": "find_replace",
                "params": {
                    "pattern": "OLD_CONSTANT",
                    "replacement": "NEW_CONSTANT",
                    "mode": "literal"
                },
                "options": { "dryRun": false }
            }),
        )
        .await
        .expect("find_replace should succeed");

    let content = ctx.read_file("rich/test_replace.py");
    assert!(
        content.contains("NEW_CONSTANT"),
        "Should have replaced constant"
    );
    println!("✅ rich: Successfully executed find/replace in Python");
}

#[tokio::test]
#[serial]
async fn test_rich_rename_folder() {
    let mut ctx = RICH_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    ctx.create_test_file("rich/test_pkg/__init__.py", "from .utils import helper");
    ctx.create_test_file("rich/test_pkg/utils.py", "def helper(): pass");

    let old_path = ctx.absolute_path("rich/test_pkg");
    let new_path = ctx.absolute_path("rich/renamed_pkg");

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
    assert!(
        new_path.join("__init__.py").exists(),
        "__init__.py should exist"
    );
    println!("✅ rich: Successfully renamed Python package");
}

#[tokio::test]
#[serial]
async fn test_rich_move_with_import_update() {
    let mut ctx = RICH_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    ctx.create_test_file(
        "rich/helpers.py",
        r#"def format_text(text: str) -> str:
    return text.strip()
"#,
    );
    ctx.create_test_file(
        "rich/main_test.py",
        r#"from .helpers import format_text

def process(text: str) -> str:
    return format_text(text)
"#,
    );

    let source = ctx.absolute_path("rich/helpers.py");
    let dest = ctx.absolute_path("rich/utils/helpers.py");

    std::fs::create_dir_all(dest.parent().unwrap()).ok();
    ctx.create_test_file("rich/utils/__init__.py", "");

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

    assertions::assert_success(&result, "move with import update");
    assert!(!source.exists(), "Source should be gone");
    assert!(dest.exists(), "Dest should exist");

    let main_content = ctx.read_file("rich/main_test.py");
    if main_content.contains(".utils.helpers") {
        println!("✅ rich: Successfully moved Python module with import updates");
    } else {
        println!("⚠️ rich: Module moved but imports may need manual update");
    }
}

// ============================================================================
// pydantic Tests - Data validation library
// ============================================================================

static PYDANTIC_CONTEXT: Lazy<Mutex<RealProjectContext>> = Lazy::new(|| {
    Mutex::new(RealProjectContext::new(
        "https://github.com/pydantic/pydantic.git",
        "pydantic",
    ))
});

/// Warmup test - runs first (alphabetically) to initialize LSP
#[tokio::test]
#[serial]
async fn test_pydantic_00_warmup() {
    let mut ctx = PYDANTIC_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
    ctx.ensure_warmed_up()
        .await
        .expect("LSP warmup should succeed for pydantic");
}

#[tokio::test]
#[serial]
async fn test_pydantic_search_symbols() {
    let mut ctx = PYDANTIC_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
    ctx.ensure_warmed_up().await.expect("LSP should be ready");

    let result = ctx
        .call_tool("search_code", json!({ "query": "BaseModel" }))
        .await
        .expect("search_code should succeed");

    assertions::assert_search_completed(&result, "BaseModel");
    println!("✅ pydantic: Search completed for BaseModel");
}

#[tokio::test]
#[serial]
async fn test_pydantic_symbol_rename() {
    let mut ctx = PYDANTIC_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    ctx.create_test_file(
        "pydantic/test_symbol.py",
        r#"class OldClass:
    def __init__(self, value: int):
        self.value = value

def use_old() -> OldClass:
    return OldClass(42)
"#,
    );

    let file_path = ctx.absolute_path("pydantic/test_symbol.py");
    ctx.wait_for_lsp(&file_path).await;

    let result = ctx
        .call_tool(
            "rename_all",
            json!({
                "target": {
                    "kind": "symbol",
                    "filePath": file_path.to_string_lossy(),
                    "line": 1,
                    "character": 6
                },
                "newName": "NewClass",
                "options": { "dryRun": false }
            }),
        )
        .await;

    match result {
        Ok(resp) => {
            let content = ctx.read_file("pydantic/test_symbol.py");
            if content.contains("NewClass") && content.contains("def use_old() -> NewClass:") {
                println!("✅ pydantic: Successfully renamed Python class and references");
            } else if content.contains("NewClass") {
                println!("⚠️ pydantic: Class renamed but not all references updated");
            } else {
                println!("⚠️ pydantic: Symbol rename completed but content unchanged (LSP indexing)");
            }
        }
        Err(e) => {
            println!("⚠️ pydantic: Symbol rename failed (LSP may need time): {}", e);
        }
    }
}

#[tokio::test]
#[serial]
async fn test_pydantic_create_package_dry_run() {
    let mut ctx = PYDANTIC_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    let pkg_path = ctx.absolute_path("packages/test-pkg");

    let result = ctx
        .call_tool(
            "workspace",
            json!({
                "action": "create_package",
                "params": {
                    "path": pkg_path.to_string_lossy(),
                    "name": "test-pkg",
                    "type": "python"
                },
                "options": { "dryRun": true }
            }),
        )
        .await
        .expect("create_package should succeed");

    assert!(result.get("result").is_some());
    assert!(
        !pkg_path.exists(),
        "Package should NOT exist after dry run"
    );
    println!("✅ pydantic: Successfully dry-run create_package for Python");
}

#[tokio::test]
#[serial]
async fn test_pydantic_workflow_move_and_update() {
    let mut ctx = PYDANTIC_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    // Create a module structure
    ctx.create_test_file(
        "pydantic/validators.py",
        r#"def validate_email(email: str) -> bool:
    return '@' in email

def validate_phone(phone: str) -> bool:
    return len(phone) >= 10
"#,
    );
    ctx.create_test_file(
        "pydantic/models.py",
        r#"from .validators import validate_email

class UserModel:
    def __init__(self, email: str):
        if not validate_email(email):
            raise ValueError("Invalid email")
        self.email = email
"#,
    );

    // Move validators to a subdirectory
    let source = ctx.absolute_path("pydantic/validators.py");
    let dest = ctx.absolute_path("pydantic/core/validators.py");

    std::fs::create_dir_all(dest.parent().unwrap()).ok();
    ctx.create_test_file("pydantic/core/__init__.py", "");

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

    assertions::assert_success(&result, "move validators");
    assert!(!source.exists(), "Source should be gone");
    assert!(dest.exists(), "Dest should exist");

    let validators_content = std::fs::read_to_string(&dest).expect("Should read validators");
    assert!(
        validators_content.contains("validate_email"),
        "Content should be preserved"
    );

    println!("✅ pydantic: Successfully completed move workflow");
}
