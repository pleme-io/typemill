//! System and health tool handlers
//!
//! Handles: health_check, web_fetch

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::system_handler::SystemHandler as LegacySystemHandler;
use crate::handlers::tool_handler::{ToolContext, ToolHandler as LegacyToolHandler};
use crate::ServerResult;
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use serde_json::Value;

pub struct SystemHandler {
    legacy_handler: LegacySystemHandler,
}

impl SystemHandler {
    pub fn new() -> Self {
        Self {
            legacy_handler: LegacySystemHandler::new(),
        }
    }
}

#[async_trait]
impl ToolHandler for SystemHandler {
    fn supported_tools(&self) -> &[&'static str] {
        &["health_check", "web_fetch"]
    }

    async fn handle(
        &self,
        tool_name: &str,
        params: Value,
        context: &ToolHandlerContext,
    ) -> ServerResult<Value> {
        // Convert to ToolCall for legacy handler
        let tool_call = ToolCall {
            name: tool_name.to_string(),
            arguments: Some(params),
        };

        // Convert new context to legacy context
        let legacy_context = ToolContext {
            app_state: context.app_state.clone(),
            plugin_manager: context.plugin_manager.clone(),
            lsp_adapter: context.lsp_adapter.clone(),
        };

        // Delegate to legacy handler
        self.legacy_handler
            .handle_tool(tool_call, &legacy_context)
            .await
    }
}
