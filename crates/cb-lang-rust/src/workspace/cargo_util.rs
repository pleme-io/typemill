//! Cargo package detection and manifest handling for directory moves

use mill_foundation::protocol::{ ApiError as ServerError , ApiResult as ServerResult , EditLocation , EditType , TextEdit , };
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Check if a directory is a Cargo package (contains Cargo.toml with [package] section)
pub async fn is_cargo_package(dir_path: &Path) -> ServerResult<bool> {
    let cargo_toml = dir_path.join("Cargo.toml");
    if !cargo_toml.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&cargo_toml)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to read Cargo.toml: {}", e)))?;

    // Check if it has a [package] section (distinguishes packages from workspaces)
    Ok(content.contains("[package]"))
}

/// Extract cargo rename information for use in import updates
pub async fn extract_cargo_rename_info(old_dir: &Path, new_dir: &Path) -> ServerResult<Value> {
    let cargo_toml = old_dir.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to read Cargo.toml: {}", e)))?;

    // Parse package name from Cargo.toml
    let old_package_name = extract_package_name(&content)?;

    // Infer new package name from directory name
    let new_package_name = new_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| ServerError::Internal("Invalid new directory name".to_string()))?
        .to_string();

    // Convert to snake_case for use in import paths
    let old_crate_name = old_package_name.replace('-', "_");
    let new_crate_name = new_package_name.replace('-', "_");

    Ok(json!({
        "old_package_name": old_package_name,
        "new_package_name": new_package_name,
        "old_crate_name": old_crate_name,
        "new_crate_name": new_crate_name,
    }))
}

/// Extract consolidation rename information for import updating
///
/// This calculates:
/// - old_crate_name: The name from the old Cargo.toml
/// - new_import_prefix: The new import path (e.g., "target_crate::submodule")
/// - submodule_name: The name of the subdirectory that will contain the consolidated code
/// - target_crate_name: The name of the target crate
pub async fn extract_consolidation_rename_info(
    old_package_path: &Path,
    new_package_path: &Path,
) -> ServerResult<Value> {
    // Read the old Cargo.toml to get the old crate name
    let old_cargo_toml = old_package_path.join("Cargo.toml");
    let old_content = fs::read_to_string(&old_cargo_toml)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to read old Cargo.toml: {}", e)))?;

    let old_crate_name = extract_package_name(&old_content)?.replace('-', "_");

    // Find the target crate by looking for Cargo.toml in parent directories
    let mut target_crate_name = String::new();
    let mut current = new_package_path;

    while let Some(parent) = current.parent() {
        let cargo_toml = parent.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = fs::read_to_string(&cargo_toml).await {
                if content.contains("[package]") {
                    // Found the target crate
                    if let Ok(name) = extract_package_name(&content) {
                        target_crate_name = name.replace('-', "_");
                        break;
                    }
                }
            }
        }
        current = parent;
    }

    if target_crate_name.is_empty() {
        return Err(ServerError::Internal(
            "Could not find target crate Cargo.toml".to_string(),
        ));
    }

    // Extract submodule name from the new path
    // e.g., "crates/cb-types/src/protocol" -> "protocol"
    let submodule_name = new_package_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| ServerError::Internal("Invalid new directory path".to_string()))?
        .to_string();

    // Build the new import prefix
    // e.g., "mill_foundation::protocol"
    let new_import_prefix = format!("{}::{}", target_crate_name, submodule_name);

    tracing::info!(
        old_crate_name = %old_crate_name,
        new_import_prefix = %new_import_prefix,
        submodule_name = %submodule_name,
        target_crate_name = %target_crate_name,
        "Extracted consolidation rename information"
    );

    Ok(json!({
        "old_crate_name": old_crate_name,
        "new_crate_name": new_import_prefix.clone(), // For compatibility with update_imports_for_rename
        "new_import_prefix": new_import_prefix,
        "submodule_name": submodule_name,
        "target_crate_name": target_crate_name,
    }))
}

/// Extract package name from Cargo.toml content
fn extract_package_name(content: &str) -> ServerResult<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name") && trimmed.contains('=') {
            if let Some(name_part) = trimmed.split('=').nth(1) {
                let name = name_part
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                return Ok(name);
            }
        }
    }
    Err(ServerError::Internal(
        "Could not find package name in Cargo.toml".to_string(),
    ))
}

/// Convert manifest update tuples into TextEdit entries
///
/// Adjusts file paths for files that will be moved by the directory rename:
/// - Files inside old_dir_path are adjusted to point to new_dir_path
/// - Files outside old_dir_path remain unchanged
pub fn convert_manifest_updates_to_edits(
    updates: Vec<(PathBuf, String, String)>,
    old_dir_path: &Path,
    new_dir_path: &Path,
) -> Vec<TextEdit> {
    updates
        .into_iter()
        .map(|(file_path, old_content, new_content)| {
            // Adjust file path if it's inside the directory being renamed
            let adjusted_path = if file_path.starts_with(old_dir_path) {
                // File is inside renamed directory, adjust path to new location
                if let Ok(rel_path) = file_path.strip_prefix(old_dir_path) {
                    new_dir_path.join(rel_path)
                } else {
                    file_path.clone()
                }
            } else {
                // File is outside renamed directory, path unchanged
                file_path.clone()
            };

            // Normalize trailing newlines in new_content to match old_content
            // toml_edit ensures files end with '\n', but we need to match the original
            let normalized_new_content = if old_content.ends_with('\n') {
                // Original has trailing newline - ensure new content has exactly one
                new_content.trim_end().to_string() + "\n"
            } else {
                // Original has no trailing newline - remove from new content
                new_content.trim_end().to_string()
            };

            // Calculate range covering the entire file
            // Always use the end of the last line of content, regardless of trailing newline
            let total_lines = old_content.lines().count() as u32;
            let last_line_len = old_content
                .lines()
                .last()
                .map(|l| l.chars().count() as u32)
                .unwrap_or(0);
            let end_line = total_lines.saturating_sub(1);
            let end_column = last_line_len;

            TextEdit {
                file_path: Some(adjusted_path.to_string_lossy().to_string()),
                edit_type: EditType::Replace,
                location: EditLocation {
                    start_line: 0,
                    start_column: 0,
                    end_line,
                    end_column,
                },
                original_text: String::new(), // Skip safety check - this is a trusted full-file replacement
                new_text: normalized_new_content,
                priority: 10, // Give manifest updates high priority
                description: format!(
                    "Update Cargo.toml manifest: {}",
                    adjusted_path.to_string_lossy()
                ),
            }
        })
        .collect()
}

/// Adjust a relative path based on depth change
///
/// When a package moves to a different depth in the directory tree,
/// its relative path dependencies need to be adjusted.
///
/// # Arguments
/// * `path` - The current relative path (e.g., "../sibling")
/// * `old_depth` - How deep the package was (number of components from workspace root)
/// * `new_depth` - How deep the package is now
///
/// # Returns
/// The adjusted relative path
fn adjust_relative_path(path: &str, old_depth: usize, new_depth: usize) -> String {
    let depth_diff = new_depth as i32 - old_depth as i32;

    if depth_diff > 0 {
        // Moved deeper, add more "../"
        let additional_uplevels = "../".repeat(depth_diff as usize);
        format!("{}{}", additional_uplevels, path)
    } else if depth_diff < 0 {
        // Moved shallower, remove "../"
        let uplevels_to_remove = (-depth_diff) as usize;
        let mut remaining = path;
        for _ in 0..uplevels_to_remove {
            remaining = remaining.strip_prefix("../").unwrap_or(remaining);
        }
        remaining.to_string()
    } else {
        path.to_string()
    }
}

/// Plan workspace manifest updates for a Cargo package rename
///
/// Returns a list of (file_path, old_content, new_content) tuples for each Cargo.toml
/// that would be updated.
///
/// This updates:
/// 1. Workspace Cargo.toml members list
/// 2. Package's own Cargo.toml name and relative path dependencies
pub async fn plan_workspace_manifest_updates(
    old_package_path: &Path,
    new_package_path: &Path,
    project_root: &Path,
) -> ServerResult<Vec<(PathBuf, String, String)>> {
    let mut planned_updates = Vec::new();
    let mut current_path = old_package_path.parent();

    while let Some(path) = current_path {
        let workspace_toml_path = path.join("Cargo.toml");
        if workspace_toml_path.exists() {
            let content = fs::read_to_string(&workspace_toml_path)
                .await
                .map_err(|e| {
                    ServerError::Internal(format!("Failed to read workspace Cargo.toml: {}", e))
                })?;

            if content.contains("[workspace]") {
                let mut doc = content.parse::<toml_edit::DocumentMut>().map_err(|e| {
                    ServerError::Internal(format!("Failed to parse workspace Cargo.toml: {}", e))
                })?;

                let old_rel_path = old_package_path.strip_prefix(path).map_err(|_| {
                    ServerError::Internal("Failed to calculate old relative path".to_string())
                })?;
                let new_rel_path = new_package_path.strip_prefix(path).map_err(|_| {
                    ServerError::Internal("Failed to calculate new relative path".to_string())
                })?;

                let old_path_str = old_rel_path.to_string_lossy().to_string();
                let new_path_str = new_rel_path.to_string_lossy().to_string();

                let members = doc["workspace"]["members"].as_array_mut().ok_or_else(|| {
                    ServerError::Internal("`[workspace.members]` is not a valid array".to_string())
                })?;

                let index_opt = members
                    .iter()
                    .position(|m| m.as_str() == Some(&old_path_str));

                if let Some(index) = index_opt {
                    members.remove(index);
                    members.push(new_path_str.as_str());

                    let new_content = doc.to_string();
                    planned_updates.push((
                        workspace_toml_path.clone(),
                        content.clone(),
                        new_content,
                    ));

                    // Also plan updates for the package's own Cargo.toml
                    // IMPORTANT: Read from OLD path since directory hasn't been renamed yet during planning
                    let package_cargo_toml = old_package_path.join("Cargo.toml");
                    if package_cargo_toml.exists() {
                        if let Ok((pkg_path, pkg_old, pkg_new)) = plan_package_manifest_update(
                            &package_cargo_toml,
                            old_package_path,
                            new_package_path,
                            path,
                        )
                        .await
                        {
                            planned_updates.push((pkg_path, pkg_old, pkg_new));
                        }
                    }
                }

                return Ok(planned_updates);
            }
        }

        if path == project_root {
            break;
        }
        current_path = path.parent();
    }

    Ok(planned_updates)
}

/// Plan package manifest update (name and relative paths)
///
/// Returns (file_path, old_content, new_content) tuple.
///
/// This updates:
/// 1. [package].name to match new directory name
/// 2. Relative path dependencies based on depth change
async fn plan_package_manifest_update(
    package_cargo_toml: &Path,
    old_package_path: &Path,
    new_package_path: &Path,
    workspace_root: &Path,
) -> ServerResult<(PathBuf, String, String)> {
    let content = fs::read_to_string(package_cargo_toml)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to read package Cargo.toml: {}", e)))?;

    let mut doc = content
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| ServerError::Internal(format!("Failed to parse package Cargo.toml: {}", e)))?;

    let mut updated = false;

    // Update [package].name
    let new_dir_name = new_package_path.file_name().and_then(|n| n.to_str());
    if let Some(new_name) = new_dir_name {
        let new_crate_name = new_name.replace('_', "-");
        if let Some(package_section) = doc.get_mut("package") {
            if let Some(name_field) = package_section.get_mut("name") {
                let old_name = name_field.as_str().unwrap_or("");
                if old_name != new_crate_name {
                    *name_field = toml_edit::value(new_crate_name);
                    updated = true;
                }
            }
        }
    }

    // Update relative path dependencies
    let old_depth = old_package_path
        .strip_prefix(workspace_root)
        .map(|p| p.components().count())
        .unwrap_or(0);
    let new_depth = new_package_path
        .strip_prefix(workspace_root)
        .map(|p| p.components().count())
        .unwrap_or(0);

    let update_deps_in_table = |deps: &mut toml_edit::Table, updated: &mut bool| {
        for (_name, value) in deps.iter_mut() {
            if let Some(table) = value.as_inline_table_mut() {
                if let Some(path_value) = table.get_mut("path") {
                    if let Some(old_path_str) = path_value.as_str() {
                        let new_path_str = adjust_relative_path(old_path_str, old_depth, new_depth);
                        if new_path_str != old_path_str {
                            *path_value = new_path_str.as_str().into();
                            *updated = true;
                        }
                    }
                }
            } else if let Some(table) = value.as_table_mut() {
                if let Some(path_value) = table.get_mut("path") {
                    if let Some(old_path_str) = path_value.as_str() {
                        let new_path_str = adjust_relative_path(old_path_str, old_depth, new_depth);
                        if new_path_str != old_path_str {
                            *path_value = new_path_str.as_str().into();
                            *updated = true;
                        }
                    }
                }
            }
        }
    };

    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(deps) = doc[section].as_table_mut() {
            update_deps_in_table(deps, &mut updated);
        }
    }

    if updated {
        // IMPORTANT: Return the path at the OLD location (where file currently exists)
        // The directory rename operation will move this file to the new location,
        // but during planning the file is still at the old location.
        // The handler will calculate checksums from files at their current (old) locations.
        Ok((package_cargo_toml.to_path_buf(), content, doc.to_string()))
    } else {
        Err(ServerError::Internal("No updates needed".to_string()))
    }
}

/// Plan dependent crate path updates for a Cargo package rename
///
/// Scans all Cargo.toml files in the workspace and plans updates for any that
/// depend on the renamed crate.
///
/// Returns a list of (file_path, old_content, new_content) tuples.
pub async fn plan_dependent_crate_path_updates(
    old_crate_name: &str,
    new_crate_name: &str,
    new_crate_path: &Path,
    project_root: &Path,
) -> ServerResult<Vec<(PathBuf, String, String)>> {
    let mut planned_updates = Vec::new();

    let walker = ignore::WalkBuilder::new(project_root).hidden(false).build();

    for entry in walker.flatten() {
        let path = entry.path();
        if path.file_name() == Some(std::ffi::OsStr::new("Cargo.toml")) {
            if path.parent() == Some(new_crate_path) {
                continue;
            }

            let content = match fs::read_to_string(path).await {
                Ok(c) => c,
                Err(_) => continue,
            };

            if !content.contains(old_crate_name) {
                continue;
            }

            match plan_single_cargo_toml_dependency_update(
                path,
                old_crate_name,
                new_crate_name,
                new_crate_path,
                &content,
            )
            .await
            {
                Ok(Some((file_path, old_content, new_content))) => {
                    planned_updates.push((file_path, old_content, new_content));
                }
                Ok(None) => {}
                Err(_) => continue,
            }
        }
    }

    Ok(planned_updates)
}

/// Plan a single Cargo.toml dependency update
///
/// Updates the dependency name and path if the old crate name is found.
async fn plan_single_cargo_toml_dependency_update(
    cargo_toml_path: &Path,
    old_crate_name: &str,
    new_crate_name: &str,
    new_crate_path: &Path,
    content: &str,
) -> ServerResult<Option<(PathBuf, String, String)>> {
    let mut doc = content
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| ServerError::Internal(format!("Failed to parse Cargo.toml: {}", e)))?;

    let mut updated = false;
    let cargo_toml_dir = cargo_toml_path.parent().ok_or_else(|| {
        ServerError::Internal(format!(
            "Cannot get parent directory of Cargo.toml: {}",
            cargo_toml_path.display()
        ))
    })?;

    let update_dep_in_table = |table: &mut dyn toml_edit::TableLike,
                               updated: &mut bool|
     -> ServerResult<()> {
        if let Some(mut dep) = table.remove(old_crate_name) {
            if let Some(dep_table) = dep.as_inline_table_mut() {
                if dep_table.contains_key("path") {
                    let new_rel_path = pathdiff::diff_paths(new_crate_path, cargo_toml_dir)
                        .ok_or_else(|| {
                            ServerError::Internal("Failed to calculate relative path".to_string())
                        })?;
                    dep_table.insert(
                        "path",
                        toml_edit::Value::from(new_rel_path.to_string_lossy().to_string()),
                    );
                }
            } else if let Some(dep_table) = dep.as_table_mut() {
                if dep_table.contains_key("path") {
                    let new_rel_path = pathdiff::diff_paths(new_crate_path, cargo_toml_dir)
                        .ok_or_else(|| {
                            ServerError::Internal("Failed to calculate relative path".to_string())
                        })?;
                    dep_table.insert(
                        "path",
                        toml_edit::value(new_rel_path.to_string_lossy().to_string()),
                    );
                }
            }
            table.insert(new_crate_name, dep);
            *updated = true;
        }
        Ok(())
    };

    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(deps) = doc.get_mut(section).and_then(|d| d.as_table_like_mut()) {
            update_dep_in_table(deps, &mut updated)?;
        }
    }

    if let Some(workspace) = doc.get_mut("workspace").and_then(|w| w.as_table_mut()) {
        if let Some(deps) = workspace
            .get_mut("dependencies")
            .and_then(|d| d.as_table_like_mut())
        {
            update_dep_in_table(deps, &mut updated)?;
        }
    }

    // Update feature flags that reference the renamed crate
    // Feature flags use basename-only syntax: "crate-name/feature" or "dep:crate-name"
    if let Some(features) = doc.get_mut("features").and_then(|f| f.as_table_mut()) {
        for (_feature_name, feature_value) in features.iter_mut() {
            if let Some(feature_list) = feature_value.as_array_mut() {
                for item in feature_list.iter_mut() {
                    if let Some(feature_ref) = item.as_str() {
                        // Handle "crate-name/feature-name" syntax
                        if let Some((crate_part, feature_part)) = feature_ref.split_once('/') {
                            if crate_part == old_crate_name {
                                *item = toml_edit::Value::from(format!("{}/{}", new_crate_name, feature_part));
                                updated = true;
                            }
                        }
                        // Handle "dep:crate-name" syntax
                        else if let Some(dep_name) = feature_ref.strip_prefix("dep:") {
                            if dep_name == old_crate_name {
                                *item = toml_edit::Value::from(format!("dep:{}", new_crate_name));
                                updated = true;
                            }
                        }
                        // Handle bare crate name (implicit dependency feature)
                        // e.g. runtime = ["codebuddy-foundation", "mill-config"]
                        else if feature_ref == old_crate_name {
                            *item = toml_edit::Value::from(new_crate_name.to_string());
                            updated = true;
                        }
                    }
                }
            }
        }
    }

    if updated {
        Ok(Some((
            cargo_toml_path.to_path_buf(),
            content.to_string(),
            doc.to_string(),
        )))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_package_name() {
        let content = r#"
[package]
name = "my-package"
version = "0.1.0"
"#;
        let name = extract_package_name(content).unwrap();
        assert_eq!(name, "my-package");
    }

    #[test]
    fn test_extract_package_name_single_quotes() {
        let content = r#"
[package]
name = 'my-package'
version = "0.1.0"
"#;
        let name = extract_package_name(content).unwrap();
        assert_eq!(name, "my-package");
    }

    #[test]
    fn test_convert_manifest_updates_inside_dir() {
        let old_dir = PathBuf::from("/project/old-crate");
        let new_dir = PathBuf::from("/project/new-crate");

        let updates = vec![(
            PathBuf::from("/project/old-crate/Cargo.toml"),
            "old content\n".to_string(),
            "new content\n".to_string(),
        )];

        let edits = convert_manifest_updates_to_edits(updates, &old_dir, &new_dir);

        assert_eq!(edits.len(), 1);
        assert_eq!(
            edits[0].file_path,
            Some("/project/new-crate/Cargo.toml".to_string())
        );
    }

    #[test]
    fn test_convert_manifest_updates_outside_dir() {
        let old_dir = PathBuf::from("/project/old-crate");
        let new_dir = PathBuf::from("/project/new-crate");

        let updates = vec![(
            PathBuf::from("/project/Cargo.toml"),
            "old content\n".to_string(),
            "new content\n".to_string(),
        )];

        let edits = convert_manifest_updates_to_edits(updates, &old_dir, &new_dir);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].file_path, Some("/project/Cargo.toml".to_string()));
    }

    #[tokio::test]
    async fn test_bare_crate_name_in_features() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");

        let content = r#"
[package]
name = "test-package"
version = "0.1.0"

[dependencies]
codebuddy-foundation = { path = "../codebuddy-foundation", optional = true }
mill-config = { path = "../mill-config", optional = true }

[features]
default = ["runtime"]
runtime = ["codebuddy-foundation", "mill-config"]
experimental = ["dep:codebuddy-foundation"]
advanced = ["codebuddy-foundation/serde"]
"#;

        std::fs::write(&cargo_toml, content).unwrap();

        let new_crate_path = temp_dir.path().join("../mill-foundation");
        let result = plan_single_cargo_toml_dependency_update(
            &cargo_toml,
            "codebuddy-foundation",
            "mill-foundation",
            &new_crate_path,
            content,
        )
        .await
        .unwrap();

        assert!(result.is_some(), "Should return an update");

        let (_path, _original_content, updated_content) = result.unwrap();

        // Verify bare crate name was updated
        assert!(
            updated_content.contains(r#"runtime = ["mill-foundation", "mill-config"]"#),
            "Bare crate name in feature array should be updated.\nActual content:\n{}",
            updated_content
        );

        // Verify dep: syntax was updated
        assert!(
            updated_content.contains(r#"experimental = ["dep:mill-foundation"]"#),
            "dep: syntax should be updated.\nActual content:\n{}",
            updated_content
        );

        // Verify crate/feature syntax was updated
        assert!(
            updated_content.contains(r#"advanced = ["mill-foundation/serde"]"#),
            "crate/feature syntax should be updated.\nActual content:\n{}",
            updated_content
        );

        // Verify dependency was also updated
        assert!(
            updated_content.contains("mill-foundation = { path ="),
            "Dependency declaration should be updated.\nActual content:\n{}",
            updated_content
        );
    }
}