//! End-to-end integration tests for Rust package consolidation feature
//!
//! Tests the complete workflow of consolidating one Rust crate into another,
//! including file moving, dependency merging, workspace updates, and import rewriting.

use integration_tests::harness::{TestClient, TestWorkspace};
use serde_json::json;
use std::fs;
use std::path::Path;

/// Test basic consolidation: move source_crate into target_crate
#[tokio::test]
async fn test_consolidate_rust_package_basic() {
    // Create a temporary workspace
    let workspace = TestWorkspace::new();
    let workspace_path = workspace.path();

    // Copy the consolidation test fixture into the workspace
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/consolidation-test");
    copy_dir_recursive(&fixture_path, workspace_path).expect("Failed to copy test fixture");

    // Initialize MCP client
    let mut client = TestClient::new(workspace_path);

    // Perform consolidation
    let old_path = workspace_path.join("source_crate");
    let new_path = workspace_path.join("target_crate/src/source");

    let response = client
        .call_tool(
            "rename_directory",
            json!({
                "old_path": old_path.to_str().unwrap(),
                "new_path": new_path.to_str().unwrap(),
                "consolidate": true,
                "dry_run": false
            }),
        )
        .await;

    // Assert the response was successful
    if let Err(e) = &response {
        panic!("Consolidation failed: {:?}", e);
    }

    let response = response.unwrap();

    // Debug: print the response structure
    eprintln!("DEBUG: Full response: {:?}", response);

    let result = response.get("result").unwrap_or_else(|| {
        panic!(
            "Response should have result field. Full response: {:?}",
            response
        )
    });

    // Verify the response indicates success
    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Consolidation should indicate success"
    );

    // === Verify File Operations ===

    // 1. Old crate directory should be deleted
    assert!(
        !old_path.exists(),
        "source_crate directory should be deleted after consolidation"
    );

    // 2. Files should exist in new location
    assert!(
        new_path.join("lib.rs").exists(),
        "source files should be moved to target_crate/src/source/"
    );

    // 3. Verify file contents were preserved
    let lib_rs_content =
        fs::read_to_string(new_path.join("lib.rs")).expect("Should be able to read moved lib.rs");
    assert!(
        lib_rs_content.contains("say_hello"),
        "Moved file should preserve original content"
    );

    // === Verify Cargo.toml Merging ===

    let target_cargo_toml = workspace_path.join("target_crate/Cargo.toml");
    let target_toml_content =
        fs::read_to_string(&target_cargo_toml).expect("Should be able to read target Cargo.toml");

    // Dependency from source_crate should be merged
    assert!(
        target_toml_content.contains("serde"),
        "Dependencies from source_crate should be merged into target_crate"
    );

    // === Verify Workspace Members Updated ===

    let workspace_cargo_toml = workspace_path.join("Cargo.toml");
    let workspace_toml_content = fs::read_to_string(&workspace_cargo_toml)
        .expect("Should be able to read workspace Cargo.toml");

    assert!(
        !workspace_toml_content.contains("\"source_crate\""),
        "source_crate should be removed from workspace members"
    );

    // === Verify Success Message ===

    // Check that next_steps guidance is provided
    let result_obj = result.as_object().expect("Result should be an object");
    assert!(
        result_obj.get("next_steps").is_some(),
        "Result should include next_steps guidance"
    );

    let next_steps = result_obj["next_steps"].as_str().unwrap();
    assert!(
        next_steps.contains("pub mod source"),
        "Next steps should mention adding pub mod declaration"
    );

    // Note: Import updates in consumer_crate require manual Cargo.toml update first
    // The consumer still depends on source-crate which no longer exists.
    // In a real scenario, user would:
    // 1. Update consumer_crate/Cargo.toml to depend on target-crate
    // 2. Then imports would be automatically updated to target_crate::source::*
}

/// Test consolidation dry-run mode
#[tokio::test]
async fn test_consolidate_dry_run() {
    let workspace = TestWorkspace::new();
    let workspace_path = workspace.path();

    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/consolidation-test");
    copy_dir_recursive(&fixture_path, workspace_path).expect("Failed to copy test fixture");

    let mut client = TestClient::new(workspace_path);

    let old_path = workspace_path.join("source_crate");
    let new_path = workspace_path.join("target_crate/src/source");

    // Run with dry_run=true
    let response = client
        .call_tool(
            "rename_directory",
            json!({
                "old_path": old_path.to_str().unwrap(),
                "new_path": new_path.to_str().unwrap(),
                "consolidate": true,
                "dry_run": true
            }),
        )
        .await
        .expect("Dry run should succeed");

    let result = response
        .get("result")
        .expect("Response should have result field");

    // Dry run should show preview of actions
    assert!(
        result.get("actions").is_some() || result.get("import_changes").is_some(),
        "Dry run should preview the consolidation actions. Result: {:?}",
        result
    );

    // Verify NO changes were made
    assert!(
        old_path.exists(),
        "source_crate should still exist after dry run"
    );
    assert!(
        !new_path.exists(),
        "target location should not exist after dry run"
    );
}

/// Helper function to recursively copy directories
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}
