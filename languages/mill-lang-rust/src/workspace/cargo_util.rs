//! Cargo package detection and manifest handling for directory moves

use mill_foundation::protocol::{
    ApiError as ServerError, ApiResult as ServerResult, EditLocation, EditType, TextEdit,
};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::debug;

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

                // Normalize comparison: trim whitespace from both sides to handle formatting quirks
                let index_opt = members
                    .iter()
                    .position(|m| m.as_str().map(|s| s.trim()) == Some(old_path_str.trim()));

                if let Some(index) = index_opt {
                    // Update in-place to preserve array order and formatting
                    members.replace(index, new_path_str.as_str());

                    // Also update workspace.dependencies if the key matches the crate name
                    // Derive crate names from directory names (convert _ → -)
                    let old_crate_name = old_package_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.replace('_', "-"))
                        .unwrap_or_default();
                    let new_crate_name = new_package_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.replace('_', "-"))
                        .unwrap_or_default();

                    if let Some(deps) = doc
                        .get_mut("workspace")
                        .and_then(|w| w.get_mut("dependencies"))
                        .and_then(|d| d.as_table_mut())
                    {
                        // Check if old crate name exists in dependencies
                        if let Some(dep_value) = deps.remove(&old_crate_name) {
                            // Update path in the dependency if it exists
                            let mut updated_dep = dep_value;
                            if let Some(dep_table) = updated_dep.as_inline_table_mut() {
                                if let Some(path_entry) = dep_table.get_mut("path") {
                                    *path_entry = toml_edit::Value::from(new_path_str.as_str());
                                }
                            }
                            // Reinsert under new crate name
                            deps.insert(&new_crate_name, updated_dep);
                        }
                    }

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

/// Plan updates for the moved crate's own path dependencies
///
/// When a crate moves, its path dependencies need to be recalculated relative to the new location.
/// For example, moving from `crates/mill-lang-markdown` to `languages/mill-lang-markdown`
/// changes `{ path = "../mill-plugin-api" }` to `{ path = "../../crates/mill-plugin-api" }`.
///
/// Returns (file_path, old_content, new_content) if updates are needed, or None if no path deps exist.
pub async fn plan_moved_crate_own_path_dependencies(
    old_crate_path: &Path,
    new_crate_path: &Path,
) -> ServerResult<Option<(PathBuf, String, String)>> {
    let cargo_toml = old_crate_path.join("Cargo.toml");

    // Read current Cargo.toml content
    let content = fs::read_to_string(&cargo_toml)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to read Cargo.toml: {}", e)))?;

    let mut doc = content
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| ServerError::Internal(format!("Failed to parse Cargo.toml: {}", e)))?;

    let mut updated = false;

    // Helper to update path dependencies in a dependency table
    let update_paths_in_table = |table: &mut dyn toml_edit::TableLike,
                                   old_dir: &Path,
                                   new_dir: &Path,
                                   updated: &mut bool| -> ServerResult<()> {
        for (_dep_name, dep_value) in table.iter_mut() {
            // Handle inline table: { path = "...", version = "..." }
            if let Some(dep_table) = dep_value.as_inline_table_mut() {
                if let Some(path_value) = dep_table.get("path") {
                    if let Some(old_rel_path_str) = path_value.as_str() {
                        // Resolve the old relative path to an absolute path and normalize it
                        let abs_dep_path = old_dir.join(old_rel_path_str);
                        // Canonicalize to normalize the path (resolve ..)
                        let abs_dep_path = match abs_dep_path.canonicalize() {
                            Ok(p) => p,
                            Err(_) => abs_dep_path, // Fallback to unnormalized if canonicalize fails
                        };

                        // Calculate new relative path from new location
                        if let Some(new_rel_path) = pathdiff::diff_paths(&abs_dep_path, new_dir) {
                            let new_rel_path_str = new_rel_path.to_string_lossy().to_string();
                            if new_rel_path_str != old_rel_path_str {
                                dep_table.insert(
                                    "path",
                                    toml_edit::Value::from(new_rel_path_str)
                                );
                                *updated = true;
                            }
                        }
                    }
                }
            }
            // Handle regular table: [dependencies.foo] / path = "..."
            else if let Some(dep_table) = dep_value.as_table_mut() {
                if let Some(path_value) = dep_table.get("path") {
                    if let Some(old_rel_path_str) = path_value.as_str() {
                        // Resolve the old relative path to an absolute path and normalize it
                        let abs_dep_path = old_dir.join(old_rel_path_str);
                        // Canonicalize to normalize the path (resolve ..)
                        let abs_dep_path = match abs_dep_path.canonicalize() {
                            Ok(p) => p,
                            Err(_) => abs_dep_path, // Fallback to unnormalized if canonicalize fails
                        };

                        // Calculate new relative path from new location
                        if let Some(new_rel_path) = pathdiff::diff_paths(&abs_dep_path, new_dir) {
                            let new_rel_path_str = new_rel_path.to_string_lossy().to_string();
                            if new_rel_path_str != old_rel_path_str {
                                dep_table.insert(
                                    "path",
                                    toml_edit::value(new_rel_path_str)
                                );
                                *updated = true;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    };

    // Update path dependencies in [dependencies]
    if let Some(deps) = doc.get_mut("dependencies").and_then(|d| d.as_table_mut()) {
        update_paths_in_table(deps, old_crate_path, new_crate_path, &mut updated)?;
    }

    // Update path dependencies in [dev-dependencies]
    if let Some(dev_deps) = doc.get_mut("dev-dependencies").and_then(|d| d.as_table_mut()) {
        update_paths_in_table(dev_deps, old_crate_path, new_crate_path, &mut updated)?;
    }

    // Update path dependencies in [build-dependencies]
    if let Some(build_deps) = doc.get_mut("build-dependencies").and_then(|d| d.as_table_mut()) {
        update_paths_in_table(build_deps, old_crate_path, new_crate_path, &mut updated)?;
    }

    if updated {
        Ok(Some((
            cargo_toml.to_path_buf(),
            content,
            doc.to_string(),
        )))
    } else {
        Ok(None)
    }
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
                                *item = toml_edit::Value::from(format!(
                                    "{}/{}",
                                    new_crate_name, feature_part
                                ));
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
                        // e.g. runtime = ["mill-foundation", "mill-config"]
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

/// Plan workspace manifest updates for batch rename operations
pub async fn plan_workspace_manifest_updates_for_batch(
    moves: &[(PathBuf, PathBuf)],
    project_root: &Path,
) -> ServerResult<Vec<(PathBuf, String, String)>> {
    if moves.is_empty() {
        return Ok(Vec::new());
    }

    let mut planned_updates = Vec::new();
    let mut current_path = moves[0].0.parent();

    while let Some(path) = current_path {
        let workspace_toml_path = path.join("Cargo.toml");
        if workspace_toml_path.exists() {
            let content = fs::read_to_string(&workspace_toml_path).await.map_err(|e| {
                ServerError::Internal(format!("Failed to read workspace Cargo.toml: {}", e))
            })?;

            if content.contains("[workspace]") {
                let mut doc = content.parse::<toml_edit::DocumentMut>().map_err(|e| {
                    ServerError::Internal(format!("Failed to parse workspace Cargo.toml: {}", e))
                })?;

                for (old_package_path, new_package_path) in moves {
                    let old_rel_path = old_package_path.strip_prefix(path).map_err(|_| {
                        ServerError::Internal("Failed to calculate old relative path".to_string())
                    })?;
                    let new_rel_path = new_package_path.strip_prefix(path).map_err(|_| {
                        ServerError::Internal("Failed to calculate new relative path".to_string())
                    })?;

                    let old_path_str = old_rel_path.to_string_lossy().to_string();
                    let new_path_str = new_rel_path.to_string_lossy().to_string();

                    // Update members array
                    if let Some(members) = doc["workspace"]["members"].as_array_mut() {
                        let index_opt = members.iter().position(|m| m.as_str().map(|s| s.trim()) == Some(old_path_str.trim()));

                        debug!(
                            old_path_str = %old_path_str,
                            new_path_str = %new_path_str,
                            found_index = ?index_opt,
                            members_count = members.len(),
                            "Batch workspace member update"
                        );

                        if let Some(index) = index_opt {
                            members.replace(index, new_path_str.as_str());
                            debug!(index = index, "Replaced member in workspace array");
                        } else {
                            debug!(old_path_str = %old_path_str, "Member not found in workspace array");
                        }
                    }

                    // Update workspace dependencies (separate borrow)
                    let old_crate_name = old_package_path.file_name().and_then(|n| n.to_str()).map(|s| s.replace('_', "-")).unwrap_or_default();
                    let new_crate_name = new_package_path.file_name().and_then(|n| n.to_str()).map(|s| s.replace('_', "-")).unwrap_or_default();

                    if let Some(deps) = doc.get_mut("workspace").and_then(|w| w.get_mut("dependencies")).and_then(|d| d.as_table_mut()) {
                        if let Some(dep_value) = deps.remove(&old_crate_name) {
                            let mut updated_dep = dep_value;
                            if let Some(dep_table) = updated_dep.as_inline_table_mut() {
                                if let Some(path_entry) = dep_table.get_mut("path") {
                                    *path_entry = toml_edit::Value::from(new_path_str.as_str());
                                }
                            }
                            deps.insert(&new_crate_name, updated_dep);
                        }
                    }
                }

                let new_content = doc.to_string();
                planned_updates.push((workspace_toml_path.clone(), content.clone(), new_content));
                break;
            }
        }

        if path == project_root {
            break;
        }
        current_path = path.parent();
    }

    // Also plan dependent crate path updates (once per dependent Cargo.toml, aggregating all moves)
    debug!("Calling plan_dependent_crate_path_updates_for_batch");
    let mut dependent_updates = plan_dependent_crate_path_updates_for_batch(moves, project_root).await?;
    debug!(dependent_updates_count = dependent_updates.len(), "Received dependent crate updates");
    planned_updates.append(&mut dependent_updates);

    Ok(planned_updates)
}

/// Plan dependent crate path updates for batch operations
///
/// Scans all Cargo.toml files in the workspace and updates path dependencies
/// for ALL moved crates in a single pass. This prevents conflicting full-file
/// edits when multiple crates are renamed in batch.
async fn plan_dependent_crate_path_updates_for_batch(
    moves: &[(PathBuf, PathBuf)],
    project_root: &Path,
) -> ServerResult<Vec<(PathBuf, String, String)>> {
    use std::collections::HashMap;
    use tokio::fs;

    debug!(moves_count = moves.len(), project_root = ?project_root, "Entering plan_dependent_crate_path_updates_for_batch");

    let mut planned_updates = Vec::new();

    // Normalize project_root to absolute
    let abs_project_root = if project_root.is_absolute() {
        project_root.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| ServerError::Internal(format!("Failed to get current directory: {}", e)))?
            .join(project_root)
    };

    // Normalize all move paths to absolute upfront to avoid pathdiff::diff_paths returning None
    let normalized_moves: Vec<(PathBuf, PathBuf)> = moves
        .iter()
        .map(|(old_path, new_path)| {
            let abs_old = if old_path.is_absolute() {
                old_path.clone()
            } else {
                abs_project_root.join(old_path)
            };
            let abs_new = if new_path.is_absolute() {
                new_path.clone()
            } else {
                abs_project_root.join(new_path)
            };
            debug!(old_path = ?old_path, abs_old = ?abs_old, new_path = ?new_path, abs_new = ?abs_new, "Normalized move paths");
            (abs_old, abs_new)
        })
        .collect();

    // Build lookup map: old_crate_name → (new_crate_name, new_crate_absolute_path)
    let mut move_map: HashMap<String, (String, PathBuf)> = HashMap::new();
    for (old_path, new_path) in &normalized_moves {
        let old_name = old_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.replace('_', "-"))
            .unwrap_or_default();
        let new_name = new_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.replace('_', "-"))
            .unwrap_or_default();

        debug!(old_name = %old_name, new_name = %new_name, old_path = ?old_path, new_path = ?new_path, "Added to move_map");
        move_map.insert(old_name, (new_name, new_path.clone()));
    }

    // Walk workspace and find all Cargo.toml files
    let walker = ignore::WalkBuilder::new(&abs_project_root).hidden(false).build();

    for entry in walker.flatten() {
        let path = entry.path();
        if path.file_name() != Some(std::ffi::OsStr::new("Cargo.toml")) {
            continue;
        }

        // Skip the moved crates' own Cargo.toml files
        // Belt-and-suspenders: check both old (during planning) and new (if retried after partial move)
        let parent = match path.parent() {
            Some(p) => p,
            None => continue,
        };

        let is_moved_crate = normalized_moves.iter().any(|(old, new)| {
            parent == old.as_path() || parent == new.as_path()
        });
        if is_moved_crate {
            debug!(cargo_toml = ?path, "Skipping moved crate's own Cargo.toml");
            continue;
        }

        // Read Cargo.toml
        let content = match fs::read_to_string(path).await {
            Ok(c) => c,
            Err(e) => {
                debug!(cargo_toml = ?path, error = ?e, "Failed to read Cargo.toml");
                continue;
            }
        };

        // Check if this Cargo.toml depends on any of the moved crates
        let depends_on_moved = move_map.keys().any(|old_name| content.contains(old_name));
        if !depends_on_moved {
            continue;
        }

        debug!(cargo_toml = ?path, "Found Cargo.toml that depends on moved crates");

        // Parse and update all relevant dependencies
        let mut doc = match content.parse::<toml_edit::DocumentMut>() {
            Ok(d) => d,
            Err(e) => {
                debug!(cargo_toml = ?path, error = ?e, "Failed to parse Cargo.toml");
                continue;
            }
        };

        let mut updated = false;

        // Update dependencies in [dependencies], [dev-dependencies], [build-dependencies]
        for section_name in &["dependencies", "dev-dependencies", "build-dependencies"] {
            if let Some(deps_table) = doc.get_mut(*section_name).and_then(|s| s.as_table_mut()) {
                for (dep_name_key, dep_value) in deps_table.iter_mut() {
                    let dep_name = dep_name_key.get();
                    if let Some((_new_name, new_abs_path)) = move_map.get(dep_name) {
                        // Update path dependency
                        if let Some(dep_table) = dep_value.as_inline_table_mut() {
                            if let Some(old_path_value) = dep_table.get("path") {
                                // Calculate new relative path from this Cargo.toml to new location
                                // parent is absolute, new_abs_path is absolute, so diff_paths should succeed
                                if let Some(new_rel_path) = pathdiff::diff_paths(new_abs_path, parent) {
                                    let new_rel_path_str = new_rel_path.to_string_lossy().to_string();
                                    debug!(
                                        dep_name = %dep_name,
                                        old_path = %old_path_value,
                                        new_path = %new_rel_path_str,
                                        section = %section_name,
                                        "Updating inline table path dependency"
                                    );
                                    dep_table.insert("path", toml_edit::Value::from(new_rel_path_str));
                                    updated = true;
                                } else {
                                    debug!(
                                        dep_name = %dep_name,
                                        new_abs_path = ?new_abs_path,
                                        parent = ?parent,
                                        "pathdiff::diff_paths returned None for inline table"
                                    );
                                }
                            }
                        } else if let Some(dep_table) = dep_value.as_table_mut() {
                            if let Some(old_path_value) = dep_table.get("path") {
                                // Calculate new relative path
                                if let Some(new_rel_path) = pathdiff::diff_paths(new_abs_path, parent) {
                                    let new_rel_path_str = new_rel_path.to_string_lossy().to_string();
                                    debug!(
                                        dep_name = %dep_name,
                                        old_path = %old_path_value,
                                        new_path = %new_rel_path_str,
                                        section = %section_name,
                                        "Updating table path dependency"
                                    );
                                    dep_table.insert("path", toml_edit::Item::Value(toml_edit::Value::from(new_rel_path_str)));
                                    updated = true;
                                } else {
                                    debug!(
                                        dep_name = %dep_name,
                                        new_abs_path = ?new_abs_path,
                                        parent = ?parent,
                                        "pathdiff::diff_paths returned None for table"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        if updated {
            let new_content = doc.to_string();
            // Ensure path is absolute
            let abs_path = if path.is_absolute() {
                path.to_path_buf()
            } else {
                abs_project_root.join(path)
            };
            debug!(cargo_toml = ?abs_path, "Adding to planned_updates");
            planned_updates.push((abs_path, content, new_content));
        }
    }

    debug!(planned_updates_count = planned_updates.len(), "Exiting plan_dependent_crate_path_updates_for_batch");
    Ok(planned_updates)
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
mill-foundation = { path = "../mill-foundation", optional = true }
mill-config = { path = "../mill-config", optional = true }

[features]
default = ["runtime"]
runtime = ["mill-foundation", "mill-config"]
experimental = ["dep:mill-foundation"]
advanced = ["mill-foundation/serde"]
"#;

        std::fs::write(&cargo_toml, content).unwrap();

        let new_crate_path = temp_dir.path().join("../mill-foundation");
        let result = plan_single_cargo_toml_dependency_update(
            &cargo_toml,
            "mill-foundation",
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
