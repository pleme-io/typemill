//! Tests for consolidation detection and metadata
//!
//! Verifies that:
//! 1. is_consolidation flag is set correctly in the plan
//! 2. Consolidation-specific warnings are added
//! 3. Auto-detection works for typical patterns
//! 4. Explicit consolidate: false can override auto-detection

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

/// Test that is_consolidation flag is set when explicitly requested
#[tokio::test]
async fn test_consolidation_flag_explicit() {
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

    // Generate consolidation plan with explicit flag
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

    // Verify is_consolidation flag is set
    assert_eq!(
        plan.get("is_consolidation").and_then(|v| v.as_bool()),
        Some(true),
        "Plan should have is_consolidation=true when explicitly requested"
    );

    // Verify consolidation warning is present
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

    assert!(
        has_consolidation_warning,
        "Plan should have CONSOLIDATION_MANUAL_STEP warning"
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

    println!("✓ Consolidation flag and warning correctly set with explicit consolidate: true");
}

/// Test that is_consolidation is auto-detected for typical patterns
#[tokio::test]
async fn test_consolidation_auto_detection() {
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
"#,
    );

    // Create source crate (has Cargo.toml)
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

    // Create target crate (target is inside src/ directory)
    workspace.create_directory("crates/app/src");
    workspace.create_file(
        "crates/app/Cargo.toml",
        r#"
[package]
name = "app"
version = "0.1.0"
edition = "2021"
"#,
    );
    workspace.create_file("crates/app/src/lib.rs", "// app crate");

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
                "new_name": workspace.absolute_path("crates/app/src/lib_mod").to_string_lossy()
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
    assert_eq!(
        plan.get("is_consolidation").and_then(|v| v.as_bool()),
        Some(true),
        "Plan should have is_consolidation=true via auto-detection"
    );

    // Verify consolidation warning is present
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

    assert!(
        has_consolidation_warning,
        "Auto-detected consolidation should have CONSOLIDATION_MANUAL_STEP warning"
    );

    println!("✓ Consolidation auto-detected correctly (source=crate, target=inside src/)");
}

/// Test that explicit consolidate: false overrides auto-detection
#[tokio::test]
async fn test_consolidation_override_auto_detection() {
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
"#,
    );

    // Create source crate (has Cargo.toml - would normally trigger auto-detection)
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

    // Create target crate (target is inside src/ - would normally trigger auto-detection)
    workspace.create_directory("crates/app/src");
    workspace.create_file(
        "crates/app/Cargo.toml",
        r#"
[package]
name = "app"
version = "0.1.0"
edition = "2021"
"#,
    );
    workspace.create_file("crates/app/src/lib.rs", "// app crate");

    // Generate plan with explicit consolidate: false to override auto-detection
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
    assert_eq!(
        plan.get("is_consolidation").and_then(|v| v.as_bool()),
        Some(false),
        "Plan should have is_consolidation=false when explicitly set to false"
    );

    // Verify NO consolidation warning
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

    assert!(
        !has_consolidation_warning,
        "Plan with consolidate=false should NOT have consolidation warning"
    );

    println!("✓ Explicit consolidate=false correctly overrides auto-detection");
}

/// Test that is_consolidation is false for non-consolidation renames
#[tokio::test]
async fn test_non_consolidation_rename() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a simple directory (no Cargo.toml)
    workspace.create_directory("src/old_dir");
    workspace.create_file("src/old_dir/file.rs", "pub fn test() {}");

    // Rename to another simple directory (not inside another crate's src/)
    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "directory",
                    "path": workspace.absolute_path("src/old_dir").to_string_lossy()
                },
                "new_name": workspace.absolute_path("src/new_dir").to_string_lossy()
            }),
        )
        .await
        .expect("rename.plan should succeed");

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    // Verify is_consolidation is false
    let is_consolidation = plan
        .get("is_consolidation")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    assert_eq!(
        is_consolidation, false,
        "Normal directory rename should have is_consolidation=false"
    );

    // Verify NO consolidation warning
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

    assert!(
        !has_consolidation_warning,
        "Normal rename should NOT have consolidation warning"
    );

    println!("✓ Normal directory rename correctly has is_consolidation=false");
}
