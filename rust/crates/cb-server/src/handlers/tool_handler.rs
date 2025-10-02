//! Tool handler trait for non-LSP operations
//!
//! This trait enables plugin-style extensibility for special operations
//! like file operations, workflows, refactoring, and workspace operations.
//!
//! # Architecture
//!
//! The `ToolHandler` trait provides a standardized interface for implementing
//! MCP tool operations outside of the LSP plugin system. Each handler is
//! responsible for a category of related tools.
//!
//! # Example
//!
//! ```rust,ignore
//! use cb_server::handlers::tool_handler::{ToolHandler, ToolContext};
//! use cb_core::model::mcp::ToolCall;
//! use async_trait::async_trait;
//!
//! struct MyHandler;
//!
//! #[async_trait]
//! impl ToolHandler for MyHandler {
//!     fn supported_tools(&self) -> Vec<&'static str> {
//!         vec!["my_tool"]
//!     }
//!
//!     async fn handle_tool(
//!         &self,
//!         tool_call: ToolCall,
//!         context: &ToolContext
//!     ) -> ServerResult<Value> {
//!         // Implementation
//!         Ok(json!({"success": true}))
//!     }
//! }
//! ```

use crate::{ServerError, ServerResult};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use serde_json::Value;

use super::plugin_dispatcher::{AppState, DirectLspAdapter};
use cb_plugins::PluginManager;
use tokio::sync::Mutex;

/// Context provided to tool handlers with access to application services
pub struct ToolContext {
    /// Application state containing all services (file_service, ast_service, etc.)
    pub app_state: std::sync::Arc<AppState>,
    /// Plugin manager for LSP operations
    pub plugin_manager: std::sync::Arc<PluginManager>,
    /// LSP adapter for refactoring operations
    pub lsp_adapter: std::sync::Arc<Mutex<Option<std::sync::Arc<DirectLspAdapter>>>>,
}

/// Handler for MCP tool operations
///
/// Implementations of this trait handle specific categories of MCP tools
/// (e.g., file operations, refactoring, workspace operations).
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Returns the list of tool names this handler supports
    ///
    /// These names are used for automatic routing in the ToolRegistry.
    /// Tool names must be unique across all handlers.
    fn supported_tools(&self) -> Vec<&'static str>;

    /// Handle a tool call
    ///
    /// # Arguments
    ///
    /// * `tool_call` - The MCP tool call containing the tool name and arguments
    /// * `context` - Context providing access to application services
    ///
    /// # Returns
    ///
    /// Returns a JSON value containing the tool result on success,
    /// or a ServerError on failure.
    ///
    /// # Errors
    ///
    /// Returns `ServerError::InvalidRequest` if arguments are missing or invalid.
    /// Returns `ServerError::Unsupported` if the tool name is not supported.
    /// Returns other `ServerError` variants for operation failures.
    async fn handle_tool(
        &self,
        tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value>;
}
