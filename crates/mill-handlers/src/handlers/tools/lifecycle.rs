//! LSP lifecycle tool handlers
//!
//! Handles: notify_file_opened, notify_file_saved, notify_file_closed

use super::ToolHandler;
use crate::handlers::system_handler::SystemHandler;
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::MillResult as ServerResult;
use serde_json::Value;

pub struct LifecycleHandler {
    system_handler: SystemHandler,
}

impl LifecycleHandler {
    pub fn new() -> Self {
        Self {
            system_handler: SystemHandler::new(),
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
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // Delegate directly to SystemHandler (which now implements the new trait)
        self.system_handler
            .handle_tool_call(context, tool_call)
            .await
    }
}
