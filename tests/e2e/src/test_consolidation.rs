//! Rust crate consolidation tests (CONSOLIDATED VERSION)
//!
//! BEFORE: 2 files with 631 total lines and duplicated setup
//! AFTER: Single file with shared helpers (~400 lines)
//!
//! Consolidation combines these test scenarios:
//! - Metadata & Auto-Detection (from test_consolidation_metadata.rs)
//! - Execution & Bug Fixes (from test_consolidation_bug_fix.rs)
//!
//! Consolidation merges one crate into another's src/ directory:
//! 1. Moves source-crate/src/* into target-crate/src/module/*
//! 2. Merges dependencies from source Cargo.toml into target Cargo.toml
//! 3. Removes source crate from workspace members
//! 4. Updates all imports across workspace
//! 5. Deletes the source crate directory

use crate::harness::{TestClient, TestWorkspace};
use crate::test_helpers::*;
use serde_json::json;

// ============================================================================
// Shared Helper Functions
// ============================================================================

/// Helper to setup standard consolidation test workspace
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

/// Helper to validate consolidation metadata flag and warning
fn validate_consolidation_metadata(
    plan: &serde_json::Value,
    expected_consolidation: bool,
) -> anyhow::Result<()> {
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

// ============================================================================
// Section 1: Metadata & Auto-Detection Tests
// ============================================================================

/// Test that is_consolidation flag is set when explicitly requested
#[tokio::test]
async fn test_consolidation_flag_explicit() {
    let workspace = TestWorkspace::new();
    setup_consolidation_workspace(&workspace);

    let mut client = TestClient::new(workspace.path());

    // Generate consolidation plan with explicit flag
    let plan_result = client
        .call_tool(
            "rename_all",
            json!({
                "target": {
                    "kind": "directory",
                    "filePath": workspace.absolute_path("crates/lib").to_string_lossy()
                },
                "newName": workspace.absolute_path("crates/app/src/lib_mod").to_string_lossy(),
                "options": {
                    "consolidate": true,
                    "dryRun": true
                }
            }),
        )
        .await
        .expect("Rename should succeed");

    // M7 response: Tool returns {"content": WriteResponse}, and WriteResponse has status, summary, filesChanged, diagnostics, changes
    let result = plan_result.get("result").expect("Result should exist");
    let response = result.get("content").expect("Content should exist");
    assert_eq!(
        response.get("status").and_then(|v| v.as_str()),
        Some("preview"),
        "Dry run should return preview status"
    );

    let plan = response.get("changes").expect("Plan should exist in changes field");

    // Validate consolidation metadata
    validate_consolidation_metadata(plan, true).unwrap();

    println!("✓ Consolidation flag and warning correctly set with explicit consolidate: true");
}

/// Test that is_consolidation is auto-detected for typical patterns
#[tokio::test]
async fn test_consolidation_auto_detection() {
    let workspace = TestWorkspace::new();
    setup_consolidation_workspace(&workspace);

    let mut client = TestClient::new(workspace.path());

    // Generate plan WITHOUT explicit consolidate flag
    // Auto-detection should kick in: source has Cargo.toml, target is inside another crate's src/
    let plan_result = client
        .call_tool(
            "rename_all",
            json!({
                "target": {
                    "kind": "directory",
                    "filePath": workspace.absolute_path("crates/lib").to_string_lossy()
                },
                "newName": workspace.absolute_path("crates/app/src/lib_mod").to_string_lossy(),
                "options": {
                    "dryRun": true  // Get plan to inspect metadata
                }
                // NOTE: No "options.consolidate" specified - auto-detection should work
            }),
        )
        .await
        .expect("Rename should succeed");

    // M7 response: Tool returns {"content": WriteResponse}, and WriteResponse has status, summary, filesChanged, diagnostics, changes
    let result = plan_result.get("result").expect("Result should exist");
    let response = result.get("content").expect("Content should exist");
    assert_eq!(
        response.get("status").and_then(|v| v.as_str()),
        Some("preview"),
        "Dry run should return preview status"
    );

    let plan = response.get("changes").expect("Plan should exist in changes field");

    // Verify is_consolidation was auto-detected
    validate_consolidation_metadata(plan, true).unwrap();

    println!("✓ Consolidation auto-detected correctly (source=crate, target=inside src/)");
}

/// Test that explicit consolidate: false overrides auto-detection
#[tokio::test]
async fn test_consolidation_override_auto_detection() {
    let workspace = TestWorkspace::new();
    setup_consolidation_workspace(&workspace);

    let mut client = TestClient::new(workspace.path());

    // Generate plan with explicit consolidate: false to override auto-detection
    let plan_result = client
        .call_tool(
            "rename_all",
            json!({
                "target": {
                    "kind": "directory",
                    "filePath": workspace.absolute_path("crates/lib").to_string_lossy()
                },
                "newName": workspace.absolute_path("crates/app/src/lib_mod").to_string_lossy(),
                "options": {
                    "consolidate": false,  // Explicitly disable consolidation
                    "dryRun": true
                }
            }),
        )
        .await
        .expect("Rename should succeed");

    // M7 response: Tool returns {"content": WriteResponse}, and WriteResponse has status, summary, filesChanged, diagnostics, changes
    let result = plan_result.get("result").expect("Result should exist");
    let response = result.get("content").expect("Content should exist");
    assert_eq!(
        response.get("status").and_then(|v| v.as_str()),
        Some("preview"),
        "Dry run should return preview status"
    );

    let plan = response.get("changes").expect("Plan should exist in changes field");

    // Verify is_consolidation is false (override worked)
    validate_consolidation_metadata(plan, false).unwrap();

    println!("✓ Explicit consolidate=false correctly overrides auto-detection");
}

/// Test that is_consolidation is false for non-consolidation renames
#[tokio::test]
async fn test_non_consolidation_rename() {
    run_tool_test_with_plan_validation(
        &[("src/old_dir/file.rs", "pub fn test() {}")],
        "rename_all",
        |ws| {
            json!({
                "target": {
                    "kind": "directory",
                    "filePath": ws.absolute_path("src/old_dir").to_string_lossy()
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

            assert!(
                !is_consolidation,
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
        },
    )
    .await
    .unwrap();
}

// ============================================================================
// Section 2: Execution & Bug Fixes Tests
// ============================================================================

/// Test consolidation plan generation - verifies Cargo.toml files are NOT in the plan
/// This test verifies the bug fix: Cargo.toml updates should NOT appear in generic
/// path update edits. They're handled separately via semantic updates.
#[tokio::test]
async fn test_consolidation_plan_excludes_cargo_toml() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create root workspace Cargo.toml
    workspace.create_file(
        "Cargo.toml",
        r#"
[workspace]
members = [
    "crates/app",
    "crates/lib",
]

[workspace.dependencies]
serde = "1.0"
"#,
    );

    // Create lib crate (to be consolidated)
    workspace.create_directory("crates/lib/src");
    workspace.create_file(
        "crates/lib/Cargo.toml",
        r#"
[package]
name = "lib"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
"#,
    );
    workspace.create_file(
        "crates/lib/src/lib.rs",
        "pub fn helper() { println!(\"hello\"); }",
    );

    // Create app crate (consolidation target)
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
serde = { workspace = true }
"#,
    );
    workspace.create_file(
        "crates/app/src/main.rs",
        r#"
fn main() {
    lib::helper();
}
"#,
    );

    // Generate consolidation plan: crates/lib → crates/app/src/lib_mod
    let plan_result = client
        .call_tool(
            "rename_all",
            json!({
                "target": {
                    "kind": "directory",
                    "filePath": workspace.absolute_path("crates/lib").to_string_lossy()
                },
                "newName": workspace.absolute_path("crates/app/src/lib_mod").to_string_lossy(),
                "options": {
                    "consolidate": true,
                    "dryRun": true
                }
            }),
        )
        .await
        .expect("Rename should succeed");

    // M7 response: Tool returns {"content": WriteResponse}, and WriteResponse has status, summary, filesChanged, diagnostics, changes
    let result = plan_result.get("result").expect("Result should exist");
    let response = result.get("content").expect("Content should exist");
    assert_eq!(
        response.get("status").and_then(|v| v.as_str()),
        Some("preview"),
        "Dry run should return preview status"
    );

    let plan = response.get("changes").expect("Plan should exist in changes field");

    // Debug: print the plan metadata
    if let Some(metadata) = plan.get("metadata") {
        println!(
            "Plan metadata: {}",
            serde_json::to_string_pretty(metadata).unwrap()
        );
    }

    // Extract the edits array
    let edits = plan
        .get("edits")
        .and_then(|e| e.get("documentChanges"))
        .and_then(|dc| dc.as_array())
        .expect("documentChanges should be an array");

    // Debug: print all Cargo.toml edit URIs
    println!("\nAll edits in plan:");
    for (i, edit) in edits.iter().enumerate() {
        if let Some(text_doc) = edit.get("textDocument") {
            if let Some(uri) = text_doc.get("uri").and_then(|u| u.as_str()) {
                println!("  Edit {}: {}", i, uri);
            }
        } else if let Some(kind) = edit.get("kind") {
            println!("  Edit {}: {:?} operation", i, kind);
        }
    }

    // CRITICAL ASSERTION 1: Cargo.toml files should NOT be in the generic path update edits
    // Count how many edits affect Cargo.toml files
    let cargo_toml_edits: Vec<_> = edits
        .iter()
        .filter(|edit| {
            if let Some(text_doc) = edit.get("textDocument") {
                if let Some(uri) = text_doc.get("uri").and_then(|u| u.as_str()) {
                    return uri.ends_with("Cargo.toml");
                }
            }
            false
        })
        .collect();

    // We expect ZERO generic Cargo.toml edits in the plan
    // (The semantic Cargo.toml updates happen during execution, not in the plan)
    assert_eq!(
        cargo_toml_edits.len(),
        0,
        "Consolidation plan should NOT contain Cargo.toml edits from generic path updates. \
         Found {} Cargo.toml edits. This indicates the filter is not working.",
        cargo_toml_edits.len()
    );

    // ASSERTION 2: The plan MAY contain code file updates (use statements)
    // Note: For consolidation, import updates might happen during execution rather than planning
    let code_file_edits: Vec<_> = edits
        .iter()
        .filter(|edit| {
            if let Some(text_doc) = edit.get("textDocument") {
                if let Some(uri) = text_doc.get("uri").and_then(|u| u.as_str()) {
                    return uri.ends_with("main.rs") || uri.ends_with("lib.rs");
                }
            }
            false
        })
        .collect();

    println!("  Plan contains {} code file edits", code_file_edits.len());

    // ASSERTION 3: Verify the plan has a rename/move operation
    let has_rename_op = edits.iter().any(|edit| edit.get("kind").is_some());
    assert!(has_rename_op, "Plan should contain a rename/move operation");

    println!(
        "✓ Consolidation plan correctly excludes Cargo.toml files (found: {})",
        cargo_toml_edits.len()
    );
}

/// Test consolidation execution - verifies workspace members and dependencies are correct
/// This test verifies the bug fix worked end-to-end:
/// - Workspace members should NOT contain nested paths like "crates/app/src/lib_mod"
/// - App's Cargo.toml should NOT have incorrect path dependencies
#[tokio::test]
async fn test_consolidation_execution_correctness() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create root workspace Cargo.toml
    workspace.create_file(
        "Cargo.toml",
        r#"
[workspace]
members = [
    "crates/app",
    "crates/lib",
]

[workspace.dependencies]
serde = "1.0"
"#,
    );

    // Create lib crate (to be consolidated)
    workspace.create_directory("crates/lib/src");
    workspace.create_file(
        "crates/lib/Cargo.toml",
        r#"
[package]
name = "lib"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
"#,
    );
    workspace.create_file(
        "crates/lib/src/lib.rs",
        "pub fn helper() { println!(\"hello\"); }",
    );

    // Create app crate (consolidation target)
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
serde = { workspace = true }
"#,
    );
    workspace.create_file(
        "crates/app/src/main.rs",
        r#"
fn main() {
    lib::helper();
}
"#,
    );

    // Generate and apply consolidation plan
    let plan_result = client
        .call_tool(
            "rename_all",
            json!({
                "target": {
                    "kind": "directory",
                    "filePath": workspace.absolute_path("crates/lib").to_string_lossy()
                },
                "newName": workspace.absolute_path("crates/app/src/lib_mod").to_string_lossy(),
                "options": {
                    "consolidate": true
                }
            }),
        )
        .await
        .expect("Rename should succeed");

    // Apply the plan with unified API (dryRun: false)
    let params_exec = json!({
        "target": {
            "kind": "directory",
            "filePath": workspace.absolute_path("crates/lib").to_string_lossy()
        },
        "newName": workspace.absolute_path("crates/app/src/lib_mod").to_string_lossy(),
        "options": {
            "consolidate": true,
            "dryRun": false
        }
    });

    client
        .call_tool("rename_all", params_exec)
        .await
        .expect("Apply should succeed");

    // VERIFICATION 1: Root workspace members should NOT contain nested path
    let root_cargo = workspace.read_file("Cargo.toml");

    // Debug: print the actual Cargo.toml content
    println!("\n=== ROOT CARGO.TOML AFTER CONSOLIDATION ===");
    println!("{}", root_cargo);
    println!("===========================================\n");

    assert!(
        !root_cargo.contains("crates/app/src/lib_mod"),
        "Root Cargo.toml should NOT contain nested module path in workspace members. \
         This was bug symptom #1."
    );
    assert!(
        root_cargo.contains("crates/app"),
        "Root Cargo.toml should still contain the target crate"
    );
    assert!(
        !root_cargo.contains("crates/lib") || root_cargo.contains("# crates/lib"),
        "Root Cargo.toml should have removed or commented out the old lib crate. \
         Actual content: {}",
        root_cargo
    );

    // VERIFICATION 2: App's Cargo.toml should NOT have incorrect path dependency
    let app_cargo = workspace.read_file("crates/app/Cargo.toml");

    // Debug: print the app's Cargo.toml
    println!("\n=== APP CARGO.TOML AFTER CONSOLIDATION ===");
    println!("{}", app_cargo);
    println!("==========================================\n");

    assert!(
        !app_cargo.contains("path = \"../lib\"") && !app_cargo.contains("path = \"src/lib_mod\""),
        "App's Cargo.toml should NOT contain path dependencies to the consolidated crate. \
         This was bug symptom #2. The dependency should resolve via workspace. Actual content: {}",
        app_cargo
    );

    // VERIFICATION 3: Code should have updated imports
    let main_rs = workspace.read_file("crates/app/src/main.rs");
    assert!(
        main_rs.contains("app::lib_mod::helper") || main_rs.contains("use app::lib_mod"),
        "main.rs should have updated import to use app::lib_mod path"
    );

    // VERIFICATION 4: Consolidated files should exist in new location
    // Note: lib.rs gets renamed to mod.rs for directory modules
    assert!(
        workspace.file_exists("crates/app/src/lib_mod/mod.rs"),
        "Consolidated mod.rs should exist in new location (renamed from lib.rs)"
    );

    // VERIFICATION 5: Old directory should be gone
    assert!(
        !workspace.file_exists("crates/lib/Cargo.toml"),
        "Old lib crate directory should be deleted"
    );

    println!("✓ All consolidation correctness checks passed");
    println!("  - Workspace members correctly updated");
    println!("  - No incorrect path dependencies created");
    println!("  - Import statements updated");
    println!("  - Files moved to correct location");
}
