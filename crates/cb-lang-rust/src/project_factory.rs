//! Rust project factory implementation
//!
//! Handles creation of new Rust crates with proper workspace integration.

use cb_plugin_api::project_factory::{
    CreatePackageConfig, CreatePackageResult, PackageInfo, PackageType, ProjectFactory, Template,
};
use cb_plugin_api::{PluginError, PluginResult};
use std::fs;
use std::path::{Path, PathBuf};
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

        // Resolve paths
        let workspace_root = Path::new(&config.workspace_root);
        let package_path = resolve_package_path(workspace_root, &config.package_path)?;

        // Validate package path doesn't exist
        if package_path.exists() {
            return Err(PluginError::invalid_input(format!(
                "Package already exists at {}",
                package_path.display()
            )));
        }

        // Validate parent exists
        let parent = package_path.parent().ok_or_else(|| {
            PluginError::invalid_input(format!("Invalid package path: {}", package_path.display()))
        })?;

        if !parent.exists() {
            return Err(PluginError::invalid_input(format!(
                "Parent directory does not exist: {}",
                parent.display()
            )));
        }

        // Derive package name
        let package_name = package_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                PluginError::invalid_input(format!("Invalid package path: {}", package_path.display()))
            })?
            .to_string();

        debug!(package_name = %package_name, "Derived package name");

        // Create directory structure
        create_directory_structure(&package_path, config.package_type)?;

        // Generate and write files
        let mut created_files = Vec::new();

        // Write Cargo.toml
        let cargo_toml_path = package_path.join("Cargo.toml");
        let cargo_toml_content = generate_cargo_toml(&package_name, config.package_type);
        write_file(&cargo_toml_path, &cargo_toml_content)?;
        created_files.push(cargo_toml_path.display().to_string());

        // Write entry file
        let entry_file_path = package_path.join(entry_file(config.package_type));
        let entry_content = generate_entry_content(&package_name, config.package_type);
        write_file(&entry_file_path, &entry_content)?;
        created_files.push(entry_file_path.display().to_string());

        // Create additional files for full template
        if matches!(config.template, Template::Full) {
            let additional = create_full_template(&package_path, &package_name)?;
            created_files.extend(additional);
        }

        // Update workspace if requested
        let workspace_updated = if config.add_to_workspace {
            update_workspace_members(workspace_root, &package_path)?
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

fn resolve_package_path(workspace_root: &Path, package_path: &str) -> PluginResult<PathBuf> {
    let path = Path::new(package_path);

    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    };

    // Ensure within workspace
    let canonical_root = workspace_root.canonicalize().map_err(|e| {
        PluginError::internal(format!("Failed to canonicalize workspace root: {}", e))
    })?;

    if !resolved.starts_with(&canonical_root) {
        return Err(PluginError::invalid_input(format!(
            "Package path {} is outside workspace",
            package_path
        )));
    }

    Ok(resolved)
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

fn write_file(path: &Path, content: &str) -> PluginResult<()> {
    debug!(path = %path.display(), "Writing file");
    fs::write(path, content).map_err(|e| {
        error!(error = %e, path = %path.display(), "Failed to write file");
        PluginError::internal(format!("Failed to write file: {}", e))
    })
}

fn create_full_template(package_path: &Path, package_name: &str) -> PluginResult<Vec<String>> {
    let mut created = Vec::new();

    // README.md
    let readme_path = package_path.join("README.md");
    let readme_content = format!(
        "# {}\n\nTODO: Add project description\n\n## Usage\n\nTODO: Add usage examples\n",
        package_name
    );
    write_file(&readme_path, &readme_content)?;
    created.push(readme_path.display().to_string());

    // tests/integration_test.rs
    let tests_dir = package_path.join("tests");
    fs::create_dir_all(&tests_dir).map_err(|e| {
        PluginError::internal(format!("Failed to create tests directory: {}", e))
    })?;

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
    write_file(&test_path, &test_content)?;
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
    write_file(&example_path, &example_content)?;
    created.push(example_path.display().to_string());

    Ok(created)
}

fn update_workspace_members(workspace_root: &Path, package_path: &Path) -> PluginResult<bool> {
    // Find workspace Cargo.toml
    let workspace_manifest = find_workspace_manifest(workspace_root)?;

    debug!(
        workspace_manifest = %workspace_manifest.display(),
        "Found workspace manifest"
    );

    // Read manifest
    let content = fs::read_to_string(&workspace_manifest).map_err(|e| {
        error!(
            error = %e,
            workspace_manifest = %workspace_manifest.display(),
            "Failed to read workspace manifest"
        );
        PluginError::internal(format!("Failed to read workspace Cargo.toml: {}", e))
    })?;

    // Calculate relative path
    let workspace_dir = workspace_manifest
        .parent()
        .ok_or_else(|| PluginError::internal("Invalid workspace manifest path"))?;

    let relative_path = pathdiff::diff_paths(package_path, workspace_dir)
        .ok_or_else(|| PluginError::internal("Failed to calculate relative path"))?;

    let member_str = relative_path.to_string_lossy();

    debug!(member = %member_str, "Adding workspace member");

    // Use workspace support to add member
    use cb_plugin_api::WorkspaceSupport;
    let workspace_support = crate::workspace_support::RustWorkspaceSupport;
    let updated_content = workspace_support.add_workspace_member(&content, &member_str);

    if updated_content != content {
        // Write updated manifest
        fs::write(&workspace_manifest, &updated_content).map_err(|e| {
            error!(
                error = %e,
                workspace_manifest = %workspace_manifest.display(),
                "Failed to write workspace manifest"
            );
            PluginError::internal(format!("Failed to write workspace Cargo.toml: {}", e))
        })?;

        Ok(true)
    } else {
        Ok(false)
    }
}

fn find_workspace_manifest(workspace_root: &Path) -> PluginResult<PathBuf> {
    let mut current = workspace_root.to_path_buf();

    loop {
        let manifest = current.join("Cargo.toml");

        if manifest.exists() {
            let content = fs::read_to_string(&manifest).map_err(|e| {
                PluginError::internal(format!("Failed to read Cargo.toml: {}", e))
            })?;

            if content.contains("[workspace]") {
                return Ok(manifest);
            }
        }

        // Move up
        current = current
            .parent()
            .ok_or_else(|| {
                PluginError::invalid_input("No workspace Cargo.toml found in hierarchy")
            })?
            .to_path_buf();

        // Stop at root
        if current == current.parent().unwrap_or(&current) {
            break;
        }
    }

    Err(PluginError::invalid_input("No workspace Cargo.toml found"))
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
