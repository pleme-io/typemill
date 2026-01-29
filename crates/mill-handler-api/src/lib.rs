//! Handler API for TypeMill
//!
//! This crate defines the core traits and types for tool handlers, separating
//! handler contracts from server implementations.

use async_trait::async_trait;
use mill_foundation::core::dry_run::DryRunnable;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError, MillResult as ServerResult};
use mill_foundation::protocol::EditPlan;
use mill_lsp::lsp_system::client::LspClient;
use mill_plugin_api::LanguagePlugin;
use mill_plugin_system::PluginManager;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Trait for file service operations
#[async_trait]
pub trait FileService: Send + Sync {
    /// Read the contents of a file
    async fn read_file(&self, path: &Path) -> Result<String, MillError>;

    /// List files in a directory (optionally recursive)
    async fn list_files(&self, path: &Path, recursive: bool) -> Result<Vec<String>, MillError>;

    /// Write content to a file
    ///
    /// Returns the result directly when dry_run is false (actual write).
    /// When dry_run is true, returns preview information wrapped in DryRunnable.
    async fn write_file(
        &self,
        path: &Path,
        content: &str,
        dry_run: bool,
    ) -> Result<DryRunnable<Value>, MillError>;

    /// Delete a file
    async fn delete_file(
        &self,
        path: &Path,
        force: bool,
        dry_run: bool,
    ) -> Result<DryRunnable<Value>, MillError>;

    /// Create a new file with content
    async fn create_file(
        &self,
        path: &Path,
        content: Option<&str>,
        overwrite: bool,
        dry_run: bool,
    ) -> Result<DryRunnable<Value>, MillError>;

    /// Rename a file and update all imports
    async fn rename_file_with_imports(
        &self,
        old_path: &Path,
        new_path: &Path,
        dry_run: bool,
        scan_scope: Option<mill_plugin_api::ScanScope>,
    ) -> Result<DryRunnable<Value>, MillError>;

    /// Rename a directory and update all imports
    async fn rename_directory_with_imports(
        &self,
        old_path: &Path,
        new_path: &Path,
        dry_run: bool,
        scan_scope: Option<mill_plugin_api::ScanScope>,
        details: bool,
    ) -> Result<DryRunnable<Value>, MillError>;

    /// List files matching a pattern
    async fn list_files_with_pattern(
        &self,
        path: &Path,
        recursive: bool,
        pattern: Option<&str>,
    ) -> Result<Vec<String>, MillError>;

    /// Convert a path to an absolute path and verify it's within project root
    fn to_absolute_path_checked(&self, path: &Path) -> Result<PathBuf, MillError>;

    /// Apply an edit plan to the filesystem
    async fn apply_edit_plan(
        &self,
        plan: &EditPlan,
    ) -> Result<mill_foundation::protocol::EditPlanResult, MillError>;
}

/// Trait for language plugin registry
pub trait LanguagePluginRegistry: Send + Sync {
    /// Get a language plugin by file extension
    fn get_plugin(&self, extension: &str) -> Option<&dyn LanguagePlugin>;

    /// Get all supported file extensions
    fn supported_extensions(&self) -> Vec<String>;

    /// Get a plugin that can handle the given manifest file (e.g., Cargo.toml, package.json)
    fn get_plugin_for_manifest(&self, file_path: &Path) -> Option<&dyn LanguagePlugin>;

    /// Access to the inner registry for builders (used by dependency analysis)
    fn inner(&self) -> &dyn std::any::Any;
}

/// Trait for LSP adapter
#[async_trait]
pub trait LspAdapter: Send + Sync {
    /// Get or create an LSP client for the given file extension
    async fn get_or_create_client(&self, file_extension: &str)
        -> Result<Arc<LspClient>, MillError>;

    /// Access to the inner adapter for downcasting
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Application state containing all services
pub struct AppState {
    /// File service for file operations
    pub file_service: Arc<dyn FileService>,
    /// Language plugin registry
    pub language_plugins: Arc<dyn LanguagePluginRegistry>,
    /// Project root directory
    pub project_root: PathBuf,
    /// Extension point for concrete implementations to store additional state
    /// Handlers can downcast this to access concrete types
    pub extensions: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

/// Context provided to tool handlers
pub struct ToolHandlerContext {
    /// The ID of the user making the request, for multi-tenancy.
    pub user_id: Option<String>,
    /// Application state containing all services
    pub app_state: Arc<AppState>,
    /// Plugin manager for LSP operations
    pub plugin_manager: Arc<PluginManager>,
    /// Direct LSP adapter for refactoring operations
    pub lsp_adapter: Arc<Mutex<Option<Arc<dyn LspAdapter>>>>,
}

// Type alias for convenience
pub type ToolContext = ToolHandlerContext;

/// Unified trait for all tool handlers
///
/// This is the single, canonical trait that all handlers must implement.
/// It provides direct access to the shared context and handles tool calls uniformly.
///
/// Only the Magnificent Seven tools implement this trait:
/// - inspect_code, search_code, rename_all, relocate, prune, refactor, workspace
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Returns a slice of tool names this handler is responsible for.
    fn tool_names(&self) -> &[&str];

    /// Handles an incoming tool call.
    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value>;
}
