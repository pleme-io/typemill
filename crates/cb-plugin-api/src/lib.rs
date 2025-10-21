//! Core Plugin API for Language Support
//!
//! This crate defines the foundational traits and types for implementing
//! language-specific plugins in the Codebuddy system. Each supported programming
//! language should implement the `LanguagePlugin` trait to provide parsing,
//! analysis, and refactoring capabilities.
//!
//! # Architecture
//!
//! The plugin system follows a vertical slice architecture where each language
//! is a self-contained module with its own:
//! - AST parsing logic
//! - Manifest file handling (Cargo.toml, package.json, etc.)
//! - Code analysis and intelligence
//! - Refactoring operations

use async_trait::async_trait;
use codebuddy_foundation::error::ApiError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

// ============================================================================
// Module Declarations
// ============================================================================

pub mod capabilities;
pub mod import_support;
pub mod language;
pub mod metadata;
pub mod plugin_registry;
pub mod project_factory;
pub mod reference_detector;
pub mod server;
pub mod test_fixtures;
pub mod workspace_support;

// Re-exports
pub use capabilities::{
    ExtractParams, ImportAnalyzer, InlineParams, ManifestUpdater, ModuleLocator,
    ModuleReferenceScanner, RefactoringProvider, TextEdit, WorkspaceEdit,
};
pub use import_support::{
    ImportAdvancedSupport, ImportMoveSupport, ImportMutationSupport, ImportParser,
    ImportRenameSupport,
};
pub use metadata::LanguageMetadata;
pub use plugin_registry::{PluginDescriptor, iter_plugins};
pub use project_factory::{
    CreatePackageConfig, CreatePackageResult, PackageInfo, PackageType, ProjectFactory, Template,
};
pub use reference_detector::ReferenceDetector;
// Note: codebuddy_plugin! macro is automatically exported at crate root due to #[macro_export]
pub use server::PluginServer;
pub use test_fixtures::{
    ComplexityFixture, LanguageTestFixtures, RefactoringFixture, RefactoringOperation,
};
pub use workspace_support::{MoveManifestPlan, WorkspaceSupport};

// ============================================================================
// Error Types
// ============================================================================

/// Result type for plugin operations
pub type PluginResult<T> = Result<T, PluginError>;

/// Errors that can occur during plugin operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum PluginError {
    /// Failed to parse source code
    #[error("Parse error: {message}")]
    Parse {
        message: String,
        /// Optional line and column information
        location: Option<SourceLocation>,
    },

    /// Failed to analyze manifest file
    #[error("Manifest error: {message}")]
    Manifest { message: String },

    /// Operation not supported by this language plugin
    #[error("Operation not supported: {operation}")]
    NotSupported { operation: String },

    /// Invalid input provided to plugin
    #[error("Invalid input: {message}")]
    InvalidInput { message: String },

    /// Internal plugin error
    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl PluginError {
    /// Create a parse error
    pub fn parse(message: impl Into<String>) -> Self {
        Self::Parse {
            message: message.into(),
            location: None,
        }
    }

    /// Create a parse error with location information
    pub fn parse_at(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self::Parse {
            message: message.into(),
            location: Some(SourceLocation { line, column }),
        }
    }

    /// Create a manifest error
    pub fn manifest(message: impl Into<String>) -> Self {
        Self::Manifest {
            message: message.into(),
        }
    }

    /// Create a not supported error
    pub fn not_supported(operation: impl Into<String>) -> Self {
        Self::NotSupported {
            operation: operation.into(),
        }
    }

    /// Create an invalid input error
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput {
            message: message.into(),
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }
}

/// Convert PluginError to ApiError for MCP responses
impl From<PluginError> for ApiError {
    fn from(err: PluginError) -> Self {
        use codebuddy_foundation::error::error_codes::*;

        match err {
            PluginError::Parse { message, location } => {
                let mut error = ApiError::new(E1008_INVALID_DATA, message);
                if let Some(loc) = location {
                    error = error.details(serde_json::json!({
                        "line": loc.line,
                        "column": loc.column
                    }));
                }
                error
            }
            PluginError::Manifest { message } => ApiError::new(E1008_INVALID_DATA, message),
            PluginError::NotSupported { operation } => ApiError::new(
                E1007_NOT_SUPPORTED,
                format!("Operation not supported: {}", operation),
            ),
            PluginError::InvalidInput { message } => ApiError::new(E1001_INVALID_REQUEST, message),
            PluginError::Internal { message } => {
                ApiError::new(E1000_INTERNAL_SERVER_ERROR, message)
            }
        }
    }
}

// ============================================================================
// Core Data Types
// ============================================================================

/// Location in source code (line and column)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

/// Parsed source code representation
///
/// This is a generic container for parsed AST data. Each language plugin
/// can store its language-specific AST in the `data` field as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSource {
    /// Language-specific AST data (serialized as JSON for flexibility)
    pub data: Value,

    /// List of top-level symbols found in the source
    pub symbols: Vec<Symbol>,
}

/// A symbol in the source code (function, class, variable, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Symbol {
    /// Symbol name
    pub name: String,

    /// Symbol kind (function, class, variable, etc.)
    pub kind: SymbolKind,

    /// Location in source code
    pub location: SourceLocation,

    /// Optional documentation/comments
    pub documentation: Option<String>,
}

/// Kind of symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Class,
    Struct,
    Enum,
    Interface,
    Variable,
    Constant,
    Module,
    Method,
    Field,
    Other,
}

/// Plugin capability flags
///
/// Indicates which optional features a language plugin supports.
#[derive(Debug, Clone, Copy, Default)]
pub struct PluginCapabilities {
    /// Supports import parsing and rewriting
    pub imports: bool,
    /// Supports workspace manifest operations
    pub workspace: bool,
    /// Supports project/package creation
    pub project_factory: bool,
}

impl PluginCapabilities {
    /// Creates a new `PluginCapabilities` with all fields set to false (no capabilities).
    ///
    /// This is the safest default - use builder methods to enable specific capabilities.
    pub const fn none() -> Self {
        Self {
            imports: false,
            workspace: false,
            project_factory: false,
        }
    }

    /// Creates a new `PluginCapabilities` with all features enabled.
    pub const fn all() -> Self {
        Self {
            imports: true,
            workspace: true,
            project_factory: true,
        }
    }

    /// Enable import support
    pub const fn with_imports(mut self) -> Self {
        self.imports = true;
        self
    }

    /// Enable workspace support
    pub const fn with_workspace(mut self) -> Self {
        self.workspace = true;
        self
    }

    /// Enable project factory support
    pub const fn with_project_factory(mut self) -> Self {
        self.project_factory = true;
        self
    }
}

/// Configuration for a Language Server Protocol (LSP) server.
#[derive(Debug, Clone)]
pub struct LspConfig {
    /// The command to execute to start the LSP server (e.g., "rust-analyzer").
    pub command: &'static str,
    /// The arguments to pass to the LSP server command.
    pub arguments: &'static [&'static str],
}

impl LspConfig {
    /// Creates a new `LspConfig`.
    pub const fn new(command: &'static str, arguments: &'static [&'static str]) -> Self {
        Self { command, arguments }
    }
}

/// Manifest file data (package.json, Cargo.toml, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestData {
    /// Package/project name
    pub name: String,

    /// Package version
    pub version: String,

    /// Dependencies (name -> version/path)
    pub dependencies: Vec<Dependency>,

    /// Dev dependencies
    pub dev_dependencies: Vec<Dependency>,

    /// Raw manifest data (language-specific)
    pub raw_data: Value,
}

/// A dependency entry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dependency {
    /// Dependency name
    pub name: String,

    /// Version specifier or path
    pub source: DependencySource,
}

/// Where a dependency comes from
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DependencySource {
    /// Registry version (e.g., "1.0.0", "^1.0", etc.)
    Version(String),

    /// Local path dependency
    Path(String),

    /// Git repository
    Git { url: String, rev: Option<String> },
}

// ============================================================================
// Refactoring Support Types (from LanguageAdapter)
// ============================================================================

/// Defines the scope of the import/reference scan
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScanScope {
    /// Only find top-level `import`/`use` statements
    TopLevelOnly,
    /// Find all `use` or `import` statements, including those inside functions
    AllUseStatements,
    /// Find all `use` statements and qualified paths (e.g., `my_module::MyStruct`)
    QualifiedPaths,
    /// Find all references, including string literals (requires confirmation)
    All,
}

/// Represents a found reference to a module within a source file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleReference {
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (0-indexed)
    pub column: usize,
    /// Length of the reference in characters
    pub length: usize,
    /// The actual text that was found
    pub text: String,
    /// The type of reference
    pub kind: ReferenceKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceKind {
    /// An `import` or `export` or `use` declaration
    Declaration,
    /// A qualified path (e.g., `my_module.MyStruct` or `my_module::function`)
    QualifiedPath,
    /// A reference inside a string literal
    StringLiteral,
}

// ============================================================================
// Core Plugin Trait
// ============================================================================

/// Core language plugin trait
///
/// Reduced from 22 methods to 6 core methods. Optional capabilities
/// (imports, workspace) are now separate traits accessed via trait objects.
#[async_trait]
pub trait LanguagePlugin: Send + Sync {
    /// Get static language metadata
    fn metadata(&self) -> &LanguageMetadata;

    /// Parse source code into AST representation
    async fn parse(&self, source: &str) -> PluginResult<ParsedSource>;

    /// Analyze manifest file (Cargo.toml, package.json, etc.)
    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData>;

    /// Get plugin capabilities
    fn capabilities(&self) -> PluginCapabilities;

    /// Analyze detailed imports from source code, returning full ImportGraph.
    ///
    /// This method provides comprehensive import analysis including:
    /// - Detailed import information (module paths, named imports, aliases)
    /// - External dependency detection
    /// - Source locations for each import
    ///
    /// # Arguments
    ///
    /// * `source` - Source code content to analyze
    /// * `file_path` - Optional file path for context (used for relative imports)
    ///
    /// # Returns
    ///
    /// Full `ImportGraph` with detailed metadata, or error if parsing fails.
    ///
    /// # Default Implementation
    ///
    /// Returns an empty ImportGraph. Plugins should override this to provide
    /// language-specific import analysis.
    fn analyze_detailed_imports(
        &self,
        _source: &str,
        _file_path: Option<&Path>,
    ) -> PluginResult<codebuddy_foundation::protocol::ImportGraph> {
        use chrono::Utc;
        // Default: return empty graph
        Ok(codebuddy_foundation::protocol::ImportGraph {
            source_file: _file_path
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            imports: vec![],
            importers: vec![],
            metadata: codebuddy_foundation::protocol::ImportGraphMetadata {
                language: self.metadata().name.to_string(),
                parsed_at: Utc::now(),
                parser_version: "0.0.0".to_string(),
                circular_dependencies: vec![],
                external_dependencies: vec![],
            },
        })
    }

    /// Get import parser if available
    fn import_parser(&self) -> Option<&dyn ImportParser> {
        None
    }

    /// Get import rename support if available
    fn import_rename_support(&self) -> Option<&dyn ImportRenameSupport> {
        None
    }

    /// Get import move support if available
    fn import_move_support(&self) -> Option<&dyn ImportMoveSupport> {
        None
    }

    /// Get import mutation support if available
    fn import_mutation_support(&self) -> Option<&dyn ImportMutationSupport> {
        None
    }

    /// Get import advanced support if available
    fn import_advanced_support(&self) -> Option<&dyn ImportAdvancedSupport> {
        None
    }

    /// Get workspace support if available
    fn workspace_support(&self) -> Option<&dyn WorkspaceSupport> {
        None
    }

    /// Get reference detector if available
    fn reference_detector(&self) -> Option<&dyn ReferenceDetector> {
        None
    }

    /// Get manifest updater if available
    fn manifest_updater(&self) -> Option<&dyn ManifestUpdater> {
        None
    }

    /// Get module locator if available
    fn module_locator(&self) -> Option<&dyn ModuleLocator> {
        None
    }

    /// Get project factory if available
    fn project_factory(&self) -> Option<&dyn ProjectFactory> {
        None
    }

    /// Provide test fixtures for integration testing (optional)
    fn test_fixtures(&self) -> Option<LanguageTestFixtures> {
        None
    }

    // Default implementations
    async fn list_functions(&self, source: &str) -> PluginResult<Vec<String>> {
        let parsed = self.parse(source).await?;
        Ok(parsed
            .symbols
            .into_iter()
            .filter(|s| matches!(s.kind, SymbolKind::Function | SymbolKind::Method))
            .map(|s| s.name)
            .collect())
    }

    fn handles_extension(&self, extension: &str) -> bool {
        self.metadata().extensions.contains(&extension)
    }

    fn handles_manifest(&self, filename: &str) -> bool {
        self.metadata().manifest_filename == filename
    }

    // ============================================================================
    // Capability Discovery Methods
    // ============================================================================

    /// Get module reference scanner capability if available
    fn module_reference_scanner(&self) -> Option<&dyn crate::capabilities::ModuleReferenceScanner> {
        None
    }

    /// Get refactoring provider capability if available
    fn refactoring_provider(&self) -> Option<&dyn crate::capabilities::RefactoringProvider> {
        None
    }

    /// Get import analyzer capability if available
    fn import_analyzer(&self) -> Option<&dyn crate::capabilities::ImportAnalyzer> {
        None
    }

    /// Enable downcasting to concrete plugin types
    ///
    /// This allows service layers to access implementation-specific methods
    /// that are not part of the core trait contract.
    fn as_any(&self) -> &dyn std::any::Any;

    /// Rewrite file references when a file is renamed.
    ///
    /// This method provides a generic interface for rewriting import paths or
    /// other file references within a source file's content.
    ///
    /// # Default Implementation
    /// The default implementation uses the `ImportRenameSupport` trait if available,
    /// making it work automatically for simple plugins like Markdown.
    fn rewrite_file_references(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
        _current_file: &Path,
        project_root: &Path,
        _rename_info: Option<&serde_json::Value>,
    ) -> Option<(String, usize)> {
        self.import_rename_support().map(|support| {
            // Use project-relative paths for consistent matching with markdown links
            let old_name = old_path
                .strip_prefix(project_root)
                .unwrap_or(old_path)
                .to_string_lossy();
            let new_name = new_path
                .strip_prefix(project_root)
                .unwrap_or(new_path)
                .to_string_lossy();
            support.rewrite_imports_for_rename(content, &old_name, &new_name)
        })
    }
}

// ============================================================================
// Plugin Registry
// ============================================================================

/// A registry for managing language plugins
///
/// This will be used by the main server to register and look up plugins
/// based on file extensions.
pub struct PluginRegistry {
    plugins: Vec<std::sync::Arc<dyn LanguagePlugin>>,
}

impl PluginRegistry {
    /// Create a new empty plugin registry
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Register a new language plugin
    pub fn register(&mut self, plugin: std::sync::Arc<dyn LanguagePlugin>) {
        self.plugins.push(plugin);
    }

    /// Find a plugin that handles the given file extension
    pub fn find_by_extension(&self, extension: &str) -> Option<&dyn LanguagePlugin> {
        self.plugins
            .iter()
            .find(|p| p.handles_extension(extension))
            .map(|arc| arc.as_ref())
    }

    /// Get all registered plugins
    pub fn all(&self) -> &[std::sync::Arc<dyn LanguagePlugin>] {
        &self.plugins
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct MockPlugin {
        metadata: LanguageMetadata,
    }

    impl MockPlugin {
        fn new() -> Self {
            Self {
                metadata: LanguageMetadata {
                    name: "Mock",
                    extensions: &["mock"],
                    manifest_filename: "mock.toml",
                    source_dir: "src",
                    entry_point: "lib.mock",
                    module_separator: "::",
                },
            }
        }
    }

    #[async_trait]
    impl LanguagePlugin for MockPlugin {
        fn metadata(&self) -> &LanguageMetadata {
            &self.metadata
        }

        async fn parse(&self, _source: &str) -> PluginResult<ParsedSource> {
            Ok(ParsedSource {
                data: serde_json::json!({}),
                symbols: vec![],
            })
        }

        async fn analyze_manifest(&self, _path: &Path) -> PluginResult<ManifestData> {
            Ok(ManifestData {
                name: "test".to_string(),
                version: "1.0.0".to_string(),
                dependencies: vec![],
                dev_dependencies: vec![],
                raw_data: serde_json::json!({}),
            })
        }

        fn capabilities(&self) -> PluginCapabilities {
            PluginCapabilities::default()
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[test]
    fn test_plugin_registry() {
        let mut registry = PluginRegistry::new();
        registry.register(Arc::new(MockPlugin::new()));

        let plugin = registry.find_by_extension("mock");
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().metadata().name, "Mock");
    }
}
