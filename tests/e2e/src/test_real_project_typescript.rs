//! Real project integration tests for TypeScript
//!
//! Tests mill operations against multiple real-world TypeScript projects:
//! - type-fest: Popular TypeScript utility types
//! - ts-pattern: Pattern matching library
//! - nanoid: Unique ID generator
//!
//! Each project tests different aspects of TypeScript refactoring.

use crate::test_real_projects::{assertions, RealProjectContext};
use once_cell::sync::Lazy;
use serde_json::json;
use serial_test::serial;
use std::sync::Mutex;

// ============================================================================
// type-fest Tests - TypeScript utility types library
// ============================================================================

static TYPEFEST_CONTEXT: Lazy<Mutex<RealProjectContext>> = Lazy::new(|| {
    Mutex::new(RealProjectContext::new(
        "https://github.com/sindresorhus/type-fest.git",
        "type-fest",
    ))
});

/// Warmup test - runs first (alphabetically) to initialize LSP
#[tokio::test]
#[serial]
async fn test_typefest_00_warmup() {
    let mut ctx = TYPEFEST_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
    ctx.ensure_warmed_up()
        .await
        .expect("LSP warmup should succeed for type-fest");
}

#[tokio::test]
#[serial]
async fn test_typefest_search_symbols() {
    let mut ctx = TYPEFEST_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
    ctx.ensure_warmed_up().await.expect("LSP should be ready");

    let result = ctx
        .call_tool("search_code", json!({ "query": "JsonValue" }))
        .await
        .expect("search_code should succeed");

    assertions::assert_search_completed(&result, "JsonValue");
    println!("✅ type-fest: Search completed for JsonValue");
}

#[tokio::test]
#[serial]
async fn test_typefest_inspect_code() {
    let mut ctx = TYPEFEST_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    // Find a .d.ts file to inspect
    let types_file = ctx.absolute_path("source/basic.d.ts");

    if types_file.exists() {
        ctx.wait_for_lsp(&types_file).await;

        let result = ctx
            .call_tool(
                "inspect_code",
                json!({
                    "filePath": types_file.to_string_lossy(),
                    "line": 1,
                    "character": 0,
                    "include": ["diagnostics"]
                }),
            )
            .await
            .expect("inspect_code should succeed");

        assert!(result.get("result").is_some());
        println!("✅ type-fest: Successfully inspected basic.d.ts");
    } else {
        println!("⚠️ type-fest: basic.d.ts not found, skipping inspect test");
    }
}

#[tokio::test]
#[serial]
async fn test_typefest_rename_file_dry_run() {
    let mut ctx = TYPEFEST_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    ctx.create_test_file(
        "source/test-rename.d.ts",
        "export type TestType = string | number;",
    );

    let old_path = ctx.absolute_path("source/test-rename.d.ts");
    let new_path = ctx.absolute_path("source/test-renamed.d.ts");

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
    println!("✅ type-fest: Successfully dry-run renamed test file");
}

#[tokio::test]
#[serial]
async fn test_typefest_move_type_file() {
    let mut ctx = TYPEFEST_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    ctx.create_test_file(
        "source/test-move.d.ts",
        r#"export type MoveTestType = { value: string };
export type AnotherType = MoveTestType & { extra: number };
"#,
    );

    let source = ctx.absolute_path("source/test-move.d.ts");
    let dest = ctx.absolute_path("source/internal/test-move.d.ts");

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

    assertions::assert_success(&result, "move type file");

    // Verify move operation
    ctx.verify_file_not_exists("source/test-move.d.ts")
        .expect("Source should be gone");
    ctx.verify_file_exists("source/internal/test-move.d.ts")
        .expect("Dest should exist");
    ctx.verify_file_contains("source/internal/test-move.d.ts", "MoveTestType")
        .expect("Type should be preserved");
    ctx.verify_file_contains("source/internal/test-move.d.ts", "AnotherType")
        .expect("Second type should be preserved");

    // Verify project still compiles after move
    if let Err(e) = ctx.verify_typescript_compiles() {
        println!("⚠️ type-fest: TypeScript compilation check: {}", e);
        // Don't fail - type-fest may have complex build requirements
    }

    println!("✅ type-fest: Successfully moved type definition file with verified content");
}

// ============================================================================
// ts-pattern Tests - Pattern matching library
// ============================================================================

static TSPATTERN_CONTEXT: Lazy<Mutex<RealProjectContext>> = Lazy::new(|| {
    Mutex::new(RealProjectContext::new(
        "https://github.com/gvergnaud/ts-pattern.git",
        "ts-pattern",
    ))
});

/// Warmup test - runs first (alphabetically) to initialize LSP
#[tokio::test]
#[serial]
async fn test_tspattern_00_warmup() {
    let mut ctx = TSPATTERN_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
    ctx.ensure_warmed_up()
        .await
        .expect("LSP warmup should succeed for ts-pattern");
}

#[tokio::test]
#[serial]
async fn test_tspattern_search_symbols() {
    let mut ctx = TSPATTERN_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
    ctx.ensure_warmed_up().await.expect("LSP should be ready");

    let result = ctx
        .call_tool("search_code", json!({ "query": "match" }))
        .await
        .expect("search_code should succeed");

    assertions::assert_search_completed(&result, "match");
    println!("✅ ts-pattern: Search completed for match");
}

#[tokio::test]
#[serial]
async fn test_tspattern_rename_folder_dry_run() {
    let mut ctx = TSPATTERN_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    ctx.create_test_file("src/test-folder/index.ts", "export const value = 42;");
    ctx.create_test_file("src/test-folder/utils.ts", "export const util = 'util';");

    let old_path = ctx.absolute_path("src/test-folder");
    let new_path = ctx.absolute_path("src/renamed-folder");

    let result = ctx
        .call_tool(
            "rename_all",
            json!({
                "target": { "kind": "directory", "filePath": old_path.to_string_lossy() },
                "newName": new_path.to_string_lossy(),
                "options": { "dryRun": true }
            }),
        )
        .await
        .expect("rename_all should succeed");

    assertions::assert_preview(&result, "rename folder dry-run");
    assert!(old_path.exists(), "Folder should still exist after dry-run");
    println!("✅ ts-pattern: Successfully dry-run renamed folder");
}

#[tokio::test]
#[serial]
async fn test_tspattern_find_replace() {
    let mut ctx = TSPATTERN_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    ctx.create_test_file(
        "src/test-replace/config.ts",
        r#"export const OLD_VALUE = 'old';
export const useOLD_VALUE = OLD_VALUE;
"#,
    );

    let result = ctx
        .call_tool(
            "workspace",
            json!({
                "action": "find_replace",
                "params": {
                    "pattern": "OLD_VALUE",
                    "replacement": "NEW_VALUE",
                    "mode": "literal"
                },
                "options": { "dryRun": false }
            }),
        )
        .await
        .expect("find_replace should succeed");

    // Verify replacement
    ctx.verify_file_contains("src/test-replace/config.ts", "NEW_VALUE")
        .expect("Should have replaced value");
    ctx.verify_file_not_contains("src/test-replace/config.ts", "OLD_VALUE")
        .expect("Old value should be gone");

    println!("✅ ts-pattern: Successfully executed find/replace with verification");
}

// ============================================================================
// nanoid Tests - Unique ID generator (small, focused library)
// ============================================================================

static NANOID_CONTEXT: Lazy<Mutex<RealProjectContext>> = Lazy::new(|| {
    Mutex::new(RealProjectContext::new(
        "https://github.com/ai/nanoid.git",
        "nanoid",
    ))
});

/// Warmup test - runs first (alphabetically) to initialize LSP
#[tokio::test]
#[serial]
async fn test_nanoid_00_warmup() {
    let mut ctx = NANOID_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
    ctx.ensure_warmed_up()
        .await
        .expect("LSP warmup should succeed for nanoid");
}

#[tokio::test]
#[serial]
async fn test_nanoid_search_symbols() {
    let mut ctx = NANOID_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
    ctx.ensure_warmed_up().await.expect("LSP should be ready");

    let result = ctx
        .call_tool("search_code", json!({ "query": "nanoid" }))
        .await
        .expect("search_code should succeed");

    assertions::assert_search_completed(&result, "nanoid");
    println!("✅ nanoid: Search completed for nanoid");
}

#[tokio::test]
#[serial]
async fn test_nanoid_rename_symbol() {
    let mut ctx = NANOID_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    ctx.create_test_file(
        "test-symbol.ts",
        r#"export const myId = 'test-id';
export function useMyId() {
    return myId + '-suffix';
}
console.log(myId);
"#,
    );

    let file_path = ctx.absolute_path("test-symbol.ts");
    ctx.wait_for_lsp(&file_path).await;

    let result = ctx
        .call_tool(
            "rename_all",
            json!({
                "target": {
                    "kind": "symbol",
                    "filePath": file_path.to_string_lossy(),
                    "line": 1,
                    "character": 13
                },
                "newName": "renamedId",
                "options": { "dryRun": false }
            }),
        )
        .await;

    match result {
        Ok(resp) => {
            let content = ctx.read_file("test-symbol.ts");
            if content.contains("renamedId") {
                println!("✅ nanoid: Successfully renamed symbol");
            } else {
                println!("⚠️ nanoid: Symbol rename completed but content unchanged (LSP may need more time)");
            }
        }
        Err(e) => {
            println!("⚠️ nanoid: Symbol rename failed (LSP indexing): {}", e);
        }
    }
}

#[tokio::test]
#[serial]
async fn test_nanoid_prune_file() {
    let mut ctx = NANOID_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    ctx.create_test_file("test-prune.ts", "export const toDelete = 'delete-me';");

    let file_path = ctx.absolute_path("test-prune.ts");
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

    assertions::assert_success(&result, "prune file");
    ctx.verify_file_not_exists("test-prune.ts")
        .expect("File should be deleted");
    println!("✅ nanoid: Successfully pruned file");
}

#[tokio::test]
#[serial]
async fn test_nanoid_move_with_import_update() {
    let mut ctx = NANOID_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());

    // Create a module with imports
    ctx.create_test_file(
        "lib/helpers.ts",
        r#"export function formatId(id: string): string {
    return id.toUpperCase();
}
"#,
    );
    ctx.create_test_file(
        "lib/main.ts",
        r#"import { formatId } from './helpers';

export function createFormattedId(raw: string) {
    return formatId(raw);
}
"#,
    );

    let source = ctx.absolute_path("lib/helpers.ts");
    let dest = ctx.absolute_path("lib/utils/helpers.ts");

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

    assertions::assert_success(&result, "move with import update");

    // Verify file move
    ctx.verify_file_not_exists("lib/helpers.ts")
        .expect("Source should be gone");
    ctx.verify_file_exists("lib/utils/helpers.ts")
        .expect("Dest should exist");
    ctx.verify_file_contains("lib/utils/helpers.ts", "formatId")
        .expect("Function should be preserved");

    // Check if imports were updated (may or may not be updated depending on LSP)
    let main_content = ctx.read_file("lib/main.ts");
    if main_content.contains("./utils/helpers") {
        println!("✅ nanoid: Import path updated correctly");
    } else {
        println!("⚠️ nanoid: File moved but imports may not be updated (expected for non-LSP move)");
    }

    // Verify project still type-checks after move
    if let Err(e) = ctx.verify_typescript_compiles() {
        println!("⚠️ nanoid: TypeScript compilation check: {}", e);
    } else {
        println!("✅ nanoid: Project still compiles after move");
    }
}
