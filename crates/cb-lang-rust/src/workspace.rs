//! Workspace manifest handling for Cargo.toml
//!
//! This module provides functionality for manipulating workspace Cargo.toml files,
//! including adding members, managing workspace configuration, and generating
//! new workspace manifests.

use cb_plugin_api::{PluginError, PluginResult};
use std::path::Path;
use toml_edit::DocumentMut;
use tracing::debug;

/// Add a new member to a workspace Cargo.toml
///
/// # Arguments
///
/// * `workspace_content` - Current workspace Cargo.toml content
/// * `new_member_path` - Absolute path to the new workspace member
/// * `workspace_root` - Absolute path to the workspace root directory
///
/// # Returns
///
/// Updated workspace Cargo.toml content with the new member added
///
/// # Example
///
/// ```rust,ignore
/// let workspace_content = r#"
/// [workspace]
/// members = ["crate1"]
/// "#;
///
/// let updated = add_workspace_member(
///     workspace_content,
///     "/path/to/workspace/crate2",
///     Path::new("/path/to/workspace")
/// )?;
/// // Result will include "crate2" in members array
/// ```
pub fn add_workspace_member(
    workspace_content: &str,
    new_member_path: &str,
    workspace_root: &Path,
) -> PluginResult<String> {
    let mut doc = workspace_content.parse::<DocumentMut>().map_err(|e| {
        PluginError::manifest(format!("Failed to parse workspace Cargo.toml: {}", e))
    })?;

    // Calculate relative path from workspace root to new member
    let target_path = Path::new(new_member_path);
    let relative_path = pathdiff::diff_paths(target_path, workspace_root).ok_or_else(|| {
        PluginError::internal("Failed to calculate relative path for workspace member")
    })?;

    // Ensure [workspace.members] exists
    if !doc.contains_key("workspace") {
        doc["workspace"] = toml_edit::table();
    }

    let workspace = doc["workspace"]
        .as_table_mut()
        .ok_or_else(|| PluginError::manifest("[workspace] is not a table"))?;

    if !workspace.contains_key("members") {
        workspace["members"] = toml_edit::value(toml_edit::Array::new());
    }

    let members = workspace["members"]
        .as_array_mut()
        .ok_or_else(|| PluginError::manifest("[workspace.members] is not an array"))?;

    // Add new member if not already present
    let member_str = relative_path.to_string_lossy();
    let member_exists = members
        .iter()
        .any(|v| v.as_str() == Some(member_str.as_ref()));

    if !member_exists {
        members.push(member_str.as_ref());
        debug!(
            member = %member_str,
            "Added new member to workspace"
        );
    } else {
        debug!(
            member = %member_str,
            "Member already exists in workspace"
        );
    }

    Ok(doc.to_string())
}

/// Add a path dependency to a Cargo.toml file
///
/// # Arguments
///
/// * `cargo_content` - Current Cargo.toml content
/// * `dep_name` - Name of the dependency to add
/// * `dep_path` - Absolute path to the dependency
/// * `source_path` - Absolute path to the source crate directory
///
/// # Returns
///
/// Updated Cargo.toml content with the new dependency added
///
/// # Example
///
/// ```rust,ignore
/// let cargo_content = r#"
/// [package]
/// name = "my-crate"
/// "#;
///
/// let updated = add_path_dependency(
///     cargo_content,
///     "my-dep",
///     "/path/to/workspace/my-dep",
///     Path::new("/path/to/workspace/my-crate")
/// )?;
/// // Result will include: my-dep = { path = "../my-dep" }
/// ```
pub fn add_path_dependency(
    cargo_content: &str,
    dep_name: &str,
    dep_path: &str,
    source_path: &Path,
) -> PluginResult<String> {
    let mut doc = cargo_content
        .parse::<DocumentMut>()
        .map_err(|e| PluginError::manifest(format!("Failed to parse Cargo.toml: {}", e)))?;

    // Calculate relative path from source to target
    let source_cargo_dir = source_path;
    let target_path = Path::new(dep_path);
    let relative_path = pathdiff::diff_paths(target_path, source_cargo_dir)
        .ok_or_else(|| PluginError::internal("Failed to calculate relative path for dependency"))?;

    // Add dependency to [dependencies] section
    if !doc.contains_key("dependencies") {
        doc["dependencies"] = toml_edit::table();
    }

    let deps = doc["dependencies"]
        .as_table_mut()
        .ok_or_else(|| PluginError::manifest("[dependencies] is not a table"))?;

    // Create inline table for path dependency
    let mut dep_table = toml_edit::InlineTable::new();
    dep_table.insert(
        "path",
        toml_edit::Value::from(relative_path.to_string_lossy().as_ref()),
    );

    deps[dep_name] = toml_edit::value(toml_edit::Value::InlineTable(dep_table));

    debug!(
        dependency = %dep_name,
        path = %relative_path.display(),
        "Added path dependency to Cargo.toml"
    );

    Ok(doc.to_string())
}

/// Generate a new workspace Cargo.toml with initial members
///
/// # Arguments
///
/// * `member_paths` - Absolute paths to initial workspace members
/// * `workspace_root` - Absolute path to the workspace root directory
///
/// # Returns
///
/// New workspace Cargo.toml content
///
/// # Example
///
/// ```rust,ignore
/// let workspace_toml = generate_workspace_manifest(
///     &["/workspace/crate1", "/workspace/crate2"],
///     Path::new("/workspace")
/// )?;
/// // Result:
/// // [workspace]
/// // members = ["crate1", "crate2"]
/// // resolver = "2"
/// ```
pub fn generate_workspace_manifest(
    member_paths: &[&str],
    workspace_root: &Path,
) -> PluginResult<String> {
    let mut members_relative = Vec::new();

    for member_path in member_paths {
        let target_path = Path::new(member_path);
        let relative_path = pathdiff::diff_paths(target_path, workspace_root)
            .ok_or_else(|| PluginError::internal("Failed to calculate relative path for member"))?;
        members_relative.push(relative_path.to_string_lossy().to_string());
    }

    let mut lines = vec!["[workspace]".to_string(), "members = [".to_string()];

    for member in &members_relative {
        lines.push(format!("    \"{}\",", member));
    }

    lines.push("]".to_string());
    lines.push("resolver = \"2\"".to_string());

    Ok(lines.join("\n"))
}

/// Check if content represents a workspace manifest
///
/// For Rust, this checks for the presence of [workspace] section
///
/// # Arguments
///
/// * `content` - Manifest file content to check
///
/// # Returns
///
/// true if this is a workspace manifest, false otherwise
pub fn is_workspace_manifest(content: &str) -> bool {
    content.contains("[workspace]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_add_workspace_member() {
        let content = r#"
[workspace]
members = ["crate1"]
"#;

        let result =
            add_workspace_member(content, "/workspace/crate2", &PathBuf::from("/workspace"))
                .unwrap();

        assert!(result.contains("[workspace]"));
        assert!(result.contains("crate1"));
        assert!(result.contains("crate2"));
    }

    #[test]
    fn test_add_workspace_member_existing() {
        let content = r#"
[workspace]
members = ["crate1"]
"#;

        let result =
            add_workspace_member(content, "/workspace/crate1", &PathBuf::from("/workspace"))
                .unwrap();

        // Should not duplicate
        assert!(result.contains("crate1"));
        assert_eq!(result.matches("crate1").count(), 1);
    }

    #[test]
    fn test_add_path_dependency() {
        let content = r#"
[package]
name = "my-crate"
version = "0.1.0"
"#;

        let result = add_path_dependency(
            content,
            "my-dep",
            "/workspace/my-dep",
            &PathBuf::from("/workspace/my-crate"),
        )
        .unwrap();

        assert!(result.contains("[dependencies]"));
        assert!(result.contains("my-dep"));
        assert!(result.contains("path"));
        assert!(result.contains("../my-dep"));
    }

    #[test]
    fn test_generate_workspace_manifest() {
        let result = generate_workspace_manifest(
            &["/workspace/crate1", "/workspace/crate2"],
            &PathBuf::from("/workspace"),
        )
        .unwrap();

        assert!(result.contains("[workspace]"));
        assert!(result.contains("members"));
        assert!(result.contains("crate1"));
        assert!(result.contains("crate2"));
        assert!(result.contains("resolver"));
    }

    #[test]
    fn test_is_workspace_manifest() {
        assert!(is_workspace_manifest("[workspace]\nmembers = []"));
        assert!(!is_workspace_manifest("[package]\nname = \"foo\""));
    }
}
