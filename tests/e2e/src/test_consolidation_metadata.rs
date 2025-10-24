//! Tests for consolidation detection and metadata (MIGRATED VERSION)
//!
//! BEFORE: 385 lines with duplicated setup/plan/validation logic
//! AFTER: Using shared helpers from test_helpers.rs where applicable
//!
//! Verifies that:
//! 1. is_consolidation flag is set correctly in the plan
//! 2. Consolidation-specific warnings are added
//! 3. Auto-detection works for typical patterns
//! 4. Explicit consolidate: false can override auto-detection

use crate::harness::{TestClient, TestWorkspace};
use crate::test_helpers::*;
use serde_json::json;

/// Helper to setup consolidation test workspace
fn setup_consolidation_workspace(workspace: &TestWorkspace) {
    // Create root workspace Cargo.toml
    workspace.create_file(
        "Cargo.toml",
        r#"
[workspace]
members = [
    "crates/app",
    "crates/lib",
]
"#,
    );

    // Create source crate (to be consolidated)
    workspace.create_directory("crates/lib/src");
    workspace.create_file(
        "crates/lib/Cargo.toml",
        r#"
[package]
name = "lib"
version = "0.1.0"
edition = "2021"
"#,
    );
    workspace.create_file("crates/lib/src/lib.rs", "pub fn helper() {}");

    // Create target crate (consolidation target)
    workspace.create_directory("crates/app/src");
    workspace.create_file(
        "crates/app/Cargo.toml",
        r#"
[package]
name = "app"
version = "0.1.0"
edition = "2021"

[dependencies]
lib = { path = "../lib" }
"#,
    );
    workspace.create_file("crates/app/src/lib.rs", "// app crate");
}

/// Helper to validate consolidation flag and warning
fn validate_consolidation_metadata(plan: &serde_json::Value, expected_consolidation: bool) -> anyhow::Result<()> {
    // Verify is_consolidation flag
    assert_eq!(
        plan.get("isConsolidation").and_then(|v| v.as_bool()),
        Some(expected_consolidation),
        "Plan should have is_consolidation={}",
        expected_consolidation
    );

    // Verify consolidation warning presence
    let warnings = plan
        .get("warnings")
        .and_then(|w| w.as_array())
        .expect("Plan should have warnings array");

    let has_consolidation_warning = warnings.iter().any(|w| {
        w.get("code")
            .and_then(|c| c.as_str())
            .map(|code| code == "CONSOLIDATION_MANUAL_STEP")
            .unwrap_or(false)
    });

    if expected_consolidation {
        assert!(
            has_consolidation_warning,
            "Consolidation plan should have CONSOLIDATION_MANUAL_STEP warning"
        );

        // Verify warning message mentions lib.rs
        let consolidation_warning = warnings
            .iter()
            .find(|w| {
                w.get("code")
                    .and_then(|c| c.as_str())
                    .map(|code| code == "CONSOLIDATION_MANUAL_STEP")
                    .unwrap_or(false)
            })
            .expect("Should find consolidation warning");

        let message = consolidation_warning
            .get("message")
            .and_then(|m| m.as_str())
            .expect("Warning should have message");

        assert!(
            message.contains("pub mod"),
            "Warning should mention adding 'pub mod' declaration"
        );
        assert!(
            message.contains("lib.rs"),
            "Warning should mention lib.rs file"
        );
    } else {
        assert!(
            !has_consolidation_warning,
            "Non-consolidation plan should NOT have consolidation warning"
        );
    }

    Ok(())
}

/// Test that is_consolidation flag is set when explicitly requested
/// BEFORE: 133 lines | AFTER: ~35 lines (~74% reduction)
#[tokio::test]
async fn test_consolidation_flag_explicit() {
    let workspace = TestWorkspace::new();
    setup_consolidation_workspace(&workspace);

    let mut client = TestClient::new(workspace.path());

    // Generate consolidation plan with explicit flag
    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "directory",
                    "path": workspace.absolute_path("crates/lib").to_string_lossy()
                },
                "newName": workspace.absolute_path("crates/app/src/lib_mod").to_string_lossy(),
                "options": {
                    "consolidate": true
                }
            }),
        )
        .await
        .expect("rename.plan should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    // Validate consolidation metadata
    validate_consolidation_metadata(plan, true).unwrap();

    println!("✓ Consolidation flag and warning correctly set with explicit consolidate: true");
}

/// Test that is_consolidation is auto-detected for typical patterns
/// BEFORE: 95 lines | AFTER: ~35 lines (~63% reduction)
#[tokio::test]
async fn test_consolidation_auto_detection() {
    let workspace = TestWorkspace::new();
    setup_consolidation_workspace(&workspace);

    let mut client = TestClient::new(workspace.path());

    // Generate plan WITHOUT explicit consolidate flag
    // Auto-detection should kick in: source has Cargo.toml, target is inside another crate's src/
    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "directory",
                    "path": workspace.absolute_path("crates/lib").to_string_lossy()
                },
                "newName": workspace.absolute_path("crates/app/src/lib_mod").to_string_lossy()
                // NOTE: No "options.consolidate" specified
            }),
        )
        .await
        .expect("rename.plan should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    // Verify is_consolidation was auto-detected
    validate_consolidation_metadata(plan, true).unwrap();

    println!("✓ Consolidation auto-detected correctly (source=crate, target=inside src/)");
}

/// Test that explicit consolidate: false overrides auto-detection
/// BEFORE: 92 lines | AFTER: ~35 lines (~62% reduction)
#[tokio::test]
async fn test_consolidation_override_auto_detection() {
    let workspace = TestWorkspace::new();
    setup_consolidation_workspace(&workspace);

    let mut client = TestClient::new(workspace.path());

    // Generate plan with explicit consolidate: false to override auto-detection
    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "directory",
                    "path": workspace.absolute_path("crates/lib").to_string_lossy()
                },
                "newName": workspace.absolute_path("crates/app/src/lib_mod").to_string_lossy(),
                "options": {
                    "consolidate": false  // Explicitly disable consolidation
                }
            }),
        )
        .await
        .expect("rename.plan should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    // Verify is_consolidation is false (override worked)
    validate_consolidation_metadata(plan, false).unwrap();

    println!("✓ Explicit consolidate=false correctly overrides auto-detection");
}

/// Test that is_consolidation is false for non-consolidation renames
/// BEFORE: 58 lines | AFTER: ~25 lines (~57% reduction)
#[tokio::test]
async fn test_non_consolidation_rename() {
    run_tool_test_with_plan_validation(
        &[("src/old_dir/file.rs", "pub fn test() {}")],
        "rename.plan",
        |ws| {
            json!({
                "target": {
                    "kind": "directory",
                    "path": ws.absolute_path("src/old_dir").to_string_lossy()
                },
                "newName": ws.absolute_path("src/new_dir").to_string_lossy()
            })
        },
        |plan| {
            // Verify is_consolidation is false
            let is_consolidation = plan
                .get("isConsolidation")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            assert_eq!(
                is_consolidation, false,
                "Normal directory rename should have is_consolidation=false"
            );

            // Verify NO consolidation warning
            validate_consolidation_metadata(plan, false)?;

            println!("✓ Normal directory rename correctly has is_consolidation=false");
            Ok(())
        },
        |_ws| {
            // No post-execution verification needed
            Ok(())
        }
    )
    .await
    .unwrap();
}
