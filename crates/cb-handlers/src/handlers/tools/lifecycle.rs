//! LSP lifecycle tool handlers
//!
//! Handles: notify_file_opened, notify_file_saved, notify_file_closed

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::system_handler::SystemHandler;
use async_trait::async_trait;
use codebuddy_core::model::mcp::ToolCall;
use codebuddy_foundation::protocol::ApiResult as ServerResult;
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

    fn is_internal(&self) -> bool {
        // Lifecycle tools are internal - they're backend hooks for editors/IDEs
        // to notify LSP servers and trigger plugin lifecycle events.
        // AI agents don't need these - they directly read/write files.
        true
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // Delegate directly to SystemHandler (which now implements the new trait)
        self.system_handler
            .handle_tool_call(context, tool_call)
            .await
    }
}