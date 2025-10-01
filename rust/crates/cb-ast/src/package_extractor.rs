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
        _package_path: &Path,
        _module_path: &str,
    ) -> AstResult<Vec<std::path::PathBuf>> {
        unimplemented!("RustAdapter::locate_module_files not yet implemented")
    }

    async fn parse_imports(&self, _file_path: &Path) -> AstResult<Vec<String>> {
        unimplemented!("RustAdapter::parse_imports not yet implemented")
    }

    fn generate_manifest(&self, _package_name: &str, _dependencies: &[String]) -> String {
        unimplemented!("RustAdapter::generate_manifest not yet implemented")
    }

    fn rewrite_import(&self, _old_import: &str, _new_package_name: &str) -> String {
        unimplemented!("RustAdapter::rewrite_import not yet implemented")
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

    // Step 3: Generate EditPlan with metadata confirming adapter selection
    // For Phase 3, we return a minimal EditPlan that verifies the dispatch logic works
    let edit_plan = EditPlan {
        source_file: params.source_package.clone(),
        edits: vec![],
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
            }),
            created_at: chrono::Utc::now(),
            complexity: 1,
            impact_areas: vec!["package_extraction".to_string()],
        },
    };

    info!(
        adapter = %adapter.language().as_str(),
        "Successfully created EditPlan with adapter selection"
    );

    Ok(edit_plan)
}
