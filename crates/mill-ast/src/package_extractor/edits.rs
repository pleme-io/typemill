use crate::error::AstError;
use crate::package_extractor::ExtractModuleToPackageParams;
use futures::stream::StreamExt;
use mill_foundation::protocol::{EditLocation, EditType, TextEdit};
use mill_plugin_api::{FileDiscovery, LanguagePlugin, StandardFileDiscovery};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::debug;

pub(crate) fn add_manifest_creation_edit(
    edits: &mut Vec<TextEdit>,
    params: &ExtractModuleToPackageParams,
    plugin: &Arc<dyn LanguagePlugin>,
    generated_manifest: &str,
) {
    let manifest_path = Path::new(&params.target_package_path)
        .join(plugin.metadata().manifest_filename)
        .to_string_lossy()
        .to_string();

    edits.push(TextEdit {
        file_path: Some(manifest_path.clone()),
        edit_type: EditType::Insert,
        location: EditLocation {
            start_line: 0,
            start_column: 0,
            end_line: 0,
            end_column: 0,
        },
        original_text: String::new(),
        new_text: generated_manifest.to_string(),
        priority: 100,
        description: "Create Cargo.toml for new crate".to_string(),
    });
}

pub(crate) async fn add_entrypoint_creation_edit(
    edits: &mut Vec<TextEdit>,
    params: &ExtractModuleToPackageParams,
    plugin: &Arc<dyn LanguagePlugin>,
    original_file_path: &Path,
) -> Result<String, AstError> {
    let original_content = tokio::fs::read_to_string(original_file_path)
        .await
        .map_err(|e| AstError::Analysis {
            message: format!(
                "Failed to read original module file {}: {}",
                original_file_path.display(),
                e
            ),
        })?;

    let new_entrypoint_path = Path::new(&params.target_package_path)
        .join(plugin.metadata().source_dir)
        .join(plugin.metadata().entry_point)
        .to_string_lossy()
        .to_string();

    edits.push(TextEdit {
        file_path: Some(new_entrypoint_path.clone()),
        edit_type: EditType::Insert,
        location: EditLocation {
            start_line: 0,
            start_column: 0,
            end_line: 0,
            end_column: 0,
        },
        original_text: String::new(),
        new_text: original_content.clone(),
        priority: 90,
        description: "Create entrypoint file for new crate".to_string(),
    });

    Ok(original_content)
}

pub(crate) fn add_delete_original_file_edit(
    edits: &mut Vec<TextEdit>,
    original_file_path: &Path,
    original_content: &str,
) {
    edits.push(TextEdit {
        file_path: Some(original_file_path.to_string_lossy().to_string()),
        edit_type: EditType::Delete,
        location: EditLocation {
            start_line: 0,
            start_column: 0,
            end_line: original_content.lines().count() as u32,
            end_column: 0,
        },
        original_text: original_content.to_string(),
        new_text: String::new(),
        priority: 80,
        description: "Delete original module file".to_string(),
    });
}

pub(crate) async fn add_remove_mod_declaration_edit(
    edits: &mut Vec<TextEdit>,
    params: &ExtractModuleToPackageParams,
    source_path: &Path,
    plugin: &dyn LanguagePlugin,
) {
    let module_segments: Vec<&str> = params
        .module_path
        .split([':', '.'])
        .filter(|s| !s.is_empty())
        .collect();

    if !module_segments.is_empty() {
        let final_module_name = module_segments[module_segments.len() - 1];

        let parent_file_path = if module_segments.len() == 1 {
            source_path
                .join(plugin.metadata().source_dir)
                .join(plugin.metadata().entry_point)
        } else {
            let mut parent_path = source_path.join(plugin.metadata().source_dir);
            for segment in &module_segments[..module_segments.len() - 1] {
                parent_path = parent_path.join(segment);
            }

            let mod_rs = parent_path.join("mod.rs");
            if mod_rs.exists() {
                mod_rs
            } else {
                parent_path.with_extension("rs")
            }
        };

        if parent_file_path.exists() {
            if let Ok(parent_content) = tokio::fs::read_to_string(&parent_file_path).await {
                // Use ModuleDeclarationSupport capability if available
                if let Some(mod_support) = plugin.module_declaration_support() {
                    if let Ok(updated_content) = mod_support
                        .remove_module_declaration(&parent_content, final_module_name)
                        .await
                    {
                        if updated_content != parent_content {
                            edits.push(TextEdit {
                                file_path: Some(parent_file_path.to_string_lossy().to_string()),
                                edit_type: EditType::Replace,
                                location: EditLocation {
                                    start_line: 0,
                                    start_column: 0,
                                    end_line: parent_content.lines().count() as u32,
                                    end_column: 0,
                                },
                                original_text: parent_content,
                                new_text: updated_content,
                                priority: 70,
                                description: format!(
                                    "Remove mod {} declaration from parent",
                                    final_module_name
                                ),
                            });
                        }
                    }
                } else {
                    debug!("Language plugin does not support module declaration removal");
                }
            }
        }
    }
}

pub(crate) async fn add_dependency_to_source_edit(
    edits: &mut Vec<TextEdit>,
    params: &ExtractModuleToPackageParams,
    source_path: &Path,
    plugin: &dyn LanguagePlugin,
) {
    let manifest_path = source_path.join(plugin.metadata().manifest_filename);
    if manifest_path.exists() {
        if let Ok(manifest_content) = tokio::fs::read_to_string(&manifest_path).await {
            // Use ManifestUpdater capability to add path dependency
            if let Some(manifest_updater) = plugin.manifest_updater() {
                if let Ok(updated_manifest) = manifest_updater
                    .add_path_dependency(
                        &manifest_content,
                        &params.target_package_name,
                        &params.target_package_path,
                        source_path,
                    )
                    .await
                {
                    if updated_manifest != manifest_content {
                        edits.push(TextEdit {
                            file_path: Some(manifest_path.to_string_lossy().to_string()),
                            edit_type: EditType::Replace,
                            location: EditLocation {
                                start_line: 0,
                                start_column: 0,
                                end_line: manifest_content.lines().count() as u32,
                                end_column: 0,
                            },
                            original_text: manifest_content,
                            new_text: updated_manifest,
                            priority: 60,
                            description: format!(
                                "Add {} dependency to source manifest",
                                params.target_package_name
                            ),
                        });
                    }
                }
            } else {
                debug!("Plugin does not support manifest updates, skipping dependency addition");
            }
        }
    }
}

pub(crate) async fn add_import_update_edits(
    edits: &mut Vec<TextEdit>,
    params: &ExtractModuleToPackageParams,
    source_path: &Path,
    plugin: &dyn LanguagePlugin,
    located_files: &[PathBuf],
) -> Result<(), AstError> {
    debug!("Starting use statement updates across workspace");

    // Use FileDiscovery capability or fallback to standard discovery
    let source_files = if let Some(discovery) = plugin.file_discovery() {
        discovery
            .find_source_files(source_path)
            .await
            .map_err(|e| AstError::Analysis {
                message: format!("Failed to find source files: {}", e),
            })?
    } else {
        let extensions = &plugin.metadata().extensions;
        let discovery = StandardFileDiscovery::new(extensions);
        discovery
            .find_source_files(source_path)
            .await
            .map_err(|e| AstError::Analysis {
                message: format!("Failed to find source files: {}", e),
            })?
    };

    debug!(
        source_files_count = source_files.len(),
        "Found source files to scan for imports"
    );

    let located_files_set: std::collections::HashSet<_> = located_files.iter().collect();
    let files_to_scan: Vec<_> = source_files
        .into_iter()
        .filter(|f| !located_files_set.contains(f))
        .collect();

    let module_path = params.module_path.clone();
    let target_package_name = params.target_package_name.clone();

    let new_edits = futures::stream::iter(files_to_scan)
        .map(|file_path| {
            let module_path = module_path.clone();
            let target_package_name = target_package_name.clone();

            async move {
                let mut local_edits = Vec::new();
                if let Ok(content) = tokio::fs::read_to_string(&file_path).await {
                    // Use ImportParser capability to detect imports
                    let import_paths = if let Some(import_parser) = plugin.import_parser() {
                        import_parser.parse_imports(&content)
                    } else {
                        // Fallback: no import parsing available for this language
                        debug!(
                            plugin_name = ?plugin.metadata().name,
                            "ImportParser not available for this language, skipping import updates"
                        );
                        return local_edits;
                    };

                    debug!(
                        file = %file_path.display(),
                        import_count = import_paths.len(),
                        "Parsed imports from file"
                    );

                    for import_path in import_paths {
                        let module_path_normalized = module_path.replace('.', "::");
                        let patterns_to_match = [
                            format!("crate::{}", module_path_normalized),
                            format!("self::{}", module_path_normalized),
                            module_path_normalized.clone(),
                        ];

                        let is_match = patterns_to_match
                            .iter()
                            .any(|pattern| import_path.starts_with(pattern));

                        if is_match {
                            // Use ImportAdvancedSupport if available, otherwise fall back to hardcoded logic
                            if let Some(advanced_support) = plugin.import_advanced_support() {
                                use mill_foundation::planning::DependencyUpdate;
                                use mill_foundation::planning::DependencyUpdateType;

                                let old_reference = import_path.clone();
                                let new_reference =
                                    format!("{}::{}", target_package_name, &import_path[6..]);

                                let update = DependencyUpdate {
                                    target_file: file_path.to_string_lossy().to_string(),
                                    update_type: DependencyUpdateType::ImportPath,
                                    old_reference: old_reference.clone(),
                                    new_reference: new_reference.clone(),
                                };

                                match advanced_support
                                    .update_import_reference(&file_path, &content, &update)
                                {
                                    Ok(updated_content) if updated_content != content => {
                                        // Find what changed to create accurate TextEdit
                                        local_edits.push(TextEdit {
                                            file_path: Some(
                                                file_path.to_string_lossy().to_string(),
                                            ),
                                            edit_type: EditType::Replace,
                                            location: EditLocation {
                                                start_line: 1,
                                                start_column: 0,
                                                end_line: content.lines().count() as u32,
                                                end_column: 0,
                                            },
                                            original_text: content.clone(),
                                            new_text: updated_content,
                                            priority: 40,
                                            description: format!(
                                                "Update import from {} to {}",
                                                old_reference, new_reference
                                            ),
                                        });

                                        debug!(
                                            file = %file_path.display(),
                                            old_ref = %old_reference,
                                            new_ref = %new_reference,
                                            "Used ImportAdvancedSupport to update import"
                                        );
                                    }
                                    Ok(_) => {
                                        debug!(
                                            file = %file_path.display(),
                                            "ImportAdvancedSupport returned no changes"
                                        );
                                    }
                                    Err(e) => {
                                        debug!(
                                            file = %file_path.display(),
                                            error = %e,
                                            "ImportAdvancedSupport failed, skipping"
                                        );
                                    }
                                }
                            } else {
                                // Fallback: Use basic line-based replacement
                                if let Some((line_num, line)) = content
                                    .lines()
                                    .enumerate()
                                    .find(|(_, line)| line.contains(&import_path))
                                {
                                    let old_use_statement = format!("use {};", import_path);
                                    let new_use_statement = format!(
                                        "use {}::{};",
                                        target_package_name,
                                        &import_path[6..]
                                    );

                                    let start_column = line.find("use").unwrap_or(0);
                                    let end_column =
                                        line.find(';').map(|pos| pos + 1).unwrap_or(line.len());

                                    local_edits.push(TextEdit {
                                        file_path: Some(file_path.to_string_lossy().to_string()),
                                        edit_type: EditType::Replace,
                                        location: EditLocation {
                                            start_line: (line_num + 1) as u32,
                                            start_column: start_column as u32,
                                            end_line: (line_num + 1) as u32,
                                            end_column: end_column as u32,
                                        },
                                        original_text: old_use_statement.clone(),
                                        new_text: new_use_statement.clone(),
                                        priority: 40,
                                        description: format!(
                                            "Update import to use new package {}",
                                            target_package_name
                                        ),
                                    });

                                    debug!(
                                        file = %file_path.display(),
                                        old_import = %old_use_statement,
                                        new_import = %new_use_statement,
                                        "Used fallback line-based replacement for import"
                                    );
                                }
                            }
                        }
                    }
                }
                local_edits
            }
        })
        .buffer_unordered(50)
        .fold(Vec::new(), |mut acc, mut file_edits| async move {
            acc.append(&mut file_edits);
            acc
        })
        .await;

    edits.extend(new_edits);
    Ok(())
}

