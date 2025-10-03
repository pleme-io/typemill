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
