//! Planning logic for file and directory moves

use crate::services::reference_updater::{LspImportFinder, ReferenceUpdater};
use crate::services::reference_updater::helpers::create_import_update_edit;
use crate::services::reference_updater::LspImportFinder;
use mill_foundation::errors::MillError as ServerError;
use mill_foundation::protocol::EditPlan;
use mill_plugin_api::{PluginDiscovery, ScanScope};
use std::path::Path;
use tracing::{info, warn};

type ServerResult<T> = Result<T, ServerError>;

/// Plan a file move with import updates
pub async fn plan_file_move(
    old_abs: &Path,
    new_abs: &Path,
    reference_updater: &ReferenceUpdater,
    plugin_registry: &PluginDiscovery,
    scan_scope: Option<ScanScope>,
    rename_scope: Option<&mill_foundation::core::rename_scope::RenameScope>,
    lsp_finder: Option<&dyn LspImportFinder>,
) -> ServerResult<EditPlan> {
    info!(
        old_path = %old_abs.display(),
        new_path = %new_abs.display(),
        "Planning file move with import updates"
    );

    // Call reference updater to find all affected imports
    // dry_run = true means we're just planning, not executing
    let mut edit_plan = reference_updater
        .update_references(
            old_abs,
            new_abs,
            plugin_registry.all(),
            None, // No rename_info for simple file moves
            true, // dry_run = true
            scan_scope,
            rename_scope,
            lsp_finder,
        )
        .await?;

    // Fix: Remap edits targeting the old file path to the new file path
    // This ensures that updates to the moved file itself are applied to the NEW location,
    // preventing the resurrection of the old file during execution.
    for edit in &mut edit_plan.edits {
        if let Some(ref path) = edit.file_path {
            if Path::new(path) == old_abs {
                info!(
                    old_path = %path,
                    new_path = %new_abs.display(),
                    "Remapping edit from old file path to new file path"
                );
                edit.file_path = Some(new_abs.to_string_lossy().to_string());
            }
        }
    }

    // Log what we found
    info!(
        edits_count = edit_plan.edits.len(),
        old_path = %old_abs.display(),
        new_path = %new_abs.display(),
        "File move plan generated"
    );

    #[cfg(feature = "lang-svelte")]
    {
        append_svelte_import_edits(
            &mut edit_plan,
            old_abs,
            new_abs,
            reference_updater.project_root(),
        )
        .await?;
    }

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

#[cfg(feature = "lang-svelte")]
async fn append_svelte_import_edits(
    edit_plan: &mut EditPlan,
    old_abs: &Path,
    new_abs: &Path,
    project_root: &Path,
) -> ServerResult<()> {
    use ignore::WalkBuilder;
    use mill_lang_svelte::SveltePlugin;
    use mill_plugin_api::LanguagePlugin;

    let plugin = SveltePlugin::new();
    let mut files = Vec::new();

    let walker = WalkBuilder::new(project_root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("svelte") {
            files.push(path.to_path_buf());
        }
    }

    for file in files {
        let content = match tokio::fs::read_to_string(&file).await {
            Ok(c) => c,
            Err(_) => continue,
        };

        if let Some((updated, count)) = plugin.rewrite_file_references(
            &content,
            old_abs,
            new_abs,
            &file,
            project_root,
            None,
        ) {
            if count > 0 && updated != content {
                edit_plan.edits.push(create_import_update_edit(
                    &file,
                    content,
                    updated,
                    count,
                    "svelte import update",
                ));
            }
        }
    }

    Ok(())
}

/// Plan a directory move with import updates and workspace package support
///
/// This function is language-agnostic and uses the plugin system to handle
/// language-specific manifest updates (e.g., Cargo.toml for Rust, package.json for TypeScript).
pub async fn plan_directory_move(
    old_abs: &Path,
    new_abs: &Path,
    reference_updater: &ReferenceUpdater,
    plugin_registry: &PluginDiscovery,
    project_root: &Path,
    scan_scope: Option<ScanScope>,
    rename_scope: Option<&mill_foundation::core::rename_scope::RenameScope>,
    lsp_finder: Option<&dyn LspImportFinder>,
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
            lsp_finder,
        )
        .await?;

    // Fix: Remap edits targeting files inside the old directory to the new directory
    // This ensures that updates to moved files are applied to their NEW locations.
    for edit in &mut edit_plan.edits {
        if let Some(ref path_str) = edit.file_path {
            let path = Path::new(path_str);
            if path.starts_with(old_abs) {
                if let Ok(relative) = path.strip_prefix(old_abs) {
                    let new_file_path = new_abs.join(relative);
                    info!(
                        old_path = %path.display(),
                        new_path = %new_file_path.display(),
                        "Remapping edit from old directory to new directory"
                    );
                    edit.file_path = Some(new_file_path.to_string_lossy().to_string());
                }
            }
        }
    }

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

    match plan_documentation_and_config_edits(
        old_abs,
        new_abs,
        plugin_registry,
        project_root,
        rename_scope,
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
        is_package,
        "Directory move plan generated"
    );

    Ok(edit_plan)
}

/// Plan documentation and config file edits for a directory move
///
/// Scans for markdown files, config files (TOML, YAML), and code files (Rust)
/// that may reference the old path in string literals, and generates edits to update those references.
///
/// # Performance Optimizations
/// - **Batch API**: Uses `rewrite_file_references_batch` to process all renames in a single
///   plugin call per file, reducing O(N×M) to O(N) plugin calls.
/// - **Parallel IO**: Reads files in parallel using `JoinSet` for improved throughput.
/// - **Single walk**: Collects all target files in one directory traversal.
async fn plan_documentation_and_config_edits(
    old_path: &Path,
    new_path: &Path,
    plugin_registry: &PluginDiscovery,
    project_root: &Path,
    rename_scope: Option<&mill_foundation::core::rename_scope::RenameScope>,
) -> ServerResult<Vec<mill_foundation::protocol::TextEdit>> {
    use crate::services::reference_updater::create_path_reference_edit;
    use std::path::PathBuf;
    use std::sync::Arc;

    // Find all markdown, TOML, YAML files in the project
    // Note: .rs files are NOT included here - they're fully handled by reference_updater
    // which updates both imports AND qualified paths (e.g., crate_b::func()).
    // Including .rs here would create duplicate overlapping edits.
    let file_extensions: std::collections::HashSet<&str> =
        ["md", "markdown", "toml", "yaml", "yml"].into_iter().collect();

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

    // OPTIMIZATION: Build all renames upfront for batch processing
    // This includes: (1) the directory rename, (2) all files inside the moved directory
    // Using batch API reduces O(N×M) plugin calls to O(N) - one call per file
    let mut all_renames: Vec<(PathBuf, PathBuf)> = Vec::with_capacity(moved_files.len() + 1);
    all_renames.push((old_path.to_path_buf(), new_path.to_path_buf()));
    all_renames.extend(moved_files.iter().cloned());
    let all_renames = Arc::new(all_renames);

    // Convert rename_scope to JSON once for all files
    let rename_info = rename_scope.and_then(|s| serde_json::to_value(s).ok());
    let rename_info = Arc::new(rename_info);

    // Wrap moved_files in Arc for sharing across parallel tasks
    let moved_files = Arc::new(moved_files);

    // OPTIMIZATION: Walk the project ONCE and collect all files with target extensions
    // This replaces the previous O(5N) approach (walking 5 times for each extension)
    // with O(N) - a single walk that collects files grouped by extension
    let mut files_by_extension: std::collections::HashMap<String, Vec<PathBuf>> =
        std::collections::HashMap::new();

    let walker = ignore::WalkBuilder::new(project_root)
        .hidden(false)
        .git_ignore(true)
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if file_extensions.contains(ext) {
                    files_by_extension
                        .entry(ext.to_string())
                        .or_default()
                        .push(path.to_path_buf());
                }
            }
        }
    }

    info!(
        extensions_found = files_by_extension.len(),
        total_files = files_by_extension.values().map(|v| v.len()).sum::<usize>(),
        renames_count = all_renames.len(),
        "Collected files for doc/config update with batch renames"
    );

    // OPTIMIZATION: Read all files in parallel using JoinSet
    // This improves IO throughput significantly on SSDs
    let mut join_set = tokio::task::JoinSet::new();
    let project_root = project_root.to_path_buf();

    for (ext, files_to_scan) in &files_by_extension {
        if let Some(plugin) = plugin_registry.find_by_extension(ext) {
            info!(
                extension = ext,
                plugin_name = plugin.metadata().name,
                files_count = files_to_scan.len(),
                "Processing extension with batch API"
            );

            // Spawn parallel file reads
            for file_path in files_to_scan {
                let file_path = file_path.clone();
                let all_renames = Arc::clone(&all_renames);
                let rename_info = Arc::clone(&rename_info);
                let moved_files = Arc::clone(&moved_files);
                let project_root = project_root.clone();
                let ext = ext.clone();

                join_set.spawn(async move {
                    // Read file asynchronously
                    let content = match tokio::fs::read_to_string(&file_path).await {
                        Ok(c) => c,
                        Err(e) => {
                            warn!(
                                file = %file_path.display(),
                                error = %e,
                                "Failed to read file, skipping"
                            );
                            return None;
                        }
                    };

                    // Return file info for plugin processing
                    Some((file_path, content, all_renames, rename_info, moved_files, project_root, ext))
                });
            }
        } else {
            info!(extension = ext, "No plugin found for extension");
        }
    }

    // Collect file reads and process with plugins
    // Note: Plugin calls are synchronous and happen after parallel IO completes
    let mut file_infos = Vec::new();
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Some(info)) => file_infos.push(info),
            Ok(None) => {} // File read failed, already logged
            Err(e) => {
                warn!(error = %e, "Task failed during file read");
            }
        }
    }

    info!(
        files_read = file_infos.len(),
        "Parallel file reads completed, processing with plugins"
    );

    // Process files with plugins (using batch API)
    let mut edits = Vec::new();
    for (file_path, content, all_renames, rename_info, moved_files, project_root, ext) in file_infos {
        // Find the plugin again for this extension
        if let Some(plugin) = plugin_registry.find_by_extension(&ext) {
            // OPTIMIZATION: Use batch API - single call with all renames
            // This replaces the O(M) nested loop with O(1) plugin call
            if let Some((combined_content, total_changes)) = plugin.rewrite_file_references_batch(
                &content,
                &all_renames,
                &file_path,
                &project_root,
                rename_info.as_ref().as_ref(),
            ) {
                if total_changes > 0 && combined_content != content {
                    // CRITICAL: Check if this file will be moved as part of directory rename
                    // If so, use NEW path in edit (edit_plan.rs will map back to OLD path for snapshots)
                    let target_file_path = moved_files
                        .iter()
                        .find(|(old, _new)| old == &file_path)
                        .map(|(_old, new)| new.as_path())
                        .unwrap_or(&file_path);

                    if target_file_path != file_path {
                        info!(
                            old_path = %file_path.display(),
                            new_path = %target_file_path.display(),
                            "File will be moved - using NEW path in edit for correct snapshot lookup"
                        );
                    }

                    let edit = create_path_reference_edit(
                        target_file_path,
                        content.clone(),
                        combined_content,
                        total_changes,
                    );

                    info!(
                        file = %file_path.display(),
                        extension = ext,
                        changes = total_changes,
                        "Generated edit for file"
                    );

                    edits.push(edit);
                }
            }
        }
    }

    Ok(edits)
}
