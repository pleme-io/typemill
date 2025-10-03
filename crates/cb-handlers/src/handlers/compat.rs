//! Compatibility module for legacy tool handlers
//!
//! This module provides compatibility types and traits for legacy handlers
//! that haven't been fully migrated to the unified architecture yet.

use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::ApiResult as ServerResult;
use serde_json::Value;

use super::lsp_adapter::DirectLspAdapter;
use super::plugin_dispatcher::AppState;
use cb_plugins::PluginManager;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Legacy context for old tool handlers
pub struct ToolContext {
    /// Application state containing all services
    pub app_state: Arc<AppState>,
    /// Plugin manager for LSP operations
    pub plugin_manager: Arc<PluginManager>,
    /// LSP adapter for refactoring operations
    pub lsp_adapter: Arc<Mutex<Option<Arc<DirectLspAdapter>>>>,
}

/// Legacy tool handler trait (for backwards compatibility)
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Returns the list of tool names this handler supports
    fn supported_tools(&self) -> Vec<&'static str>;

    /// Handle a tool call
    async fn handle_tool(&self, tool_call: ToolCall, context: &ToolContext) -> ServerResult<Value>;
}
