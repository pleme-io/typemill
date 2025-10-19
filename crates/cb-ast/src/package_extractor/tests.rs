use super::*;
use crate::package_extractor::planner::plan_extract_module_to_package;
use cb_lang_rust::RustPlugin;
use codebuddy_foundation::protocol::EditType;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
async fn test_locate_module_files_single_file() {
    // Create a temporary Rust project structure
    let temp_dir = tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    // Create lib.rs
    fs::write(src_dir.join("lib.rs"), "// lib.rs").unwrap();

    // Create a module as a single file: src/my_module.rs
    fs::write(src_dir.join("my_module.rs"), "// my_module.rs").unwrap();

    let plugin = RustPlugin::default();
    let result = plugin
        .locate_module_files(temp_dir.path(), "my_module")
        .await;

    assert!(result.is_ok());
    let files = result.unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("my_module.rs"));
}

#[tokio::test]
async fn test_locate_module_files_mod_rs() {
    // Create a temporary Rust project structure
    let temp_dir = tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    // Create lib.rs
    fs::write(src_dir.join("lib.rs"), "// lib.rs").unwrap();

    // Create a module as a directory with mod.rs: src/my_module/mod.rs
    let module_dir = src_dir.join("my_module");
    fs::create_dir(&module_dir).unwrap();
    fs::write(module_dir.join("mod.rs"), "// mod.rs").unwrap();

    let plugin = RustPlugin::default();
    let result = plugin
        .locate_module_files(temp_dir.path(), "my_module")
        .await;

    assert!(result.is_ok());
    let files = result.unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("my_module/mod.rs") || files[0].ends_with("my_module\\mod.rs"));
}

#[tokio::test]
async fn test_locate_module_files_nested_module() {
    // Create a temporary Rust project structure
    let temp_dir = tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    // Create lib.rs
    fs::write(src_dir.join("lib.rs"), "// lib.rs").unwrap();

    // Create nested module structure: src/services/planner.rs
    let services_dir = src_dir.join("services");
    fs::create_dir(&services_dir).unwrap();
    fs::write(services_dir.join("planner.rs"), "// planner.rs").unwrap();

    let plugin = RustPlugin::default();
    let result = plugin
        .locate_module_files(temp_dir.path(), "services::planner")
        .await;

    assert!(result.is_ok());
    let files = result.unwrap();
    assert_eq!(files.len(), 1);
    assert!(
        files[0].ends_with("services/planner.rs") || files[0].ends_with("services\\planner.rs")
    );
}

#[tokio::test]
async fn test_locate_module_files_dot_separator() {
    // Test that the function accepts both :: and . as separators
    let temp_dir = tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    // Create lib.rs
    fs::write(src_dir.join("lib.rs"), "// lib.rs").unwrap();

    // Create nested module structure: src/services/planner.rs
    let services_dir = src_dir.join("services");
    fs::create_dir(&services_dir).unwrap();
    fs::write(services_dir.join("planner.rs"), "// planner.rs").unwrap();

    let plugin = RustPlugin::default();
    let result = plugin
        .locate_module_files(temp_dir.path(), "services.planner")
        .await;

    assert!(result.is_ok());
    let files = result.unwrap();
    assert_eq!(files.len(), 1);
    assert!(
        files[0].ends_with("services/planner.rs") || files[0].ends_with("services\\planner.rs")
    );
}

#[tokio::test]
async fn test_locate_module_files_not_found() {
    // Create a temporary Rust project structure
    let temp_dir = tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    // Create lib.rs but no module files
    fs::write(src_dir.join("lib.rs"), "// lib.rs").unwrap();

    let plugin = RustPlugin::default();
    let result = plugin
        .locate_module_files(temp_dir.path(), "nonexistent")
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_locate_module_files_no_src_dir() {
    // Create a temporary directory without src/
    let temp_dir = tempdir().unwrap();

    let plugin = RustPlugin::default();
    let result = plugin
        .locate_module_files(temp_dir.path(), "my_module")
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_locate_module_files_empty_module_path() {
    let temp_dir = tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    let plugin = RustPlugin::default();
    let result = plugin.locate_module_files(temp_dir.path(), "").await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_parse_imports_empty_file() {
    let temp_dir = tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    let rust_content = r#"
fn main() {
    println!("Hello, world!");
}
"#;
    let test_file = src_dir.join("test_module.rs");
    fs::write(&test_file, rust_content).unwrap();

    let plugin = RustPlugin::default();
    let result = plugin.parse_imports(&test_file).await;

    assert!(result.is_ok());
    let dependencies = result.unwrap();
    assert_eq!(dependencies.len(), 0);
}

#[test]
fn test_generate_manifest_with_dependencies() {
    let plugin = RustPlugin::default();
    let dependencies = vec![
        "serde".to_string(),
        "tokio".to_string(),
        "async-trait".to_string(),
    ];

    let manifest = plugin.generate_manifest("my-test-crate", &dependencies);

    assert!(manifest.contains("[package]"));
    assert!(manifest.contains("name = \"my-test-crate\""));
    assert!(manifest.contains("version = \"0.1.0\""));
    assert!(manifest.contains("edition = \"2021\""));

    assert!(manifest.contains("[dependencies]"));
    assert!(manifest.contains("serde = \"*\""));
    assert!(manifest.contains("tokio = \"*\""));
    assert!(manifest.contains("async-trait = \"*\""));
}

#[test]
fn test_generate_manifest_no_dependencies() {
    let plugin = RustPlugin::default();
    let dependencies: Vec<String> = vec![];

    let manifest = plugin.generate_manifest("simple-crate", &dependencies);

    assert!(manifest.contains("[package]"));
    assert!(manifest.contains("name = \"simple-crate\""));
    assert!(!manifest.contains("[dependencies]"));
}

#[test]
fn test_generate_manifest_single_dependency() {
    let plugin = RustPlugin::default();
    let dependencies = vec!["serde".to_string()];

    let manifest = plugin.generate_manifest("test-crate", &dependencies);

    assert!(manifest.contains("[package]"));
    assert!(manifest.contains("[dependencies]"));
    assert!(manifest.contains("serde = \"*\""));
}

#[test]
fn test_generate_manifest_special_characters_in_name() {
    let plugin = RustPlugin::default();
    let dependencies = vec![];

    let manifest = plugin.generate_manifest("my-special_crate123", &dependencies);

    assert!(manifest.contains("name = \"my-special_crate123\""));
}

#[test]
fn test_rust_plugin_no_changes_different_crate() {
    use serde_json::json;

    let plugin = RustPlugin::default();
    let source = r#"use some_other_crate::SomeType;"#;

    let rename_info = json!({
        "old_crate_name": "old_crate",
        "new_crate_name": "new_crate",
    });

    let (new_content, count) = plugin
        .rewrite_imports_for_rename(
            source,
            Path::new(""),
            Path::new(""),
            Path::new(""),
            Path::new(""),
            Some(&rename_info),
        )
        .unwrap();

    assert_eq!(count, 0);
    assert_eq!(new_content, source);
}

#[test]
fn test_rust_plugin_no_rename_info() {
    let plugin = RustPlugin::default();
    let source = r#"use old_crate::SomeType;"#;

    let (new_content, count) = plugin
        .rewrite_imports_for_rename(
            source,
            Path::new(""),
            Path::new(""),
            Path::new(""),
            Path::new(""),
            None,
        )
        .unwrap();

    assert_eq!(count, 0);
    assert_eq!(new_content, source);
}

#[tokio::test]
async fn test_workspace_member_creation() {
    let temp_dir = tempdir().unwrap();
    let project_root = temp_dir.path();

    let src_crate = project_root.join("src_crate");
    let src_dir = src_crate.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(
        src_crate.join("Cargo.toml"),
        r#"[package]
name = "src_crate"
version = "0.1.0"
"#,
    )
    .unwrap();
    fs::write(
        src_dir.join("lib.rs"),
        r#"pub mod my_module;
pub fn main_function() {}
"#,
    )
    .unwrap();
    fs::write(
        src_dir.join("my_module.rs"),
        r#"use std::collections::HashMap;
pub fn module_function() {}
"#,
    )
    .unwrap();

    let target_crate = project_root.join("extracted_crate");
    fs::create_dir_all(&target_crate).unwrap();

    let params = ExtractModuleToPackageParams {
        source_package: src_crate.to_string_lossy().to_string(),
        module_path: "my_module".to_string(),
        target_package_path: target_crate.to_string_lossy().to_string(),
        target_package_name: "extracted_module".to_string(),
        update_imports: Some(true),
        create_manifest: Some(true),
        dry_run: Some(false),
        is_workspace_member: Some(true),
    };

    let mut registry = cb_plugin_api::PluginRegistry::new();
    registry.register(Arc::new(RustPlugin::default()));

    let result = plan_extract_module_to_package(params, &registry).await;
    assert!(result.is_ok(), "Plan should succeed: {:?}", result.err());

    let edit_plan = result.unwrap();
    let workspace_cargo_edit = edit_plan.edits.iter().find(|e| {
        e.file_path
            .as_ref()
            .map(|p| {
                p.ends_with("Cargo.toml")
                    && !p.contains("src_crate")
                    && !p.contains("extracted_crate")
            })
            .unwrap_or(false)
            && (e.description.contains("workspace") || e.description.contains("members"))
    });

    assert!(
        workspace_cargo_edit.is_some(),
        "Should have workspace Cargo.toml edit when is_workspace_member=true"
    );
}

#[tokio::test]
async fn test_no_workspace_member_creation() {
    let temp_dir = tempdir().unwrap();
    let project_root = temp_dir.path();
    let src_crate = project_root.join("src_crate");
    let src_dir = src_crate.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(
        src_crate.join("Cargo.toml"),
        r#"[package]
name = "src_crate"
version = "0.1.0"
"#,
    )
    .unwrap();
    fs::write(src_dir.join("lib.rs"), r#"pub mod my_module;"#).unwrap();
    fs::write(
        src_dir.join("my_module.rs"),
        r#"pub fn module_function() {}"#,
    )
    .unwrap();

    let target_crate = project_root.join("extracted_crate");
    fs::create_dir_all(&target_crate).unwrap();

    let params = ExtractModuleToPackageParams {
        source_package: src_crate.to_string_lossy().to_string(),
        module_path: "my_module".to_string(),
        target_package_path: target_crate.to_string_lossy().to_string(),
        target_package_name: "extracted_module".to_string(),
        update_imports: Some(true),
        create_manifest: Some(true),
        dry_run: Some(false),
        is_workspace_member: Some(false),
    };

    let mut registry = cb_plugin_api::PluginRegistry::new();
    registry.register(Arc::new(RustPlugin::default()));

    let result = plan_extract_module_to_package(params, &registry).await;
    assert!(result.is_ok(), "Plan should succeed: {:?}", result.err());

    let edit_plan = result.unwrap();
    let workspace_cargo_edit = edit_plan.edits.iter().find(|e| {
        e.file_path
            .as_ref()
            .map(|p| {
                p.ends_with("Cargo.toml")
                    && !p.contains("src_crate")
                    && !p.contains("extracted_crate")
            })
            .unwrap_or(false)
            && (e.description.contains("workspace") || e.description.contains("members"))
    });

    assert!(
        workspace_cargo_edit.is_none(),
        "Should NOT have workspace Cargo.toml edit when is_workspace_member=false"
    );
}