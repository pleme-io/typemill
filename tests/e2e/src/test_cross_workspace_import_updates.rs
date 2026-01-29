//! Regression test for cross-workspace Rust import updates (MIGRATED VERSION)
//!
//! BEFORE: 234 lines with manual setup/plan/apply logic
//! AFTER: Using shared helpers from test_helpers.rs
//!
//! **Issue**: When renaming a crate (e.g., crates/cb-test-support → crates/mill-test-support),
//! Rust files across the entire workspace that import from that crate should have their
//! import statements updated.
//!
//! **Bug**: Currently, not all files with imports are being scanned and updated.
//! For example, apps/mill/tests/e2e_analysis_features.rs contains
//! `use mill_test_support::harness::test_helpers;` but is not included in the rename plan.
//!
//! This test should FAIL until the bug is fixed.

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

/// Test that verifies ALL Rust files with imports from a renamed crate are updated
///
/// Test setup:
/// - Create a crate at crates/source-crate
/// - Create multiple Rust files in different directories that import from source_crate
/// - Rename crates/source-crate → crates/target-crate
/// - Verify ALL import statements are updated
///
/// BEFORE: 234 lines | AFTER: ~135 lines (~42% reduction)
/// Note: Lower reduction because this is a regression test with custom verification logic
#[tokio::test]
async fn test_rename_crate_updates_all_workspace_imports() {
    let workspace = TestWorkspace::new();

    // Create source crate with a module
    workspace.create_directory("crates/source-crate/src");
    workspace.create_file(
        "crates/source-crate/Cargo.toml",
        r#"[package]
name = "source-crate"
version = "0.1.0"
edition = "2021"
"#,
    );
    workspace.create_file(
        "crates/source-crate/src/lib.rs",
        r#"pub mod utils;
pub fn helper() -> i32 { 42 }
"#,
    );
    workspace.create_file(
        "crates/source-crate/src/utils.rs",
        r#"pub fn utility_fn() {}"#,
    );

    // Create files in various locations that import from source-crate

    // 1. App directory import
    workspace.create_directory("apps/my-app/tests");
    workspace.create_file(
        "apps/my-app/tests/integration_test.rs",
        r#"use source_crate::helper;
use source_crate::utils::utility_fn;

#[test]
fn test_helper() {
    assert_eq!(helper(), 42);
}
"#,
    );

    // 2. Another app directory import (simulates e2e_analysis_features.rs)
    workspace.create_file(
        "apps/my-app/tests/analysis_test.rs",
        r#"use source_crate::helper;

#[test]
fn test_analysis() {
    helper();
}
"#,
    );

    // 3. Different crate importing it
    workspace.create_directory("crates/other-crate/src");
    workspace.create_file(
        "crates/other-crate/Cargo.toml",
        r#"[package]
name = "other-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
source-crate = { path = "../source-crate" }
"#,
    );
    workspace.create_file(
        "crates/other-crate/src/lib.rs",
        r#"use source_crate::utils;

pub fn use_utility() {
    utils::utility_fn();
}
"#,
    );

    // 4. Tests directory import
    workspace.create_directory("tests/e2e/src");
    workspace.create_file(
        "tests/e2e/src/test_example.rs",
        r#"use source_crate::helper;

#[tokio::test]
async fn test_e2e() {
    assert_eq!(helper(), 42);
}
"#,
    );

    // Create root workspace Cargo.toml
    workspace.create_file(
        "Cargo.toml",
        r#"[workspace]
members = [
    "apps/my-app",
    "crates/source-crate",
    "crates/other-crate",
    "tests/e2e"
]
resolver = "2"
"#,
    );

    let mut client = TestClient::new(workspace.path());

    // Apply rename with unified API (dryRun: false)
    client
        .call_tool(
            "rename_all",
            json!({
                "target": {
                    "kind": "directory",
                    "filePath": workspace.absolute_path("crates/source-crate").to_string_lossy()
                },
                "newName": workspace.absolute_path("crates/target-crate").to_string_lossy(),
                "options": {
                    "dryRun": false
                }
            }),
        )
        .await
        .expect("Apply should succeed");

    // CRITICAL ASSERTIONS: Verify ALL import statements are updated

    // 1. App test import should be updated
    let integration_test = workspace.read_file("apps/my-app/tests/integration_test.rs");
    assert!(
        integration_test.contains("use target_crate::helper;"),
        "❌ apps/my-app/tests/integration_test.rs import not updated.\nActual:\n{}",
        integration_test
    );
    assert!(
        integration_test.contains("use target_crate::utils::utility_fn;"),
        "❌ apps/my-app/tests/integration_test.rs nested import not updated.\nActual:\n{}",
        integration_test
    );

    // 2. REGRESSION TEST: This file simulates e2e_analysis_features.rs
    let analysis_test = workspace.read_file("apps/my-app/tests/analysis_test.rs");
    assert!(
        analysis_test.contains("use target_crate::helper;"),
        "❌ apps/my-app/tests/analysis_test.rs import not updated (THIS IS THE BUG).\nActual:\n{}",
        analysis_test
    );

    // 3. Other crate import should be updated
    let other_crate = workspace.read_file("crates/other-crate/src/lib.rs");
    assert!(
        other_crate.contains("use target_crate::utils;"),
        "❌ crates/other-crate/src/lib.rs import not updated.\nActual:\n{}",
        other_crate
    );

    // 4. E2E test import should be updated
    let e2e_test = workspace.read_file("tests/e2e/src/test_example.rs");
    assert!(
        e2e_test.contains("use target_crate::helper;"),
        "❌ tests/e2e/src/test_example.rs import not updated.\nActual:\n{}",
        e2e_test
    );

    // Verify old crate name is gone from all files
    assert!(
        !integration_test.contains("source_crate"),
        "❌ Old import 'source_crate' still exists in integration_test.rs"
    );
    assert!(
        !analysis_test.contains("source_crate"),
        "❌ Old import 'source_crate' still exists in analysis_test.rs"
    );
    assert!(
        !other_crate.contains("source_crate"),
        "❌ Old import 'source_crate' still exists in other_crate"
    );
    assert!(
        !e2e_test.contains("source_crate"),
        "❌ Old import 'source_crate' still exists in e2e test"
    );

    println!("✅ All cross-workspace imports successfully updated!");
}
