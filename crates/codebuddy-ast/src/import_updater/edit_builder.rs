use crate::error::AstResult;
use crate::import_updater::{
    file_scanner,
    path_resolver::ImportPathResolver,
    reference_finder::{create_text_edits_from_references, find_inline_crate_references},
};
use codebuddy_foundation::protocol::{EditPlan, EditPlanMetadata};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

pub(crate) async fn build_import_update_plan(
    old_path: &Path,
    new_path: &Path,
    project_root: &Path,
    plugins: &[std::sync::Arc<dyn cb_plugin_api::LanguagePlugin>],
    rename_info: Option<&serde_json::Value>,
    dry_run: bool,
    scan_scope: Option<cb_plugin_api::ScanScope>,
) -> AstResult<codebuddy_foundation::protocol::EditPlan> {
    let resolver = ImportPathResolver::new(project_root);

    debug!(
        plugins_count = plugins.len(),
        old_path = ?old_path,
        new_path = ?new_path,
        project_root = ?project_root,
        "Starting update_imports_for_rename"
    );

    // Find all files that the plugins handle
    let project_files = file_scanner::find_project_files(project_root, plugins).await?;

    debug!(
        project_files_count = project_files.len(),
        "Found project files"
    );

    // Check if this is a directory rename
    let is_directory_rename = old_path.is_dir();

    info!(
        old_path = ?old_path,
        is_dir = is_directory_rename,
        exists = old_path.exists(),
        "Checking if this is a directory rename"
    );

    // Find files that import the renamed file/directory
    let mut affected_files = if is_directory_rename {
        // For directory renames, find all files inside the directory
        // and then find all files that import ANY of those files
        let mut all_affected = std::collections::HashSet::new();

        info!(old_path = ?old_path, "Directory rename detected, scanning for files inside");

        // Find all files in the directory by filtering project_files
        let files_in_directory: Vec<&PathBuf> = project_files
            .iter()
            .filter(|f| f.starts_with(old_path) && f.is_file())
            .collect();

        debug!(
            files_in_directory_count = files_in_directory.len(),
            "Found files inside directory being renamed"
        );

        for file_in_dir in files_in_directory {
            // Find files that import this specific file
            let importers = resolver
                .find_affected_files(file_in_dir, &project_files, plugins)
                .await?;

            debug!(
                file_in_directory = ?file_in_dir,
                importers_count = importers.len(),
                "Found importers for file in renamed directory"
            );

            all_affected.extend(importers);
        }

        all_affected.into_iter().collect()
    } else {
        // For file renames, use the standard method
        resolver
            .find_affected_files(old_path, &project_files, plugins)
            .await?
    };

    debug!(
        affected_files_count = affected_files.len(),
        "Found affected files that import the renamed file/directory"
    );

    // If scan_scope is provided, use enhanced scanning to find additional references
    if let Some(scope) = scan_scope {
        use std::collections::HashSet;

        // Get module name from old path for searching
        // For Cargo packages, use the crate name from rename_info (with underscores)
        // instead of the directory name (which has hyphens)
        let module_name = if let Some(info) = rename_info {
            info.get("old_crate_name")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| old_path.file_stem().and_then(|s| s.to_str()).unwrap_or(""))
        } else {
            old_path.file_stem().and_then(|s| s.to_str()).unwrap_or("")
        };

        // Use HashSet to avoid duplicates
        let mut all_affected: HashSet<PathBuf> = affected_files.iter().cloned().collect();

        debug!(
            scan_scope = ?scope,
            module_name = %module_name,
            "Using enhanced scanning to find additional references"
        );

        // Scan all project files for module references
        for file_path in &project_files {
            // Find the appropriate plugin for this file
            let plugin = if let Some(ext) = file_path.extension() {
                let ext_str = ext.to_str().unwrap_or("");
                plugins.iter().find(|p| p.handles_extension(ext_str))
            } else {
                None
            };

            if let Some(plugin) = plugin {
                // Read file content
                if let Ok(content) = tokio::fs::read_to_string(file_path).await {
                    // Find module references using the capability trait
                    // This works for any language plugin that implements ModuleReferenceScanner
                    if let Some(scanner) = plugin.module_reference_scanner() {
                        if let Ok(refs) = scanner.scan_references(&content, module_name, scope) {
                            if !refs.is_empty() {
                                debug!(
                                    file = ?file_path,
                                    references = refs.len(),
                                    "Found module references via capability trait"
                                );
                                all_affected.insert(file_path.clone());
                            }
                        }
                    }
                }
            }
        }

        affected_files = all_affected.into_iter().collect();
    }

    // Filter out files that shouldn't be updated based on the type of move
    // This logic applies ONLY to directory renames, not file renames
    // For file renames, all affected files (including those in the same directory) need updates
    let is_directory = old_path.is_dir() || (!old_path.exists() && new_path.extension().is_none());

    if is_directory {
        // Only filter when renaming directories
        // We need to exclude files INSIDE the renamed directory (they use relative imports)
        // but keep files OUTSIDE that reference the directory
        affected_files.retain(|file| !file.starts_with(old_path));
    }
    // For file renames, do NOT filter affected_files - all importers need updates

    info!(
        dry_run = dry_run,
        affected_files = affected_files.len(),
        old_path = ?old_path,
        scan_scope = ?scan_scope,
        "Found files potentially affected by rename (excluding files inside moved directory)"
    );

    // Get module names for reference replacement
    let old_module_name = old_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let new_module_name = new_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

    // For Rust crate renames, also extract crate names for inline reference updates
    let (old_crate_name, new_crate_name) = if let Some(info) = rename_info {
        let old_crate = info
            .get("old_crate_name")
            .and_then(|v| v.as_str())
            .unwrap_or(old_module_name);
        let new_crate = info
            .get("new_crate_name")
            .and_then(|v| v.as_str())
            .unwrap_or(new_module_name);
        (old_crate.to_string(), new_crate.to_string())
    } else {
        (old_module_name.to_string(), new_module_name.to_string())
    };

    debug!(
        old_module = %old_module_name,
        new_module = %new_module_name,
        old_crate = %old_crate_name,
        new_crate = %new_crate_name,
        "Extracted rename information for import and inline reference updates"
    );

    let mut all_edits = Vec::new();
    let mut edited_file_count = 0;

    // Build TextEdits for each affected file
    for file_path in affected_files {
        // Find the appropriate plugin for this file
        let plugin = if let Some(ext) = file_path.extension() {
            let ext_str = ext.to_str().unwrap_or("");
            plugins.iter().find(|p| p.handles_extension(ext_str))
        } else {
            None
        };

        let plugin = match plugin {
            Some(p) => p,
            None => {
                debug!(file = ?file_path, "No adapter found for file extension");
                continue;
            }
        };

        // Read file content
        let content = match tokio::fs::read_to_string(&file_path).await {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, file = ?file_path, "Failed to read file");
                continue;
            }
        };

        // Special handling for different rename scenarios
        // PRIORITY 1: scan_scope (for Rust crate renames and TypeScript module renames)
        // PRIORITY 2: directory renames (for path-based imports)
        // PRIORITY 3: file renames (for path-based imports)
        if let Some(scope) = scan_scope {
            // Use find_module_references for precise edits (works for both file and directory renames)
            // Use capability trait for language-agnostic module reference scanning
            if let Some(scanner) = plugin.module_reference_scanner() {
                let refs = scanner
                    .scan_references(&content, old_module_name, scope)
                    .ok();

                if let Some(refs) = refs {
                    if !refs.is_empty() {
                        let edits = create_text_edits_from_references(
                            &refs,
                            &file_path,
                            old_module_name,
                            new_module_name,
                        );
                        debug!(
                            file = ?file_path,
                            edits = edits.len(),
                            "Created precise TextEdits from module references"
                        );
                        all_edits.extend(edits);
                        edited_file_count += 1;
                    }
                }
            }

            // ADDITIONAL SCAN: Find inline fully-qualified paths
            // This catches references like `old_crate::module::function()`
            // that are NOT in import statements
            if old_crate_name != new_crate_name {
                let inline_refs =
                    find_inline_crate_references(&content, &file_path, &old_crate_name);

                if !inline_refs.is_empty() {
                    debug!(
                        file = ?file_path,
                        inline_references = inline_refs.len(),
                        old_crate = %old_crate_name,
                        "Found inline fully-qualified path references"
                    );

                    // Create text edits for inline references
                    let inline_edits = create_text_edits_from_references(
                        &inline_refs,
                        &file_path,
                        &old_crate_name,
                        &new_crate_name,
                    );

                    if !inline_edits.is_empty() {
                        all_edits.extend(inline_edits);
                    }
                }
            }
        } else if is_directory_rename {
            // For directory renames without scan_scope, process path-based imports
            // This allows the plugin to match exact import paths like './core/api' → './legacy/api'

            // Get all files inside the renamed directory
            let files_in_directory: Vec<PathBuf> = project_files
                .iter()
                .filter(|f| f.starts_with(old_path) && f.is_file())
                .cloned()
                .collect();

            debug!(
                affected_file = ?file_path,
                files_in_directory_count = files_in_directory.len(),
                "Processing directory rename - will check each file inside for imports"
            );

            // For each file that was inside the renamed directory,
            // rewrite imports in the affected file
            // We need to accumulate changes across all files in the directory
            let mut current_content = content.clone();
            let mut total_changes = 0;

            for old_file_in_dir in &files_in_directory {
                // Calculate the file's new path
                let relative_path = old_file_in_dir
                    .strip_prefix(old_path)
                    .unwrap_or(old_file_in_dir);
                let new_file_path = new_path.join(relative_path);

                debug!(
                    old_file = ?old_file_in_dir,
                    new_file = ?new_file_path,
                    "Checking if affected file imports this renamed file"
                );

                // Call the plugin with INDIVIDUAL FILE paths, not directory paths
                // This way the plugin can match exact imports like './core/api'
                let rewrite_result = plugin.rewrite_file_references(
                    &current_content,
                    old_file_in_dir, // Actual file path: /workspace/src/core/api.ts
                    &new_file_path,  // Actual file path: /workspace/src/legacy/api.ts
                    &file_path,
                    project_root,
                    rename_info,
                );

                match rewrite_result {
                    Some((updated_content, count)) => {
                        if count > 0 && updated_content != current_content {
                            // Found imports to update! Accumulate the changes
                            total_changes += count;
                            current_content = updated_content;

                            debug!(
                                affected_file = ?file_path,
                                renamed_file = ?old_file_in_dir,
                                imports_updated = count,
                                "Updated imports for this file in the directory"
                            );
                        }
                    }
                    None => {
                        // Plugin doesn't support this operation, skip
                    }
                }
            }

            // If we accumulated any changes, create a TextEdit
            if total_changes > 0 && current_content != content {
                use codebuddy_foundation::protocol::{EditLocation, EditType, TextEdit};
                let line_count = current_content.lines().count();
                let last_line_len = current_content.lines().last().map(|l| l.len()).unwrap_or(0);

                all_edits.push(TextEdit {
                    file_path: Some(file_path.to_string_lossy().to_string()),
                    edit_type: EditType::UpdateImport,
                    location: EditLocation {
                        start_line: 0,
                        start_column: 0,
                        end_line: line_count.saturating_sub(1) as u32,
                        end_column: last_line_len as u32,
                    },
                    original_text: content,
                    new_text: current_content,
                    priority: 1,
                    description: format!(
                        "Update imports in {} for directory rename ({} files)",
                        file_path.display(),
                        files_in_directory.len()
                    ),
                });
                edited_file_count += 1;
                info!(
                    affected_file = ?file_path,
                    files_in_directory = files_in_directory.len(),
                    total_imports_updated = total_changes,
                    "Created TextEdit for directory rename import updates"
                );
            }
        } else {
            // For file renames without scan_scope, use the generic trait method
            let rewrite_result = plugin.rewrite_file_references(
                &content,
                old_path,
                new_path,
                &file_path,
                project_root,
                rename_info,
            );

            match rewrite_result {
                Some((updated_content, count)) => {
                    if count > 0 && updated_content != content {
                        // Create a single TextEdit for the entire file content replacement
                        use codebuddy_foundation::protocol::{EditLocation, EditType, TextEdit};
                        let line_count = content.lines().count();
                        let last_line_len = content.lines().last().map(|l| l.len()).unwrap_or(0);

                        all_edits.push(TextEdit {
                            file_path: Some(file_path.to_string_lossy().to_string()),
                            edit_type: EditType::UpdateImport,
                            location: EditLocation {
                                start_line: 0,
                                start_column: 0,
                                end_line: line_count.saturating_sub(1) as u32,
                                end_column: last_line_len as u32,
                            },
                            original_text: content.clone(),
                            new_text: updated_content,
                            priority: 1,
                            description: format!(
                                "Update imports in {} for file rename",
                                file_path.display()
                            ),
                        });
                        edited_file_count += 1;
                        debug!(
                            file = ?file_path,
                            imports_updated = count,
                            "Created TextEdit for file rename import updates"
                        );
                    }
                }
                None => {
                    warn!(file = ?file_path, "Plugin does not support rewrite_file_references");
                }
            }
        }
    }

    info!(
        files_affected = edited_file_count,
        edits_created = all_edits.len(),
        dry_run = dry_run,
        scan_scope = ?scan_scope,
        "Built EditPlan for import updates"
    );

    // Build and return the EditPlan
    Ok(EditPlan {
        source_file: old_path.to_string_lossy().to_string(),
        edits: all_edits,
        dependency_updates: Vec::new(),
        validations: Vec::new(),
        metadata: EditPlanMetadata {
            intent_name: "rename_file_or_directory".to_string(),
            intent_arguments: serde_json::json!({
                "old_path": old_path.to_string_lossy(),
                "new_path": new_path.to_string_lossy(),
                "scan_scope": scan_scope.map(|s| format!("{:?}", s)),
                "dry_run": dry_run,
            }),
            created_at: chrono::Utc::now(),
            complexity: if scan_scope.is_some() { 7 } else { 5 },
            impact_areas: vec!["imports".to_string(), "file_references".to_string()],
            consolidation: None,
        },
    })
}
