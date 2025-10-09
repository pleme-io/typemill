use crate::error::AstResult;
use crate::import_updater::{
    file_scanner,
    path_resolver::ImportPathResolver,
    reference_finder::{create_text_edits_from_references, find_inline_crate_references},
};
use cb_protocol::{EditPlan, EditPlanMetadata};
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
) -> AstResult<cb_protocol::EditPlan> {
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

    // Find files that import the renamed file
    let mut affected_files = resolver
        .find_affected_files(old_path, &project_files)
        .await?;

    debug!(
        affected_files_count = affected_files.len(),
        "Found affected files that import the renamed file"
    );

    // If scan_scope is provided, use enhanced scanning to find additional references
    if let Some(scope) = scan_scope {
        use std::collections::HashSet;

        // Get module name from old path for searching
        let module_name = old_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

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
                    // Find module references using the enhanced scanner
                    // We need to downcast to concrete plugin types to access find_module_references
                    use cb_lang_go::GoPlugin;
                    use cb_lang_rust::RustPlugin;
                    use cb_lang_typescript::TypeScriptPlugin;

                    let refs_opt =
                        if let Some(rust_plugin) = plugin.as_any().downcast_ref::<RustPlugin>() {
                            rust_plugin
                                .find_module_references(&content, module_name, scope)
                                .ok()
                        } else if let Some(ts_plugin) =
                            plugin.as_any().downcast_ref::<TypeScriptPlugin>()
                        {
                            Some(ts_plugin.find_module_references(&content, module_name, scope))
                        } else if let Some(go_plugin) = plugin.as_any().downcast_ref::<GoPlugin>() {
                            go_plugin
                                .find_module_references(&content, module_name, scope)
                                .ok()
                        } else {
                            None
                        };

                    if let Some(refs) = refs_opt {
                        if !refs.is_empty() {
                            debug!(
                                file = ?file_path,
                                references = refs.len(),
                                "Found module references via enhanced scanning"
                            );
                            all_affected.insert(file_path.clone());
                        }
                    }
                }
            }
        }

        affected_files = all_affected.into_iter().collect();
    }

    // Filter out files that shouldn't be updated based on the type of move
    // Two cases:
    // 1. Moving within same parent (e.g., renaming subdir): exclude all files in that parent
    // 2. Moving to different parent: exclude files inside the new destination
    if let (Some(old_parent), Some(new_parent)) = (old_path.parent(), new_path.parent()) {
        if old_parent == new_parent {
            // Case 1: Renaming within same parent directory
            // Files in the parent use relative imports and don't need updating
            affected_files.retain(|file| !file.starts_with(old_parent));
        } else {
            // Case 2: Moving to a different parent directory
            // Exclude files inside the moved directory (they use relative imports)
            affected_files.retain(|file| !file.starts_with(new_path));
        }
    } else {
        // Fallback: exclude files inside new_path if we can't determine parents
        affected_files.retain(|file| !file.starts_with(new_path));
    }

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

        // If scan_scope is provided, use find_module_references for precise edits
        if let Some(scope) = scan_scope {
            // Downcast to concrete plugin types to access find_module_references
            use cb_lang_go::GoPlugin;
            use cb_lang_rust::RustPlugin;
            use cb_lang_typescript::TypeScriptPlugin;

            let refs_opt = if let Some(rust_plugin) = plugin.as_any().downcast_ref::<RustPlugin>() {
                rust_plugin
                    .find_module_references(&content, old_module_name, scope)
                    .ok()
            } else if let Some(ts_plugin) = plugin.as_any().downcast_ref::<TypeScriptPlugin>() {
                Some(ts_plugin.find_module_references(&content, old_module_name, scope))
            } else if let Some(go_plugin) = plugin.as_any().downcast_ref::<GoPlugin>() {
                go_plugin
                    .find_module_references(&content, old_module_name, scope)
                    .ok()
            } else {
                None
            };

            if let Some(refs) = refs_opt {
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
        } else {
            // Fallback to the old rewrite logic
            // Downcast to concrete plugin types to access rewrite_imports_for_rename
            use cb_lang_go::GoPlugin;
            use cb_lang_rust::RustPlugin;
            use cb_lang_typescript::TypeScriptPlugin;

            let rewrite_result =
                if let Some(rust_plugin) = plugin.as_any().downcast_ref::<RustPlugin>() {
                    rust_plugin
                        .rewrite_imports_for_rename(
                            &content,
                            old_path,
                            new_path,
                            &file_path,
                            project_root,
                            rename_info,
                        )
                        .ok()
                } else if let Some(ts_plugin) = plugin.as_any().downcast_ref::<TypeScriptPlugin>() {
                    ts_plugin
                        .rewrite_imports_for_rename(
                            &content,
                            old_path,
                            new_path,
                            &file_path,
                            project_root,
                            rename_info,
                        )
                        .ok()
                } else if let Some(go_plugin) = plugin.as_any().downcast_ref::<GoPlugin>() {
                    go_plugin
                        .rewrite_imports_for_rename(
                            &content,
                            old_path,
                            new_path,
                            &file_path,
                            project_root,
                            rename_info,
                        )
                        .ok()
                } else {
                    None
                };

            match rewrite_result {
                Some((updated_content, count)) => {
                    if count > 0 && updated_content != content {
                        // Create a single TextEdit for the entire file content replacement
                        use cb_protocol::{EditLocation, EditType, TextEdit};
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
                                "Update imports in {} (legacy rewrite)",
                                file_path.display()
                            ),
                        });
                        edited_file_count += 1;
                        debug!(
                            file = ?file_path,
                            imports_updated = count,
                            "Created full-file TextEdit from legacy rewrite"
                        );
                    }
                }
                None => {
                    warn!(file = ?file_path, "Plugin does not support rewrite_imports_for_rename");
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
        },
    })
}