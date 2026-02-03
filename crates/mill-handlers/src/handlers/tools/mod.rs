//! Tool handler modules organized by functional domain
//!
//! This module contains specialized tool handlers for different categories of MCP tools.
//! Each handler is responsible for a specific domain of functionality.

// Re-export core trait from mill-handler-api
pub use mill_handler_api::ToolHandler;

use super::lsp_adapter::DirectLspAdapter;
use super::plugin_dispatcher::AppState;
use mill_plugin_system::PluginManager;
use std::sync::Arc;
use tokio::sync::Mutex;

// Tool handler modules
pub mod advanced;
pub mod cross_file_references;
pub mod editing;
pub mod file_ops;
pub mod internal_intelligence;
pub mod internal_workspace;
pub mod lifecycle;
pub mod plan;
pub mod workspace;
pub mod workspace_create;
pub mod workspace_extract;

#[cfg(test)]
pub mod perf_benchmark;
#[cfg(test)]
pub mod perf_benchmark_extract;

// Re-export handlers
pub use advanced::AdvancedToolsHandler;
pub use editing::EditingToolsHandler;
pub use file_ops::FileToolsHandler;
pub use internal_intelligence::InternalIntelligenceHandler;
pub use internal_workspace::InternalWorkspaceHandler;
pub use lifecycle::LifecycleHandler;
pub use plan::PlanToolsHandler;
pub use workspace_create::WorkspaceCreateService;
pub use workspace_extract::WorkspaceExtractService;

// Re-export dispatch helpers
pub use dispatch::dispatch_to_language_plugin;

/// Dispatch helpers for language plugin operations
mod dispatch {

    use mill_foundation::errors::{MillError, MillResult};
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
    /// ```text
    /// let result = dispatch_to_language_plugin(
    ///     &context,
    ///     "src/main.rs",
    ///     |plugin, content| async move {
    ///         plugin.parse(content).await
    ///     }
    /// ).await?;
    /// ```
    pub async fn dispatch_to_language_plugin<F, Fut, T>(
        context: &mill_handler_api::ToolHandlerContext,
        file_path: &str,
        operation: F,
    ) -> MillResult<T>
    where
        F: FnOnce(&dyn mill_plugin_api::LanguagePlugin, String) -> Fut,
        Fut: std::future::Future<Output = mill_plugin_api::PluginResult<T>>,
    {
        // Get file extension
        let path = Path::new(file_path);
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| {
                MillError::invalid_request(format!("File has no extension: {}", file_path))
            })?;

        // Read file content using FileService (respects caching, locking, virtual workspaces)
        let content = context
            .app_state
            .file_service
            .read_file(path)
            .await
            .map_err(|e| MillError::internal(format!("Failed to read file: {}", e)))?;

        // Look up language plugin by extension
        let plugin = context
            .app_state
            .language_plugins
            .get_plugin(extension)
            .ok_or_else(|| {
                MillError::not_supported(format!(
                    "No language plugin found for extension: {}",
                    extension
                ))
            })?;

        // Execute the operation with the plugin
        operation(plugin, content).await.map_err(|e| {
            // Convert PluginApiError to MillError
            match e {
                mill_plugin_api::PluginApiError::Parse { message, .. } => MillError::parse(message),
                mill_plugin_api::PluginApiError::Manifest { message } => MillError::parse(message),
                mill_plugin_api::PluginApiError::NotSupported { operation } => {
                    MillError::not_supported(operation)
                }
                mill_plugin_api::PluginApiError::InvalidInput { message } => {
                    MillError::invalid_request(message)
                }
                mill_plugin_api::PluginApiError::Internal { message } => {
                    MillError::internal(message)
                }
            }
        })
    }
}

/// Context provided to tool handlers in mill-handlers
///
/// This is the concrete context used within mill-handlers, which has access
/// to the full AppState with all services. External handlers use the trait-based
/// context from mill-handler-api.
pub struct ToolHandlerContext {
    /// The ID of the user making the request, for multi-tenancy.
    pub user_id: Option<String>,
    /// Application state containing all services
    pub app_state: Arc<AppState>,
    /// Plugin manager for LSP operations
    pub plugin_manager: Arc<PluginManager>,
    /// Direct LSP adapter for refactoring operations
    pub lsp_adapter: Arc<Mutex<Option<Arc<DirectLspAdapter>>>>,
}

// Type alias for convenience
pub type ToolContext = ToolHandlerContext;

impl ToolHandlerContext {
    /// Convert to mill_handler_api::ToolHandlerContext for handler compatibility
    ///
    /// This creates a trait-based context that:
    /// - Works with handler implementations that only depend on mill-handler-api
    /// - Provides access to the full concrete AppState via the extensions field
    /// - Allows handlers to downcast to access concrete types when needed
    pub async fn to_api_context(&self) -> mill_handler_api::ToolHandlerContext {
        use super::plugin_dispatcher::{FileServiceWrapper, LanguagePluginRegistryWrapper};

        mill_handler_api::ToolHandlerContext {
            user_id: self.user_id.clone(),
            app_state: Arc::new(mill_handler_api::AppState {
                file_service: Arc::new(FileServiceWrapper(self.app_state.file_service.clone()))
                    as Arc<dyn mill_handler_api::FileService>,
                language_plugins: Arc::new(LanguagePluginRegistryWrapper(
                    self.app_state.language_plugins.clone(),
                ))
                    as Arc<dyn mill_handler_api::LanguagePluginRegistry>,
                project_root: self.app_state.project_root.clone(),
                extensions: Some(self.app_state.clone() as Arc<dyn std::any::Any + Send + Sync>),
            }),
            plugin_manager: self.plugin_manager.clone(),
            lsp_adapter: Arc::new(Mutex::new(
                // Convert Option<Arc<DirectLspAdapter>> to Option<Arc<dyn LspAdapter>>
                // DirectLspAdapter now implements LspAdapter trait directly
                self.lsp_adapter
                    .lock()
                    .await
                    .as_ref()
                    .map(|adapter| adapter.clone() as Arc<dyn mill_handler_api::LspAdapter>),
            )),
        }
    }
}

/// Helper functions to extract concrete types from trait-based AppState
pub mod extensions {
    use super::AppState;
    use mill_foundation::errors::{MillError, MillResult};
    use std::sync::Arc;

    /// Extract the concrete AppState from the extensions field
    pub fn get_concrete_app_state(
        api_state: &mill_handler_api::AppState,
    ) -> MillResult<Arc<AppState>> {
        api_state
            .extensions
            .as_ref()
            .ok_or_else(|| MillError::internal("AppState extensions not set"))?
            .clone()
            .downcast::<AppState>()
            .map_err(|_| MillError::internal("Failed to downcast AppState from extensions"))
    }
}
