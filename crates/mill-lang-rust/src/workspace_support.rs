//! Workspace support implementation for Rust language plugin
//!
//! This module implements the `WorkspaceSupport` trait for Rust, providing
//! synchronous methods for manipulating Cargo.toml workspace manifests.

use async_trait::async_trait;
use mill_plugin_api::workspace_support::WorkspaceSupport;
use std::path::Path;
use toml_edit::DocumentMut;
use tracing::debug;

/// Rust workspace support implementation
#[derive(Default)]
pub struct RustWorkspaceSupport;

#[async_trait]
impl WorkspaceSupport for RustWorkspaceSupport {
    fn add_workspace_member(&self, content: &str, member: &str) -> String {
        debug!(member = %member, "Adding workspace member");

        match add_workspace_member_impl(content, member) {
            Ok(result) => result,
            Err(e) => {
                debug!(error = %e, "Failed to add workspace member, returning original content");
                content.to_string()
            }
        }
    }

    fn remove_workspace_member(&self, content: &str, member: &str) -> String {
        debug!(member = %member, "Removing workspace member");

        match remove_workspace_member_impl(content, member) {
            Ok(result) => result,
            Err(e) => {
                debug!(error = %e, "Failed to remove workspace member, returning original content");
                content.to_string()
            }
        }
    }

    fn is_workspace_manifest(&self, content: &str) -> bool {
        // Check for the presence of [workspace] section
        let is_workspace = content.contains("[workspace]");
        debug!(is_workspace = %is_workspace, "Checked if manifest is workspace");
        is_workspace
    }

    fn list_workspace_members(&self, content: &str) -> Vec<String> {
        debug!("Listing workspace members");

        match list_workspace_members_impl(content) {
            Ok(members) => {
                debug!(members_count = members.len(), "Found workspace members");
                members
            }
            Err(e) => {
                debug!(error = %e, "Failed to list workspace members");
                Vec::new()
            }
        }
    }

    fn update_package_name(&self, content: &str, new_name: &str) -> String {
        debug!(new_name = %new_name, "Updating package name");

        match update_package_name_impl(content, new_name) {
            Ok(result) => result,
            Err(e) => {
                debug!(error = %e, "Failed to update package name, returning original content");
                content.to_string()
            }
        }
    }

    async fn is_package(&self, dir_path: &std::path::Path) -> bool {
        // Check if Cargo.toml exists
        dir_path.join("Cargo.toml").exists()
    }

    async fn plan_directory_move(
        &self,
        old_path: &std::path::Path,
        new_path: &std::path::Path,
        project_root: &std::path::Path,
    ) -> Option<mill_plugin_api::MoveManifestPlan> {
        use mill_plugin_api::MoveManifestPlan;
        use tracing::{info, warn};

        // Check if this is a Cargo package
        if !self.is_package(old_path).await {
            return None;
        }

        info!(
            old_path = %old_path.display(),
            new_path = %new_path.display(),
            "Planning Cargo manifest updates for directory move"
        );

        // Use the existing cargo_util functions to plan manifest updates
        use crate::workspace::cargo_util;

        // Detect if this is a consolidation move
        let is_consolidation = new_path
            .components()
            .collect::<Vec<_>>()
            .windows(2)
            .any(|w| w[0].as_os_str() == "src");

        // Extract rename info
        let rename_info = if is_consolidation {
            info!("Detected consolidation move, extracting consolidation rename info");
            cargo_util::extract_consolidation_rename_info(old_path, new_path)
                .await
                .ok()
        } else {
            info!("Detected regular package rename, extracting rename info");
            cargo_util::extract_cargo_rename_info(old_path, new_path)
                .await
                .ok()
        };

        // For consolidation moves, don't plan manifest edits (handled during execution)
        if is_consolidation {
            info!("Consolidation move - skipping manifest planning (handled during execution)");
            return Some(MoveManifestPlan {
                manifest_edits: Vec::new(),
                rename_info,
                is_consolidation: true,
            });
        }

        // Plan workspace manifest updates (workspace members + package name)
        let mut all_edits = Vec::new();

        match cargo_util::plan_workspace_manifest_updates(old_path, new_path, project_root).await {
            Ok(updates) if !updates.is_empty() => {
                info!(
                    workspace_manifests = updates.len(),
                    "Planning workspace Cargo.toml updates"
                );

                // Convert manifest updates to TextEdits
                let manifest_edits =
                    cargo_util::convert_manifest_updates_to_edits(updates, old_path, new_path);

                all_edits.extend(manifest_edits);
            }
            Ok(_) => {
                info!("No workspace manifest updates needed");
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "Failed to plan workspace manifest updates, continuing without them"
                );
            }
        }

        // Plan dependent crate path updates
        if let Some(ref info) = rename_info {
            if let (Some(old_name), Some(new_name)) = (
                info.get("old_package_name").and_then(|v| v.as_str()),
                info.get("new_package_name").and_then(|v| v.as_str()),
            ) {
                match cargo_util::plan_dependent_crate_path_updates(
                    old_name,
                    new_name,
                    new_path,
                    project_root,
                )
                .await
                {
                    Ok(updates) if !updates.is_empty() => {
                        info!(
                            dependent_manifests = updates.len(),
                            "Planning dependent crate path updates"
                        );

                        // Convert to TextEdits
                        let dep_edits = cargo_util::convert_manifest_updates_to_edits(
                            updates, old_path, new_path,
                        );

                        all_edits.extend(dep_edits);
                    }
                    Ok(_) => {
                        info!("No dependent crate path updates needed");
                    }
                    Err(e) => {
                        warn!(
                            error = %e,
                            "Failed to plan dependent crate updates, continuing without them"
                        );
                    }
                }
            }
        }

        // Plan updates for the moved crate's own path dependencies
        match cargo_util::plan_moved_crate_own_path_dependencies(old_path, new_path).await {
            Ok(Some(update)) => {
                info!("Planning moved crate's own path dependency updates");

                // Convert to TextEdits
                let own_deps_edits = cargo_util::convert_manifest_updates_to_edits(
                    vec![update],
                    old_path,
                    new_path,
                );

                all_edits.extend(own_deps_edits);
            }
            Ok(None) => {
                info!("No path dependency updates needed for moved crate");
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "Failed to plan moved crate's path dependencies, continuing without them"
                );
            }
        }

        info!(
            manifest_edits = all_edits.len(),
            "Cargo manifest planning complete"
        );

        Some(MoveManifestPlan {
            manifest_edits: all_edits,
            rename_info,
            is_consolidation: false,
        })
    }

    async fn generate_workspace_manifest(
        &self,
        member_paths: &[&str],
        workspace_root: &Path,
    ) -> Result<String, String> {
        // Delegate to workspace module implementation
        crate::workspace::generate_workspace_manifest(member_paths, workspace_root)
            .map_err(|e| e.to_string())
    }

    async fn execute_consolidation_post_processing(
        &self,
        source_crate_name: &str,
        target_crate_name: &str,
        target_module_name: &str,
        source_crate_path: &Path,
        target_crate_path: &Path,
        target_module_path: &Path,
        project_root: &Path,
    ) -> Result<(), String> {
        use crate::consolidation::execute_consolidation_post_processing;
        use mill_foundation::protocol::ConsolidationMetadata;

        // Build metadata from parameters
        let metadata = ConsolidationMetadata {
            is_consolidation: true,
            source_crate_name: source_crate_name.to_string(),
            source_crate_path: source_crate_path.display().to_string(),
            target_crate_name: target_crate_name.to_string(),
            target_crate_path: target_crate_path.display().to_string(),
            target_module_name: target_module_name.to_string(),
            target_module_path: target_module_path.display().to_string(),
        };

        // Call the standalone function from consolidation module
        execute_consolidation_post_processing(&metadata, project_root)
            .await
            .map_err(|e| e.to_string())
    }
}

// Implementation functions that return Results for error handling

fn add_workspace_member_impl(content: &str, member: &str) -> Result<String, String> {
    let mut doc = content
        .parse::<DocumentMut>()
        .map_err(|e| format!("Failed to parse Cargo.toml: {}", e))?;

    // Ensure [workspace.members] exists
    if !doc.contains_key("workspace") {
        doc["workspace"] = toml_edit::table();
    }

    let workspace = doc["workspace"]
        .as_table_mut()
        .ok_or_else(|| "[workspace] is not a table".to_string())?;

    if !workspace.contains_key("members") {
        workspace["members"] = toml_edit::value(toml_edit::Array::new());
    }

    let members = workspace["members"]
        .as_array_mut()
        .ok_or_else(|| "[workspace.members] is not an array".to_string())?;

    // Add new member if not already present
    let member_exists = members.iter().any(|v| v.as_str() == Some(member));

    if !member_exists {
        members.push(member);
        debug!(member = %member, "Added new member to workspace");
    } else {
        debug!(member = %member, "Member already exists in workspace");
    }

    Ok(doc.to_string())
}

fn remove_workspace_member_impl(content: &str, member: &str) -> Result<String, String> {
    let mut doc = content
        .parse::<DocumentMut>()
        .map_err(|e| format!("Failed to parse Cargo.toml: {}", e))?;

    // Get workspace members array
    if let Some(workspace) = doc.get_mut("workspace").and_then(|w| w.as_table_mut()) {
        if let Some(members) = workspace.get_mut("members").and_then(|m| m.as_array_mut()) {
            // Find and remove the member
            let original_len = members.len();
            members.retain(|v| v.as_str() != Some(member));

            if members.len() < original_len {
                debug!(member = %member, "Removed member from workspace");
            } else {
                debug!(member = %member, "Member not found in workspace");
            }
        }
    }

    Ok(doc.to_string())
}

fn list_workspace_members_impl(content: &str) -> Result<Vec<String>, String> {
    let doc = content
        .parse::<DocumentMut>()
        .map_err(|e| format!("Failed to parse Cargo.toml: {}", e))?;

    // Get workspace members array
    if let Some(workspace) = doc.get("workspace").and_then(|w| w.as_table()) {
        if let Some(members) = workspace.get("members").and_then(|m| m.as_array()) {
            let member_list: Vec<String> = members
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            return Ok(member_list);
        }
    }

    Ok(Vec::new())
}

fn update_package_name_impl(content: &str, new_name: &str) -> Result<String, String> {
    let mut doc = content
        .parse::<DocumentMut>()
        .map_err(|e| format!("Failed to parse Cargo.toml: {}", e))?;

    // Update package name in [package] section
    if let Some(package) = doc.get_mut("package").and_then(|p| p.as_table_mut()) {
        package["name"] = toml_edit::value(new_name);
        debug!(new_name = %new_name, "Updated package name");
    } else {
        return Err("No [package] section found in manifest".to_string());
    }

    Ok(doc.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_workspace_member() {
        let support = RustWorkspaceSupport;
        let content = r#"
[workspace]
members = ["crate1"]
"#;

        let result = support.add_workspace_member(content, "crate2");
        assert!(result.contains("[workspace]"));
        assert!(result.contains("crate1"));
        assert!(result.contains("crate2"));
    }

    #[test]
    fn test_add_workspace_member_duplicate() {
        let support = RustWorkspaceSupport;
        let content = r#"
[workspace]
members = ["crate1"]
"#;

        let result = support.add_workspace_member(content, "crate1");
        // Should not duplicate
        assert_eq!(result.matches("crate1").count(), 1);
    }

    #[test]
    fn test_remove_workspace_member() {
        let support = RustWorkspaceSupport;
        let content = r#"
[workspace]
members = ["crate1", "crate2", "crate3"]
"#;

        let result = support.remove_workspace_member(content, "crate2");
        assert!(result.contains("crate1"));
        assert!(!result.contains("crate2"));
        assert!(result.contains("crate3"));
    }

    #[test]
    fn test_is_workspace_manifest() {
        let support = RustWorkspaceSupport;

        let workspace_content = r#"
[workspace]
members = []
"#;
        assert!(support.is_workspace_manifest(workspace_content));

        let package_content = r#"
[package]
name = "my-crate"
"#;
        assert!(!support.is_workspace_manifest(package_content));
    }

    #[test]
    fn test_list_workspace_members() {
        let support = RustWorkspaceSupport;
        let content = r#"
[workspace]
members = ["crate1", "crate2", "crate3"]
"#;

        let members = support.list_workspace_members(content);
        assert_eq!(members.len(), 3);
        assert!(members.contains(&"crate1".to_string()));
        assert!(members.contains(&"crate2".to_string()));
        assert!(members.contains(&"crate3".to_string()));
    }

    #[test]
    fn test_list_workspace_members_empty() {
        let support = RustWorkspaceSupport;
        let content = r#"
[package]
name = "my-crate"
"#;

        let members = support.list_workspace_members(content);
        assert_eq!(members.len(), 0);
    }

    #[test]
    fn test_update_package_name() {
        let support = RustWorkspaceSupport;
        let content = r#"
[package]
name = "old-name"
version = "0.1.0"
"#;

        let result = support.update_package_name(content, "new-name");
        assert!(result.contains("new-name"));
        assert!(!result.contains("old-name"));
        assert!(result.contains("version"));
    }
}
