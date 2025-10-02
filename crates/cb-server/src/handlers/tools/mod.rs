//! Tool handler modules organized by functional domain
//!
//! This module contains specialized tool handlers for different categories of MCP tools.
//! Each handler is responsible for a specific domain of functionality.

use crate::ServerResult;
use async_trait::async_trait;
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

/// Context provided to tool handlers
pub struct ToolHandlerContext {
    /// Application state containing all services
    pub app_state: Arc<AppState>,
    /// Plugin manager for LSP operations
    pub plugin_manager: Arc<PluginManager>,
    /// Direct LSP adapter for refactoring operations
    pub lsp_adapter: Arc<Mutex<Option<Arc<DirectLspAdapter>>>>,
}

/// Base trait for tool handlers
///
/// Each handler implements this trait to provide a set of related tools.
/// Handlers are registered in the dispatcher and invoked when their tools are called.
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Get the list of tools this handler supports
    fn supported_tools(&self) -> &[&'static str];

    /// Handle a tool call
    ///
    /// # Arguments
    ///
    /// * `tool_name` - The name of the tool being called
    /// * `params` - The parameters for the tool call
    /// * `context` - The execution context with access to services
    ///
    /// # Returns
    ///
    /// The result of the tool execution as a JSON value
    async fn handle(
        &self,
        tool_name: &str,
        params: Value,
        context: &ToolHandlerContext,
    ) -> ServerResult<Value>;

    /// Optional: Initialize handler (for setup, validation, etc.)
    async fn initialize(&self) -> ServerResult<()> {
        Ok(())
    }
}
