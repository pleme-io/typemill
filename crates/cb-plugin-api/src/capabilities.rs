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
/// operations like inline variable and extract function.
///
/// # Example
///
/// ```rust,ignore
/// use cb_plugin_api::capabilities::RefactoringProvider;
///
/// if let Some(provider) = plugin.refactoring_provider() {
///     if provider.supports_inline_variable() {
///         let edit = provider.inline_variable(params)?;
///         // Apply workspace edit...
///     }
/// }
/// ```
pub trait RefactoringProvider: Send + Sync {
    /// Check if inline variable refactoring is supported
    fn supports_inline_variable(&self) -> bool {
        false
    }

    /// Perform inline variable refactoring
    fn inline_variable(&self, _params: InlineParams) -> PluginResult<WorkspaceEdit> {
        Err(crate::PluginError::not_supported("inline_variable"))
    }

    /// Check if extract function refactoring is supported
    fn supports_extract_function(&self) -> bool {
        false
    }

    /// Perform extract function refactoring
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
