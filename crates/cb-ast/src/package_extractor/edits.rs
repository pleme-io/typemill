use crate::error::AstError;
use crate::package_extractor::ExtractModuleToPackageParams;
use cb_lang_rust::RustPlugin;
use cb_plugin_api::LanguagePlugin;
use cb_protocol::{EditLocation, EditType, TextEdit};
use std::path::{Path, PathBuf};
use tracing::debug;
use std::sync::Arc;

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
    let original_content =
        tokio::fs::read_to_string(original_file_path)
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
    rust_plugin: &RustPlugin,
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
                .join(rust_plugin.metadata().source_dir)
                .join(rust_plugin.metadata().entry_point)
        } else {
            let mut parent_path = source_path.join(rust_plugin.metadata().source_dir);
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
                if let Ok(updated_content) = rust_plugin
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
            }
        }
    }
}

pub(crate) async fn add_dependency_to_source_edit(
    edits: &mut Vec<TextEdit>,
    params: &ExtractModuleToPackageParams,
    source_path: &Path,
    rust_plugin: &RustPlugin,
) {
    let source_cargo_toml = source_path.join("Cargo.toml");
    if source_cargo_toml.exists() {
        if let Ok(cargo_content) = tokio::fs::read_to_string(&source_cargo_toml).await {
            if let Ok(updated_cargo) = rust_plugin
                .add_manifest_path_dependency(
                    &cargo_content,
                    &params.target_package_name,
                    &params.target_package_path,
                    source_path,
                )
                .await
            {
                if updated_cargo != cargo_content {
                    edits.push(TextEdit {
                        file_path: Some(source_cargo_toml.to_string_lossy().to_string()),
                        edit_type: EditType::Replace,
                        location: EditLocation {
                            start_line: 0,
                            start_column: 0,
                            end_line: cargo_content.lines().count() as u32,
                            end_column: 0,
                        },
                        original_text: cargo_content,
                        new_text: updated_cargo,
                        priority: 60,
                        description: format!(
                            "Add {} dependency to source Cargo.toml",
                            params.target_package_name
                        ),
                    });
                }
            }
        }
    }
}

pub(crate) async fn add_import_update_edits(
    edits: &mut Vec<TextEdit>,
    params: &ExtractModuleToPackageParams,
    source_path: &Path,
    rust_plugin: &RustPlugin,
    located_files: &[PathBuf],
) -> Result<(), AstError> {
    debug!("Starting use statement updates across workspace");

    let rust_files =
        rust_plugin
            .find_source_files(source_path)
            .await
            .map_err(|e| AstError::Analysis {
                message: format!("Failed to find source files: {}", e),
            })?;

    debug!(
        rust_files_count = rust_files.len(),
        "Found Rust files to scan for imports"
    );

    for file_path in rust_files {
        if located_files.iter().any(|f| f == &file_path) {
            continue;
        }

        if let Ok(_content) = tokio::fs::read_to_string(&file_path).await {
            let imports: Vec<cb_protocol::ImportInfo> = vec![];

            for import in imports {
                let module_path_normalized = params.module_path.replace('.', "::");
                let patterns_to_match = [
                    format!("crate::{}", module_path_normalized),
                    format!("self::{}", module_path_normalized),
                    module_path_normalized.clone(),
                ];

                let is_match = patterns_to_match
                    .iter()
                    .any(|pattern| import.module_path.starts_with(pattern));

                if is_match {
                    let old_use_statement = format!("use {};", import.module_path);
                    let new_use_statement =
                        rust_plugin.rewrite_import(&import.module_path, &params.target_package_name);

                    edits.push(TextEdit {
                        file_path: Some(file_path.to_string_lossy().to_string()),
                        edit_type: EditType::Replace,
                        location: EditLocation {
                            start_line: import.location.start_line,
                            start_column: import.location.start_column,
                            end_line: import.location.end_line,
                            end_column: import.location.end_column,
                        },
                        original_text: old_use_statement.clone(),
                        new_text: new_use_statement.clone(),
                        priority: 40,
                        description: format!(
                            "Update import to use new crate {}",
                            params.target_package_name
                        ),
                    });

                    debug!(
                        file = %file_path.display(),
                        old_import = %old_use_statement,
                        new_import = %new_use_statement,
                        "Created use statement update TextEdit"
                    );
                }
            }
        }
    }
    Ok(())
}