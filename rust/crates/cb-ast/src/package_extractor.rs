//! Logic for the extract_module_to_package refactoring tool.
//!
//! This module provides language-agnostic package extraction capabilities
//! using a trait-based adapter pattern to support multiple languages.

use crate::error::AstResult;
use async_trait::async_trait;
use cb_api::EditPlan;
use cb_core::language::ProjectLanguage;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct ExtractModuleToPackageParams {
    pub source_package: String,
    pub module_path: String,
    pub target_package_path: String,
    pub target_package_name: String,
    pub update_imports: Option<bool>,
    pub create_manifest: Option<bool>,
    pub dry_run: Option<bool>,
}

/// Language-specific adapter for package extraction operations
///
/// This trait abstracts language-specific operations needed for extracting
/// modules to packages, enabling support for multiple programming languages.
#[async_trait]
pub trait LanguageAdapter: Send + Sync {
    /// Get the language this adapter supports
    fn language(&self) -> ProjectLanguage;

    /// Get the package manifest filename (e.g., "Cargo.toml", "package.json")
    fn manifest_filename(&self) -> &'static str;

    /// Get the source directory name (e.g., "src" for Rust/TS, "" for Python)
    fn source_dir(&self) -> &'static str;

    /// Get the entry point filename (e.g., "lib.rs", "index.ts", "__init__.py")
    fn entry_point(&self) -> &'static str;

    /// Get the module path separator (e.g., "::" for Rust, "." for Python/TS)
    fn module_separator(&self) -> &'static str;

    /// Locate module files given a module path within a package
    ///
    /// # Arguments
    ///
    /// * `package_path` - Path to the source package
    /// * `module_path` - Dotted module path (e.g., "services.planner")
    ///
    /// # Returns
    ///
    /// Vector of file paths that comprise the module
    async fn locate_module_files(
        &self,
        package_path: &Path,
        module_path: &str,
    ) -> AstResult<Vec<std::path::PathBuf>>;

    /// Parse imports/dependencies from a file
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to analyze
    ///
    /// # Returns
    ///
    /// Vector of import statements/paths found in the file
    async fn parse_imports(&self, file_path: &Path) -> AstResult<Vec<String>>;

    /// Generate a package manifest for a new package
    ///
    /// # Arguments
    ///
    /// * `package_name` - Name of the new package
    /// * `dependencies` - List of dependencies the package needs
    ///
    /// # Returns
    ///
    /// String containing the manifest file content
    fn generate_manifest(&self, package_name: &str, dependencies: &[String]) -> String;

    /// Update an import statement from internal to external
    ///
    /// # Arguments
    ///
    /// * `old_import` - Original import path (e.g., "crate::services::planner")
    /// * `new_package_name` - New package name (e.g., "cb_planner")
    ///
    /// # Returns
    ///
    /// Updated import statement
    fn rewrite_import(&self, old_import: &str, new_package_name: &str) -> String;
}

/// Rust language adapter
pub struct RustAdapter;

#[async_trait]
impl LanguageAdapter for RustAdapter {
    fn language(&self) -> ProjectLanguage {
        ProjectLanguage::Rust
    }

    fn manifest_filename(&self) -> &'static str {
        "Cargo.toml"
    }

    fn source_dir(&self) -> &'static str {
        "src"
    }

    fn entry_point(&self) -> &'static str {
        "lib.rs"
    }

    fn module_separator(&self) -> &'static str {
        "::"
    }

    async fn locate_module_files(
        &self,
        package_path: &Path,
        module_path: &str,
    ) -> AstResult<Vec<std::path::PathBuf>> {
        use std::path::PathBuf;
        use tracing::debug;

        debug!(
            package_path = %package_path.display(),
            module_path = %module_path,
            "Locating Rust module files"
        );

        // Start at the crate's source root (e.g., package_path/src)
        let src_root = package_path.join(self.source_dir());

        if !src_root.exists() {
            return Err(crate::error::AstError::Analysis {
                message: format!("Source directory not found: {}", src_root.display()),
            });
        }

        // Split module path by either "::" or "." into segments
        let segments: Vec<&str> = module_path
            .split(|c| c == ':' || c == '.')
            .filter(|s| !s.is_empty())
            .collect();

        if segments.is_empty() {
            return Err(crate::error::AstError::Analysis {
                message: "Module path cannot be empty".to_string(),
            });
        }

        // Build path by joining segments
        let mut current_path = src_root.clone();

        // Navigate through all segments except the last
        for segment in &segments[..segments.len() - 1] {
            current_path = current_path.join(segment);
        }

        // For the final segment, check both naming conventions
        let final_segment = segments[segments.len() - 1];
        let mut found_files = Vec::new();

        // Check for module_name.rs
        let file_path = current_path.join(format!("{}.rs", final_segment));
        if file_path.exists() && file_path.is_file() {
            debug!(file_path = %file_path.display(), "Found module file");
            found_files.push(file_path);
        }

        // Check for module_name/mod.rs
        let mod_path = current_path.join(final_segment).join("mod.rs");
        if mod_path.exists() && mod_path.is_file() {
            debug!(file_path = %mod_path.display(), "Found mod.rs file");
            found_files.push(mod_path);
        }

        if found_files.is_empty() {
            return Err(crate::error::AstError::Analysis {
                message: format!(
                    "Module '{}' not found at {} (checked both {}.rs and {}/mod.rs)",
                    module_path,
                    current_path.display(),
                    final_segment,
                    final_segment
                ),
            });
        }

        debug!(
            files_count = found_files.len(),
            "Successfully located module files"
        );

        Ok(found_files)
    }

    async fn parse_imports(&self, file_path: &Path) -> AstResult<Vec<String>> {
        use std::collections::HashSet;
        use tracing::debug;

        debug!(
            file_path = %file_path.display(),
            "Parsing Rust imports"
        );

        // Read the file content
        let content = tokio::fs::read_to_string(file_path).await.map_err(|e| {
            crate::error::AstError::Analysis {
                message: format!("Failed to read file {}: {}", file_path.display(), e),
            }
        })?;

        // Parse imports using the refactored AST parser
        let import_infos = crate::rust_parser::parse_rust_imports_ast(&content)?;

        // Extract unique external crate names
        let mut dependencies = HashSet::new();

        for import_info in import_infos {
            // Split the module path by "::" to get segments
            let segments: Vec<&str> = import_info.module_path.split("::").collect();

            if let Some(first_segment) = segments.first() {
                // Filter out internal imports (crate, self, super)
                if *first_segment != "crate" && *first_segment != "self" && *first_segment != "super" {
                    // This is an external crate dependency
                    dependencies.insert(first_segment.to_string());
                }
            }
        }

        // Convert HashSet to sorted Vec for consistent output
        let mut result: Vec<String> = dependencies.into_iter().collect();
        result.sort();

        debug!(
            dependencies_count = result.len(),
            "Extracted external dependencies"
        );

        Ok(result)
    }

    fn generate_manifest(&self, package_name: &str, dependencies: &[String]) -> String {
        use std::fmt::Write;

        let mut manifest = String::new();

        // [package] section
        writeln!(manifest, "[package]").unwrap();
        writeln!(manifest, "name = \"{}\"", package_name).unwrap();
        writeln!(manifest, "version = \"0.1.0\"").unwrap();
        writeln!(manifest, "edition = \"2021\"").unwrap();

        // Add blank line before dependencies section if there are any
        if !dependencies.is_empty() {
            writeln!(manifest).unwrap();
            writeln!(manifest, "[dependencies]").unwrap();

            // Add each dependency with wildcard version
            for dep in dependencies {
                writeln!(manifest, "{} = \"*\"", dep).unwrap();
            }
        }

        manifest
    }

    fn rewrite_import(&self, old_import: &str, new_package_name: &str) -> String {
        // Transform internal import path to external crate import
        // e.g., "crate::services::planner" -> "use cb_planner;"
        // e.g., "crate::services::planner::Config" -> "use cb_planner::Config;"

        // Remove common prefixes like "crate::", "self::", "super::"
        let trimmed = old_import
            .trim_start_matches("crate::")
            .trim_start_matches("self::")
            .trim_start_matches("super::");

        // Find the extracted module name and what comes after
        // The path segments after the module name become the new import path
        let segments: Vec<&str> = trimmed.split("::").collect();

        if segments.is_empty() {
            // Just use the new package name
            format!("use {};", new_package_name)
        } else if segments.len() == 1 {
            // Only the module name itself
            format!("use {};", new_package_name)
        } else {
            // Module name plus additional path
            // Skip the first segment (the old module name) and use the rest
            let remaining_path = segments[1..].join("::");
            format!("use {}::{};", new_package_name, remaining_path)
        }
    }
}

/// TypeScript/JavaScript language adapter
pub struct TypeScriptAdapter;

#[async_trait]
impl LanguageAdapter for TypeScriptAdapter {
    fn language(&self) -> ProjectLanguage {
        ProjectLanguage::TypeScript
    }

    fn manifest_filename(&self) -> &'static str {
        "package.json"
    }

    fn source_dir(&self) -> &'static str {
        "src"
    }

    fn entry_point(&self) -> &'static str {
        "index.ts"
    }

    fn module_separator(&self) -> &'static str {
        "."
    }

    async fn locate_module_files(
        &self,
        _package_path: &Path,
        _module_path: &str,
    ) -> AstResult<Vec<std::path::PathBuf>> {
        unimplemented!("TypeScriptAdapter::locate_module_files not yet implemented")
    }

    async fn parse_imports(&self, _file_path: &Path) -> AstResult<Vec<String>> {
        unimplemented!("TypeScriptAdapter::parse_imports not yet implemented")
    }

    fn generate_manifest(&self, _package_name: &str, _dependencies: &[String]) -> String {
        unimplemented!("TypeScriptAdapter::generate_manifest not yet implemented")
    }

    fn rewrite_import(&self, _old_import: &str, _new_package_name: &str) -> String {
        unimplemented!("TypeScriptAdapter::rewrite_import not yet implemented")
    }
}

/// Python language adapter
pub struct PythonAdapter;

#[async_trait]
impl LanguageAdapter for PythonAdapter {
    fn language(&self) -> ProjectLanguage {
        ProjectLanguage::Python
    }

    fn manifest_filename(&self) -> &'static str {
        "pyproject.toml"
    }

    fn source_dir(&self) -> &'static str {
        ""
    }

    fn entry_point(&self) -> &'static str {
        "__init__.py"
    }

    fn module_separator(&self) -> &'static str {
        "."
    }

    async fn locate_module_files(
        &self,
        _package_path: &Path,
        _module_path: &str,
    ) -> AstResult<Vec<std::path::PathBuf>> {
        unimplemented!("PythonAdapter::locate_module_files not yet implemented")
    }

    async fn parse_imports(&self, _file_path: &Path) -> AstResult<Vec<String>> {
        unimplemented!("PythonAdapter::parse_imports not yet implemented")
    }

    fn generate_manifest(&self, _package_name: &str, _dependencies: &[String]) -> String {
        unimplemented!("PythonAdapter::generate_manifest not yet implemented")
    }

    fn rewrite_import(&self, _old_import: &str, _new_package_name: &str) -> String {
        unimplemented!("PythonAdapter::rewrite_import not yet implemented")
    }
}

/// Update a Cargo.toml file to add a new path dependency
fn update_cargo_toml_dependency(
    cargo_content: &str,
    dep_name: &str,
    dep_path: &str,
    source_path: &Path,
) -> AstResult<String> {
    use toml_edit::DocumentMut;

    let mut doc = cargo_content.parse::<DocumentMut>().map_err(|e| {
        crate::error::AstError::Analysis {
            message: format!("Failed to parse Cargo.toml: {}", e),
        }
    })?;

    // Calculate relative path from source to target
    let source_cargo_dir = source_path;
    let target_path = Path::new(dep_path);
    let relative_path = pathdiff::diff_paths(target_path, source_cargo_dir)
        .ok_or_else(|| crate::error::AstError::Analysis {
            message: "Failed to calculate relative path".to_string(),
        })?;

    // Add dependency to [dependencies] section
    if !doc.contains_key("dependencies") {
        doc["dependencies"] = toml_edit::table();
    }

    let deps = doc["dependencies"].as_table_mut().ok_or_else(|| {
        crate::error::AstError::Analysis {
            message: "[dependencies] is not a table".to_string(),
        }
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

    let mut doc = workspace_content.parse::<DocumentMut>().map_err(|e| {
        crate::error::AstError::Analysis {
            message: format!("Failed to parse workspace Cargo.toml: {}", e),
        }
    })?;

    // Calculate relative path from workspace root to new member
    let target_path = Path::new(new_member_path);
    let relative_path = pathdiff::diff_paths(target_path, workspace_root)
        .ok_or_else(|| crate::error::AstError::Analysis {
            message: "Failed to calculate relative path for workspace member".to_string(),
        })?;

    // Ensure [workspace.members] exists
    if !doc.contains_key("workspace") {
        doc["workspace"] = toml_edit::table();
    }

    let workspace = doc["workspace"].as_table_mut().ok_or_else(|| {
        crate::error::AstError::Analysis {
            message: "[workspace] is not a table".to_string(),
        }
    })?;

    if !workspace.contains_key("members") {
        workspace["members"] = toml_edit::value(toml_edit::Array::new());
    }

    let members = workspace["members"].as_array_mut().ok_or_else(|| {
        crate::error::AstError::Analysis {
            message: "[workspace.members] is not an array".to_string(),
        }
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
    let mut syntax_tree: File = syn::parse_str(source).map_err(|e| {
        crate::error::AstError::Analysis {
            message: format!("Failed to parse Rust source for mod removal: {}", e),
        }
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
/// 2. Selecting the appropriate adapter
/// 3. Generating an EditPlan for the refactoring
pub async fn plan_extract_module_to_package(
    params: ExtractModuleToPackageParams,
) -> AstResult<EditPlan> {
    use cb_api::{EditPlanMetadata, ValidationRule, ValidationType};
    use cb_core::language::detect_project_language;
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

    // Step 2: Create appropriate language adapter based on detection
    let adapter: Box<dyn LanguageAdapter> = match detected_language {
        ProjectLanguage::Rust => {
            info!("Selected RustAdapter for extraction");
            Box::new(RustAdapter)
        }
        ProjectLanguage::TypeScript => {
            info!("Selected TypeScriptAdapter for extraction");
            Box::new(TypeScriptAdapter)
        }
        ProjectLanguage::Python => {
            info!("Selected PythonAdapter for extraction");
            Box::new(PythonAdapter)
        }
        ProjectLanguage::Go | ProjectLanguage::Java => {
            return Err(crate::error::AstError::UnsupportedSyntax {
                feature: format!(
                    "{} language not yet supported for extract_module_to_package",
                    detected_language.as_str()
                ),
            });
        }
        ProjectLanguage::Unknown => {
            return Err(crate::error::AstError::Analysis {
                message: "Could not detect project language - no manifest files found".to_string(),
            });
        }
    };

    // Step 3: Locate module files using the adapter
    let located_files = adapter
        .locate_module_files(source_path, &params.module_path)
        .await?;

    debug!(
        files_count = located_files.len(),
        "Located module files"
    );

    // Step 4: Parse imports from all located files and aggregate dependencies
    let mut all_dependencies = std::collections::HashSet::new();

    for file_path in &located_files {
        debug!(
            file_path = %file_path.display(),
            "Parsing dependencies from file"
        );

        match adapter.parse_imports(file_path).await {
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
    let generated_manifest = adapter.generate_manifest(&params.target_package_name, &dependencies);

    debug!(
        manifest_lines = generated_manifest.lines().count(),
        "Generated Cargo.toml manifest"
    );

    // Step 6: Construct file modification plan
    use cb_api::{EditLocation, EditType, TextEdit};
    let mut edits = Vec::new();

    // Edit 1: Create new Cargo.toml
    let manifest_path = Path::new(&params.target_package_path)
        .join(adapter.manifest_filename())
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
            .join(adapter.source_dir())
            .join(adapter.entry_point())
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
            .split(|c| c == ':' || c == '.')
            .filter(|s| !s.is_empty())
            .collect();

        if !module_segments.is_empty() {
            let final_module_name = module_segments[module_segments.len() - 1];

            // Determine parent file path
            let parent_file_path = if module_segments.len() == 1 {
                // Top-level module, parent is lib.rs
                source_path.join(adapter.source_dir()).join(adapter.entry_point())
            } else {
                // Nested module, parent is the containing module's mod.rs or .rs file
                let mut parent_path = source_path.join(adapter.source_dir());
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
                                        description: format!("Remove mod {} declaration from parent", final_module_name),
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
                match update_cargo_toml_dependency(&cargo_content, &params.target_package_name, &params.target_package_path, source_path) {
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
                                description: format!("Add {} dependency to source Cargo.toml", params.target_package_name),
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

    // Step 8: Update workspace Cargo.toml to add new member
    // Find workspace root by looking for Cargo.toml with [workspace]
    let mut workspace_root = source_path.to_path_buf();
    while let Some(parent) = workspace_root.parent() {
        let potential_workspace = parent.join("Cargo.toml");
        if potential_workspace.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&potential_workspace).await {
                if content.contains("[workspace]") {
                    workspace_root = parent.to_path_buf();
                    break;
                }
            }
        }
        workspace_root = parent.to_path_buf();
        if workspace_root.parent().is_none() {
            break;
        }
    }

    let workspace_cargo_toml = workspace_root.join("Cargo.toml");
    if workspace_cargo_toml.exists() && workspace_cargo_toml != source_cargo_toml {
        match tokio::fs::read_to_string(&workspace_cargo_toml).await {
            Ok(workspace_content) => {
                if workspace_content.contains("[workspace]") {
                    match update_workspace_members(&workspace_content, &params.target_package_path, &workspace_root) {
                        Ok(updated_workspace) => {
                            if updated_workspace != workspace_content {
                                edits.push(TextEdit {
                                    file_path: Some(workspace_cargo_toml.to_string_lossy().to_string()),
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
                "adapter_selected": adapter.language().as_str(),
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
        adapter = %adapter.language().as_str(),
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
    use cb_api::EditType;
    use std::fs;
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

        let adapter = RustAdapter;
        let result = adapter
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

        let adapter = RustAdapter;
        let result = adapter
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

        let adapter = RustAdapter;
        let result = adapter
            .locate_module_files(temp_dir.path(), "services::planner")
            .await;

        assert!(result.is_ok());
        let files = result.unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("services/planner.rs") || files[0].ends_with("services\\planner.rs"));
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

        let adapter = RustAdapter;
        let result = adapter
            .locate_module_files(temp_dir.path(), "services.planner")
            .await;

        assert!(result.is_ok());
        let files = result.unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("services/planner.rs") || files[0].ends_with("services\\planner.rs"));
    }

    #[tokio::test]
    async fn test_locate_module_files_not_found() {
        // Create a temporary Rust project structure
        let temp_dir = tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        // Create lib.rs but no module files
        fs::write(src_dir.join("lib.rs"), "// lib.rs").unwrap();

        let adapter = RustAdapter;
        let result = adapter
            .locate_module_files(temp_dir.path(), "nonexistent")
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, crate::error::AstError::Analysis { .. }));
    }

    #[tokio::test]
    async fn test_locate_module_files_no_src_dir() {
        // Create a temporary directory without src/
        let temp_dir = tempdir().unwrap();

        let adapter = RustAdapter;
        let result = adapter
            .locate_module_files(temp_dir.path(), "my_module")
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, crate::error::AstError::Analysis { .. }));
    }

    #[tokio::test]
    async fn test_locate_module_files_empty_module_path() {
        let temp_dir = tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        let adapter = RustAdapter;
        let result = adapter.locate_module_files(temp_dir.path(), "").await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, crate::error::AstError::Analysis { .. }));
    }

    #[tokio::test]
    async fn test_parse_imports_filters_external_crates() {
        // Create a temporary Rust project with a file containing various imports
        let temp_dir = tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        // Create a Rust file with mixed imports
        let rust_content = r#"
use std::collections::HashMap;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use crate::models::User;
use self::helpers::validate;
use super::config::Config;
"#;
        let test_file = src_dir.join("test_module.rs");
        fs::write(&test_file, rust_content).unwrap();

        let adapter = RustAdapter;
        let result = adapter.parse_imports(&test_file).await;

        assert!(result.is_ok());
        let dependencies = result.unwrap();

        // Should only include external crates: std, tokio, serde
        // Should NOT include: crate, self, super
        assert!(dependencies.contains(&"std".to_string()));
        assert!(dependencies.contains(&"tokio".to_string()));
        assert!(dependencies.contains(&"serde".to_string()));
        assert!(!dependencies.contains(&"crate".to_string()));
        assert!(!dependencies.contains(&"self".to_string()));
        assert!(!dependencies.contains(&"super".to_string()));

        // Should be sorted and deduplicated
        assert_eq!(dependencies.len(), 3);
        let mut sorted = dependencies.clone();
        sorted.sort();
        assert_eq!(dependencies, sorted);
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

        let adapter = RustAdapter;
        let result = adapter.parse_imports(&test_file).await;

        assert!(result.is_ok());
        let dependencies = result.unwrap();
        assert_eq!(dependencies.len(), 0);
    }

    #[tokio::test]
    async fn test_parse_imports_deduplication() {
        // Create a temporary Rust file with duplicate imports from same crate
        let temp_dir = tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        let rust_content = r#"
use std::collections::HashMap;
use std::sync::Arc;
use std::io::Read;
"#;
        let test_file = src_dir.join("test_module.rs");
        fs::write(&test_file, rust_content).unwrap();

        let adapter = RustAdapter;
        let result = adapter.parse_imports(&test_file).await;

        assert!(result.is_ok());
        let dependencies = result.unwrap();

        // Should only have "std" once, even though it's imported multiple times
        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies[0], "std");
    }

    #[tokio::test]
    async fn test_parse_imports_nonexistent_file() {
        let temp_dir = tempdir().unwrap();
        let nonexistent_file = temp_dir.path().join("nonexistent.rs");

        let adapter = RustAdapter;
        let result = adapter.parse_imports(&nonexistent_file).await;

        assert!(result.is_err());
    }

    #[test]
    fn test_generate_manifest_with_dependencies() {
        let adapter = RustAdapter;
        let dependencies = vec!["serde".to_string(), "tokio".to_string(), "async-trait".to_string()];

        let manifest = adapter.generate_manifest("my-test-crate", &dependencies);

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
        let adapter = RustAdapter;
        let dependencies: Vec<String> = vec![];

        let manifest = adapter.generate_manifest("simple-crate", &dependencies);

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
        let adapter = RustAdapter;
        let dependencies = vec!["serde".to_string()];

        let manifest = adapter.generate_manifest("test-crate", &dependencies);

        assert!(manifest.contains("[package]"));
        assert!(manifest.contains("name = \"test-crate\""));
        assert!(manifest.contains("[dependencies]"));
        assert!(manifest.contains("serde = \"*\""));
    }

    #[test]
    fn test_generate_manifest_special_characters_in_name() {
        let adapter = RustAdapter;
        let dependencies = vec![];

        let manifest = adapter.generate_manifest("my-special_crate123", &dependencies);

        assert!(manifest.contains("name = \"my-special_crate123\""));
        assert!(manifest.contains("[package]"));
    }

    #[tokio::test]
    async fn test_plan_extract_module_to_package_integration() {
        // Create a comprehensive test project structure
        let temp_dir = tempdir().unwrap();
        let project_root = temp_dir.path();

        // Create source crate structure
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
use serde::Serialize;

pub fn module_function() {
    let map: HashMap<String, i32> = HashMap::new();
    println!("Module function");
}
"#,
        )
        .unwrap();

        // Create target directory
        let target_crate = project_root.join("target_crate");
        fs::create_dir_all(&target_crate).unwrap();

        // Run the extraction plan
        let params = ExtractModuleToPackageParams {
            source_package: src_crate.to_string_lossy().to_string(),
            module_path: "my_module".to_string(),
            target_package_path: target_crate.to_string_lossy().to_string(),
            target_package_name: "extracted_module".to_string(),
            update_imports: Some(true),
            create_manifest: Some(true),
            dry_run: Some(false),
        };

        let result = plan_extract_module_to_package(params).await;

        assert!(result.is_ok(), "Plan should succeed: {:?}", result.err());
        let edit_plan = result.unwrap();

        // Verify metadata
        assert_eq!(edit_plan.metadata.intent_name, "extract_module_to_package");
        assert_eq!(
            edit_plan.metadata.intent_arguments["adapter_selected"],
            "rust"
        );

        // Verify we have the expected number of edits (at least 3: create manifest, create lib.rs, delete old file)
        // May have 4 if parent mod removal succeeded
        assert!(
            edit_plan.edits.len() >= 3,
            "Should have at least 3 edits, got {}",
            edit_plan.edits.len()
        );

        // Verify Edit 1: Create Cargo.toml
        let manifest_edit = &edit_plan.edits[0];
        assert_eq!(manifest_edit.edit_type, EditType::Insert);
        assert_eq!(manifest_edit.priority, 100);
        assert!(manifest_edit
            .file_path
            .as_ref()
            .unwrap()
            .contains("Cargo.toml"));
        assert!(manifest_edit.new_text.contains("name = \"extracted_module\""));
        assert!(manifest_edit.new_text.contains("[package]"));
        assert!(manifest_edit.new_text.contains("[dependencies]"));
        assert!(manifest_edit.new_text.contains("serde = \"*\""));
        assert!(manifest_edit.new_text.contains("std = \"*\""));

        // Verify Edit 2: Create src/lib.rs
        let entrypoint_edit = &edit_plan.edits[1];
        assert_eq!(entrypoint_edit.edit_type, EditType::Insert);
        assert_eq!(entrypoint_edit.priority, 90);
        assert!(entrypoint_edit
            .file_path
            .as_ref()
            .unwrap()
            .contains("lib.rs"));
        assert!(entrypoint_edit.new_text.contains("module_function"));
        assert!(entrypoint_edit.new_text.contains("use std::collections::HashMap"));

        // Verify Edit 3: Delete old module file
        let delete_edit = &edit_plan.edits[2];
        assert_eq!(delete_edit.edit_type, EditType::Delete);
        assert_eq!(delete_edit.priority, 80);
        assert!(delete_edit
            .file_path
            .as_ref()
            .unwrap()
            .contains("my_module.rs"));
        assert!(delete_edit.original_text.contains("module_function"));

        // If there's a 4th edit, it should be the parent mod removal
        if edit_plan.edits.len() >= 4 {
            let parent_edit = &edit_plan.edits[3];
            assert_eq!(parent_edit.edit_type, EditType::Replace);
            assert_eq!(parent_edit.priority, 70);
            assert!(parent_edit
                .file_path
                .as_ref()
                .unwrap()
                .contains("lib.rs"));
            assert!(parent_edit.description.contains("Remove mod my_module"));
        }

        // Verify dependencies were collected
        let dependencies = edit_plan.metadata.intent_arguments["dependencies"]
            .as_array()
            .unwrap();
        assert!(dependencies.len() >= 2); // Should have at least serde and std
        assert!(dependencies
            .iter()
            .any(|d| d.as_str() == Some("serde")));
        assert!(dependencies.iter().any(|d| d.as_str() == Some("std")));
    }
}
