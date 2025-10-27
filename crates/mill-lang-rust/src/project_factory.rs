//! Rust project factory implementation
//!
//! Handles creation of new Rust crates with proper workspace integration.

use mill_lang_common::project_factory::{
    derive_package_name, resolve_package_path, update_workspace_manifest,
    validate_package_path_not_exists, write_project_file, WorkspaceManifestDetector,
};
use mill_plugin_api::project_factory::{
    CreatePackageConfig, CreatePackageResult, PackageInfo, PackageType, ProjectFactory, Template,
};
use mill_plugin_api::{PluginError, PluginResult, WorkspaceSupport};
use std::fs;
use std::path::Path;
use tracing::{debug, error};

/// Rust project factory implementation
#[derive(Default)]
pub struct RustProjectFactory;

impl ProjectFactory for RustProjectFactory {
    fn create_package(&self, config: &CreatePackageConfig) -> PluginResult<CreatePackageResult> {
        debug!(
            package_path = %config.package_path,
            package_type = ?config.package_type,
            template = ?config.template,
            "Creating Rust package"
        );

        // Resolve and validate paths
        let workspace_root = Path::new(&config.workspace_root);
        let package_path = resolve_package_path(workspace_root, &config.package_path)?;
        validate_package_path_not_exists(&package_path)?;

        // Derive package name
        let package_name = derive_package_name(&package_path)?;

        debug!(package_name = %package_name, "Derived package name");

        // Create directory structure
        create_directory_structure(&package_path, config.package_type)?;

        // Generate and write files
        let mut created_files = Vec::new();

        // Write Cargo.toml
        let cargo_toml_path = package_path.join("Cargo.toml");
        let cargo_toml_content = generate_cargo_toml(&package_name, config.package_type);
        write_project_file(&cargo_toml_path, &cargo_toml_content)?;
        created_files.push(cargo_toml_path.display().to_string());

        // Write entry file
        let entry_file_path = package_path.join(entry_file(config.package_type));
        let entry_content = generate_entry_content(&package_name, config.package_type);
        write_project_file(&entry_file_path, &entry_content)?;
        created_files.push(entry_file_path.display().to_string());

        // Create additional files for full template
        if matches!(config.template, Template::Full) {
            let additional = create_full_template(&package_path, &package_name)?;
            created_files.extend(additional);
        }

        // Update workspace if requested
        let workspace_updated = if config.add_to_workspace {
            let workspace_support = crate::workspace_support::RustWorkspaceSupport;
            update_workspace_manifest(
                workspace_root,
                &package_path,
                "Cargo.toml",
                &RustManifestDetector,
                |content, member| workspace_support.add_workspace_member(content, member),
            )?
        } else {
            false
        };

        Ok(CreatePackageResult {
            created_files,
            workspace_updated,
            package_info: PackageInfo {
                name: package_name,
                version: "0.1.0".to_string(),
                manifest_path: cargo_toml_path.display().to_string(),
            },
        })
    }
}

// Helper functions

/// Workspace manifest detector for Rust projects
struct RustManifestDetector;

impl WorkspaceManifestDetector for RustManifestDetector {
    fn is_workspace_manifest(&self, content: &str) -> bool {
        crate::workspace_support::RustWorkspaceSupport.is_workspace_manifest(content)
    }
}

fn create_directory_structure(package_path: &Path, _package_type: PackageType) -> PluginResult<()> {
    debug!(package_path = %package_path.display(), "Creating directory structure");

    fs::create_dir_all(package_path).map_err(|e| {
        error!(error = %e, package_path = %package_path.display(), "Failed to create package directory");
        PluginError::internal(format!("Failed to create directory: {}", e))
    })?;

    let src_dir = package_path.join("src");
    fs::create_dir_all(&src_dir).map_err(|e| {
        error!(error = %e, src_dir = %src_dir.display(), "Failed to create src directory");
        PluginError::internal(format!("Failed to create src directory: {}", e))
    })?;

    Ok(())
}

fn entry_file(package_type: PackageType) -> &'static str {
    match package_type {
        PackageType::Library => "src/lib.rs",
        PackageType::Binary => "src/main.rs",
    }
}

fn generate_cargo_toml(package_name: &str, package_type: PackageType) -> String {
    match package_type {
        PackageType::Library => format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
            package_name
        ),
        PackageType::Binary => format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "{}"
path = "src/main.rs"

[dependencies]
"#,
            package_name, package_name
        ),
    }
}

fn generate_entry_content(package_name: &str, package_type: PackageType) -> String {
    match package_type {
        PackageType::Library => format!(
            r#"//! {} crate
//!
//! TODO: Add crate description

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn it_works() {{
        // TODO: Add tests
    }}
}}
"#,
            package_name
        ),
        PackageType::Binary => r#"fn main() {
    println!("Hello, world!");
}
"#
        .to_string(),
    }
}

fn create_full_template(package_path: &Path, package_name: &str) -> PluginResult<Vec<String>> {
    let mut created = Vec::new();

    // README.md
    let readme_path = package_path.join("README.md");
    let readme_content = format!(
        "# {}\n\nTODO: Add project description\n\n## Usage\n\nTODO: Add usage examples\n",
        package_name
    );
    write_project_file(&readme_path, &readme_content)?;
    created.push(readme_path.display().to_string());

    // tests/integration_test.rs
    let tests_dir = package_path.join("tests");
    fs::create_dir_all(&tests_dir)
        .map_err(|e| PluginError::internal(format!("Failed to create tests directory: {}", e)))?;

    let test_path = tests_dir.join("integration_test.rs");
    let test_content = format!(
        r#"//! Integration tests for {}

#[test]
fn test_basic() {{
    // TODO: Add integration tests
}}
"#,
        package_name
    );
    write_project_file(&test_path, &test_content)?;
    created.push(test_path.display().to_string());

    // examples/basic.rs
    let examples_dir = package_path.join("examples");
    fs::create_dir_all(&examples_dir).map_err(|e| {
        PluginError::internal(format!("Failed to create examples directory: {}", e))
    })?;

    let example_path = examples_dir.join("basic.rs");
    let example_content = format!(
        r#"//! Basic usage example for {}

fn main() {{
    println!("Example for {crate_name}");
    // TODO: Add example code
}}
"#,
        package_name,
        crate_name = package_name
    );
    write_project_file(&example_path, &example_content)?;
    created.push(example_path.display().to_string());

    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_cargo_toml_lib() {
        let content = generate_cargo_toml("test-crate", PackageType::Library);
        assert!(content.contains("[package]"));
        assert!(content.contains("name = \"test-crate\""));
        assert!(content.contains("version = \"0.1.0\""));
        assert!(content.contains("[dependencies]"));
        assert!(!content.contains("[[bin]]"));
    }

    #[test]
    fn test_generate_cargo_toml_bin() {
        let content = generate_cargo_toml("test-bin", PackageType::Binary);
        assert!(content.contains("[package]"));
        assert!(content.contains("name = \"test-bin\""));
        assert!(content.contains("[[bin]]"));
        assert!(content.contains("path = \"src/main.rs\""));
    }

    #[test]
    fn test_entry_file() {
        assert_eq!(entry_file(PackageType::Library), "src/lib.rs");
        assert_eq!(entry_file(PackageType::Binary), "src/main.rs");
    }
}
