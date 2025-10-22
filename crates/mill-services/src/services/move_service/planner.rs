//! Planning logic for file and directory moves

use crate::services::reference_updater::ReferenceUpdater;
use cb_plugin_api::{PluginRegistry, ScanScope};
use codebuddy_foundation::protocol::{ApiResult as ServerResult, EditPlan};
use std::path::Path;
use tracing::{info, warn};

/// Plan a file move with import updates
pub async fn plan_file_move(
    old_abs: &Path,
    new_abs: &Path,
    reference_updater: &ReferenceUpdater,
    plugin_registry: &PluginRegistry,
    scan_scope: Option<ScanScope>,
    rename_scope: Option<&codebuddy_foundation::core::rename_scope::RenameScope>,
) -> ServerResult<EditPlan> {
    info!(
        old_path = %old_abs.display(),
        new_path = %new_abs.display(),
        "Planning file move with import updates"
    );

    // Call reference updater to find all affected imports
    // dry_run = true means we're just planning, not executing
    let edit_plan = reference_updater
        .update_references(
            old_abs,
            new_abs,
            plugin_registry.all(),
            None, // No rename_info for simple file moves
            true, // dry_run = true
            scan_scope,
            rename_scope,
        )
        .await?;

    // Log what we found
    info!(
        edits_count = edit_plan.edits.len(),
        old_path = %old_abs.display(),
        new_path = %new_abs.display(),
        "File move plan generated"
    );

    if !edit_plan.edits.is_empty() {
        info!(
            first_edit_file = ?edit_plan.edits.first().and_then(|e| e.file_path.as_ref()),
            first_edit_type = ?edit_plan.edits.first().map(|e| &e.edit_type),
            total_edits = edit_plan.edits.len(),
            "First edit in plan"
        );
    } else {
        warn!(
            old_path = %old_abs.display(),
            new_path = %new_abs.display(),
            "No edits returned from reference updater (may be expected if no imports)"
        );
    }

    Ok(edit_plan)
}

/// Plan a directory move with import updates and workspace package support
///
/// This function is language-agnostic and uses the plugin system to handle
/// language-specific manifest updates (e.g., Cargo.toml for Rust, package.json for TypeScript).
pub async fn plan_directory_move(
    old_abs: &Path,
    new_abs: &Path,
    reference_updater: &ReferenceUpdater,
    plugin_registry: &PluginRegistry,
    project_root: &Path,
    scan_scope: Option<ScanScope>,
    rename_scope: Option<&codebuddy_foundation::core::rename_scope::RenameScope>,
) -> ServerResult<EditPlan> {
    info!(
        old_path = %old_abs.display(),
        new_path = %new_abs.display(),
        "Planning directory move with import updates"
    );

    // Try to find a language plugin that can handle this directory's workspace operations
    // Check all plugins to see if any recognize this as a package
    let mut rename_info = None;
    let mut is_package = false;

    for plugin in plugin_registry.all() {
        // Check if plugin has workspace support
        if let Some(workspace_support) = plugin.workspace_support() {
            // Check if this is a package for this language
            if workspace_support.is_package(old_abs).await {
                is_package = true;
                info!(
                    language = %plugin.metadata().name,
                    "Detected package, planning manifest updates"
                );

                // Get manifest planning from the plugin
                if let Some(plan) = workspace_support
                    .plan_directory_move(old_abs, new_abs, project_root)
                    .await
                {
                    info!(
                        manifest_edits = plan.manifest_edits.len(),
                        is_consolidation = plan.is_consolidation,
                        "Language plugin provided manifest plan"
                    );

                    rename_info = plan.rename_info;

                    // We'll add manifest edits later, after reference updates
                    // Store the plan temporarily (we'll reconstruct it below)
                    break;
                }
            }
        }
    }

    // Pass through the scan scope - don't override it
    // The caller (via RenameScope) determines whether to scan for string literals
    let effective_scan_scope = scan_scope;

    // Call reference updater to find all affected imports
    let mut edit_plan = reference_updater
        .update_references(
            old_abs,
            new_abs,
            plugin_registry.all(),
            rename_info.as_ref(),
            true, // dry_run = true
            effective_scan_scope,
            rename_scope,
        )
        .await?;

    // Add language-specific manifest edits if this is a package
    if is_package {
        info!("Adding language-specific manifest edits to plan");

        let edits_before = edit_plan.edits.len();

        // Ask each plugin with workspace support to contribute manifest edits
        for plugin in plugin_registry.all() {
            if let Some(workspace_support) = plugin.workspace_support() {
                if let Some(plan) = workspace_support
                    .plan_directory_move(old_abs, new_abs, project_root)
                    .await
                {
                    info!(
                        language = %plugin.metadata().name,
                        manifest_edits = plan.manifest_edits.len(),
                        "Adding manifest edits from plugin"
                    );

                    edit_plan.edits.extend(plan.manifest_edits);
                    break; // Only use the first plugin that handles this package
                }
            }
        }

        let edits_after = edit_plan.edits.len();
        let manifest_edits_added = edits_after - edits_before;

        info!(
            manifest_edits_added,
            total_edits = edits_after,
            "Language-specific manifest edits added to plan"
        );
    }

    // Add documentation and config file edits (markdown, TOML, YAML)
    info!("Scanning for documentation and config file updates");
    let doc_config_edits_before = edit_plan.edits.len();

    match plan_documentation_and_config_edits(old_abs, new_abs, plugin_registry, project_root, rename_scope).await
    {
        Ok(edits) if !edits.is_empty() => {
            info!(
                doc_config_edits = edits.len(),
                "Adding documentation and config file edits to plan"
            );
            edit_plan.edits.extend(edits);
        }
        Ok(_) => {
            info!("No documentation or config file updates needed");
        }
        Err(e) => {
            warn!(
                error = %e,
                "Failed to plan documentation/config updates, continuing without them"
            );
        }
    }

    let doc_config_edits_added = edit_plan.edits.len() - doc_config_edits_before;
    if doc_config_edits_added > 0 {
        info!(
            doc_config_edits_added,
            "Documentation and config file edits added to plan"
        );
    }

    info!(
        edits_count = edit_plan.edits.len(),
        old_path = %old_abs.display(),
        new_path = %new_abs.display(),
        is_package,
        "Directory move plan generated"
    );

    Ok(edit_plan)
}

/// Plan documentation and config file edits for a directory move
///
/// Scans for markdown files, config files (TOML, YAML), and code files (Rust)
/// that may reference the old path in string literals, and generates edits to update those references.
async fn plan_documentation_and_config_edits(
    old_path: &Path,
    new_path: &Path,
    plugin_registry: &PluginRegistry,
    project_root: &Path,
    rename_scope: Option<&codebuddy_foundation::core::rename_scope::RenameScope>,
) -> ServerResult<Vec<codebuddy_foundation::protocol::TextEdit>> {
    use codebuddy_foundation::protocol::{EditLocation, EditType, TextEdit};
    use std::path::PathBuf;

    let mut edits = Vec::new();
    let mut files_to_scan: Vec<PathBuf> = Vec::new();

    // Find all markdown, TOML, YAML files in the project
    // Note: .rs files are NOT included here - they're fully handled by reference_updater
    // which updates both imports AND qualified paths (e.g., crate_b::func()).
    // Including .rs here would create duplicate overlapping edits.
    let file_extensions = ["md", "markdown", "toml", "yaml", "yml"];

    // Pre-compute the files inside the directory being moved so we can
    // update references to specific files (e.g., docs/guide.md)
    let mut moved_files: Vec<(PathBuf, PathBuf)> = Vec::new();
    let moved_walker = ignore::WalkBuilder::new(old_path)
        .hidden(false)
        .git_ignore(true)
        .build();

    for entry in moved_walker.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Ok(relative) = path.strip_prefix(old_path) {
                moved_files.push((path.to_path_buf(), new_path.join(relative)));
            }
        }
    }

    for ext in &file_extensions {
        info!(
            extension = ext,
            "Looking for plugin for extension"
        );

        if let Some(plugin) = plugin_registry.find_by_extension(ext) {
            info!(
                extension = ext,
                plugin_name = plugin.metadata().name,
                "Found plugin for extension"
            );

            // Walk the project to find files with this extension
            let walker = ignore::WalkBuilder::new(project_root)
                .hidden(false)
                .git_ignore(true)
                .build();

            for entry in walker.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(file_ext) = path.extension().and_then(|e| e.to_str()) {
                        if file_ext == *ext {
                            files_to_scan.push(path.to_path_buf());
                        }
                    }
                }
            }

            info!(
                extension = ext,
                files_found = files_to_scan.len(),
                "Found files for extension"
            );

            // Process each file with its plugin
            for file_path in &files_to_scan {

                match tokio::fs::read_to_string(file_path).await {
                    Ok(content) => {
                        let mut combined_content = content.clone();
                        let mut total_changes = 0usize;

                        // Call the plugin's rewrite_file_references to get updated content
                        // Returns Option<(String, usize)> where String is new content and usize is change count
                        // Pass rename_scope as JSON for plugins to customize behavior
                        let rename_info = rename_scope
                            .and_then(|s| serde_json::to_value(s).ok());

                        if let Some((updated_content, change_count)) = plugin
                            .rewrite_file_references(
                                &combined_content,
                                old_path,
                                new_path,
                                file_path,
                                project_root,
                                rename_info.as_ref(),
                            )
                        {
                            if change_count > 0 && updated_content != combined_content {
                                total_changes += change_count;
                                combined_content = updated_content;
                            }
                        }

                        // Also update references to specific files inside the moved directory
                        for (old_file, new_file) in &moved_files {
                            if let Some((updated_content, change_count)) = plugin
                                .rewrite_file_references(
                                    &combined_content,
                                    old_file,
                                    new_file,
                                    file_path,
                                    project_root,
                                    rename_info.as_ref(),  // Pass rename_info for consistent behavior
                                )
                            {
                                if change_count > 0 && updated_content != combined_content {
                                    total_changes += change_count;
                                    combined_content = updated_content;
                                }
                            }
                        }

                        if total_changes > 0 && combined_content != content {
                            // File needs updating - create a full-file replacement edit
                            let line_count = content.lines().count().max(1);
                            let last_line_len =
                                content.lines().last().map(|l| l.len()).unwrap_or(0);

                            // CRITICAL: Check if this file will be moved as part of directory rename
                            // If so, use NEW path in edit (edit_plan.rs will map back to OLD path for snapshots)
                            let target_file_path = moved_files
                                .iter()
                                .find(|(old, _new)| old == file_path)
                                .map(|(_old, new)| new)
                                .unwrap_or(file_path);

                            if target_file_path != file_path {
                                info!(
                                    old_path = %file_path.display(),
                                    new_path = %target_file_path.display(),
                                    "File will be moved - using NEW path in edit for correct snapshot lookup"
                                );
                            }

                            let edit = TextEdit {
                                file_path: Some(target_file_path.to_string_lossy().to_string()),
                                edit_type: EditType::Replace,
                                location: EditLocation {
                                    start_line: 0,
                                    start_column: 0,
                                    end_line: (line_count - 1) as u32,
                                    end_column: last_line_len as u32,
                                },
                                original_text: content.clone(),
                                new_text: combined_content,
                                priority: 0,
                                description: format!(
                                    "Update {} path references in {}",
                                    total_changes,
                                    file_path.display()
                                ),
                            };

                            info!(
                                file = %file_path.display(),
                                extension = ext,
                                changes = total_changes,
                                "Generated edit for file"
                            );

                            edits.push(edit);
                        }
                    }
                    Err(e) => {
                        warn!(
                            file = %file_path.display(),
                            error = %e,
                            "Failed to read file, skipping"
                        );
                    }
                }
            }

            files_to_scan.clear(); // Clear for next extension
        } else {
            info!(
                extension = ext,
                "No plugin found for extension"
            );
        }
    }

    Ok(edits)
}
