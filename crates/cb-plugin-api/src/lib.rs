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
//!
//! # Example
//!
//! ```rust,ignore
//! use cb_plugin_api::{LanguagePlugin, PluginResult, ParsedSource, ManifestData};
//! use std::path::Path;
//!
//! struct RustPlugin;
//!
//! impl LanguagePlugin for RustPlugin {
//!     fn name(&self) -> &'static str {
//!         "rust"
//!     }
//!
//!     fn file_extensions(&self) -> Vec<&'static str> {
//!         vec!["rs"]
//!     }
//!
//!     fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
//!         // Use syn crate to parse Rust code
//!         todo!("Implement Rust parsing")
//!     }
//!
//!     fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
//!         // Parse Cargo.toml
//!         todo!("Implement Cargo.toml parsing")
//!     }
//! }
//! ```

use async_trait::async_trait;
use cb_types::error::ApiError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

// ============================================================================
// Module Declarations
// ============================================================================

pub mod metadata;
pub mod import_support;
pub mod workspace_support;

// Re-exports
pub use cb_core::language::ProjectLanguage;
pub use metadata::LanguageMetadata;
pub use import_support::ImportSupport;
pub use workspace_support::WorkspaceSupport;

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
        use cb_types::error::error_codes::*;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

/// Parsed source code representation
///
/// This is a generic container for parsed AST data. Each language plugin
/// can store its language-specific AST in the `data` field as JSON.
#[derive(Debug, Clone)]
pub struct ParsedSource {
    /// Language-specific AST data (serialized as JSON for flexibility)
    pub data: Value,

    /// List of top-level symbols found in the source
    pub symbols: Vec<Symbol>,
}

/// A symbol in the source code (function, class, variable, etc.)
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Manifest file data (package.json, Cargo.toml, etc.)
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct Dependency {
    /// Dependency name
    pub name: String,

    /// Version specifier or path
    pub source: DependencySource,
}

/// Where a dependency comes from
#[derive(Debug, Clone)]
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

/// The core trait that every language intelligence plugin must implement.
///
/// This trait defines the contract for providing language-specific intelligence
/// including parsing, manifest handling, and code analysis.
///
/// This layer focuses on STATIC ANALYSIS and should be pure - no file I/O,
/// no refactoring operations. For refactoring operations, see LanguageAdapter
/// in cb-ast which composes this trait.
///
/// All methods are async to support both synchronous and asynchronous
/// implementations (e.g., spawning Python subprocess for Python parsing).
#[async_trait]
pub trait LanguageIntelligencePlugin: Send + Sync {
    /// Returns the official name of the language (e.g., "Rust", "Python", "TypeScript")
    fn name(&self) -> &'static str;

    /// Returns a list of file extensions this plugin handles (e.g., ["rs"] for Rust)
    fn file_extensions(&self) -> Vec<&'static str>;

    /// Parses source code into a generic AST representation
    ///
    /// # Arguments
    /// * `source` - The source code as a string
    ///
    /// # Returns
    /// A `ParsedSource` containing the AST and extracted symbols
    async fn parse(&self, source: &str) -> PluginResult<ParsedSource>;

    /// Analyzes a manifest file and extracts dependency information
    ///
    /// # Arguments
    /// * `path` - Path to the manifest file (Cargo.toml, package.json, etc.)
    ///
    /// # Returns
    /// A `ManifestData` containing parsed manifest information
    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData>;

    /// Lists all function names in the source code
    ///
    /// This is a convenience method that many tools need. The default
    /// implementation extracts functions from the parsed symbols.
    ///
    /// # Arguments
    /// * `source` - The source code as a string
    ///
    /// # Returns
    /// A list of function names
    async fn list_functions(&self, source: &str) -> PluginResult<Vec<String>> {
        let parsed = self.parse(source).await?;
        Ok(parsed
            .symbols
            .into_iter()
            .filter(|s| matches!(s.kind, SymbolKind::Function | SymbolKind::Method))
            .map(|s| s.name)
            .collect())
    }

    /// Updates a dependency in a manifest file
    ///
    /// # Arguments
    /// * `manifest_path` - Path to the manifest file
    /// * `old_name` - Current dependency name
    /// * `new_name` - New dependency name
    /// * `new_path` - Optional new path for the dependency
    ///
    /// # Returns
    /// The updated manifest content as a string
    async fn update_dependency(
        &self,
        _manifest_path: &Path,
        _old_name: &str,
        _new_name: &str,
        _new_path: Option<&str>,
    ) -> PluginResult<String> {
        // Default implementation returns not supported
        Err(PluginError::not_supported(format!(
            "Dependency updates not supported for {}",
            self.name()
        )))
    }

    /// Checks if this plugin can handle a given file extension
    ///
    /// # Arguments
    /// * `extension` - File extension (without the dot)
    ///
    /// # Returns
    /// `true` if this plugin handles the extension
    fn handles_extension(&self, extension: &str) -> bool {
        self.file_extensions().contains(&extension)
    }

    /// Checks if this plugin handles a specific manifest file
    ///
    /// # Arguments
    /// * `filename` - The manifest filename (e.g., "Cargo.toml", "package.json")
    ///
    /// # Returns
    /// `true` if this plugin can handle the manifest file
    fn handles_manifest(&self, _filename: &str) -> bool {
        // Default implementation - override in specific plugins
        false
    }

    // ========================================================================
    // Refactoring Support Methods (from LanguageAdapter)
    // ========================================================================

    /// Get the language this plugin supports
    fn language(&self) -> cb_core::language::ProjectLanguage {
        // Default implementation based on name
        use cb_core::language::ProjectLanguage;
        match self.name().to_lowercase().as_str() {
            "rust" => ProjectLanguage::Rust,
            "go" => ProjectLanguage::Go,
            "typescript" | "javascript" => ProjectLanguage::TypeScript,
            "python" => ProjectLanguage::Python,
            "java" => ProjectLanguage::Java,
            _ => ProjectLanguage::Unknown,
        }
    }

    /// Get the package manifest filename (e.g., "Cargo.toml", "package.json")
    fn manifest_filename(&self) -> &'static str {
        "" // Default: no manifest
    }

    /// Get the source directory name (e.g., "src" for Rust/TS, "" for Python)
    fn source_dir(&self) -> &'static str {
        "" // Default: no specific source dir
    }

    /// Get the entry point filename (e.g., "lib.rs", "index.ts", "__init__.py")
    fn entry_point(&self) -> &'static str {
        "" // Default: no entry point
    }

    /// Get the module path separator (e.g., "::" for Rust, "." for Python/TS)
    fn module_separator(&self) -> &'static str {
        "." // Default: dot separator
    }

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
        _package_path: &Path,
        _module_path: &str,
    ) -> PluginResult<Vec<std::path::PathBuf>> {
        Err(PluginError::not_supported(format!(
            "locate_module_files not supported for {}",
            self.name()
        )))
    }

    /// Parse imports/dependencies from a file
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to analyze
    ///
    /// # Returns
    ///
    /// Vector of import statements/paths found in the file
    async fn parse_imports(&self, _file_path: &Path) -> PluginResult<Vec<String>> {
        Err(PluginError::not_supported(format!(
            "parse_imports not supported for {}",
            self.name()
        )))
    }

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
    fn generate_manifest(&self, _package_name: &str, _dependencies: &[String]) -> String {
        String::new() // Default: empty manifest
    }

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
    fn rewrite_import(&self, _old_import: &str, new_package_name: &str) -> String {
        new_package_name.to_string() // Default: just return new name
    }

    /// Rewrite import statements in file content for a rename operation
    ///
    /// # Arguments
    ///
    /// * `content` - The file content to process
    /// * `old_path` - Original path before rename
    /// * `new_path` - New path after rename
    /// * `importing_file` - Path of the file being processed
    /// * `project_root` - Root directory of the project
    /// * `rename_info` - Optional language-specific rename context (JSON)
    ///
    /// # Returns
    ///
    /// Tuple of (updated_content, number_of_changes)
    fn rewrite_imports_for_rename(
        &self,
        content: &str,
        _old_path: &Path,
        _new_path: &Path,
        _importing_file: &Path,
        _project_root: &Path,
        _rename_info: Option<&serde_json::Value>,
    ) -> PluginResult<(String, usize)> {
        Ok((content.to_string(), 0)) // Default: no changes
    }

    /// Find all references to a specific module within file content
    ///
    /// This is more powerful than `parse_imports` as it finds not just declarations,
    /// but also qualified paths and other usages within the code based on the scope.
    ///
    /// # Arguments
    ///
    /// * `content` - The file content to scan
    /// * `module_to_find` - The module name/path to search for
    /// * `scope` - The scope of the search (top-level only, all statements, qualified paths, etc.)
    ///
    /// # Returns
    ///
    /// Vector of all found references with their locations
    fn find_module_references(
        &self,
        _content: &str,
        _module_to_find: &str,
        _scope: ScanScope,
    ) -> PluginResult<Vec<ModuleReference>> {
        Ok(Vec::new()) // Default: no references found
    }

    // ========================================================================
    // Package Extraction Support Methods
    // ========================================================================

    /// Add a path dependency to a package manifest file
    ///
    /// This is used during package extraction to add dependencies from the source
    /// package to the newly extracted package.
    ///
    /// # Arguments
    ///
    /// * `manifest_content` - Current manifest file content
    /// * `dep_name` - Name of the dependency to add
    /// * `dep_path` - Absolute path to the dependency
    /// * `source_path` - Absolute path to the source package directory
    ///
    /// # Returns
    ///
    /// Updated manifest content with dependency added
    ///
    /// # Example (Rust)
    ///
    /// ```rust,ignore
    /// // Adds: my-dep = { path = "../my-dep" } to Cargo.toml
    /// let updated = plugin.add_manifest_path_dependency(
    ///     cargo_toml_content,
    ///     "my-dep",
    ///     "/workspace/my-dep",
    ///     Path::new("/workspace/my-crate")
    /// ).await?;
    /// ```
    async fn add_manifest_path_dependency(
        &self,
        _manifest_content: &str,
        _dep_name: &str,
        _dep_path: &str,
        _source_path: &Path,
    ) -> PluginResult<String> {
        Err(PluginError::not_supported(format!(
            "add_manifest_path_dependency not supported for {}",
            self.name()
        )))
    }

    /// Add a member to a workspace manifest file
    ///
    /// This is used during package extraction to register the new package
    /// in a workspace configuration (Cargo.toml workspace, package.json workspaces, etc.)
    ///
    /// # Arguments
    ///
    /// * `workspace_content` - Current workspace manifest content
    /// * `new_member_path` - Absolute path to the new workspace member
    /// * `workspace_root` - Absolute path to the workspace root directory
    ///
    /// # Returns
    ///
    /// Updated workspace manifest content with member added
    ///
    /// # Example (Rust)
    ///
    /// ```rust,ignore
    /// // Adds member to [workspace.members] array in Cargo.toml
    /// let updated = plugin.add_workspace_member(
    ///     workspace_cargo_toml,
    ///     "/workspace/new-crate",
    ///     Path::new("/workspace")
    /// ).await?;
    /// ```
    async fn add_workspace_member(
        &self,
        _workspace_content: &str,
        _new_member_path: &str,
        _workspace_root: &Path,
    ) -> PluginResult<String> {
        Err(PluginError::not_supported(format!(
            "add_workspace_member not supported for {}",
            self.name()
        )))
    }

    /// Generate a new workspace manifest with initial members
    ///
    /// This eliminates the need for hard-coded workspace format generation in the core.
    /// Each language plugin can generate its own workspace format.
    ///
    /// # Arguments
    ///
    /// * `member_paths` - Absolute paths to initial workspace members
    /// * `workspace_root` - Absolute path to the workspace root directory
    ///
    /// # Returns
    ///
    /// New workspace manifest content
    ///
    /// # Example (Rust)
    ///
    /// ```rust,ignore
    /// let workspace = plugin.generate_workspace_manifest(
    ///     &["/workspace/crate1", "/workspace/crate2"],
    ///     Path::new("/workspace")
    /// ).await?;
    /// // Returns Cargo.toml with [workspace] section
    /// ```
    ///
    /// # Example (TypeScript)
    ///
    /// ```rust,ignore
    /// let workspace = plugin.generate_workspace_manifest(
    ///     &["/workspace/pkg1", "/workspace/pkg2"],
    ///     Path::new("/workspace")
    /// ).await?;
    /// // Returns package.json with "workspaces" field
    /// ```
    async fn generate_workspace_manifest(
        &self,
        _member_paths: &[&str],
        _workspace_root: &Path,
    ) -> PluginResult<String> {
        Err(PluginError::not_supported(format!(
            "generate_workspace_manifest not supported for {}",
            self.name()
        )))
    }

    /// Check if manifest content represents a workspace configuration
    ///
    /// This eliminates the need for hard-coded workspace marker detection in the core.
    /// Each language plugin knows its own workspace format.
    ///
    /// # Arguments
    ///
    /// * `manifest_content` - Manifest file content to check
    ///
    /// # Returns
    ///
    /// true if this is a workspace manifest, false otherwise
    ///
    /// # Example (Rust)
    ///
    /// ```rust,ignore
    /// // Rust: checks for [workspace] section
    /// if plugin.is_workspace_manifest(&cargo_toml).await? {
    ///     // Handle as workspace
    /// }
    /// ```
    ///
    /// # Example (TypeScript)
    ///
    /// ```rust,ignore
    /// // TypeScript: checks for "workspaces" field in package.json
    /// if plugin.is_workspace_manifest(&package_json).await? {
    ///     // Handle as workspace
    /// }
    /// ```
    async fn is_workspace_manifest(&self, _manifest_content: &str) -> PluginResult<bool> {
        Ok(false) // Default: not a workspace
    }

    /// Remove a module declaration from source code
    ///
    /// This is used during package extraction to remove the module declaration
    /// from the parent file after the module has been extracted to a separate package.
    ///
    /// # Arguments
    ///
    /// * `source` - Source code content
    /// * `module_name` - Name of the module to remove
    ///
    /// # Returns
    ///
    /// Updated source content with module declaration removed
    ///
    /// # Example (Rust)
    ///
    /// ```rust,ignore
    /// // Removes: pub mod my_module; or mod my_module;
    /// let updated = plugin.remove_module_declaration(
    ///     lib_rs_content,
    ///     "my_module"
    /// ).await?;
    /// ```
    ///
    /// # Example (TypeScript)
    ///
    /// ```rust,ignore
    /// // Removes: export * from './my_module';
    /// let updated = plugin.remove_module_declaration(
    ///     index_ts_content,
    ///     "my_module"
    /// ).await?;
    /// ```
    async fn remove_module_declaration(
        &self,
        _source: &str,
        _module_name: &str,
    ) -> PluginResult<String> {
        Err(PluginError::not_supported(format!(
            "remove_module_declaration not supported for {}",
            self.name()
        )))
    }

    /// Find all source files in a directory for this language
    ///
    /// This is used during package extraction to locate all files that need
    /// import updates after extraction.
    ///
    /// **Default implementation provided** - recursively finds files by extension.
    /// Override only if you need custom logic.
    ///
    /// # Arguments
    ///
    /// * `dir` - Directory to search
    ///
    /// # Returns
    ///
    /// Vector of file paths with this language's file extensions
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // For Rust: finds all .rs files (excluding target/ and hidden dirs)
    /// let files = plugin.find_source_files(Path::new("src")).await?;
    /// ```
    async fn find_source_files(&self, dir: &Path) -> PluginResult<Vec<std::path::PathBuf>> {
        let mut result_files = Vec::new();

        if !dir.exists() || !dir.is_dir() {
            return Ok(result_files);
        }

        let entries = std::fs::read_dir(dir).map_err(|e| {
            PluginError::internal(format!("Failed to read directory {}: {}", dir.display(), e))
        })?;

        for entry_result in entries {
            let entry = entry_result.map_err(|e| {
                PluginError::internal(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();

            if path.is_dir() {
                // Skip common build/cache directories
                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                    if dir_name == "target"
                        || dir_name == "node_modules"
                        || dir_name == "dist"
                        || dir_name == "build"
                        || dir_name == "__pycache__"
                        || dir_name == ".git"
                        || dir_name.starts_with('.')
                    {
                        continue;
                    }
                }

                // Recursively search subdirectories
                let mut sub_files = Box::pin(self.find_source_files(&path)).await?;
                result_files.append(&mut sub_files);
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if self.handles_extension(ext) {
                    result_files.push(path);
                }
            }
        }

        Ok(result_files)
    }
}

// ============================================================================
// Plugin Registry
// ============================================================================

/// A registry for managing language intelligence plugins
///
/// This will be used by the main server to register and look up plugins
/// based on file extensions.
pub struct PluginRegistry {
    plugins: Vec<std::sync::Arc<dyn LanguageIntelligencePlugin>>,
}

impl PluginRegistry {
    /// Create a new empty plugin registry
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Register a new language intelligence plugin
    pub fn register(&mut self, plugin: std::sync::Arc<dyn LanguageIntelligencePlugin>) {
        self.plugins.push(plugin);
    }

    /// Find a plugin that handles the given file extension
    pub fn find_by_extension(&self, extension: &str) -> Option<&dyn LanguageIntelligencePlugin> {
        self.plugins
            .iter()
            .find(|p| p.handles_extension(extension))
            .map(|arc| arc.as_ref())
    }

    /// Get all registered plugins
    pub fn all(&self) -> &[std::sync::Arc<dyn LanguageIntelligencePlugin>] {
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

    struct MockPlugin;

    #[async_trait]
    impl LanguageIntelligencePlugin for MockPlugin {
        fn name(&self) -> &'static str {
            "mock"
        }

        fn file_extensions(&self) -> Vec<&'static str> {
            vec!["mock"]
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
    }

    #[test]
    fn test_plugin_registry() {
        let mut registry = PluginRegistry::new();
        registry.register(Arc::new(MockPlugin));

        assert!(registry.find_by_extension("mock").is_some());
        assert!(registry.find_by_extension("unknown").is_none());
    }

    #[test]
    fn test_plugin_handles_extension() {
        let plugin = MockPlugin;
        assert!(plugin.handles_extension("mock"));
        assert!(!plugin.handles_extension("rs"));
    }
}
