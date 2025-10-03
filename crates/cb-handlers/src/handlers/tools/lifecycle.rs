//! LSP lifecycle tool handlers
//!
//! Handles: notify_file_opened, notify_file_saved, notify_file_closed

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::compat::{ToolContext, ToolHandler as LegacyToolHandler};
use crate::handlers::system_handler::SystemHandler as LegacySystemHandler;
use cb_protocol::ApiResult as ServerResult;
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use serde_json::Value;

pub struct LifecycleHandler {
    system_handler: LegacySystemHandler,
}

impl LifecycleHandler {
    pub fn new() -> Self {
        Self {
            system_handler: LegacySystemHandler::new(),
        }
    }
}

#[async_trait]
impl ToolHandler for LifecycleHandler {
    fn tool_names(&self) -> &[&str] {
        &[
            "notify_file_opened",
            "notify_file_saved",
            "notify_file_closed",
        ]
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // Convert new context to legacy context
        let legacy_context = ToolContext {
            app_state: context.app_state.clone(),
            plugin_manager: context.plugin_manager.clone(),
            lsp_adapter: context.lsp_adapter.clone(),
        };

        // Delegate to legacy handler
        self.system_handler
            .handle_tool(tool_call.clone(), &legacy_context)
            .await
    }
}
