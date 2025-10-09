use crate::package_extractor::ExtractModuleToPackageParams;
use cb_lang_rust::RustPlugin;
use cb_plugin_api::LanguagePlugin;
use cb_protocol::{EditLocation, EditType, TextEdit};
use std::path::Path;
use std::sync::Arc;
use tracing::debug;

pub(crate) async fn update_workspace(
    edits: &mut Vec<TextEdit>,
    params: &ExtractModuleToPackageParams,
    source_path: &Path,
    plugin: &Arc<dyn LanguagePlugin>,
    rust_plugin: &RustPlugin,
) {
    debug!("is_workspace_member=true: searching for workspace root");

    // Find workspace root by looking for Cargo.toml with [workspace]
    let mut workspace_root = source_path.to_path_buf();
    let mut found_workspace = false;

    while let Some(parent) = workspace_root.parent() {
        let potential_workspace = parent.join("Cargo.toml");
        if potential_workspace.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&potential_workspace).await {
                // Use workspace capability to check if this is a workspace manifest
                if let Some(workspace_support) = plugin.workspace_support() {
                    if workspace_support.is_workspace_manifest(&content) {
                        workspace_root = parent.to_path_buf();
                        found_workspace = true;
                        debug!(
                            workspace_root = %workspace_root.display(),
                            "Found workspace root"
                        );
                        break;
                    }
                }
            }
        }
        workspace_root = parent.to_path_buf();
        if workspace_root.parent().is_none() {
            break;
        }
    }

    if !found_workspace {
        debug!("No workspace root found, creating workspace at source crate parent");
        // If no existing workspace found, create one at the parent of source_path
        if let Some(parent) = source_path.parent() {
            workspace_root = parent.to_path_buf();
            let workspace_cargo_toml = workspace_root.join("Cargo.toml");

            // Create a new workspace Cargo.toml if it doesn't exist
            if !workspace_cargo_toml.exists() {
                let source_crate_str = source_path.to_string_lossy().to_string();
                let member_paths = vec![source_crate_str.as_str(), &params.target_package_path];

                if let Ok(workspace_content) = rust_plugin
                    .generate_workspace_manifest(&member_paths, &workspace_root)
                    .await
                {
                    edits.push(TextEdit {
                        file_path: Some(workspace_cargo_toml.to_string_lossy().to_string()),
                        edit_type: EditType::Insert,
                        location: EditLocation {
                            start_line: 0,
                            start_column: 0,
                            end_line: 0,
                            end_column: 0,
                        },
                        original_text: String::new(),
                        new_text: workspace_content,
                        priority: 50,
                        description: "Create workspace Cargo.toml with members".to_string(),
                    });
                    debug!("Created workspace Cargo.toml creation TextEdit");
                    found_workspace = true;
                } else {
                    debug!("Failed to generate workspace manifest");
                }
            }
        }
    }

    if found_workspace {
        let workspace_cargo_toml = workspace_root.join("Cargo.toml");
        let source_cargo_toml = source_path.join("Cargo.toml");
        if workspace_cargo_toml.exists() && workspace_cargo_toml != source_cargo_toml {
            if let Ok(workspace_content) = tokio::fs::read_to_string(&workspace_cargo_toml).await {
                // Use workspace capability to check if this is a workspace manifest
                if let Some(workspace_support) = plugin.workspace_support() {
                    if workspace_support.is_workspace_manifest(&workspace_content) {
                        // Use workspace capability to add member (sync call, no .await)
                        let updated_workspace = workspace_support
                            .add_workspace_member(&workspace_content, &params.target_package_path);

                        if updated_workspace != workspace_content {
                            edits.push(TextEdit {
                                file_path: Some(
                                    workspace_cargo_toml.to_string_lossy().to_string(),
                                ),
                                edit_type: EditType::Replace,
                                location: EditLocation {
                                    start_line: 0,
                                    start_column: 0,
                                    end_line: workspace_content.lines().count() as u32,
                                    end_column: 0,
                                },
                                original_text: workspace_content,
                                new_text: updated_workspace,
                                priority: 50,
                                description: "Add new crate to workspace members".to_string(),
                            });
                            debug!("Created workspace Cargo.toml update TextEdit");
                        }
                    }
                }
            }
        }
    }
}