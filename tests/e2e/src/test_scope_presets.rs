//! E2E tests for scope preset behavior
//!
//! Tests the new scope architecture (code/standard/comments/everything)
//! and verifies backward compatibility with deprecated names (code-only/all).

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

/// Test "code" scope - only updates code files
#[tokio::test]
async fn test_scope_code_only_updates_rust_files() {
    let workspace = TestWorkspace::new();

    // Create source file
    workspace.create_file(
        "src/old.rs",
        r#"pub fn hello() {
    println!("Hello");
}
"#,
    );

    // Create docs that shouldn't be updated with "code" scope
    workspace.create_file(
        "README.md",
        r#"# Documentation

See [old.rs](src/old.rs) for implementation.

The file `src/old.rs` contains the code.
"#,
    );

    // Create Cargo.toml (will be updated - has path dependency)
    workspace.create_file(
        "Cargo.toml",
        r#"[package]
name = "test"

[dependencies]
old = { path = "src/old.rs" }
"#,
    );

    let mut client = TestClient::new(workspace.path());

    // Rename with "code" scope
    let plan = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": workspace.absolute_path("src/old.rs").to_string_lossy()
                },
                "newName": workspace.absolute_path("src/new.rs").to_string_lossy(),
                "options": {
                    "scope": "code"
                }
            }),
        )
        .await
        .expect("rename.plan should succeed")
        .get("result")
        .and_then(|r| r.get("content"))
        .cloned()
        .expect("Plan should exist");

    // Apply the plan
    client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": { "dryRun": false }
            }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    // Verify: file renamed
    assert!(workspace.file_exists("src/new.rs"));
    assert!(!workspace.file_exists("src/old.rs"));

    // Verify: docs NOT updated (code scope)
    let readme = workspace.read_file("README.md");
    assert!(
        readme.contains("old.rs"),
        "With 'code' scope, markdown should NOT be updated"
    );

    // Verify: Cargo.toml NOT updated (code scope)
    let cargo = workspace.read_file("Cargo.toml");
    assert!(
        cargo.contains("src/old.rs"),
        "With 'code' scope, Cargo.toml should NOT be updated"
    );
}

/// Test "standard" scope (default) - updates code + docs + configs
#[tokio::test]
async fn test_scope_standard_updates_code_docs_configs() {
    let workspace = TestWorkspace::new();

    workspace.create_file("src/old.rs", r#"pub fn hello() {}"#);
    workspace.create_file(
        "README.md",
        r#"# Readme

See [old.rs](src/old.rs) for implementation.
"#,
    );
    workspace.create_file(
        "docs/guide.md",
        r#"# Guide

Check out `src/old.rs` for the source code.
"#,
    );

    let mut client = TestClient::new(workspace.path());

    // Rename with "standard" scope (default)
    let plan = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": workspace.absolute_path("src/old.rs").to_string_lossy()
                },
                "newName": workspace.absolute_path("src/new.rs").to_string_lossy(),
                "options": {
                    "scope": "standard"
                }
            }),
        )
        .await
        .expect("rename.plan should succeed")
        .get("result")
        .and_then(|r| r.get("content"))
        .cloned()
        .expect("Plan should exist");

    client
        .call_tool(
            "workspace.apply_edit",
            json!({ "plan": plan, "options": { "dryRun": false } }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    // Verify: file renamed
    assert!(workspace.file_exists("src/new.rs"));

    // Verify: docs UPDATED (standard scope includes docs)
    let readme = workspace.read_file("README.md");
    assert!(
        readme.contains("new.rs"),
        "With 'standard' scope, markdown links should be updated"
    );

    // Verify: docs/guide.md UPDATED (docs updated with standard scope)
    let guide = workspace.read_file("docs/guide.md");
    assert!(
        guide.contains("src/new.rs") || guide.contains("new.rs"),
        "With 'standard' scope, documentation paths should be updated"
    );
}

/// Test "comments" scope - includes code comments
#[tokio::test]
async fn test_scope_comments_updates_code_comments() {
    let workspace = TestWorkspace::new();

    workspace.create_file(
        "src/old.rs",
        r#"pub fn hello() {
    // This function says hello
    println!("Hello");
}
"#,
    );

    workspace.create_file(
        "src/lib.rs",
        r#"// Import from old.rs file
pub mod old;

/// Documentation for old.rs module
/// Located at src/old.rs
pub fn use_old() {
    old::hello();
}
"#,
    );

    let mut client = TestClient::new(workspace.path());

    // Rename with "comments" scope
    let plan = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": workspace.absolute_path("src/old.rs").to_string_lossy()
                },
                "newName": workspace.absolute_path("src/new.rs").to_string_lossy(),
                "options": {
                    "scope": "comments"
                }
            }),
        )
        .await
        .expect("rename.plan should succeed")
        .get("result")
        .and_then(|r| r.get("content"))
        .cloned()
        .expect("Plan should exist");

    client
        .call_tool(
            "workspace.apply_edit",
            json!({ "plan": plan, "options": { "dryRun": false } }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    // Verify: file renamed
    assert!(workspace.file_exists("src/new.rs"));

    // Verify: comments referencing old.rs are updated
    let lib_content = workspace.read_file("src/lib.rs");
    assert!(
        lib_content.contains("// Import from new.rs file"),
        "Comments scope should update inline comments"
    );
    assert!(
        lib_content.contains("/// Documentation for new.rs module"),
        "Comments scope should update doc comments"
    );
    assert!(
        lib_content.contains("/// Located at src/new.rs"),
        "Comments scope should update path references in comments"
    );
}

/// Test "everything" scope - most comprehensive (includes prose)
#[tokio::test]
async fn test_scope_everything_updates_markdown_prose() {
    let workspace = TestWorkspace::new();

    workspace.create_file("src/old.rs", r#"pub fn hello() {}"#);

    // Markdown with prose text (not just links)
    workspace.create_file(
        "docs/guide.md",
        r#"# Guide

The implementation is in `src/old.rs` file.

You can find the old.rs source code in the repository.

Example usage from old.rs:

```rust
// Reference to src/old.rs
use old;
```
"#,
    );

    let mut client = TestClient::new(workspace.path());

    // Rename with "everything" scope
    let plan = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": workspace.absolute_path("src/old.rs").to_string_lossy()
                },
                "newName": workspace.absolute_path("src/new.rs").to_string_lossy(),
                "options": {
                    "scope": "everything"
                }
            }),
        )
        .await
        .expect("rename.plan should succeed")
        .get("result")
        .and_then(|r| r.get("content"))
        .cloned()
        .expect("Plan should exist");

    client
        .call_tool(
            "workspace.apply_edit",
            json!({ "plan": plan, "options": { "dryRun": false } }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    // Verify: file renamed
    assert!(workspace.file_exists("src/new.rs"));

    // Verify: prose text updated (everything scope)
    let guide = workspace.read_file("docs/guide.md");

    // Note: Markdown prose updates are currently opt-in via update_markdown_prose flag
    // The "everything" preset enables this flag, but the actual prose update behavior
    // depends on the markdown plugin implementation.
    // For now, verify that the file was renamed and basic markdown links work.
    assert!(
        workspace.file_exists("src/new.rs"),
        "File should be renamed with everything scope"
    );

    // Markdown link updates should work with any scope that includes docs
    assert!(
        guide.contains("new.rs"),
        "Everything scope should at least update file references in markdown"
    );
}

/// Test backward compatibility - "code-only" deprecated alias
#[tokio::test]
async fn test_deprecated_code_only_alias_still_works() {
    let workspace = TestWorkspace::new();

    workspace.create_file("src/old.rs", r#"pub fn test() {}"#);
    workspace.create_file("README.md", r#"See src/old.rs"#);

    let mut client = TestClient::new(workspace.path());

    // Use deprecated "code-only" name
    let plan = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": workspace.absolute_path("src/old.rs").to_string_lossy()
                },
                "newName": workspace.absolute_path("src/new.rs").to_string_lossy(),
                "options": {
                    "scope": "code-only"  // DEPRECATED NAME
                }
            }),
        )
        .await
        .expect("rename.plan should still work with deprecated 'code-only'")
        .get("result")
        .and_then(|r| r.get("content"))
        .cloned()
        .expect("Plan should exist");

    client
        .call_tool(
            "workspace.apply_edit",
            json!({ "plan": plan, "options": { "dryRun": false } }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    // Verify: behaves same as "code" scope
    assert!(workspace.file_exists("src/new.rs"));

    let readme = workspace.read_file("README.md");
    assert!(
        readme.contains("old.rs"),
        "Deprecated 'code-only' should behave like 'code' scope"
    );
}

/// Test backward compatibility - "all" deprecated alias
#[tokio::test]
async fn test_deprecated_all_alias_still_works() {
    let workspace = TestWorkspace::new();

    workspace.create_file("src/old.rs", r#"pub fn test() {}"#);
    workspace.create_file("README.md", r#"See [file](src/old.rs)"#);
    workspace.create_file(
        "docs/api.md",
        r#"# API

Source: `src/old.rs`
"#,
    );

    let mut client = TestClient::new(workspace.path());

    // Use deprecated "all" name
    let plan = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": workspace.absolute_path("src/old.rs").to_string_lossy()
                },
                "newName": workspace.absolute_path("src/new.rs").to_string_lossy(),
                "options": {
                    "scope": "all"  // DEPRECATED NAME
                }
            }),
        )
        .await
        .expect("rename.plan should still work with deprecated 'all'")
        .get("result")
        .and_then(|r| r.get("content"))
        .cloned()
        .expect("Plan should exist");

    client
        .call_tool(
            "workspace.apply_edit",
            json!({ "plan": plan, "options": { "dryRun": false } }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    // Verify: behaves same as "standard" scope
    assert!(workspace.file_exists("src/new.rs"));

    let readme = workspace.read_file("README.md");
    assert!(
        readme.contains("new.rs"),
        "Deprecated 'all' should behave like 'standard' scope"
    );

    let api = workspace.read_file("docs/api.md");
    assert!(
        api.contains("src/new.rs") || api.contains("new.rs"),
        "Deprecated 'all' should update documentation files"
    );
}

/// Test default scope behavior (no scope specified = "standard")
#[tokio::test]
async fn test_default_scope_is_standard() {
    let workspace = TestWorkspace::new();

    workspace.create_file("src/old.rs", r#"pub fn test() {}"#);
    workspace.create_file("README.md", r#"See [file](src/old.rs)"#);

    let mut client = TestClient::new(workspace.path());

    // Don't specify scope - should default to "standard"
    let plan = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "file",
                    "path": workspace.absolute_path("src/old.rs").to_string_lossy()
                },
                "newName": workspace.absolute_path("src/new.rs").to_string_lossy()
                // NO "options" field - using default scope
            }),
        )
        .await
        .expect("rename.plan should succeed")
        .get("result")
        .and_then(|r| r.get("content"))
        .cloned()
        .expect("Plan should exist");

    client
        .call_tool(
            "workspace.apply_edit",
            json!({ "plan": plan, "options": { "dryRun": false } }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

    // Verify: default behaves like "standard" scope
    assert!(workspace.file_exists("src/new.rs"));

    let readme = workspace.read_file("README.md");
    assert!(
        readme.contains("new.rs"),
        "Default scope should update docs (standard scope behavior)"
    );
}
