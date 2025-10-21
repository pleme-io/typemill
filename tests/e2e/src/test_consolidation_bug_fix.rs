//! Integration test for consolidation bug fix
//!
//! This test verifies that the consolidation bug has been fixed correctly.
//! The bug had two symptoms:
//! 1. Incorrect workspace members (nested modules like "crates/app/src/lib_mod" were added)
//! 2. Incorrect path dependencies (other crates got path = "../app/src/lib_mod")
//!
//! The fix involves three components:
//! 1. Auto-detect consolidation moves (paths ending in src/something)
//! 2. Filter Cargo.toml from generic path updates during consolidation
//! 3. Remove path attributes from dependencies (force workspace resolution)

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

/// Test consolidation plan generation - verifies Cargo.toml files are NOT in the plan
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
            "rename.plan",
            json!({
                "target": {
                    "kind": "directory",
                    "path": workspace.absolute_path("crates/lib").to_string_lossy()
                },
                "new_name": workspace.absolute_path("crates/app/src/lib_mod").to_string_lossy(),
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
            "rename.plan",
            json!({
                "target": {
                    "kind": "directory",
                    "path": workspace.absolute_path("crates/lib").to_string_lossy()
                },
                "new_name": workspace.absolute_path("crates/app/src/lib_mod").to_string_lossy(),
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

    // Apply the plan
    client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan,
                "options": {
                    "dry_run": false
                }
            }),
        )
        .await
        .expect("workspace.apply_edit should succeed");

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
