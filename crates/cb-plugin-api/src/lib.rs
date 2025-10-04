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
use serde_json::Value;
use std::path::Path;

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
// Core Plugin Trait
// ============================================================================

/// The core trait that every language plugin must implement.
///
/// This trait defines the contract for providing language-specific intelligence
/// including parsing, manifest handling, and code analysis.
///
/// All methods are async to support both synchronous and asynchronous
/// implementations (e.g., spawning Python subprocess for Python parsing).
#[async_trait]
pub trait LanguagePlugin: Send + Sync {
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
            .filter(|s| {
                matches!(
                    s.kind,
                    SymbolKind::Function | SymbolKind::Method
                )
            })
            .map(|s| s.name)
            .collect())
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
}

// ============================================================================
// Plugin Registry
// ============================================================================

/// A registry for managing language plugins
///
/// This will be used by the main server to register and look up plugins
/// based on file extensions.
pub struct PluginRegistry {
    plugins: Vec<Box<dyn LanguagePlugin>>,
}

impl PluginRegistry {
    /// Create a new empty plugin registry
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Register a new language plugin
    pub fn register(&mut self, plugin: Box<dyn LanguagePlugin>) {
        self.plugins.push(plugin);
    }

    /// Find a plugin that handles the given file extension
    pub fn find_by_extension(&self, extension: &str) -> Option<&dyn LanguagePlugin> {
        self.plugins
            .iter()
            .find(|p| p.handles_extension(extension))
            .map(|b| b.as_ref())
    }

    /// Get all registered plugins
    pub fn all(&self) -> &[Box<dyn LanguagePlugin>] {
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

    struct MockPlugin;

    #[async_trait]
    impl LanguagePlugin for MockPlugin {
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
        registry.register(Box::new(MockPlugin));

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
