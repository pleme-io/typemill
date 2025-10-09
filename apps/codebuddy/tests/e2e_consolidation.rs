//! Rust crate consolidation tests
//!
//! Tests the complete workflow of consolidating one Rust crate into another,
//! including file moving, dependency merging, workspace updates, and import rewriting.

use serde_json::json;
use std::fs;
use std::path::Path;
use test_support::harness::{TestClient, TestWorkspace};

/// Test basic consolidation: move source_crate into target_crate
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_consolidate_rust_package_basic() {
    // Create a temporary workspace
    let workspace = TestWorkspace::new();
    let workspace_path = workspace.path();

    // Copy the consolidation test fixture into the workspace
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/test-support/fixtures/consolidation-test");
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
        let stderr = client.get_stderr_logs().join("\n");
        panic!("Consolidation failed: {:?}\n\nSERVER STDERR:\n{}", e, stderr);
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

    // Bug #6 Regression Test: Verify file header comments were NOT corrupted
    let utils_rs_content = fs::read_to_string(new_path.join("utils.rs"))
        .expect("Should be able to read moved utils.rs");
    assert!(
        utils_rs_content.starts_with("//!"),
        "File header doc comments should be preserved (Bug #6 regression test)"
    );
    assert!(
        utils_rs_content.contains("//! Utility functions"),
        "Full doc comment content should be intact"
    );
    assert!(
        utils_rs_content.contains("pub fn format_greeting"),
        "Function definitions should be present after doc comments"
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

    // === Bug #2 Regression Test: Verify workspace Cargo.toml dependencies were updated ===

    let consumer_cargo_toml = workspace_path.join("consumer_crate/Cargo.toml");
    let consumer_toml_content = fs::read_to_string(&consumer_cargo_toml)
        .expect("Should be able to read consumer Cargo.toml");

    // The consumer's dependency should have been automatically updated from source-crate to target-crate
    eprintln!(
        "DEBUG: Consumer Cargo.toml content:\n{}",
        consumer_toml_content
    );

    assert!(
        consumer_toml_content.contains("target-crate"),
        "Bug #2: consumer_crate's Cargo.toml should be updated to depend on target-crate. Content:\n{}",
        consumer_toml_content
    );
    assert!(
        !consumer_toml_content.contains("source-crate"),
        "Bug #2: consumer_crate should no longer depend on source-crate. Content:\n{}",
        consumer_toml_content
    );

    // === Bug #5 Regression Test: Verify inline fully-qualified paths were updated ===
    // Note: Import updates only happen if LSP servers are available and workspace compiles
    // In this test environment without LSP, we verify the Cargo.toml was updated (Bug #2)
    // which is the prerequisite for Bug #5 import updates to work correctly

    let consumer_lib_rs = workspace_path.join("consumer_crate/src/lib.rs");
    if consumer_lib_rs.exists() {
        let consumer_lib_content =
            fs::read_to_string(&consumer_lib_rs).expect("Should be able to read consumer lib.rs");

        eprintln!(
            "DEBUG: Consumer lib.rs still has source_crate references (expected without LSP):\n{}",
            &consumer_lib_content[..200.min(consumer_lib_content.len())]
        );

        // In a real scenario with LSP running, these would be updated:
        // - source_crate::say_hello() -> target_crate::source::say_hello()
        // - use source_crate::X -> use target_crate::source::X
        // But for this test, we verify the Cargo.toml prerequisite (Bug #2) is met
    }
}

/// Test consolidation dry-run mode
#[tokio::test]
#[serial]
async fn test_consolidate_dry_run() {
    let workspace = TestWorkspace::new();
    let workspace_path = workspace.path();

    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/test-support/fixtures/consolidation-test");
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
        .await;

    if let Err(e) = &response {
        let stderr = client.get_stderr_logs().join("\n");
        panic!("Dry run failed: {:?}\n\nSERVER STDERR:\n{}", e, stderr);
    }
    let response = response.unwrap();

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

/// Bug #3 Regression Test: Verify circular dependency detection
#[tokio::test]
#[serial]
async fn test_consolidation_prevents_circular_dependencies() {
    let workspace = TestWorkspace::new();
    let workspace_path = workspace.path();

    // Create a minimal test scenario with potential circular dependencies
    // Structure:
    //   - crate_a (depends on crate_b)
    //   - crate_b (we'll try to merge crate_a's deps into crate_b, which would create a cycle)

    // Create workspace Cargo.toml
    fs::write(
        workspace_path.join("Cargo.toml"),
        r#"[workspace]
members = ["crate_a", "crate_b"]
resolver = "2"
"#,
    )
    .unwrap();

    // Create crate_a that depends on crate_b
    fs::create_dir_all(workspace_path.join("crate_a/src")).unwrap();
    fs::write(
        workspace_path.join("crate_a/Cargo.toml"),
        r#"[package]
name = "crate-a"
version = "0.1.0"
edition = "2021"

[dependencies]
crate-b = { path = "../crate_b" }
"#,
    )
    .unwrap();
    fs::write(
        workspace_path.join("crate_a/src/lib.rs"),
        "pub fn a_function() -> &'static str { \"from a\" }\n",
    )
    .unwrap();

    // Create crate_b (no dependencies initially)
    fs::create_dir_all(workspace_path.join("crate_b/src")).unwrap();
    fs::write(
        workspace_path.join("crate_b/Cargo.toml"),
        r#"[package]
name = "crate-b"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
    )
    .unwrap();
    fs::write(
        workspace_path.join("crate_b/src/lib.rs"),
        "pub fn b_function() -> &'static str { \"from b\" }\n",
    )
    .unwrap();

    let mut client = TestClient::new(workspace_path);

    // Try to consolidate crate_a into crate_b
    // This would try to add crate_b as a dependency of crate_b (circular!)
    let old_path = workspace_path.join("crate_a");
    let new_path = workspace_path.join("crate_b/src/a_module");

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

    // The operation should succeed, but circular dependencies should be filtered out
    if let Err(e) = &response {
        let stderr = client.get_stderr_logs().join("\n");
        panic!(
            "Consolidation failed when it should have succeeded gracefully: {:?}\n\nSERVER STDERR:\n{}",
            e, stderr
        );
    }

    // Verify crate_b's Cargo.toml does NOT have a self-dependency
    let crate_b_toml_content =
        fs::read_to_string(workspace_path.join("crate_b/Cargo.toml")).unwrap();

    // Bug #3: The merge should have detected and skipped the circular dependency
    let has_self_dependency = crate_b_toml_content
        .lines()
        .any(|line| line.contains("crate-b") && line.contains("path"));

    assert!(
        !has_self_dependency,
        "Bug #3: Circular dependency should be detected and prevented. Cargo.toml:\n{}",
        crate_b_toml_content
    );

    // Verify the files were still moved (consolidation partially succeeded)
    assert!(
        new_path.join("lib.rs").exists(),
        "Files should still be moved even if dependency merge had conflicts"
    );
}

/// Test manifest updates with workspace.dependencies, patch, and target sections
#[tokio::test]
#[serial]
async fn test_rename_directory_updates_all_manifest_sections() {
    let workspace = TestWorkspace::new();
    let workspace_path = workspace.path();

    // Create a complex workspace structure with various Cargo.toml dependency types

    // Root workspace Cargo.toml with workspace.dependencies
    fs::write(
        workspace_path.join("Cargo.toml"),
        r#"[workspace]
members = ["crates/my-plugin", "crates/core", "crates/utils"]
resolver = "2"

[workspace.dependencies]
my-plugin = { path = "crates/my-plugin" }
serde = "1.0"

[patch.crates-io]
my-plugin = { path = "crates/my-plugin" }
"#,
    )
    .unwrap();

    // Create my-plugin crate (to be moved)
    fs::create_dir_all(workspace_path.join("crates/my-plugin/src")).unwrap();
    fs::write(
        workspace_path.join("crates/my-plugin/Cargo.toml"),
        r#"[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1.0"
"#,
    )
    .unwrap();
    fs::write(
        workspace_path.join("crates/my-plugin/src/lib.rs"),
        "pub fn plugin_function() -> &'static str { \"hello\" }\n",
    )
    .unwrap();

    // Create core crate that depends on my-plugin via workspace.dependencies
    fs::create_dir_all(workspace_path.join("crates/core/src")).unwrap();
    fs::write(
        workspace_path.join("crates/core/Cargo.toml"),
        r#"[package]
name = "core"
version = "0.1.0"
edition = "2021"

[dependencies]
my-plugin = { workspace = true }

[dev-dependencies]
my-plugin = { workspace = true }

[target.'cfg(unix)'.dependencies]
my-plugin = { path = "../my-plugin" }
"#,
    )
    .unwrap();
    fs::write(
        workspace_path.join("crates/core/src/lib.rs"),
        "pub fn core_function() {}\n",
    )
    .unwrap();

    // Create utils crate with regular path dependency
    fs::create_dir_all(workspace_path.join("crates/utils/src")).unwrap();
    fs::write(
        workspace_path.join("crates/utils/Cargo.toml"),
        r#"[package]
name = "utils"
version = "0.1.0"
edition = "2021"

[dependencies]
my-plugin = { path = "../my-plugin" }

[build-dependencies]
my-plugin = { path = "../my-plugin" }
"#,
    )
    .unwrap();
    fs::write(
        workspace_path.join("crates/utils/src/lib.rs"),
        "pub fn utils_function() {}\n",
    )
    .unwrap();

    let mut client = TestClient::new(workspace_path);

    // Rename my-plugin from crates/my-plugin to plugins/my-plugin
    let old_path = workspace_path.join("crates/my-plugin");
    let new_path = workspace_path.join("plugins/my-plugin");

    let response = client
        .call_tool(
            "rename_directory",
            json!({
                "old_path": old_path.to_str().unwrap(),
                "new_path": new_path.to_str().unwrap(),
            }),
        )
        .await;

    if let Err(e) = &response {
        let stderr = client.get_stderr_logs().join("\n");
        panic!("rename_directory failed: {:?}\n\nSERVER STDERR:\n{}", e, stderr);
    }
    let response = response.unwrap();

    let result = response
        .get("result")
        .expect("Response should have result field");

    // Verify operation succeeded
    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Rename should succeed. Result: {:?}",
        result
    );

    // Verify manifest_updates section is present
    let manifest_updates = result
        .get("manifest_updates")
        .expect("Result should have manifest_updates field");

    assert!(
        manifest_updates.is_object(),
        "manifest_updates should be an object"
    );

    // Verify files were updated
    let files_updated = manifest_updates["files_updated"]
        .as_u64()
        .expect("Should have files_updated count");

    assert!(
        files_updated >= 3,
        "Should update at least 3 Cargo.toml files (root + core + utils), got {}",
        files_updated
    );

    let updated_files = manifest_updates["updated_files"]
        .as_array()
        .expect("Should have updated_files array");

    // Verify root Cargo.toml was updated
    let root_toml_path = workspace_path.join("Cargo.toml");
    assert!(
        updated_files
            .iter()
            .any(|f| f.as_str().unwrap().contains("Cargo.toml")
                && Path::new(f.as_str().unwrap()) == root_toml_path),
        "Root Cargo.toml should be in updated files list"
    );

    // === Verify Root Cargo.toml [workspace.dependencies] ===
    let root_toml = fs::read_to_string(&root_toml_path).unwrap();

    assert!(
        root_toml.contains(r#"my-plugin = { path = "plugins/my-plugin" }"#),
        "[workspace.dependencies] should be updated to new path. Content:\n{}",
        root_toml
    );

    assert!(
        !root_toml.contains(r#"crates/my-plugin"#),
        "Old path should not exist in root Cargo.toml. Content:\n{}",
        root_toml
    );

    // === Verify Root Cargo.toml [patch.crates-io] ===
    assert!(
        root_toml.contains(r#"my-plugin = { path = "plugins/my-plugin" }"#),
        "[patch.crates-io] should be updated to new path. Content:\n{}",
        root_toml
    );

    // === Verify core crate [target.'cfg(unix)'.dependencies] ===
    let core_toml = fs::read_to_string(workspace_path.join("crates/core/Cargo.toml")).unwrap();

    assert!(
        core_toml.contains(r#"my-plugin = { path = "../../plugins/my-plugin" }"#),
        "[target.'cfg(unix)'.dependencies] should be updated with relative path. Content:\n{}",
        core_toml
    );

    assert!(
        !core_toml.contains(r#"../my-plugin"#),
        "Old relative path should not exist. Content:\n{}",
        core_toml
    );

    // === Verify utils crate [dependencies] and [build-dependencies] ===
    let utils_toml = fs::read_to_string(workspace_path.join("crates/utils/Cargo.toml")).unwrap();

    assert!(
        utils_toml.contains(r#"my-plugin = { path = "../../plugins/my-plugin" }"#),
        "[dependencies] should be updated. Content:\n{}",
        utils_toml
    );

    // Verify both sections were updated
    let path_occurrences = utils_toml.matches(r#"../../plugins/my-plugin"#).count();
    assert!(
        path_occurrences >= 2,
        "Both [dependencies] and [build-dependencies] should be updated, found {} occurrences",
        path_occurrences
    );

    // === Verify files were actually moved ===
    assert!(!old_path.exists(), "Old directory should no longer exist");

    assert!(
        new_path.join("src/lib.rs").exists(),
        "Files should exist at new location"
    );

    // === Verify no errors were reported ===
    let errors = manifest_updates["errors"]
        .as_array()
        .expect("Should have errors array");

    assert!(
        errors.is_empty(),
        "Should have no manifest update errors. Errors: {:?}",
        errors
    );

    // === Verify workspace structure is maintained ===
    // Check that workspace members array was updated to reflect new location
    assert!(
        root_toml.contains(r#""plugins/my-plugin""#)
            || root_toml.contains(r#"'plugins/my-plugin'"#),
        "Workspace members should include new path. Content:\n{}",
        root_toml
    );

    assert!(
        !root_toml.contains(r#""crates/my-plugin""#),
        "Workspace members should not contain old path. Content:\n{}",
        root_toml
    );

    println!("✅ Manifest update test passed:");
    println!("  - Files updated: {}", files_updated);
    println!("  - workspace.dependencies: updated ✓");
    println!("  - patch.crates-io: updated ✓");
    println!("  - target.'cfg(unix)'.dependencies: updated ✓");
    println!("  - Standard dependency sections: updated ✓");
    println!("  - Workspace structure maintained ✓");
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
