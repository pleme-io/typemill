//! Core API types and traits for the mill system
//!
//! This crate provides foundational types, traits, and error handling
//! that are shared across all workspace crates. It has no dependencies
//! on other cb-* crates to prevent circular dependencies.

pub mod analysis_result;
pub mod error;
pub mod plugin_protocol;

pub use analysis_result::*;
pub use error::{ApiError, ApiResult};

pub use crate::planning::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Generic message type for protocol communication
/// This will be mapped to specific protocol types (MCP, LSP) in other crates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Option<String>,
    pub method: String,
    pub params: serde_json::Value,
}

/// Import graph representation - concrete implementation from mill-ast
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportGraph {
    /// Source file path
    pub source_file: String,
    /// Direct imports from this file
    pub imports: Vec<ImportInfo>,
    /// Files that import this file
    pub importers: Vec<String>,
    /// Dependency graph metadata
    pub metadata: ImportGraphMetadata,
}

/// Information about a single import
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportInfo {
    /// The imported module path/name
    pub module_path: String,
    /// Import type (ES module, CommonJS, etc.)
    pub import_type: ImportType,
    /// Named imports
    pub named_imports: Vec<NamedImport>,
    /// Default import name (if any)
    pub default_import: Option<String>,
    /// Namespace import name (if any)
    pub namespace_import: Option<String>,
    /// Whether this is a type-only import
    pub type_only: bool,
    /// Source location in the file
    pub location: SourceLocation,
}

/// Named import information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NamedImport {
    /// Original name in the module
    pub name: String,
    /// Local alias (if renamed)
    pub alias: Option<String>,
    /// Whether this is a type-only import
    pub type_only: bool,
}

/// Import/export type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ImportType {
    /// ES module import (import/export)
    EsModule,
    /// CommonJS require
    CommonJs,
    /// Dynamic import()
    Dynamic,
    /// AMD require
    Amd,
    /// TypeScript import type
    TypeOnly,
    /// Python import statement
    PythonImport,
    /// Python from...import statement
    PythonFromImport,
    /// C/C++ #include directive
    CInclude,
    /// C# using directive / Java import / Go import (namespace-based imports)
    Namespace,
}

/// Source location information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SourceLocation {
    /// Start line (0-based)
    pub start_line: u32,
    /// Start column (0-based)
    pub start_column: u32,
    /// End line (0-based)
    pub end_line: u32,
    /// End column (0-based)
    pub end_column: u32,
}

/// Import graph metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportGraphMetadata {
    /// File extension/language
    pub language: String,
    /// Parsing timestamp
    pub parsed_at: chrono::DateTime<chrono::Utc>,
    /// Parser version
    pub parser_version: String,
    /// Circular dependencies detected
    pub circular_dependencies: Vec<Vec<String>>,
    /// External dependencies (not in project)
    pub external_dependencies: Vec<String>,
}


/// Cache statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of cache invalidations
    pub invalidations: u64,
    /// Number of cache inserts
    pub inserts: u64,
    /// Current number of cached entries
    pub current_entries: usize,
}

impl CacheStats {
    /// Calculate hit ratio as a percentage
    pub fn hit_ratio(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }

    /// Check if cache is performing well (arbitrary threshold of 70% hit ratio)
    pub fn is_performing_well(&self) -> bool {
        self.hit_ratio() >= 70.0 && (self.hits + self.misses) >= 10
    }
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cache Stats: {} entries, {}/{} hits/total ({:.1}% hit ratio), {} invalidations, {} inserts",
            self.current_entries,
            self.hits,
            self.hits + self.misses,
            self.hit_ratio(),
            self.invalidations,
            self.inserts
        )
    }
}

// IntentSpec comes from cb-core::model::IntentSpec

/// AST service interface
#[async_trait]
pub trait AstService: Send + Sync {
    /// Build import graph for a file
    async fn build_import_graph(&self, file: &Path) -> ApiResult<ImportGraph>;

    /// Get cache statistics for monitoring
    async fn cache_stats(&self) -> CacheStats;
}

/// LSP service interface
#[async_trait]
pub trait LspService: Send + Sync {
    /// Send an LSP request and get response
    async fn request(&self, message: Message) -> ApiResult<Message>;

    /// Check if LSP server is available for file extension
    async fn is_available(&self, extension: &str) -> bool;

    /// Restart LSP server for given extensions
    async fn restart_servers(&self, extensions: Option<Vec<String>>) -> ApiResult<()>;

    /// Notify LSP server that a file has been opened
    async fn notify_file_opened(&self, file_path: &Path) -> ApiResult<()>;
}

/// Message dispatcher interface for transport layer
/// Note: For now using serde_json::Value for maximum flexibility,
/// this can be refined to more specific types later
#[async_trait]
pub trait MessageDispatcher: Send + Sync {
    /// Dispatch a message and return response
    async fn dispatch(&self, message: serde_json::Value) -> ApiResult<serde_json::Value>;
}

/// Parameters for rename_directory operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameDirectoryParams {
    /// Old directory path
    pub old_path: std::path::PathBuf,
    /// New directory path
    pub new_path: std::path::PathBuf,
    /// Whether to perform a dry run
    #[serde(default)]
    pub dry_run: bool,
    /// Consolidate source package into target directory
    #[serde(default)]
    pub consolidate: bool,
    /// Update mode for import statements
    #[serde(default)]
    pub update_mode: Option<String>,
    /// Include detailed file list in dry-run output (defaults to false for cleaner output)
    #[serde(default)]
    pub details: bool,
}
