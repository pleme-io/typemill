use super::{edits, manifest, workspace, AstResult, ExtractModuleToPackageParams};
use cb_core::language::{detect_project_language, ProjectLanguage};
use cb_protocol::{EditPlan, EditPlanMetadata, ValidationRule, ValidationType};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info};

pub(crate) async fn plan_extract_module_to_package(
    params: ExtractModuleToPackageParams,
    plugin_registry: &cb_plugin_api::PluginRegistry,
) -> AstResult<EditPlan> {
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
        ProjectLanguage::Swift => "swift",
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
        .find(|p| p.metadata().language == detected_language)
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
    // Downcast to RustPlugin to access locate_module_files
    use cb_lang_rust::RustPlugin;

    let rust_plugin = plugin
        .as_any()
        .downcast_ref::<RustPlugin>()
        .ok_or_else(|| crate::error::AstError::Analysis {
            message: "locate_module_files is only supported for Rust language".to_string(),
        })?;

    let located_files = rust_plugin
        .locate_module_files(source_path, &params.module_path)
        .await?;

    debug!(files_count = located_files.len(), "Located module files");

    // Step 4: Parse imports from all located files and aggregate dependencies
    let dependencies = manifest::extract_dependencies(rust_plugin, &located_files).await;
    debug!(
        dependencies_count = dependencies.len(),
        "Aggregated dependencies from all module files"
    );

    // Step 5: Generate new crate manifest
    let generated_manifest =
        manifest::generate_manifest_for_plugin(rust_plugin, &params.target_package_name, &dependencies);

    debug!(
        manifest_lines = generated_manifest.lines().count(),
        "Generated Cargo.toml manifest"
    );

    // Step 6: Construct file modification plan
    let mut edits = Vec::new();
    edits::add_manifest_creation_edit(&mut edits, &params, &plugin, &generated_manifest);
    debug!(edit_count = edits.len(), "Created manifest TextEdit");

    if let Some(original_file_path) = located_files.first() {
        let original_content =
            edits::add_entrypoint_creation_edit(&mut edits, &params, &plugin, original_file_path).await?;
        debug!(edit_count = edits.len(), "Created entrypoint TextEdit");

        edits::add_delete_original_file_edit(&mut edits, original_file_path, &original_content);
        debug!(edit_count = edits.len(), "Created delete TextEdit");

        edits::add_remove_mod_declaration_edit(&mut edits, &params, source_path, rust_plugin).await;
        debug!(edit_count = edits.len(), "Created parent mod removal TextEdit");
    }

    // Step 7: Update source crate's Cargo.toml to add new dependency
    edits::add_dependency_to_source_edit(&mut edits, &params, source_path, rust_plugin).await;
    debug!("Created source Cargo.toml update TextEdit");

    // Step 8: Update workspace Cargo.toml to add new member (if is_workspace_member is true)
    if params.is_workspace_member.unwrap_or(false) {
        workspace::update_workspace(&mut edits, &params, source_path, plugin, rust_plugin).await;
    } else {
        debug!("is_workspace_member=false: skipping workspace configuration");
    }

    // Step 9: Find and update all use statements in the workspace
    if params.update_imports.unwrap_or(true) {
        edits::add_import_update_edits(&mut edits, &params, source_path, rust_plugin, &located_files).await?;
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
                "plugin_selected": plugin.metadata().language.as_str(),
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
        plugin = %plugin.metadata().language.as_str(),
        files_count = located_files.len(),
        dependencies_count = dependencies.len(),
        edits_count = edit_plan.edits.len(),
        "Successfully created EditPlan with file modification operations"
    );

    Ok(edit_plan)
}