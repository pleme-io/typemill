//! Planning logic for file and directory moves

use super::cargo;
use crate::services::reference_updater::ReferenceUpdater;
use cb_plugin_api::{PluginRegistry, ScanScope};
use cb_protocol::{ApiResult as ServerResult, EditPlan};
use std::path::Path;
use tracing::{info, warn};

/// Plan a file move with import updates
pub async fn plan_file_move(
    old_abs: &Path,
    new_abs: &Path,
    reference_updater: &ReferenceUpdater,
    plugin_registry: &PluginRegistry,
    scan_scope: Option<ScanScope>,
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

/// Plan a directory move with import updates and Cargo package support
pub async fn plan_directory_move(
    old_abs: &Path,
    new_abs: &Path,
    reference_updater: &ReferenceUpdater,
    plugin_registry: &PluginRegistry,
    project_root: &Path,
    scan_scope: Option<ScanScope>,
) -> ServerResult<EditPlan> {
    info!(
        old_path = %old_abs.display(),
        new_path = %new_abs.display(),
        "Planning directory move with import updates"
    );

    // Check if this is a Cargo package
    let is_cargo_pkg = cargo::is_cargo_package(old_abs).await?;

    // Extract rename info if this is a Cargo package
    let rename_info = if is_cargo_pkg {
        info!("Detected Cargo package, extracting rename info");
        cargo::extract_cargo_rename_info(old_abs, new_abs)
            .await
            .ok()
    } else {
        None
    };

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
        )
        .await?;

    // If this is a Cargo package, add manifest edits
    if is_cargo_pkg {
        info!("Adding Cargo.toml manifest edits to plan");

        let edits_before = edit_plan.edits.len();

        // 1. Plan workspace manifest updates (workspace members + package name)
        let workspace_updates = cargo::plan_workspace_manifest_updates(
            old_abs,
            new_abs,
            project_root,
        )
        .await;

        match workspace_updates {
            Ok(updates) if !updates.is_empty() => {
                info!(
                    workspace_manifests = updates.len(),
                    "Planning workspace Cargo.toml updates"
                );

                // Convert manifest updates to TextEdits
                let manifest_edits = cargo::convert_manifest_updates_to_edits(
                    updates,
                    old_abs,
                    new_abs,
                );

                edit_plan.edits.extend(manifest_edits);
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

        // 2. Plan dependent crate path updates
        if let Some(ref info) = rename_info {
            if let (Some(old_name), Some(new_name)) = (
                info.get("old_package_name").and_then(|v| v.as_str()),
                info.get("new_package_name").and_then(|v| v.as_str()),
            ) {
                let dep_updates = cargo::plan_dependent_crate_path_updates(
                    old_name,
                    new_name,
                    new_abs,
                    project_root,
                )
                .await;

                match dep_updates {
                    Ok(updates) if !updates.is_empty() => {
                        info!(
                            dependent_manifests = updates.len(),
                            "Planning dependent crate path updates"
                        );

                        // Convert to TextEdits
                        let dep_edits = cargo::convert_manifest_updates_to_edits(
                            updates,
                            old_abs,
                            new_abs,
                        );

                        edit_plan.edits.extend(dep_edits);
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

        let edits_after = edit_plan.edits.len();
        let manifest_edits_added = edits_after - edits_before;

        info!(
            manifest_edits_added,
            total_edits = edits_after,
            "Cargo manifest edits added to plan"
        );
    }

    // Add documentation and config file edits (markdown, TOML, YAML)
    info!("Scanning for documentation and config file updates");
    let doc_config_edits_before = edit_plan.edits.len();

    match plan_documentation_and_config_edits(
        old_abs,
        new_abs,
        plugin_registry,
        project_root,
    )
    .await
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
        is_cargo_package = is_cargo_pkg,
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
) -> ServerResult<Vec<cb_protocol::TextEdit>> {
    use cb_protocol::{EditLocation, EditType, TextEdit};
    use std::path::PathBuf;

    let mut edits = Vec::new();
    let mut files_to_scan: Vec<PathBuf> = Vec::new();

    // Find all markdown, TOML, YAML, and code files in the project
    // Note: Rust files (.rs) ARE included here for string literal updates.
    // The reference_updater handles imports, but string literals are handled here.
    let file_extensions = ["md", "markdown", "toml", "yaml", "yml", "rs"];

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
        if let Some(plugin) = plugin_registry.find_by_extension(ext) {
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
                        if let Some((updated_content, change_count)) = plugin.rewrite_file_references(
                            &combined_content,
                            old_path,
                            new_path,
                            file_path,
                            project_root,
                            None, // No rename_info for simple moves
                        ) {
                            if change_count > 0 && updated_content != combined_content {
                                total_changes += change_count;
                                combined_content = updated_content;
                            }
                        }

                        // Also update references to specific files inside the moved directory
                        for (old_file, new_file) in &moved_files {
                            if let Some((updated_content, change_count)) = plugin.rewrite_file_references(
                                &combined_content,
                                old_file,
                                new_file,
                                file_path,
                                project_root,
                                None,
                            ) {
                                if change_count > 0 && updated_content != combined_content {
                                    total_changes += change_count;
                                    combined_content = updated_content;
                                }
                            }
                        }

                        if total_changes > 0 && combined_content != content {
                            // File needs updating - create a full-file replacement edit
                            let line_count = content.lines().count().max(1);
                            let last_line_len = content
                                .lines()
                                .last()
                                .map(|l| l.len())
                                .unwrap_or(0);

                            let edit = TextEdit {
                                file_path: Some(file_path.to_string_lossy().to_string()),
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
        }
    }

    Ok(edits)
}
