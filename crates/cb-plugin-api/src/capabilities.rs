//! Capability traits for language plugins
//!
//! This module defines fine-grained capability traits that language plugins can implement
//! to provide specific functionality. This approach allows for better decoupling and
//! easier extensibility compared to downcasting to concrete plugin types.
//!
//! # Design Principles
//!
//! - **Trait-based dispatch**: Use capability traits instead of downcasting
//! - **Optional capabilities**: Plugins only implement what they support
//! - **Language-agnostic**: Shared code works with any language that implements the trait
//! - **Scalable**: Adding new languages doesn't require updating shared code

use crate::{ModuleReference, PluginResult, ScanScope};
use async_trait::async_trait;
use codebuddy_foundation::protocol::ImportGraph;
use std::path::Path;

// ============================================================================
// Module Reference Scanning Capability
// ============================================================================

/// Capability for scanning module references in source code
///
/// This trait allows language plugins to provide module reference detection
/// for refactoring operations like rename and move.
///
/// # Example
///
/// ```rust,ignore
/// use cb_plugin_api::capabilities::ModuleReferenceScanner;
///
/// if let Some(scanner) = plugin.module_reference_scanner() {
///     let refs = scanner.scan_references(file_path, content, "my_module", scope)?;
///     // Process references...
/// }
/// ```
pub trait ModuleReferenceScanner: Send + Sync {
    /// Scan a file for references to a specific module
    ///
    /// # Arguments
    ///
    /// * `content` - Source code content to scan
    /// * `module_name` - Name of the module to find references to
    /// * `scope` - Scope of the scan (top-level only, all use statements, etc.)
    ///
    /// # Returns
    ///
    /// Vector of found module references with their locations
    fn scan_references(
        &self,
        content: &str,
        module_name: &str,
        scope: ScanScope,
    ) -> PluginResult<Vec<ModuleReference>>;
}

// ============================================================================
// Refactoring Provider Capability
// ============================================================================

/// Parameters for inline variable refactoring
#[derive(Debug, Clone)]
pub struct InlineParams {
    /// Source file path
    pub file_path: std::path::PathBuf,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (0-indexed)
    pub column: usize,
}

/// Parameters for extract function refactoring
#[derive(Debug, Clone)]
pub struct ExtractParams {
    /// Source file path
    pub file_path: std::path::PathBuf,
    /// Start line (1-indexed)
    pub start_line: usize,
    /// Start column (0-indexed)
    pub start_column: usize,
    /// End line (1-indexed)
    pub end_line: usize,
    /// End column (0-indexed)
    pub end_column: usize,
    /// Name for the extracted function
    pub function_name: String,
}

/// Workspace edit for LSP-style refactorings
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceEdit {
    /// Changes to apply to files
    pub changes: std::collections::HashMap<String, Vec<TextEdit>>,
}

/// A text edit operation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TextEdit {
    /// Start line (1-indexed)
    pub start_line: usize,
    /// Start column (0-indexed)
    pub start_column: usize,
    /// End line (1-indexed)
    pub end_line: usize,
    /// End column (0-indexed)
    pub end_column: usize,
    /// New text to insert
    pub new_text: String,
}

/// Capability for providing refactoring operations
///
/// This trait allows language plugins to provide language-specific refactoring
/// operations like inline variable, extract function, and extract variable.
///
/// # Example
///
/// ```rust,ignore
/// use cb_plugin_api::capabilities::RefactoringProvider;
///
/// if let Some(provider) = plugin.refactoring_provider() {
///     if provider.supports_inline_variable() {
///         let plan = provider.plan_inline_variable(
///             source,
///             variable_line,
///             variable_col,
///             file_path
///         ).await?;
///         // Apply edit plan...
///     }
/// }
/// ```
#[async_trait]
pub trait RefactoringProvider: Send + Sync {
    /// Check if inline variable refactoring is supported
    fn supports_inline_variable(&self) -> bool {
        false
    }

    /// Plan inline variable refactoring
    ///
    /// Analyzes the code and generates an edit plan for inlining a variable.
    ///
    /// # Arguments
    ///
    /// * `source` - Source code content
    /// * `variable_line` - Line number where variable is declared (0-based)
    /// * `variable_col` - Column number where variable is declared (0-based)
    /// * `file_path` - Path to the source file
    async fn plan_inline_variable(
        &self,
        _source: &str,
        _variable_line: u32,
        _variable_col: u32,
        _file_path: &str,
    ) -> PluginResult<codebuddy_foundation::protocol::EditPlan> {
        Err(crate::PluginError::not_supported("plan_inline_variable"))
    }

    /// Check if extract function refactoring is supported
    fn supports_extract_function(&self) -> bool {
        false
    }

    /// Plan extract function refactoring
    ///
    /// Analyzes the code and generates an edit plan for extracting a function.
    ///
    /// # Arguments
    ///
    /// * `source` - Source code content
    /// * `start_line` - Start line of selection (0-based)
    /// * `end_line` - End line of selection (0-based)
    /// * `function_name` - Name for the extracted function
    /// * `file_path` - Path to the source file
    async fn plan_extract_function(
        &self,
        _source: &str,
        _start_line: u32,
        _end_line: u32,
        _function_name: &str,
        _file_path: &str,
    ) -> PluginResult<codebuddy_foundation::protocol::EditPlan> {
        Err(crate::PluginError::not_supported("plan_extract_function"))
    }

    /// Check if extract variable refactoring is supported
    fn supports_extract_variable(&self) -> bool {
        false
    }

    /// Plan extract variable refactoring
    ///
    /// Analyzes the code and generates an edit plan for extracting a variable.
    ///
    /// # Arguments
    ///
    /// * `source` - Source code content
    /// * `start_line` - Start line of selection (0-based)
    /// * `start_col` - Start column of selection (0-based)
    /// * `end_line` - End line of selection (0-based)
    /// * `end_col` - End column of selection (0-based)
    /// * `variable_name` - Optional name for the variable (None = auto-generate)
    /// * `file_path` - Path to the source file
    async fn plan_extract_variable(
        &self,
        _source: &str,
        _start_line: u32,
        _start_col: u32,
        _end_line: u32,
        _end_col: u32,
        _variable_name: Option<String>,
        _file_path: &str,
    ) -> PluginResult<codebuddy_foundation::protocol::EditPlan> {
        Err(crate::PluginError::not_supported("plan_extract_variable"))
    }

    // ============================================================================
    // Legacy sync methods - DEPRECATED
    // These exist for backwards compatibility but should not be used in new code
    // ============================================================================

    /// Perform inline variable refactoring (DEPRECATED - use plan_inline_variable)
    #[deprecated(note = "Use async plan_inline_variable instead")]
    fn inline_variable(&self, _params: InlineParams) -> PluginResult<WorkspaceEdit> {
        Err(crate::PluginError::not_supported("inline_variable"))
    }

    /// Perform extract function refactoring (DEPRECATED - use plan_extract_function)
    #[deprecated(note = "Use async plan_extract_function instead")]
    fn extract_function(&self, _params: ExtractParams) -> PluginResult<WorkspaceEdit> {
        Err(crate::PluginError::not_supported("extract_function"))
    }
}

// ============================================================================
// Import Analyzer Capability
// ============================================================================

/// Capability for analyzing imports and dependencies
///
/// This trait allows language plugins to provide import graph analysis
/// for understanding project dependencies.
///
/// # Example
///
/// ```rust,ignore
/// use cb_plugin_api::capabilities::ImportAnalyzer;
///
/// if let Some(analyzer) = plugin.import_analyzer() {
///     let graph = analyzer.build_import_graph(file_path)?;
///     // Analyze import graph...
/// }
/// ```
pub trait ImportAnalyzer: Send + Sync {
    /// Build import graph for a file
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to analyze
    ///
    /// # Returns
    ///
    /// Import graph with all imports and their metadata
    fn build_import_graph(&self, file_path: &Path) -> PluginResult<ImportGraph>;

    /// Find unused imports in a file
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to analyze
    ///
    /// # Returns
    ///
    /// Vector of unused import names
    fn find_unused_imports(&self, _file_path: &Path) -> PluginResult<Vec<String>> {
        // Default implementation: not supported
        Err(crate::PluginError::not_supported("find_unused_imports"))
    }
}

// ============================================================================
// Module Locator Capability
// ============================================================================

/// Capability for locating module files within a package
///
/// This trait allows language plugins to provide module file discovery
/// for operations like extracting modules to new packages.
///
/// # Example
///
/// ```rust,ignore
/// use cb_plugin_api::capabilities::ModuleLocator;
///
/// if let Some(locator) = plugin.module_locator() {
///     let files = locator.locate_module_files(
///         package_path,
///         "my::module::path"
///     ).await?;
///     // Process located files...
/// }
/// ```
#[async_trait]
pub trait ModuleLocator: Send + Sync {
    /// Locate all files that comprise a module
    ///
    /// Given a package path and a module path, this method returns all source files
    /// that belong to that module. This is used for operations like extracting a module
    /// to a new package.
    ///
    /// # Arguments
    ///
    /// * `package_path` - Path to the package root directory
    /// * `module_path` - Module path in the language's syntax (e.g., "crate::utils::helpers" for Rust)
    ///
    /// # Returns
    ///
    /// Vector of absolute paths to all files that comprise the module.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Module path is invalid for the language
    /// - Module cannot be found in the package
    /// - File system errors occur during search
    async fn locate_module_files(
        &self,
        package_path: &Path,
        module_path: &str,
    ) -> crate::PluginResult<Vec<std::path::PathBuf>>;
}

// ============================================================================
// Manifest Updater Capability
// ============================================================================

/// Capability for updating manifest files (Cargo.toml, package.json, etc.)
///
/// This trait allows language plugins to provide manifest update operations
/// for dependency management and package configuration.
///
/// # Example
///
/// ```rust,ignore
/// use cb_plugin_api::capabilities::ManifestUpdater;
///
/// if let Some(updater) = plugin.manifest_updater() {
///     let updated_content = updater.update_dependency(
///         manifest_path,
///         "old-dep",
///         "new-dep",
///         Some("1.0.0")
///     ).await?;
///     // Write updated content to file...
/// }
/// ```
#[async_trait]
pub trait ManifestUpdater: Send + Sync {
    /// Update a dependency in the manifest file
    ///
    /// This method modifies a dependency entry in the manifest, either renaming it
    /// or changing its version/path configuration.
    ///
    /// # Arguments
    ///
    /// * `manifest_path` - Path to the manifest file (Cargo.toml, package.json, etc.)
    /// * `old_name` - Current name of the dependency
    /// * `new_name` - New name for the dependency (may be same as old_name for version-only updates)
    /// * `new_version` - Optional new version or path for the dependency
    ///
    /// # Returns
    ///
    /// Updated manifest content as a string, ready to be written to the file.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Manifest file cannot be read or parsed
    /// - Dependency not found in manifest
    /// - Invalid manifest format after update
    async fn update_dependency(
        &self,
        manifest_path: &Path,
        old_name: &str,
        new_name: &str,
        new_version: Option<&str>,
    ) -> PluginResult<String>;
}
