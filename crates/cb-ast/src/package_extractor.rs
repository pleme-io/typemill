//! Logic for the extract_module_to_package refactoring tool.
//!
//! This module provides language-agnostic package extraction capabilities
//! for extracting modules into separate packages.

use crate::error::AstResult;
use cb_core::language::ProjectLanguage;
use cb_protocol::EditPlan;
use serde::Deserialize;
use std::path::Path;
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct ExtractModuleToPackageParams {
    pub source_package: String,
    pub module_path: String,
    pub target_package_path: String,
    pub target_package_name: String,
    pub update_imports: Option<bool>,
    pub create_manifest: Option<bool>,
    pub dry_run: Option<bool>,
    pub is_workspace_member: Option<bool>,
}

/// Recursively find all .rs files in a directory
fn find_rust_files_in_dir(dir: &Path) -> AstResult<Vec<std::path::PathBuf>> {
    let mut rust_files = Vec::new();

    if !dir.exists() || !dir.is_dir() {
        return Ok(rust_files);
    }

    let entries = std::fs::read_dir(dir).map_err(|e| crate::error::AstError::Analysis {
        message: format!("Failed to read directory {}: {}", dir.display(), e),
    })?;

    for entry_result in entries {
        let entry = entry_result.map_err(|e| crate::error::AstError::Analysis {
            message: format!("Failed to read directory entry: {}", e),
        })?;

        let path = entry.path();

        if path.is_dir() {
            // Skip target and hidden directories
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                if dir_name == "target" || dir_name.starts_with('.') {
                    continue;
                }
            }

            // Recursively search subdirectories
            let mut sub_files = find_rust_files_in_dir(&path)?;
            rust_files.append(&mut sub_files);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            rust_files.push(path);
        }
    }

    Ok(rust_files)
}

/// Update a Cargo.toml file to add a new path dependency
fn update_cargo_toml_dependency(
    cargo_content: &str,
    dep_name: &str,
    dep_path: &str,
    source_path: &Path,
) -> AstResult<String> {
    use toml_edit::DocumentMut;

    let mut doc =
        cargo_content
            .parse::<DocumentMut>()
            .map_err(|e| crate::error::AstError::Analysis {
                message: format!("Failed to parse Cargo.toml: {}", e),
            })?;

    // Calculate relative path from source to target
    let source_cargo_dir = source_path;
    let target_path = Path::new(dep_path);
    let relative_path = pathdiff::diff_paths(target_path, source_cargo_dir).ok_or_else(|| {
        crate::error::AstError::Analysis {
            message: "Failed to calculate relative path".to_string(),
        }
    })?;

    // Add dependency to [dependencies] section
    if !doc.contains_key("dependencies") {
        doc["dependencies"] = toml_edit::table();
    }

    let deps =
        doc["dependencies"]
            .as_table_mut()
            .ok_or_else(|| crate::error::AstError::Analysis {
                message: "[dependencies] is not a table".to_string(),
            })?;

    // Create inline table for path dependency
    let mut dep_table = toml_edit::InlineTable::new();
    dep_table.insert(
        "path",
        toml_edit::Value::from(relative_path.to_string_lossy().as_ref()),
    );

    deps[dep_name] = toml_edit::value(toml_edit::Value::InlineTable(dep_table));

    Ok(doc.to_string())
}

/// Update a workspace Cargo.toml to add a new member
fn update_workspace_members(
    workspace_content: &str,
    new_member_path: &str,
    workspace_root: &Path,
) -> AstResult<String> {
    use toml_edit::DocumentMut;

    let mut doc =
        workspace_content
            .parse::<DocumentMut>()
            .map_err(|e| crate::error::AstError::Analysis {
                message: format!("Failed to parse workspace Cargo.toml: {}", e),
            })?;

    // Calculate relative path from workspace root to new member
    let target_path = Path::new(new_member_path);
    let relative_path = pathdiff::diff_paths(target_path, workspace_root).ok_or_else(|| {
        crate::error::AstError::Analysis {
            message: "Failed to calculate relative path for workspace member".to_string(),
        }
    })?;

    // Ensure [workspace.members] exists
    if !doc.contains_key("workspace") {
        doc["workspace"] = toml_edit::table();
    }

    let workspace =
        doc["workspace"]
            .as_table_mut()
            .ok_or_else(|| crate::error::AstError::Analysis {
                message: "[workspace] is not a table".to_string(),
            })?;

    if !workspace.contains_key("members") {
        workspace["members"] = toml_edit::value(toml_edit::Array::new());
    }

    let members =
        workspace["members"]
            .as_array_mut()
            .ok_or_else(|| crate::error::AstError::Analysis {
                message: "[workspace.members] is not an array".to_string(),
            })?;

    // Add new member if not already present
    let member_str = relative_path.to_string_lossy();
    let member_exists = members
        .iter()
        .any(|v| v.as_str() == Some(member_str.as_ref()));

    if !member_exists {
        members.push(member_str.as_ref());
    }

    Ok(doc.to_string())
}

/// Remove a module declaration from Rust source code
///
/// Parses the source with syn and removes the `mod module_name;` or `pub mod module_name;` declaration.
fn remove_module_declaration(source: &str, module_name: &str) -> AstResult<String> {
    use syn::{File, Item};

    // Parse the Rust source
    let mut syntax_tree: File =
        syn::parse_str(source).map_err(|e| crate::error::AstError::Analysis {
            message: format!("Failed to parse Rust source for mod removal: {}", e),
        })?;

    // Remove the module declaration
    syntax_tree.items.retain(|item| {
        if let Item::Mod(item_mod) = item {
            // Check if this is the module we want to remove
            item_mod.ident != module_name
        } else {
            true // Keep all other items
        }
    });

    // Convert back to string
    let updated_source = quote::quote!(#syntax_tree).to_string();

    // Format with rustfmt-style spacing (basic)
    // Note: This is a simple approximation, real rustfmt would be better
    Ok(updated_source)
}

/// Main entry point for extracting a module to a package
///
/// This function orchestrates the extraction process by:
/// 1. Detecting the source package language
/// 2. Selecting the appropriate plugin from the registry
/// 3. Generating an EditPlan for the refactoring
///
/// # Arguments
///
/// * `params` - Extraction parameters
/// * `plugin_registry` - Registry of language plugins
pub async fn plan_extract_module_to_package_with_registry(
    params: ExtractModuleToPackageParams,
    plugin_registry: &cb_plugin_api::PluginRegistry,
) -> AstResult<EditPlan> {
    use cb_core::language::detect_project_language;
    use cb_protocol::{EditPlanMetadata, ValidationRule, ValidationType};
    use serde_json::json;
    use std::collections::HashMap;
    use tracing::{debug, info};

    info!(
        source_package = %params.source_package,
        module_path = %params.module_path,
        target_package = %params.target_package_path,
        "Planning extract_module_to_package operation"
    );

    // Step 1: Detect language from source package
    let source_path = Path::new(&params.source_package);
    let detected_language = detect_project_language(source_path);

    debug!(
        language = %detected_language.as_str(),
        "Detected project language"
    );

    // Step 2: Look up appropriate language plugin from registry
    let _manifest_ext = match detected_language {
        ProjectLanguage::Rust => "toml",
        ProjectLanguage::TypeScript => "json",
        ProjectLanguage::Python => "txt",
        ProjectLanguage::Go => "mod",
        ProjectLanguage::Java => "xml",
        ProjectLanguage::Unknown => {
            return Err(crate::error::AstError::Analysis {
                message: "Could not detect project language - no manifest files found".to_string(),
            });
        }
    };

    // Find plugin by checking all plugins for one that handles this language
    let plugin = plugin_registry
        .all()
        .iter()
        .find(|p| p.language() == detected_language)
        .ok_or_else(|| crate::error::AstError::Analysis {
            message: format!(
                "No plugin registered for language: {}",
                detected_language.as_str()
            ),
        })?;

    info!(
        language = %detected_language.as_str(),
        "Selected plugin for extraction"
    );

    // Step 3: Locate module files using the plugin
    let located_files = plugin
        .locate_module_files(source_path, &params.module_path)
        .await?;

    debug!(files_count = located_files.len(), "Located module files");

    // Step 4: Parse imports from all located files and aggregate dependencies
    let mut all_dependencies = std::collections::HashSet::new();

    for file_path in &located_files {
        debug!(
            file_path = %file_path.display(),
            "Parsing dependencies from file"
        );

        match plugin.parse_imports(file_path).await {
            Ok(deps) => {
                for dep in deps {
                    all_dependencies.insert(dep);
                }
            }
            Err(e) => {
                // Log error but continue with other files
                debug!(
                    error = %e,
                    file_path = %file_path.display(),
                    "Failed to parse imports from file"
                );
            }
        }
    }

    // Convert to sorted vector for consistent output
    let mut dependencies: Vec<String> = all_dependencies.into_iter().collect();
    dependencies.sort();

    debug!(
        dependencies_count = dependencies.len(),
        "Aggregated dependencies from all module files"
    );

    // Step 5: Generate new crate manifest
    let generated_manifest = plugin.generate_manifest(&params.target_package_name, &dependencies);

    debug!(
        manifest_lines = generated_manifest.lines().count(),
        "Generated Cargo.toml manifest"
    );

    // Step 6: Construct file modification plan
    use cb_protocol::{EditLocation, EditType, TextEdit};
    let mut edits = Vec::new();

    // Edit 1: Create new Cargo.toml
    let manifest_path = Path::new(&params.target_package_path)
        .join(plugin.manifest_filename())
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
        new_text: generated_manifest.clone(),
        priority: 100,
        description: "Create Cargo.toml for new crate".to_string(),
    });

    debug!(edit_count = 1, "Created manifest TextEdit");

    // Edit 2: Create new src/lib.rs with module content
    if let Some(original_file_path) = located_files.first() {
        let original_content = tokio::fs::read_to_string(original_file_path)
            .await
            .map_err(|e| crate::error::AstError::Analysis {
                message: format!(
                    "Failed to read original module file {}: {}",
                    original_file_path.display(),
                    e
                ),
            })?;

        let new_entrypoint_path = Path::new(&params.target_package_path)
            .join(plugin.source_dir())
            .join(plugin.entry_point())
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

        debug!(edit_count = 2, "Created entrypoint TextEdit");

        // Edit 3: Delete old module file
        edits.push(TextEdit {
            file_path: Some(original_file_path.to_string_lossy().to_string()),
            edit_type: EditType::Delete,
            location: EditLocation {
                start_line: 0,
                start_column: 0,
                end_line: original_content.lines().count() as u32,
                end_column: 0,
            },
            original_text: original_content.clone(),
            new_text: String::new(),
            priority: 80,
            description: "Delete original module file".to_string(),
        });

        debug!(edit_count = 3, "Created delete TextEdit");

        // Edit 4: Remove mod declaration from parent module
        // Determine the parent module file path
        let module_segments: Vec<&str> = params
            .module_path
            .split([':', '.'])
            .filter(|s| !s.is_empty())
            .collect();

        if !module_segments.is_empty() {
            let final_module_name = module_segments[module_segments.len() - 1];

            // Determine parent file path
            let parent_file_path = if module_segments.len() == 1 {
                // Top-level module, parent is lib.rs
                source_path
                    .join(plugin.source_dir())
                    .join(plugin.entry_point())
            } else {
                // Nested module, parent is the containing module's mod.rs or .rs file
                let mut parent_path = source_path.join(plugin.source_dir());
                for segment in &module_segments[..module_segments.len() - 1] {
                    parent_path = parent_path.join(segment);
                }

                // Try mod.rs first, then .rs
                let mod_rs = parent_path.join("mod.rs");
                if mod_rs.exists() {
                    mod_rs
                } else {
                    parent_path.with_extension("rs")
                }
            };

            if parent_file_path.exists() {
                match tokio::fs::read_to_string(&parent_file_path).await {
                    Ok(parent_content) => {
                        // Parse and remove the module declaration
                        match remove_module_declaration(&parent_content, final_module_name) {
                            Ok(updated_content) => {
                                if updated_content != parent_content {
                                    edits.push(TextEdit {
                                        file_path: Some(
                                            parent_file_path.to_string_lossy().to_string(),
                                        ),
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

                                    debug!(edit_count = 4, "Created parent mod removal TextEdit");
                                } else {
                                    debug!("No mod declaration found in parent file");
                                }
                            }
                            Err(e) => {
                                debug!(
                                    error = %e,
                                    "Failed to parse parent module file for declaration removal"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        debug!(
                            error = %e,
                            file_path = %parent_file_path.display(),
                            "Failed to read parent module file"
                        );
                    }
                }
            }
        }
    }

    // Step 7: Update source crate's Cargo.toml to add new dependency
    let source_cargo_toml = source_path.join("Cargo.toml");
    if source_cargo_toml.exists() {
        match tokio::fs::read_to_string(&source_cargo_toml).await {
            Ok(cargo_content) => {
                match update_cargo_toml_dependency(
                    &cargo_content,
                    &params.target_package_name,
                    &params.target_package_path,
                    source_path,
                ) {
                    Ok(updated_cargo) => {
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
                            debug!("Created source Cargo.toml update TextEdit");
                        }
                    }
                    Err(e) => {
                        debug!(error = %e, "Failed to update source Cargo.toml");
                    }
                }
            }
            Err(e) => {
                debug!(error = %e, "Failed to read source Cargo.toml");
            }
        }
    }

    // Step 8: Update workspace Cargo.toml to add new member (if is_workspace_member is true)
    if params.is_workspace_member.unwrap_or(false) {
        debug!("is_workspace_member=true: searching for workspace root");

        // Find workspace root by looking for Cargo.toml with [workspace]
        let mut workspace_root = source_path.to_path_buf();
        let mut found_workspace = false;

        while let Some(parent) = workspace_root.parent() {
            let potential_workspace = parent.join("Cargo.toml");
            if potential_workspace.exists() {
                if let Ok(content) = tokio::fs::read_to_string(&potential_workspace).await {
                    if content.contains("[workspace]") {
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
                    let source_crate_rel = pathdiff::diff_paths(source_path, &workspace_root)
                        .unwrap_or_else(|| source_path.to_path_buf());
                    let target_crate_rel =
                        pathdiff::diff_paths(&params.target_package_path, &workspace_root)
                            .unwrap_or_else(|| {
                                Path::new(&params.target_package_path).to_path_buf()
                            });

                    let workspace_content = format!(
                        r#"[workspace]
members = [
    "{}",
    "{}",
]
resolver = "2"
"#,
                        source_crate_rel.to_string_lossy(),
                        target_crate_rel.to_string_lossy()
                    );

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
                }
            }
        }

        if found_workspace {
            let workspace_cargo_toml = workspace_root.join("Cargo.toml");
            if workspace_cargo_toml.exists() && workspace_cargo_toml != source_cargo_toml {
                match tokio::fs::read_to_string(&workspace_cargo_toml).await {
                    Ok(workspace_content) => {
                        if workspace_content.contains("[workspace]") {
                            match update_workspace_members(
                                &workspace_content,
                                &params.target_package_path,
                                &workspace_root,
                            ) {
                                Ok(updated_workspace) => {
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
                                            description: "Add new crate to workspace members"
                                                .to_string(),
                                        });
                                        debug!("Created workspace Cargo.toml update TextEdit");
                                    }
                                }
                                Err(e) => {
                                    debug!(error = %e, "Failed to update workspace Cargo.toml");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        debug!(error = %e, "Failed to read workspace Cargo.toml");
                    }
                }
            }
        }
    } else {
        debug!("is_workspace_member=false: skipping workspace configuration");
    }

    // Step 9: Find and update all use statements in the workspace
    if params.update_imports.unwrap_or(true) {
        debug!("Starting use statement updates across workspace");

        // Find all Rust files in the source crate
        let rust_files = find_rust_files_in_dir(source_path)?;

        debug!(
            rust_files_count = rust_files.len(),
            "Found Rust files to scan for imports"
        );

        for file_path in rust_files {
            // Skip the files we're already modifying
            let skip_file = located_files.iter().any(|f| f == &file_path);
            if skip_file {
                continue;
            }

            match tokio::fs::read_to_string(&file_path).await {
                Ok(_content) => {
                    // DEPRECATED: Rust parsing moved to cb-lang-rust plugin
                    // This entire extract_module_to_package functionality should be refactored
                    // to use language plugins instead of hardcoded plugins
                    //
                    // For now, we'll return empty imports to make compilation succeed
                    // match crate::rust_parser::parse_rust_imports_ast(&content) {
                    let imports: Vec<cb_protocol::ImportInfo> = vec![]; // Deprecated - always empty

                    for import in imports {
                        // Check if this import references the extracted module
                        // The module path should start with "crate::" followed by our module path
                        let module_path_normalized = params.module_path.replace('.', "::");
                        let patterns_to_match = [
                            format!("crate::{}", module_path_normalized),
                            format!("self::{}", module_path_normalized),
                            module_path_normalized.clone(),
                        ];

                        let is_match = patterns_to_match.iter().any(|pattern| {
                            import.module_path.starts_with(pattern)
                                || import.module_path == *pattern
                        });

                        if is_match {
                            // Found an import that needs to be rewritten
                            let old_use_statement = format!("use {};", import.module_path);
                            let new_use_statement = plugin
                                .rewrite_import(&import.module_path, &params.target_package_name);

                            // Create a TextEdit to replace this import
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
                    // End of deprecated rust_parser usage
                }
                Err(e) => {
                    debug!(
                        error = %e,
                        file_path = %file_path.display(),
                        "Failed to read file for import scanning"
                    );
                }
            }
        }
    }

    // Convert PathBuf to strings for JSON serialization
    let located_files_strings: Vec<String> = located_files
        .iter()
        .map(|p| p.display().to_string())
        .collect();

    let edit_plan = EditPlan {
        source_file: params.source_package.clone(),
        edits,
        dependency_updates: vec![],
        validations: vec![ValidationRule {
            rule_type: ValidationType::SyntaxCheck,
            description: "Verify syntax is valid after extraction".to_string(),
            parameters: HashMap::new(),
        }],
        metadata: EditPlanMetadata {
            intent_name: "extract_module_to_package".to_string(),
            intent_arguments: json!({
                "source_package": params.source_package,
                "module_path": params.module_path,
                "target_package_path": params.target_package_path,
                "target_package_name": params.target_package_name,
                "plugin_selected": plugin.language().as_str(),
                "located_files": located_files_strings,
                "dependencies": dependencies,
                "generated_manifest": generated_manifest,
            }),
            created_at: chrono::Utc::now(),
            complexity: 1,
            impact_areas: vec!["package_extraction".to_string()],
        },
    };

    info!(
        plugin = %plugin.language().as_str(),
        files_count = located_files.len(),
        dependencies_count = dependencies.len(),
        edits_count = edit_plan.edits.len(),
        "Successfully created EditPlan with file modification operations"
    );

    Ok(edit_plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cb_lang_rust::RustPlugin;
use cb_plugin_api::LanguageIntelligencePlugin;
    use cb_protocol::EditType;
    use std::fs;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_locate_module_files_single_file() {
        // Create a temporary Rust project structure
        let temp_dir = tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        // Create lib.rs
        fs::write(src_dir.join("lib.rs"), "// lib.rs").unwrap();

        // Create a module as a single file: src/my_module.rs
        fs::write(src_dir.join("my_module.rs"), "// my_module.rs").unwrap();

        let plugin = &RustPlugin::new();
        let result = plugin
            .locate_module_files(temp_dir.path(), "my_module")
            .await;

        assert!(result.is_ok());
        let files = result.unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("my_module.rs"));
    }

    #[tokio::test]
    async fn test_locate_module_files_mod_rs() {
        // Create a temporary Rust project structure
        let temp_dir = tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        // Create lib.rs
        fs::write(src_dir.join("lib.rs"), "// lib.rs").unwrap();

        // Create a module as a directory with mod.rs: src/my_module/mod.rs
        let module_dir = src_dir.join("my_module");
        fs::create_dir(&module_dir).unwrap();
        fs::write(module_dir.join("mod.rs"), "// mod.rs").unwrap();

        let plugin = &RustPlugin::new();
        let result = plugin
            .locate_module_files(temp_dir.path(), "my_module")
            .await;

        assert!(result.is_ok());
        let files = result.unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("my_module/mod.rs") || files[0].ends_with("my_module\\mod.rs"));
    }

    #[tokio::test]
    async fn test_locate_module_files_nested_module() {
        // Create a temporary Rust project structure
        let temp_dir = tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        // Create lib.rs
        fs::write(src_dir.join("lib.rs"), "// lib.rs").unwrap();

        // Create nested module structure: src/services/planner.rs
        let services_dir = src_dir.join("services");
        fs::create_dir(&services_dir).unwrap();
        fs::write(services_dir.join("planner.rs"), "// planner.rs").unwrap();

        let plugin = &RustPlugin::new();
        let result = plugin
            .locate_module_files(temp_dir.path(), "services::planner")
            .await;

        assert!(result.is_ok());
        let files = result.unwrap();
        assert_eq!(files.len(), 1);
        assert!(
            files[0].ends_with("services/planner.rs") || files[0].ends_with("services\\planner.rs")
        );
    }

    #[tokio::test]
    async fn test_locate_module_files_dot_separator() {
        // Test that the function accepts both :: and . as separators
        let temp_dir = tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        // Create lib.rs
        fs::write(src_dir.join("lib.rs"), "// lib.rs").unwrap();

        // Create nested module structure: src/services/planner.rs
        let services_dir = src_dir.join("services");
        fs::create_dir(&services_dir).unwrap();
        fs::write(services_dir.join("planner.rs"), "// planner.rs").unwrap();

        let plugin = &RustPlugin::new();
        let result = plugin
            .locate_module_files(temp_dir.path(), "services.planner")
            .await;

        assert!(result.is_ok());
        let files = result.unwrap();
        assert_eq!(files.len(), 1);
        assert!(
            files[0].ends_with("services/planner.rs") || files[0].ends_with("services\\planner.rs")
        );
    }

    #[tokio::test]
    async fn test_locate_module_files_not_found() {
        // Create a temporary Rust project structure
        let temp_dir = tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        // Create lib.rs but no module files
        fs::write(src_dir.join("lib.rs"), "// lib.rs").unwrap();

        let plugin = &RustPlugin::new();
        let result = plugin
            .locate_module_files(temp_dir.path(), "nonexistent")
            .await;

        assert!(result.is_err());
        // Just verify it returns an error - the specific error type may vary
    }

    #[tokio::test]
    async fn test_locate_module_files_no_src_dir() {
        // Create a temporary directory without src/
        let temp_dir = tempdir().unwrap();

        let plugin = &RustPlugin::new();
        let result = plugin
            .locate_module_files(temp_dir.path(), "my_module")
            .await;

        assert!(result.is_err());
        // Just verify it returns an error - the specific error type may vary
    }

    #[tokio::test]
    async fn test_locate_module_files_empty_module_path() {
        let temp_dir = tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        let plugin = &RustPlugin::new();
        let result = plugin.locate_module_files(temp_dir.path(), "").await;

        assert!(result.is_err());
        // Just verify it returns an error - the specific error type may vary
    }

    #[tokio::test]
    async fn test_parse_imports_empty_file() {
        // Create a temporary Rust file with no imports
        let temp_dir = tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        let rust_content = r#"
fn main() {
    println!("Hello, world!");
}
"#;
        let test_file = src_dir.join("test_module.rs");
        fs::write(&test_file, rust_content).unwrap();

        let plugin = &RustPlugin::new();
        let result = plugin.parse_imports(&test_file).await;

        assert!(result.is_ok());
        let dependencies = result.unwrap();
        assert_eq!(dependencies.len(), 0);
    }

    #[test]
    fn test_generate_manifest_with_dependencies() {
        let plugin = &RustPlugin::new();
        let dependencies = vec![
            "serde".to_string(),
            "tokio".to_string(),
            "async-trait".to_string(),
        ];

        let manifest = plugin.generate_manifest("my-test-crate", &dependencies);

        // Check [package] section
        assert!(manifest.contains("[package]"));
        assert!(manifest.contains("name = \"my-test-crate\""));
        assert!(manifest.contains("version = \"0.1.0\""));
        assert!(manifest.contains("edition = \"2021\""));

        // Check [dependencies] section
        assert!(manifest.contains("[dependencies]"));
        assert!(manifest.contains("serde = \"*\""));
        assert!(manifest.contains("tokio = \"*\""));
        assert!(manifest.contains("async-trait = \"*\""));

        // Verify it's valid TOML structure by checking line order
        let lines: Vec<&str> = manifest.lines().collect();

        // Find indices of key sections
        let package_idx = lines.iter().position(|&l| l == "[package]").unwrap();
        let deps_idx = lines.iter().position(|&l| l == "[dependencies]").unwrap();

        // [dependencies] should come after [package]
        assert!(deps_idx > package_idx);
    }

    #[test]
    fn test_generate_manifest_no_dependencies() {
        let plugin = &RustPlugin::new();
        let dependencies: Vec<String> = vec![];

        let manifest = plugin.generate_manifest("simple-crate", &dependencies);

        // Check [package] section exists
        assert!(manifest.contains("[package]"));
        assert!(manifest.contains("name = \"simple-crate\""));
        assert!(manifest.contains("version = \"0.1.0\""));
        assert!(manifest.contains("edition = \"2021\""));

        // [dependencies] section should NOT exist if there are no dependencies
        assert!(!manifest.contains("[dependencies]"));
    }

    #[test]
    fn test_generate_manifest_single_dependency() {
        let plugin = &RustPlugin::new();
        let dependencies = vec!["serde".to_string()];

        let manifest = plugin.generate_manifest("test-crate", &dependencies);

        assert!(manifest.contains("[package]"));
        assert!(manifest.contains("name = \"test-crate\""));
        assert!(manifest.contains("[dependencies]"));
        assert!(manifest.contains("serde = \"*\""));
    }

    #[test]
    fn test_generate_manifest_special_characters_in_name() {
        let plugin = &RustPlugin::new();
        let dependencies = vec![];

        let manifest = plugin.generate_manifest("my-special_crate123", &dependencies);

        assert!(manifest.contains("name = \"my-special_crate123\""));
        assert!(manifest.contains("[package]"));
    }

    #[test]
    fn test_rust_plugin_no_changes_different_crate() {
        use serde_json::json;

        let plugin = &RustPlugin::new();
        let source = r#"use some_other_crate::SomeType;"#;

        let rename_info = json!({
            "old_crate_name": "old_crate",
            "new_crate_name": "new_crate",
        });

        let (new_content, count) = plugin
            .rewrite_imports_for_rename(
                source,
                Path::new(""),
                Path::new(""),
                Path::new(""),
                Path::new(""),
                Some(&rename_info),
            )
            .unwrap();

        assert_eq!(count, 0);
        assert_eq!(new_content, source);
    }

    #[test]
    fn test_rust_plugin_no_rename_info() {
        let plugin = &RustPlugin::new();
        let source = r#"use old_crate::SomeType;"#;

        let (new_content, count) = plugin
            .rewrite_imports_for_rename(
                source,
                Path::new(""),
                Path::new(""),
                Path::new(""),
                Path::new(""),
                None,
            )
            .unwrap();

        assert_eq!(count, 0);
        assert_eq!(new_content, source);
    }

    #[tokio::test]
    async fn test_workspace_member_creation() {
        // Test that is_workspace_member=true creates/updates workspace configuration
        let temp_dir = tempdir().unwrap();
        let project_root = temp_dir.path();

        // Create source crate WITHOUT a workspace Cargo.toml
        let src_crate = project_root.join("src_crate");
        let src_dir = src_crate.join("src");
        fs::create_dir_all(&src_dir).unwrap();

        // Create Cargo.toml for source crate (no workspace)
        fs::write(
            src_crate.join("Cargo.toml"),
            r#"[package]
name = "src_crate"
version = "0.1.0"
edition = "2021"
"#,
        )
        .unwrap();

        // Create lib.rs with module declaration
        fs::write(
            src_dir.join("lib.rs"),
            r#"pub mod my_module;

pub fn main_function() {
    println!("Main function");
}
"#,
        )
        .unwrap();

        // Create the module to be extracted
        fs::write(
            src_dir.join("my_module.rs"),
            r#"use std::collections::HashMap;

pub fn module_function() {
    let map: HashMap<String, i32> = HashMap::new();
    println!("Module function");
}
"#,
        )
        .unwrap();

        // Create target directory
        let target_crate = project_root.join("extracted_crate");
        fs::create_dir_all(&target_crate).unwrap();

        // Run the extraction plan WITH is_workspace_member=true
        let params = ExtractModuleToPackageParams {
            source_package: src_crate.to_string_lossy().to_string(),
            module_path: "my_module".to_string(),
            target_package_path: target_crate.to_string_lossy().to_string(),
            target_package_name: "extracted_module".to_string(),
            update_imports: Some(true),
            create_manifest: Some(true),
            dry_run: Some(false),
            is_workspace_member: Some(true),
        };

        // Create registry with RustAdapter for test
        let mut registry = cb_plugin_api::PluginRegistry::new();
        registry.register(Arc::new(RustPlugin::new()));

        let result = plan_extract_module_to_package_with_registry(params, &registry).await;
        assert!(result.is_ok(), "Plan should succeed: {:?}", result.err());

        let edit_plan = result.unwrap();

        // Verify that a workspace Cargo.toml edit was created
        let workspace_cargo_edit = edit_plan.edits.iter().find(|e| {
            e.file_path
                .as_ref()
                .map(|p| {
                    p.ends_with("Cargo.toml")
                        && !p.contains("src_crate")
                        && !p.contains("extracted_crate")
                })
                .unwrap_or(false)
                && (e.description.contains("workspace") || e.description.contains("members"))
        });

        assert!(
            workspace_cargo_edit.is_some(),
            "Should have workspace Cargo.toml edit when is_workspace_member=true"
        );

        let ws_edit = workspace_cargo_edit.unwrap();

        // The edit should either be Insert (new workspace) or Replace (updating existing)
        assert!(
            ws_edit.edit_type == EditType::Insert || ws_edit.edit_type == EditType::Replace,
            "Workspace edit should be Insert or Replace, got {:?}",
            ws_edit.edit_type
        );

        // Verify the workspace content includes both crates
        assert!(
            ws_edit.new_text.contains("[workspace]"),
            "Workspace Cargo.toml should have [workspace] section"
        );
        assert!(
            ws_edit.new_text.contains("members"),
            "Workspace Cargo.toml should have members array"
        );
        assert!(
            ws_edit.new_text.contains("src_crate") || ws_edit.new_text.contains("./src_crate"),
            "Workspace members should include src_crate"
        );
        assert!(
            ws_edit.new_text.contains("extracted_crate")
                || ws_edit.new_text.contains("./extracted_crate"),
            "Workspace members should include extracted_crate"
        );
    }

    #[tokio::test]
    async fn test_no_workspace_member_creation() {
        // Test that is_workspace_member=false skips workspace configuration
        let temp_dir = tempdir().unwrap();
        let project_root = temp_dir.path();

        // Create source crate WITHOUT a workspace Cargo.toml
        let src_crate = project_root.join("src_crate");
        let src_dir = src_crate.join("src");
        fs::create_dir_all(&src_dir).unwrap();

        // Create Cargo.toml for source crate
        fs::write(
            src_crate.join("Cargo.toml"),
            r#"[package]
name = "src_crate"
version = "0.1.0"
edition = "2021"
"#,
        )
        .unwrap();

        // Create lib.rs with module declaration
        fs::write(
            src_dir.join("lib.rs"),
            r#"pub mod my_module;
"#,
        )
        .unwrap();

        // Create the module to be extracted
        fs::write(
            src_dir.join("my_module.rs"),
            r#"pub fn module_function() {
    println!("Module function");
}
"#,
        )
        .unwrap();

        // Create target directory
        let target_crate = project_root.join("extracted_crate");
        fs::create_dir_all(&target_crate).unwrap();

        // Run the extraction plan WITH is_workspace_member=false
        let params = ExtractModuleToPackageParams {
            source_package: src_crate.to_string_lossy().to_string(),
            module_path: "my_module".to_string(),
            target_package_path: target_crate.to_string_lossy().to_string(),
            target_package_name: "extracted_module".to_string(),
            update_imports: Some(true),
            create_manifest: Some(true),
            dry_run: Some(false),
            is_workspace_member: Some(false),
        };

        // Create registry with RustAdapter for test
        let mut registry = cb_plugin_api::PluginRegistry::new();
        registry.register(Arc::new(RustPlugin::new()));

        let result = plan_extract_module_to_package_with_registry(params, &registry).await;
        assert!(result.is_ok(), "Plan should succeed: {:?}", result.err());

        let edit_plan = result.unwrap();

        // Verify that NO workspace Cargo.toml edit was created
        let workspace_cargo_edit = edit_plan.edits.iter().find(|e| {
            e.file_path
                .as_ref()
                .map(|p| {
                    p.ends_with("Cargo.toml")
                        && !p.contains("src_crate")
                        && !p.contains("extracted_crate")
                })
                .unwrap_or(false)
                && (e.description.contains("workspace") || e.description.contains("members"))
        });

        assert!(
            workspace_cargo_edit.is_none(),
            "Should NOT have workspace Cargo.toml edit when is_workspace_member=false"
        );

        // Should still have source Cargo.toml edit (add dependency)
        let src_cargo_edit = edit_plan.edits.iter().find(|e| {
            e.file_path
                .as_ref()
                .map(|p| p.contains("src_crate/Cargo.toml"))
                .unwrap_or(false)
        });

        assert!(
            src_cargo_edit.is_some(),
            "Should still have source Cargo.toml dependency edit"
        );
    }
}
