//! System and health tool handlers
//!
//! Handles: health_check, web_fetch, system_status

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::compat::{ToolContext, ToolHandler as LegacyToolHandler};
use crate::handlers::system_handler::SystemHandler as LegacySystemHandler;
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::ApiResult as ServerResult;
use serde_json::{json, Value};

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
    fn tool_names(&self) -> &[&str] {
        &["health_check", "web_fetch", "system_status"]
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // Handle system_status directly
        if tool_call.name == "system_status" {
            // Return basic system status
            return Ok(json!({
                "status": "ok",
                "uptime_seconds": context.app_state.start_time.elapsed().as_secs(),
                "message": "System is operational"
            }));
        }

        // Convert new context to legacy context
        let legacy_context = ToolContext {
            app_state: context.app_state.clone(),
            plugin_manager: context.plugin_manager.clone(),
            lsp_adapter: context.lsp_adapter.clone(),
        };

        // Delegate to legacy handler
        self.legacy_handler
            .handle_tool(tool_call.clone(), &legacy_context)
            .await
    }
}
