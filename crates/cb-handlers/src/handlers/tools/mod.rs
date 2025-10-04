//! Tool handler modules organized by functional domain
//!
//! This module contains specialized tool handlers for different categories of MCP tools.
//! Each handler is responsible for a specific domain of functionality.

use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::ApiResult as ServerResult;
use serde_json::Value;

use super::lsp_adapter::DirectLspAdapter;
use super::plugin_dispatcher::AppState;
use cb_plugins::PluginManager;
use std::sync::Arc;
use tokio::sync::Mutex;

// Tool handler modules
pub mod advanced;
pub mod editing;
pub mod file_ops;
pub mod lifecycle;
pub mod navigation;
pub mod system;
pub mod workspace;

// Re-export handlers
pub use advanced::AdvancedHandler;
pub use editing::EditingHandler;
pub use file_ops::FileOpsHandler;
pub use lifecycle::LifecycleHandler;
pub use navigation::NavigationHandler;
pub use system::SystemHandler;
pub use workspace::WorkspaceHandler;

// Re-export dispatch helpers
pub use dispatch::dispatch_to_language_plugin;

/// Dispatch helpers for language plugin operations
mod dispatch {
    use super::ToolHandlerContext;
    use cb_plugin_api::LanguagePlugin;
    use cb_protocol::{ApiError, ApiResult};
    use std::path::Path;

    /// Dispatch a file operation to the appropriate language plugin based on file extension
    ///
    /// This helper:
    /// - Reads file content using FileService (respects caching, locking, virtual workspaces)
    /// - Looks up the appropriate language plugin by file extension
    /// - Executes the provided operation with the plugin and file content
    /// - Returns proper errors for unsupported languages
    ///
    /// # Arguments
    ///
    /// * `context` - Tool handler context with access to AppState services
    /// * `file_path` - Path to the file to operate on
    /// * `operation` - Async closure that performs the plugin operation
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let result = dispatch_to_language_plugin(
    ///     &context,
    ///     "src/main.rs",
    ///     |plugin, content| async move {
    ///         plugin.parse(content).await
    ///     }
    /// ).await?;
    /// ```
    pub async fn dispatch_to_language_plugin<F, Fut, T>(
        context: &ToolHandlerContext,
        file_path: &str,
        operation: F,
    ) -> ApiResult<T>
    where
        F: FnOnce(&dyn LanguagePlugin, String) -> Fut,
        Fut: std::future::Future<Output = cb_plugin_api::PluginResult<T>>,
    {
        // Get file extension
        let path = Path::new(file_path);
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| {
                ApiError::InvalidRequest(format!("File has no extension: {}", file_path))
            })?;

        // Read file content using FileService (respects caching, locking, virtual workspaces)
        let content = context
            .app_state
            .file_service
            .read_file(path)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to read file: {}", e)))?;

        // Look up language plugin by extension
        let plugin = context
            .app_state
            .language_plugins
            .get_plugin(extension)
            .ok_or_else(|| {
                ApiError::Unsupported(format!(
                    "No language plugin found for extension: {}",
                    extension
                ))
            })?;

        // Execute the operation with the plugin
        operation(plugin, content)
            .await
            .map_err(|e| {
                // Convert PluginError to ApiError
                match e {
                    cb_plugin_api::PluginError::Parse { message, .. } => ApiError::Parse { message },
                    cb_plugin_api::PluginError::Manifest { message } => ApiError::Parse { message },
                    cb_plugin_api::PluginError::NotSupported { operation } => ApiError::Unsupported(operation),
                    cb_plugin_api::PluginError::InvalidInput { message } => ApiError::InvalidRequest(message),
                    cb_plugin_api::PluginError::Internal { message } => ApiError::Internal(message),
                }
            })
    }
}

/// Context provided to tool handlers
pub struct ToolHandlerContext {
    /// Application state containing all services
    pub app_state: Arc<AppState>,
    /// Plugin manager for LSP operations
    pub plugin_manager: Arc<PluginManager>,
    /// Direct LSP adapter for refactoring operations
    pub lsp_adapter: Arc<Mutex<Option<Arc<DirectLspAdapter>>>>,
}

// Compatibility alias for legacy handlers that still reference the old name
pub type ToolContext = ToolHandlerContext;

/// Unified trait for all tool handlers
///
/// This is the single, canonical trait that all handlers must implement.
/// It provides direct access to the shared context and handles tool calls uniformly.
///
/// # Example
///
/// ```rust,ignore
/// use cb_server::handlers::tools::{ToolHandler, ToolHandlerContext};
/// use cb_core::model::mcp::ToolCall;
/// use async_trait::async_trait;
///
/// struct MyHandler;
///
/// #[async_trait]
/// impl ToolHandler for MyHandler {
///     fn tool_names(&self) -> &[&str] {
///         &["my_tool"]
///     }
///
///     async fn handle_tool_call(
///         &self,
///         context: &ToolHandlerContext,
///         tool_call: &ToolCall,
///     ) -> ServerResult<Value> {
///         // Implementation
///         Ok(json!({"success": true}))
///     }
/// }
/// ```
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Returns a slice of tool names this handler is responsible for.
    ///
    /// Tool names must be unique across all handlers in the system.
    fn tool_names(&self) -> &[&str];

    /// Handles an incoming tool call.
    ///
    /// # Arguments
    ///
    /// * `context` - The execution context providing access to all services
    /// * `tool_call` - The MCP tool call containing name and arguments
    ///
    /// # Returns
    ///
    /// The result of the tool execution as a JSON value, or a ServerError on failure.
    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value>;
}
