//! Core Plugin API for Language Support
//!
//! This crate defines the foundational traits and types for implementing
//! language-specific plugins in the TypeMill system. Each supported programming
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
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

// ============================================================================
// Module Declarations
// ============================================================================

pub mod capabilities;
pub mod import_support;
pub mod language;
pub mod lsp_installer;
pub mod metadata;
pub mod path_alias_resolver;
pub mod plugin_registry;
pub mod project_factory;
pub mod reference_detector;
pub mod server;
pub mod test_fixtures;
pub mod workspace_support;

// Re-exports
pub use capabilities::{
    ExtractParams, ImportAnalyzer, InlineParams, ManifestUpdater, ModuleDeclarationSupport,
    ModuleLocator, ModuleReferenceScanner, RefactoringProvider, TextEdit, WorkspaceEdit,
};
pub use import_support::{
    ImportAdvancedSupport, ImportMoveSupport, ImportMutationSupport, ImportParser,
    ImportRenameSupport,
};
pub use lsp_installer::LspInstaller;
pub use metadata::LanguageMetadata;
pub use path_alias_resolver::PathAliasResolver;
pub use plugin_registry::{iter_plugins, PluginDescriptor};
pub use project_factory::{
    CreatePackageConfig, CreatePackageResult, PackageInfo, PackageType, ProjectFactory, Template,
};
pub use reference_detector::ReferenceDetector;
// Note: mill_plugin! macro is automatically exported at crate root due to #[macro_export]
pub use server::PluginServer;
pub use test_fixtures::{
    ComplexityFixture, LanguageTestFixtures, RefactoringFixture, RefactoringOperation,
};
pub use workspace_support::{MoveManifestPlan, WorkspaceSupport};

// ============================================================================
// Error Types
// ============================================================================

/// Result type for plugin operations
pub type PluginResult<T> = Result<T, PluginApiError>;

/// Errors that can occur during plugin API operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum PluginApiError {
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

// Backward compatibility alias (will be removed in future version)
#[deprecated(since = "0.2.0", note = "Use PluginApiError instead")]
pub type PluginError = PluginApiError;

impl PluginApiError {
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

    /// Optional end location in source code
    pub end_location: Option<SourceLocation>,

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
    /// Supports path alias resolution (e.g., TypeScript path mappings)
    pub path_alias_resolver: bool,
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
            path_alias_resolver: false,
        }
    }

    /// Creates a new `PluginCapabilities` with all features enabled.
    pub const fn all() -> Self {
        Self {
            imports: true,
            workspace: true,
            project_factory: true,
            path_alias_resolver: true,
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

    /// Enable path alias resolver support
    pub const fn with_path_alias_resolver(mut self) -> Self {
        self.path_alias_resolver = true;
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
    ) -> PluginResult<mill_foundation::protocol::ImportGraph> {
        use chrono::Utc;
        // Default: return empty graph
        Ok(mill_foundation::protocol::ImportGraph {
            source_file: _file_path
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            imports: vec![],
            importers: vec![],
            metadata: mill_foundation::protocol::ImportGraphMetadata {
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

    /// Get module declaration support if available
    fn module_declaration_support(&self) -> Option<&dyn ModuleDeclarationSupport> {
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

    /// Get LSP installer if available
    fn lsp_installer(&self) -> Option<&dyn LspInstaller> {
        None
    }

    /// Get path alias resolver if available
    fn path_alias_resolver(&self) -> Option<&dyn PathAliasResolver> {
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
/// Lightweight plugin discovery registry for simple plugin lookups.
///
/// This is a minimal, dependency-free registry used in the plugin API layer
/// for basic plugin discovery by file extension. For runtime plugin management
/// with caching, priorities, and advanced features, see `RuntimePluginManager`
/// in the `mill-plugin-system` crate.
///
/// This will be used by the main server to register and look up plugins
/// based on file extensions.
pub struct PluginDiscovery {
    plugins: Vec<std::sync::Arc<dyn LanguagePlugin>>,
}

impl PluginDiscovery {
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

    /// Get the refactoring provider capability for a specific file
    ///
    /// This looks up the plugin by file extension, then returns its refactoring capability.
    /// This ensures the correct language plugin handles the file.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file being refactored
    ///
    /// # Returns
    ///
    /// The refactoring provider for the file's language, or None if not supported
    pub fn refactoring_provider_for_file(
        &self,
        file_path: &str,
    ) -> Option<&dyn RefactoringProvider> {
        // Extract file extension
        let extension = std::path::Path::new(file_path)
            .extension()
            .and_then(|ext| ext.to_str())?;

        // Find plugin by extension
        let plugin = self.find_by_extension(extension)?;

        // Get capability from that specific plugin
        plugin.refactoring_provider()
    }
}

impl Default for PluginDiscovery {
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
        let mut registry = PluginDiscovery::new();
        registry.register(Arc::new(MockPlugin::new()));

        let plugin = registry.find_by_extension("mock");
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().metadata().name, "Mock");
    }

    #[test]
    fn test_refactoring_provider_for_file_routes_by_extension() {
        use crate::RefactoringProvider;

        // Mock plugin for Rust files
        struct RustMockPlugin;

        #[async_trait]
        impl LanguagePlugin for RustMockPlugin {
            fn metadata(&self) -> &LanguageMetadata {
                static METADATA: LanguageMetadata = LanguageMetadata {
                    name: "rust-mock",
                    extensions: &["rs"],
                    manifest_filename: "Cargo.toml",
                    source_dir: "src",
                    entry_point: "lib.rs",
                    module_separator: "::",
                };
                &METADATA
            }

            fn capabilities(&self) -> PluginCapabilities {
                PluginCapabilities::none()
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            async fn parse(&self, _: &str) -> PluginResult<ParsedSource> {
                unimplemented!()
            }

            async fn analyze_manifest(&self, _: &Path) -> PluginResult<ManifestData> {
                unimplemented!()
            }

            fn refactoring_provider(&self) -> Option<&dyn RefactoringProvider> {
                Some(self)
            }
        }

        #[async_trait]
        impl RefactoringProvider for RustMockPlugin {}

        // Mock plugin for TypeScript files
        struct TypeScriptMockPlugin;

        #[async_trait]
        impl LanguagePlugin for TypeScriptMockPlugin {
            fn metadata(&self) -> &LanguageMetadata {
                static METADATA: LanguageMetadata = LanguageMetadata {
                    name: "typescript-mock",
                    extensions: &["ts", "tsx"],
                    manifest_filename: "package.json",
                    source_dir: "src",
                    entry_point: "index.ts",
                    module_separator: ".",
                };
                &METADATA
            }

            fn capabilities(&self) -> PluginCapabilities {
                PluginCapabilities::none()
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            async fn parse(&self, _: &str) -> PluginResult<ParsedSource> {
                unimplemented!()
            }

            async fn analyze_manifest(&self, _: &Path) -> PluginResult<ManifestData> {
                unimplemented!()
            }

            fn refactoring_provider(&self) -> Option<&dyn RefactoringProvider> {
                Some(self)
            }
        }

        #[async_trait]
        impl RefactoringProvider for TypeScriptMockPlugin {}

        // Register both plugins
        let mut registry = PluginDiscovery::new();
        registry.register(Arc::new(RustMockPlugin));
        registry.register(Arc::new(TypeScriptMockPlugin));

        // Test file-extension routing
        let rust_provider = registry.refactoring_provider_for_file("src/main.rs");
        assert!(
            rust_provider.is_some(),
            "Should find Rust provider for .rs file"
        );

        let ts_provider = registry.refactoring_provider_for_file("src/app.ts");
        assert!(
            ts_provider.is_some(),
            "Should find TypeScript provider for .ts file"
        );

        let tsx_provider = registry.refactoring_provider_for_file("src/Component.tsx");
        assert!(
            tsx_provider.is_some(),
            "Should find TypeScript provider for .tsx file"
        );

        // Test that non-existent extension returns None
        let unknown_provider = registry.refactoring_provider_for_file("file.unknown");
        assert!(
            unknown_provider.is_none(),
            "Should return None for unknown extension"
        );
    }

    #[test]
    fn test_capability_discovery_pattern() {
        use crate::{ManifestUpdater, ModuleLocator};

        // Plugin with multiple capabilities
        struct FullFeaturedPlugin;

        #[async_trait]
        impl LanguagePlugin for FullFeaturedPlugin {
            fn metadata(&self) -> &LanguageMetadata {
                static METADATA: LanguageMetadata = LanguageMetadata {
                    name: "full-featured",
                    extensions: &["full"],
                    manifest_filename: "manifest.toml",
                    source_dir: "src",
                    entry_point: "lib.full",
                    module_separator: "::",
                };
                &METADATA
            }

            fn capabilities(&self) -> PluginCapabilities {
                PluginCapabilities::none()
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            async fn parse(&self, _: &str) -> PluginResult<ParsedSource> {
                unimplemented!()
            }

            async fn analyze_manifest(&self, _: &Path) -> PluginResult<ManifestData> {
                unimplemented!()
            }

            fn manifest_updater(&self) -> Option<&dyn ManifestUpdater> {
                Some(self)
            }

            fn module_locator(&self) -> Option<&dyn ModuleLocator> {
                Some(self)
            }

            fn refactoring_provider(&self) -> Option<&dyn RefactoringProvider> {
                Some(self)
            }
        }

        #[async_trait]
        impl ManifestUpdater for FullFeaturedPlugin {
            async fn update_dependency(
                &self,
                _: &Path,
                _: &str,
                _: &str,
                _: Option<&str>,
            ) -> PluginResult<String> {
                Ok("updated".to_string())
            }

            fn generate_manifest(&self, _: &str, _: &[String]) -> String {
                "manifest".to_string()
            }
        }

        #[async_trait]
        impl ModuleLocator for FullFeaturedPlugin {
            async fn locate_module_files(
                &self,
                _: &Path,
                _: &str,
            ) -> PluginResult<Vec<std::path::PathBuf>> {
                Ok(vec![])
            }
        }

        #[async_trait]
        impl RefactoringProvider for FullFeaturedPlugin {}

        let mut registry = PluginDiscovery::new();
        registry.register(Arc::new(FullFeaturedPlugin));

        // Verify all capabilities are discoverable
        let plugin = registry.find_by_extension("full").unwrap();

        assert!(
            plugin.manifest_updater().is_some(),
            "Should have ManifestUpdater"
        );
        assert!(
            plugin.module_locator().is_some(),
            "Should have ModuleLocator"
        );
        assert!(
            plugin.refactoring_provider().is_some(),
            "Should have RefactoringProvider"
        );

        // Verify file-based lookup works
        let refactoring = registry.refactoring_provider_for_file("test.full");
        assert!(refactoring.is_some(), "Should find via file extension");
    }

    #[test]
    fn test_partial_capability_support() {
        use crate::ManifestUpdater;

        // Plugin with only some capabilities
        struct MinimalPlugin;

        #[async_trait]
        impl LanguagePlugin for MinimalPlugin {
            fn metadata(&self) -> &LanguageMetadata {
                static METADATA: LanguageMetadata = LanguageMetadata {
                    name: "minimal",
                    extensions: &["min"],
                    manifest_filename: "manifest.toml",
                    source_dir: "src",
                    entry_point: "lib.min",
                    module_separator: ".",
                };
                &METADATA
            }

            fn capabilities(&self) -> PluginCapabilities {
                PluginCapabilities::none()
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            async fn parse(&self, _: &str) -> PluginResult<ParsedSource> {
                unimplemented!()
            }

            async fn analyze_manifest(&self, _: &Path) -> PluginResult<ManifestData> {
                unimplemented!()
            }

            // Only has ManifestUpdater, not other capabilities
            fn manifest_updater(&self) -> Option<&dyn ManifestUpdater> {
                Some(self)
            }
        }

        #[async_trait]
        impl ManifestUpdater for MinimalPlugin {
            async fn update_dependency(
                &self,
                _: &Path,
                _: &str,
                _: &str,
                _: Option<&str>,
            ) -> PluginResult<String> {
                Ok("updated".to_string())
            }

            fn generate_manifest(&self, _: &str, _: &[String]) -> String {
                "minimal manifest".to_string()
            }
        }

        let mut registry = PluginDiscovery::new();
        registry.register(Arc::new(MinimalPlugin));

        let plugin = registry.find_by_extension("min").unwrap();

        // Has ManifestUpdater
        assert!(
            plugin.manifest_updater().is_some(),
            "Should have ManifestUpdater"
        );

        // Doesn't have other capabilities
        assert!(
            plugin.module_locator().is_none(),
            "Should NOT have ModuleLocator"
        );
        assert!(
            plugin.refactoring_provider().is_none(),
            "Should NOT have RefactoringProvider"
        );

        // File-based lookup returns None for missing capability
        let refactoring = registry.refactoring_provider_for_file("test.min");
        assert!(
            refactoring.is_none(),
            "Should return None when capability not present"
        );
    }
}

// ============================================================================
// Conversion to MillError
// ============================================================================

impl From<PluginApiError> for mill_foundation::errors::MillError {
    fn from(err: PluginApiError) -> Self {
        match err {
            PluginApiError::Parse { message, location } => {
                let mut error = mill_foundation::errors::MillError::Parse {
                    message,
                    file: None,
                    line: None,
                    column: None,
                };

                // Add location information if present
                if let mill_foundation::errors::MillError::Parse { line, column, .. } = &mut error {
                    if let Some(loc) = location {
                        *line = Some(loc.line);
                        *column = Some(loc.column);
                    }
                }

                error
            }
            PluginApiError::Manifest { message } => mill_foundation::errors::MillError::Manifest {
                message,
                file: None,
            },
            PluginApiError::NotSupported { operation } => {
                mill_foundation::errors::MillError::NotSupported {
                    operation,
                    reason: None,
                }
            }
            PluginApiError::InvalidInput { message } => {
                mill_foundation::errors::MillError::InvalidData {
                    message,
                    field: None,
                }
            }
            PluginApiError::Internal { message } => mill_foundation::errors::MillError::Internal {
                message,
                source: None,
            },
        }
    }
}

// ============================================================================
// Conversion from MillError (for `?` operator in plugins)
// ============================================================================

impl From<mill_foundation::errors::MillError> for PluginApiError {
    fn from(err: mill_foundation::errors::MillError) -> Self {
        use mill_foundation::errors::MillError;

        match err {
            // Direct mappings
            MillError::Parse { message, .. } => PluginApiError::Parse {
                message,
                location: None,
            },
            MillError::Manifest { message, .. } => PluginApiError::Manifest { message },
            MillError::NotSupported { operation, .. } => PluginApiError::NotSupported { operation },
            MillError::InvalidData { message, .. } => PluginApiError::InvalidInput { message },

            // All other variants map to Internal
            MillError::Config { message, .. } => PluginApiError::Internal { message },
            MillError::Bootstrap { message, .. } => PluginApiError::Internal { message },
            MillError::NotFound { resource, .. } => PluginApiError::Internal {
                message: format!("Resource not found: {}", resource),
            },
            MillError::AlreadyExists { resource, .. } => PluginApiError::Internal {
                message: format!("Resource already exists: {}", resource),
            },
            MillError::Io { message, .. } => PluginApiError::Internal { message },
            MillError::Validation { message, .. } => PluginApiError::InvalidInput { message },
            MillError::Json { message, .. } => PluginApiError::Parse {
                message,
                location: None,
            },
            MillError::Serialization { message, .. } => PluginApiError::Internal { message },
            MillError::PermissionDenied { operation, .. } => PluginApiError::Internal {
                message: format!("Permission denied: {}", operation),
            },
            MillError::Timeout { operation, .. } => PluginApiError::Internal {
                message: format!("Timeout during: {}", operation),
            },
            MillError::Lsp { message, .. } => PluginApiError::Internal { message },
            MillError::Ast { message, .. } => PluginApiError::Parse {
                message,
                location: None,
            },
            MillError::UnsupportedSyntax { feature, .. } => {
                PluginApiError::NotSupported { operation: feature }
            }
            MillError::PluginNotFound { name, .. } => PluginApiError::Internal {
                message: format!("Plugin not found: {}", name),
            },
            MillError::Plugin { message, .. } => PluginApiError::Internal { message },
            MillError::Connection { message, .. } => PluginApiError::Internal { message },
            MillError::Transport { message, .. } => PluginApiError::Internal { message },
            MillError::Auth { message, .. } => PluginApiError::Internal { message },
            MillError::InvalidRequest { message, .. } => PluginApiError::InvalidInput { message },
            MillError::Analysis { message, .. } => PluginApiError::Internal { message },
            MillError::Transformation { message, .. } => PluginApiError::Internal { message },
            MillError::Runtime { message, .. } => PluginApiError::Internal { message },
            MillError::Internal { message, .. } => PluginApiError::Internal { message },

            // Wildcard pattern for any future MillError variants (MillError is #[non_exhaustive])
            _ => PluginApiError::Internal {
                message: err.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod error_conversion_tests {
    use super::*;
    use mill_foundation::errors::MillError;

    #[test]
    fn test_parse_error_conversion() {
        let plugin_err = PluginApiError::parse("syntax error");
        let mill_err: MillError = plugin_err.into();

        match mill_err {
            MillError::Parse { message, .. } => {
                assert_eq!(message, "syntax error");
            }
            _ => panic!("Expected Parse error"),
        }
    }

    #[test]
    fn test_parse_error_with_location_conversion() {
        let plugin_err = PluginApiError::parse_at("syntax error", 10, 5);
        let mill_err: MillError = plugin_err.into();

        match mill_err {
            MillError::Parse {
                message,
                line,
                column,
                ..
            } => {
                assert_eq!(message, "syntax error");
                assert_eq!(line, Some(10));
                assert_eq!(column, Some(5));
            }
            _ => panic!("Expected Parse error with location"),
        }
    }

    #[test]
    fn test_manifest_error_conversion() {
        let plugin_err = PluginApiError::manifest("invalid manifest");
        let mill_err: MillError = plugin_err.into();

        match mill_err {
            MillError::Manifest { message, .. } => {
                assert_eq!(message, "invalid manifest");
            }
            _ => panic!("Expected Manifest error"),
        }
    }

    #[test]
    fn test_not_supported_conversion() {
        let plugin_err = PluginApiError::not_supported("refactor");
        let mill_err: MillError = plugin_err.into();

        match mill_err {
            MillError::NotSupported { operation, .. } => {
                assert_eq!(operation, "refactor");
            }
            _ => panic!("Expected NotSupported error"),
        }
    }

    #[test]
    fn test_invalid_input_conversion() {
        let plugin_err = PluginApiError::invalid_input("missing param");
        let mill_err: MillError = plugin_err.into();

        match mill_err {
            MillError::InvalidData { message, .. } => {
                assert_eq!(message, "missing param");
            }
            _ => panic!("Expected InvalidData error"),
        }
    }

    #[test]
    fn test_internal_error_conversion() {
        let plugin_err = PluginApiError::internal("unexpected state");
        let mill_err: MillError = plugin_err.into();

        match mill_err {
            MillError::Internal { message, .. } => {
                assert_eq!(message, "unexpected state");
            }
            _ => panic!("Expected Internal error"),
        }
    }

    // Tests for MillError -> PluginApiError conversion
    #[test]
    fn test_mill_parse_to_plugin() {
        let mill_err = MillError::parse("syntax error");
        let plugin_err: PluginApiError = mill_err.into();

        match plugin_err {
            PluginApiError::Parse { message, .. } => {
                assert_eq!(message, "syntax error");
            }
            _ => panic!("Expected Parse error"),
        }
    }

    #[test]
    fn test_mill_manifest_to_plugin() {
        let mill_err = MillError::manifest("invalid manifest");
        let plugin_err: PluginApiError = mill_err.into();

        match plugin_err {
            PluginApiError::Manifest { message } => {
                assert_eq!(message, "invalid manifest");
            }
            _ => panic!("Expected Manifest error"),
        }
    }

    #[test]
    fn test_mill_not_supported_to_plugin() {
        let mill_err = MillError::not_supported("operation");
        let plugin_err: PluginApiError = mill_err.into();

        match plugin_err {
            PluginApiError::NotSupported { operation } => {
                assert_eq!(operation, "operation");
            }
            _ => panic!("Expected NotSupported error"),
        }
    }

    #[test]
    fn test_mill_invalid_data_to_plugin() {
        let mill_err = MillError::invalid_data("bad data");
        let plugin_err: PluginApiError = mill_err.into();

        match plugin_err {
            PluginApiError::InvalidInput { message } => {
                assert_eq!(message, "bad data");
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_mill_validation_to_plugin() {
        let mill_err = MillError::validation("validation failed");
        let plugin_err: PluginApiError = mill_err.into();

        match plugin_err {
            PluginApiError::InvalidInput { message } => {
                assert_eq!(message, "validation failed");
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_mill_internal_to_plugin() {
        let mill_err = MillError::internal("internal error");
        let plugin_err: PluginApiError = mill_err.into();

        match plugin_err {
            PluginApiError::Internal { message } => {
                assert_eq!(message, "internal error");
            }
            _ => panic!("Expected Internal error"),
        }
    }

    #[test]
    fn test_mill_io_to_plugin() {
        let mill_err = MillError::io("io error");
        let plugin_err: PluginApiError = mill_err.into();

        match plugin_err {
            PluginApiError::Internal { message } => {
                assert_eq!(message, "io error");
            }
            _ => panic!("Expected Internal error"),
        }
    }

    #[test]
    fn test_mill_not_found_to_plugin() {
        let mill_err = MillError::not_found("resource");
        let plugin_err: PluginApiError = mill_err.into();

        match plugin_err {
            PluginApiError::Internal { message } => {
                assert!(message.contains("Resource not found"));
                assert!(message.contains("resource"));
            }
            _ => panic!("Expected Internal error"),
        }
    }
}
