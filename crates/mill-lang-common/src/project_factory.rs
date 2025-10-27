//! Project factory helper utilities
//!
//! This module provides shared helper functions for language plugins implementing
//! the ProjectFactory trait. These utilities handle common operations like:
//! - Path validation and resolution
//! - File writing with structured logging
//! - Workspace manifest discovery and updating
//!
//! Language-specific logic (template generation, manifest format) remains in each plugin.

use mill_plugin_api::{PluginError, PluginResult};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, error};

/// Trait for detecting language-specific workspace manifests
///
/// Implementations should check if the given file content represents a workspace
/// manifest (e.g., Cargo.toml with `[workspace]`, package.json with `workspaces`, etc.)
pub trait WorkspaceManifestDetector {
    /// Check if the content represents a workspace manifest
    fn is_workspace_manifest(&self, content: &str) -> bool;
}

/// Resolve and validate a package path within a workspace
///
/// This function:
/// - Handles absolute and relative paths
/// - Rejects parent directory traversal (`..` components)
/// - Canonicalizes paths for comparison
/// - Validates the package path is within the workspace boundary
///
/// # Arguments
///
/// * `workspace_root` - The root directory of the workspace
/// * `package_path` - The requested package path (absolute or relative)
///
/// # Returns
///
/// The resolved absolute path if valid, or an error if:
/// - Path contains `..` components (security)
/// - Path is outside the workspace boundary
/// - Workspace root cannot be canonicalized
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use mill_lang_common::project_factory::resolve_package_path;
///
/// let workspace = Path::new("/workspace");
/// let result = resolve_package_path(workspace, "crates/my-package");
/// // Returns: Ok(PathBuf::from("/workspace/crates/my-package"))
///
/// let bad_path = resolve_package_path(workspace, "../outside");
/// // Returns: Err(...) - path contains '..'
/// ```
pub fn resolve_package_path(workspace_root: &Path, package_path: &str) -> PluginResult<PathBuf> {
    let path = Path::new(package_path);

    // Reject paths with parent directory components to prevent traversal attacks
    use std::path::Component;
    for component in path.components() {
        if matches!(component, Component::ParentDir) {
            return Err(PluginError::invalid_input(format!(
                "Package path cannot contain '..' components: {}",
                package_path
            )));
        }
    }

    // Convert to absolute path
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    };

    // Canonicalize workspace root for boundary checking
    let canonical_root = workspace_root.canonicalize().map_err(|e| {
        PluginError::internal(format!("Failed to canonicalize workspace root: {}", e))
    })?;

    // For the resolved path, canonicalize parent if it exists (target doesn't exist yet)
    let canonical_resolved = if let Some(parent) = resolved.parent() {
        if parent.exists() {
            let canonical_parent = parent.canonicalize().map_err(|e| {
                PluginError::internal(format!("Failed to canonicalize parent directory: {}", e))
            })?;
            resolved
                .file_name()
                .map(|name| canonical_parent.join(name))
                .ok_or_else(|| PluginError::invalid_input("Invalid package path"))?
        } else {
            // Parent doesn't exist yet - we'll create it later
            resolved.clone()
        }
    } else {
        resolved.clone()
    };

    // Verify path is within workspace boundary
    if !canonical_resolved.starts_with(&canonical_root) {
        return Err(PluginError::invalid_input(format!(
            "Package path {} is outside workspace",
            package_path
        )));
    }

    Ok(resolved)
}

/// Validate that a package path does not already exist
///
/// # Arguments
///
/// * `package_path` - The path to check
///
/// # Returns
///
/// Ok(()) if the path does not exist, or an error if it already exists
pub fn validate_package_path_not_exists(package_path: &Path) -> PluginResult<()> {
    if package_path.exists() {
        return Err(PluginError::invalid_input(format!(
            "Package already exists at {}",
            package_path.display()
        )));
    }
    Ok(())
}

/// Derive a package name from a path
///
/// Extracts the final component of the path as the package name.
///
/// # Arguments
///
/// * `package_path` - The package path
///
/// # Returns
///
/// The package name as a String, or an error if the path is invalid
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use mill_lang_common::project_factory::derive_package_name;
///
/// let path = Path::new("/workspace/crates/my-package");
/// let name = derive_package_name(path).unwrap();
/// assert_eq!(name, "my-package");
/// ```
pub fn derive_package_name(package_path: &Path) -> PluginResult<String> {
    package_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| {
            PluginError::invalid_input(format!("Invalid package path: {}", package_path.display()))
        })
        .map(|s| s.to_string())
}

/// Write a file to the filesystem with structured logging
///
/// This function writes content to a file and logs the operation with appropriate
/// error handling.
///
/// # Arguments
///
/// * `path` - The file path to write
/// * `content` - The content to write
///
/// # Returns
///
/// Ok(()) on success, or an error if the write fails
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use mill_lang_common::project_factory::write_project_file;
///
/// write_project_file(Path::new("README.md"), "# My Project\n").unwrap();
/// ```
pub fn write_project_file(path: &Path, content: &str) -> PluginResult<()> {
    debug!(path = %path.display(), "Writing project file");
    fs::write(path, content).map_err(|e| {
        error!(error = %e, path = %path.display(), "Failed to write file");
        PluginError::internal(format!("Failed to write file: {}", e))
    })
}

/// Find a workspace manifest by traversing up the directory hierarchy
///
/// This function searches for a workspace manifest starting from the given root
/// and moving up the directory tree. It uses the provided detector to identify
/// workspace manifests (language-specific format detection).
///
/// # Arguments
///
/// * `workspace_root` - Starting directory for the search
/// * `manifest_filename` - Name of the manifest file to look for (e.g., "Cargo.toml")
/// * `detector` - Implementation of WorkspaceManifestDetector for format checking
///
/// # Returns
///
/// The path to the workspace manifest, or an error if not found
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use mill_lang_common::project_factory::{find_workspace_manifest, WorkspaceManifestDetector};
///
/// struct CargoDetector;
/// impl WorkspaceManifestDetector for CargoDetector {
///     fn is_workspace_manifest(&self, content: &str) -> bool {
///         content.contains("[workspace]")
///     }
/// }
///
/// let manifest = find_workspace_manifest(
///     Path::new("/workspace"),
///     "Cargo.toml",
///     &CargoDetector,
/// ).unwrap();
/// ```
pub fn find_workspace_manifest(
    workspace_root: &Path,
    manifest_filename: &str,
    detector: &dyn WorkspaceManifestDetector,
) -> PluginResult<PathBuf> {
    let mut current = workspace_root.to_path_buf();

    loop {
        let manifest = current.join(manifest_filename);

        if manifest.exists() {
            let content = fs::read_to_string(&manifest).map_err(|e| {
                PluginError::internal(format!("Failed to read {}: {}", manifest_filename, e))
            })?;

            if detector.is_workspace_manifest(&content) {
                return Ok(manifest);
            }
        }

        // Move up to parent directory
        current = current
            .parent()
            .ok_or_else(|| {
                PluginError::invalid_input(format!(
                    "No workspace {} found in hierarchy",
                    manifest_filename
                ))
            })?
            .to_path_buf();

        // Stop at filesystem root
        if current == current.parent().unwrap_or(&current) {
            break;
        }
    }

    Err(PluginError::invalid_input(format!(
        "No workspace {} found",
        manifest_filename
    )))
}

/// Update a workspace manifest to add a new member
///
/// This function:
/// 1. Finds the workspace manifest using the detector
/// 2. Reads the current content
/// 3. Calculates the relative path from manifest to package
/// 4. Calls the workspace support's add_workspace_member method
/// 5. Writes the updated content back if changed
///
/// # Arguments
///
/// * `workspace_root` - The workspace root directory
/// * `package_path` - The path to the new package
/// * `manifest_filename` - Name of the manifest file (e.g., "Cargo.toml")
/// * `detector` - Implementation for detecting workspace manifests
/// * `add_member_fn` - Function to add member to manifest content
///
/// # Returns
///
/// Ok(true) if the manifest was updated, Ok(false) if no change was needed,
/// or an error if the operation fails
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use mill_lang_common::project_factory::{update_workspace_manifest, WorkspaceManifestDetector};
///
/// struct CargoDetector;
/// impl WorkspaceManifestDetector for CargoDetector {
///     fn is_workspace_manifest(&self, content: &str) -> bool {
///         content.contains("[workspace]")
///     }
/// }
///
/// let updated = update_workspace_manifest(
///     Path::new("/workspace"),
///     Path::new("/workspace/crates/new-crate"),
///     "Cargo.toml",
///     &CargoDetector,
///     |content, member| {
///         // Add member to content and return updated content
///         format!("{}\nmembers = [\"{}\"]", content, member)
///     },
/// ).unwrap();
/// ```
pub fn update_workspace_manifest<F>(
    workspace_root: &Path,
    package_path: &Path,
    manifest_filename: &str,
    detector: &dyn WorkspaceManifestDetector,
    add_member_fn: F,
) -> PluginResult<bool>
where
    F: FnOnce(&str, &str) -> String,
{
    // Find workspace manifest
    let workspace_manifest = find_workspace_manifest(workspace_root, manifest_filename, detector)?;

    debug!(
        workspace_manifest = %workspace_manifest.display(),
        "Found workspace manifest"
    );

    // Read current content
    let content = fs::read_to_string(&workspace_manifest).map_err(|e| {
        error!(
            error = %e,
            workspace_manifest = %workspace_manifest.display(),
            "Failed to read workspace manifest"
        );
        PluginError::internal(format!(
            "Failed to read workspace {}: {}",
            manifest_filename, e
        ))
    })?;

    // Calculate relative path from manifest directory to package
    let workspace_dir = workspace_manifest
        .parent()
        .ok_or_else(|| PluginError::internal("Invalid workspace manifest path"))?;

    let relative_path = pathdiff::diff_paths(package_path, workspace_dir)
        .ok_or_else(|| PluginError::internal("Failed to calculate relative path"))?;

    // Normalize to forward slashes for cross-platform compatibility
    let member_str = relative_path.to_string_lossy().replace('\\', "/");

    debug!(member = %member_str, "Adding workspace member");

    // Call the language-specific add member function
    let updated_content = add_member_fn(&content, &member_str);

    // Write back if changed
    if updated_content != content {
        fs::write(&workspace_manifest, &updated_content).map_err(|e| {
            error!(
                error = %e,
                workspace_manifest = %workspace_manifest.display(),
                "Failed to write workspace manifest"
            );
            PluginError::internal(format!(
                "Failed to write workspace {}: {}",
                manifest_filename, e
            ))
        })?;

        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_package_path_relative() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path();

        let result = resolve_package_path(workspace, "crates/my-package").unwrap();
        assert_eq!(result, workspace.join("crates/my-package"));
    }

    #[test]
    fn test_resolve_package_path_rejects_parent_traversal() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path();

        let result = resolve_package_path(workspace, "../outside");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cannot contain '..'"));
    }

    #[test]
    fn test_resolve_package_path_absolute() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path();
        let package = workspace.join("crates/my-package");

        let result = resolve_package_path(workspace, &package.to_string_lossy()).unwrap();
        assert_eq!(result, package);
    }

    #[test]
    fn test_validate_package_path_not_exists() {
        let temp = TempDir::new().unwrap();
        let non_existent = temp.path().join("does-not-exist");

        assert!(validate_package_path_not_exists(&non_existent).is_ok());

        // Create the path and verify it fails
        fs::create_dir(&non_existent).unwrap();
        let result = validate_package_path_not_exists(&non_existent);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_derive_package_name() {
        let path = Path::new("/workspace/crates/my-package");
        let name = derive_package_name(path).unwrap();
        assert_eq!(name, "my-package");
    }

    #[test]
    fn test_write_project_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");

        write_project_file(&file_path, "Hello, world!").unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Hello, world!");
    }

    struct TestDetector;
    impl WorkspaceManifestDetector for TestDetector {
        fn is_workspace_manifest(&self, content: &str) -> bool {
            content.contains("[workspace]")
        }
    }

    #[test]
    fn test_find_workspace_manifest() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path();

        // Create a workspace manifest
        let manifest_path = workspace.join("Cargo.toml");
        fs::write(&manifest_path, "[workspace]\nmembers = []").unwrap();

        // Create a subdirectory
        let subdir = workspace.join("crates");
        fs::create_dir(&subdir).unwrap();

        // Find manifest from subdirectory
        let found = find_workspace_manifest(&subdir, "Cargo.toml", &TestDetector).unwrap();
        assert_eq!(found, manifest_path);
    }

    #[test]
    fn test_find_workspace_manifest_not_found() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path();

        let result = find_workspace_manifest(workspace, "Cargo.toml", &TestDetector);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        // Should contain "found" in some form: "No workspace Cargo.toml found" or "found in hierarchy"
        assert!(
            err_msg.contains("found"),
            "Error message should contain 'found', got: {}",
            err_msg
        );
    }

    #[test]
    fn test_update_workspace_manifest() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path();

        // Create workspace manifest
        let manifest_path = workspace.join("Cargo.toml");
        fs::write(&manifest_path, "[workspace]\nmembers = []").unwrap();

        // Create package directory
        let package_path = workspace.join("crates/new-package");
        fs::create_dir_all(&package_path).unwrap();

        // Update manifest
        let updated = update_workspace_manifest(
            workspace,
            &package_path,
            "Cargo.toml",
            &TestDetector,
            |content, member| format!("{}\n# Added: {}", content, member),
        )
        .unwrap();

        assert!(updated);

        // Verify content was updated
        let content = fs::read_to_string(&manifest_path).unwrap();
        assert!(content.contains("crates/new-package"));
    }

    #[test]
    fn test_update_workspace_manifest_no_change() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path();

        // Create workspace manifest
        let manifest_path = workspace.join("Cargo.toml");
        let original_content = "[workspace]\nmembers = []";
        fs::write(&manifest_path, original_content).unwrap();

        // Create package directory
        let package_path = workspace.join("crates/new-package");
        fs::create_dir_all(&package_path).unwrap();

        // Update manifest with no-op function
        let updated = update_workspace_manifest(
            workspace,
            &package_path,
            "Cargo.toml",
            &TestDetector,
            |content, _member| content.to_string(), // No change
        )
        .unwrap();

        assert!(!updated);

        // Verify content unchanged
        let content = fs::read_to_string(&manifest_path).unwrap();
        assert_eq!(content, original_content);
    }
}
