//! Python project factory implementation
//!
//! Handles creation of new Python packages with proper workspace integration.

use mill_plugin_api::project_factory::{
    CreatePackageConfig, CreatePackageResult, PackageInfo, PackageType, ProjectFactory, Template,
};
use mill_plugin_api::{PluginError, PluginResult, WorkspaceSupport};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, error};

/// Python project factory implementation
#[derive(Default)]
pub struct PythonProjectFactory;

impl ProjectFactory for PythonProjectFactory {
    fn create_package(&self, config: &CreatePackageConfig) -> PluginResult<CreatePackageResult> {
        debug!(
            package_path = %config.package_path,
            package_type = ?config.package_type,
            template = ?config.template,
            "Creating Python package"
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

        // Derive package name
        let package_name = package_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                PluginError::invalid_input(format!(
                    "Invalid package path: {}",
                    package_path.display()
                ))
            })?
            .to_string();

        debug!(package_name = %package_name, "Derived package name");

        // Create directory structure
        create_directory_structure(&package_path, &package_name)?;

        // Generate and write files
        let mut created_files = Vec::new();

        // Write pyproject.toml
        let pyproject_path = package_path.join("pyproject.toml");
        let pyproject_content = generate_pyproject_toml(&package_name, config.package_type);
        write_file(&pyproject_path, &pyproject_content)?;
        created_files.push(pyproject_path.display().to_string());

        // Write entry file
        let entry_file_path = package_path.join(entry_file(&package_name, config.package_type));
        let entry_content = generate_entry_content(&package_name, config.package_type);
        write_file(&entry_file_path, &entry_content)?;
        created_files.push(entry_file_path.display().to_string());

        // Create baseline files (README, .gitignore, tests) for minimal template
        let baseline = create_baseline_files(&package_path, &package_name)?;
        created_files.extend(baseline);

        // Create additional files for full template (setup.py)
        if matches!(config.template, Template::Full) {
            let additional = create_full_template_extras(&package_path)?;
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
                manifest_path: pyproject_path.display().to_string(),
            },
        })
    }
}

// Helper functions

fn resolve_package_path(workspace_root: &Path, package_path: &str) -> PluginResult<PathBuf> {
    let path = Path::new(package_path);

    // Reject paths with parent directory components to prevent traversal
    use std::path::Component;
    for component in path.components() {
        if matches!(component, Component::ParentDir) {
            return Err(PluginError::invalid_input(format!(
                "Package path cannot contain '..' components: {}",
                package_path
            )));
        }
    }

    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    };

    // Canonicalize both paths for comparison (handles symlinks, . and .. after join)
    let canonical_root = workspace_root.canonicalize().map_err(|e| {
        PluginError::internal(format!("Failed to canonicalize workspace root: {}", e))
    })?;

    // For the resolved path, we need to canonicalize the parent since the target doesn't exist yet
    let canonical_resolved = if let Some(parent) = resolved.parent() {
        if parent.exists() {
            let canonical_parent = parent.canonicalize().map_err(|e| {
                PluginError::internal(format!("Failed to canonicalize parent directory: {}", e))
            })?;
            resolved.file_name()
                .map(|name| canonical_parent.join(name))
                .ok_or_else(|| PluginError::invalid_input("Invalid package path"))?
        } else {
            // Parent doesn't exist yet, we'll create it - just verify it would be within workspace
            resolved.clone()
        }
    } else {
        resolved.clone()
    };

    if !canonical_resolved.starts_with(&canonical_root) {
        return Err(PluginError::invalid_input(format!(
            "Package path {} is outside workspace",
            package_path
        )));
    }

    Ok(resolved)
}

fn create_directory_structure(package_path: &Path, package_name: &str) -> PluginResult<()> {
    debug!(package_path = %package_path.display(), "Creating directory structure");

    // Create package root
    fs::create_dir_all(package_path).map_err(|e| {
        error!(error = %e, package_path = %package_path.display(), "Failed to create package directory");
        PluginError::internal(format!("Failed to create directory: {}", e))
    })?;

    // Create src/<package_name> directory
    let src_dir = package_path.join("src").join(package_name);
    fs::create_dir_all(&src_dir).map_err(|e| {
        error!(error = %e, src_dir = %src_dir.display(), "Failed to create src directory");
        PluginError::internal(format!("Failed to create src directory: {}", e))
    })?;

    Ok(())
}

fn entry_file(package_name: &str, package_type: PackageType) -> PathBuf {
    match package_type {
        PackageType::Library => PathBuf::from("src").join(package_name).join("__init__.py"),
        PackageType::Binary => PathBuf::from("src").join(package_name).join("main.py"),
    }
}

fn generate_pyproject_toml(package_name: &str, package_type: PackageType) -> String {
    match package_type {
        PackageType::Library => format!(
            r#"[project]
name = "{}"
version = "0.1.0"
description = ""
requires-python = ">=3.8"
dependencies = []

[build-system]
requires = ["setuptools>=61.0"]
build-backend = "setuptools.build_meta"

[tool.setuptools.packages.find]
where = ["src"]

[tool.setuptools.package-dir]
"" = "src"
"#,
            package_name
        ),
        PackageType::Binary => format!(
            r#"[project]
name = "{}"
version = "0.1.0"
description = ""
requires-python = ">=3.8"
dependencies = []

[project.scripts]
{} = "{}:main"

[build-system]
requires = ["setuptools>=61.0"]
build-backend = "setuptools.build_meta"

[tool.setuptools.packages.find]
where = ["src"]

[tool.setuptools.package-dir]
"" = "src"
"#,
            package_name, package_name, package_name
        ),
    }
}

fn generate_entry_content(package_name: &str, package_type: PackageType) -> String {
    match package_type {
        PackageType::Library => format!(
            r#""""{} package

TODO: Add package description
"""

__version__ = "0.1.0"
"#,
            package_name
        ),
        PackageType::Binary => r#"""Main entry point for the application"""


def main():
    """Main function"""
    print("Hello, world!")


if __name__ == "__main__":
    main()
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

fn create_baseline_files(package_path: &Path, package_name: &str) -> PluginResult<Vec<String>> {
    let mut created = Vec::new();

    // README.md
    let readme_path = package_path.join("README.md");
    let readme_content = format!(
        "# {}\n\nTODO: Add project description\n\n## Installation\n\n```bash\npip install {}\n```\n\n## Usage\n\nTODO: Add usage examples\n",
        package_name, package_name
    );
    write_file(&readme_path, &readme_content)?;
    created.push(readme_path.display().to_string());

    // .gitignore
    let gitignore_path = package_path.join(".gitignore");
    let gitignore_content = r#"# Python
__pycache__/
*.py[cod]
*$py.class
*.so
.Python
build/
develop-eggs/
dist/
downloads/
eggs/
.eggs/
lib/
lib64/
parts/
sdist/
var/
wheels/
*.egg-info/
.installed.cfg
*.egg

# Virtual environments
venv/
env/
ENV/

# IDE
.vscode/
.idea/
*.swp
*.swo
"#;
    write_file(&gitignore_path, &gitignore_content)?;
    created.push(gitignore_path.display().to_string());

    // tests/test_basic.py
    let tests_dir = package_path.join("tests");
    fs::create_dir_all(&tests_dir)
        .map_err(|e| PluginError::internal(format!("Failed to create tests directory: {}", e)))?;

    let test_path = tests_dir.join("test_basic.py");
    let test_content = format!(
        r#"""Basic tests for {}"""

import pytest


def test_basic():
    """TODO: Add basic tests"""
    assert True
"#,
        package_name
    );
    write_file(&test_path, &test_content)?;
    created.push(test_path.display().to_string());

    Ok(created)
}

fn create_full_template_extras(package_path: &Path) -> PluginResult<Vec<String>> {
    let mut created = Vec::new();

    // setup.py (backwards compatibility, Full template only)
    let setup_path = package_path.join("setup.py");
    let setup_content = r#"""Legacy setup.py for backwards compatibility"""

from setuptools import setup

setup()
"#;
    write_file(&setup_path, &setup_content)?;
    created.push(setup_path.display().to_string());

    Ok(created)
}

fn update_workspace_members(workspace_root: &Path, package_path: &Path) -> PluginResult<bool> {
    // Find workspace pyproject.toml
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
        PluginError::internal(format!("Failed to read workspace pyproject.toml: {}", e))
    })?;

    // Calculate relative path
    let workspace_dir = workspace_manifest
        .parent()
        .ok_or_else(|| PluginError::internal("Invalid workspace manifest path"))?;

    let relative_path = pathdiff::diff_paths(package_path, workspace_dir)
        .ok_or_else(|| PluginError::internal("Failed to calculate relative path"))?;

    // Normalize to forward slashes for cross-platform compatibility
    // PDM/Poetry/Hatch expect forward slashes even on Windows
    let member_str = relative_path
        .to_string_lossy()
        .replace('\\', "/");

    debug!(member = %member_str, "Adding workspace member");

    // Use workspace support to add member
    let workspace_support = crate::workspace_support::PythonWorkspaceSupport;
    let updated_content = workspace_support.add_workspace_member(&content, &member_str);

    if updated_content != content {
        // Write updated manifest
        fs::write(&workspace_manifest, &updated_content).map_err(|e| {
            error!(
                error = %e,
                workspace_manifest = %workspace_manifest.display(),
                "Failed to write workspace manifest"
            );
            PluginError::internal(format!("Failed to write workspace pyproject.toml: {}", e))
        })?;

        Ok(true)
    } else {
        Ok(false)
    }
}

fn find_workspace_manifest(workspace_root: &Path) -> PluginResult<PathBuf> {
    let mut current = workspace_root.to_path_buf();

    loop {
        let manifest = current.join("pyproject.toml");

        if manifest.exists() {
            let content = fs::read_to_string(&manifest).map_err(|e| {
                PluginError::internal(format!("Failed to read pyproject.toml: {}", e))
            })?;

            // Check if this is a workspace manifest using workspace support
            let workspace_support = crate::workspace_support::PythonWorkspaceSupport;
            if workspace_support.is_workspace_manifest(&content) {
                return Ok(manifest);
            }
        }

        // Move up
        current = current
            .parent()
            .ok_or_else(|| {
                PluginError::invalid_input("No workspace pyproject.toml found in hierarchy")
            })?
            .to_path_buf();

        // Stop at root
        if current == current.parent().unwrap_or(&current) {
            break;
        }
    }

    Err(PluginError::invalid_input(
        "No workspace pyproject.toml found",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_pyproject_toml_lib() {
        let content = generate_pyproject_toml("test_package", PackageType::Library);
        assert!(content.contains("[project]"));
        assert!(content.contains("name = \"test_package\""));
        assert!(content.contains("version = \"0.1.0\""));
        assert!(content.contains("[build-system]"));
        assert!(!content.contains("[project.scripts]"));
    }

    #[test]
    fn test_generate_pyproject_toml_bin() {
        let content = generate_pyproject_toml("test_app", PackageType::Binary);
        assert!(content.contains("[project]"));
        assert!(content.contains("name = \"test_app\""));
        assert!(content.contains("[project.scripts]"));
        assert!(content.contains("test_app = \"test_app:main\""));
    }

    #[test]
    fn test_entry_file() {
        assert_eq!(
            entry_file("mylib", PackageType::Library),
            PathBuf::from("src/mylib/__init__.py")
        );
        assert_eq!(
            entry_file("myapp", PackageType::Binary),
            PathBuf::from("src/myapp/main.py")
        );
    }
}
