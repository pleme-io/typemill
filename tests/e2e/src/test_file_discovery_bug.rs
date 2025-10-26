//! Test to reproduce the file discovery bug (MIGRATED VERSION)
//!
//! BEFORE: 180 lines with manual TestWorkspace/TestClient setup
//! AFTER: Using shared helpers from test_helpers.rs where applicable
//!
//! The real codebase has files in various locations (apps/, examples/, docs/, proposals/)
//! that contain imports but aren't being discovered by the reference updater.
//!
//! This is a REGRESSION TEST that validates the bug fix for file discovery filtering.
//! The bug was caused by should_skip_file_for_examples() filtering out these files
//! before they could be added to the plan, bypassing RenameScope settings.
//!
//! NOTE: This test requires manual approach due to complex validation of plan contents.

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

/// Test that files in non-standard locations are discovered during rename
///
/// This is a regression test for the bug where:
/// - apps/mill/tests/e2e_analysis_features.rs had imports but wasn't found
/// - examples/tests/data_driven_fixture_example.rs had imports but wasn't found
/// - docs/*.md have references but weren't found
/// - proposals/*.md have references but weren't found
///
/// The bug was caused by should_skip_file_for_examples() filtering out these files
/// before they could be added to the plan, bypassing RenameScope settings.
#[tokio::test]
async fn test_file_discovery_in_non_standard_locations() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create source crate
    workspace.create_directory("crates/my-crate/src");
    workspace.create_file(
        "crates/my-crate/Cargo.toml",
        r#"[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"
"#,
    );
    workspace.create_file(
        "crates/my-crate/src/lib.rs",
        r#"pub fn helper() -> i32 { 42 }"#,
    );

    // Create root Cargo.toml
    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = ["crates/my-crate", "apps/client-app"]
resolver = "2"
"#,
    );

    // Create app with tests directory (mirrors apps/mill structure)
    workspace.create_directory("apps/client-app/tests");
    workspace.create_file(
        "apps/client-app/Cargo.toml",
        r#"[package]
name = "client-app"
version = "0.1.0"
edition = "2021"

[dependencies]
my-crate = { path = "../../crates/my-crate" }
"#,
    );
    workspace.create_file(
        "apps/client-app/tests/feature_test.rs",
        r#"use my_crate::helper;

#[test]
fn test_feature() {
    assert_eq!(helper(), 42);
}
"#,
    );

    // Create examples directory (mirrors examples/ structure)
    workspace.create_directory("examples/tests");
    workspace.create_file(
        "examples/tests/example_test.rs",
        r#"use my_crate::helper;

fn main() {
    println!("Result: {}", helper());
}
"#,
    );

    // Create docs with markdown references (using FILE PATHS, not just identifiers)
    workspace.create_directory("docs");
    workspace.create_file(
        "docs/guide.md",
        r#"# Guide

See the documentation at [README](crates/my-crate/README.md).

Example source code: `crates/my-crate/src/lib.rs`
"#,
    );

    // Create proposals directory
    workspace.create_directory("proposals");
    workspace.create_file(
        "proposals/01_feature.md",
        r#"# Feature Proposal

This feature is located in `crates/my-crate/src/` directory.

Configuration: `crates/my-crate/Cargo.toml`
"#,
    );

    // Generate rename plan (dry run to inspect changes)
    let plan_result = client
        .call_tool(
            "rename",
            json!({
                "target": {
                    "kind": "directory",
                    "path": workspace.absolute_path("crates/my-crate").to_string_lossy()
                },
                "newName": workspace.absolute_path("crates/renamed-crate").to_string_lossy(),
                "options": {
                    "dryRun": true
                }
            }),
        )
        .await
        .expect("rename should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    // Extract all files that will be updated
    let files_in_plan: Vec<String> = plan
        .get("edits")
        .and_then(|e| e.get("documentChanges"))
        .and_then(|dc| dc.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    item.get("textDocument")
                        .and_then(|td| td.get("uri"))
                        .and_then(|uri| uri.as_str())
                        .map(|s| {
                            s.replace("file://", "")
                                .replace(&workspace.path().to_string_lossy().to_string(), "")
                        })
                })
                .collect()
        })
        .unwrap_or_default();

    println!("\n=== Files in plan ({}) ===", files_in_plan.len());
    for file in &files_in_plan {
        println!("  {}", file);
    }

    // CRITICAL ASSERTIONS: These files MUST be in the plan

    // 1. App test file (mirrors apps/mill/tests/e2e_analysis_features.rs)
    assert!(
        files_in_plan
            .iter()
            .any(|f| f.contains("apps/client-app/tests/feature_test.rs")),
        "❌ BUG: apps/client-app/tests/feature_test.rs not in plan (has Rust import!)"
    );

    // 2. Examples file
    assert!(
        files_in_plan
            .iter()
            .any(|f| f.contains("examples/tests/example_test.rs")),
        "❌ BUG: examples/tests/example_test.rs not in plan"
    );

    // 3. Docs markdown file
    assert!(
        files_in_plan.iter().any(|f| f.contains("docs/guide.md")),
        "❌ BUG: docs/guide.md not in plan"
    );

    // 4. Proposals markdown file
    assert!(
        files_in_plan
            .iter()
            .any(|f| f.contains("proposals/01_feature.md")),
        "❌ BUG: proposals/01_feature.md not in plan"
    );

    println!("✅ All files discovered correctly!");
}
