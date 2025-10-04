//! LSP lifecycle tool handlers
//!
//! Handles: notify_file_opened, notify_file_saved, notify_file_closed

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::compat::ToolHandler as LegacyToolHandler;
use crate::handlers::system_handler::SystemHandler as LegacySystemHandler;
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::ApiResult as ServerResult;
use serde_json::Value;

pub struct LifecycleHandler {
    legacy_handler: LegacySystemHandler,
}

impl LifecycleHandler {
    pub fn new() -> Self {
        Self {
            legacy_handler: LegacySystemHandler::new(),
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
        crate::delegate_to_legacy!(self, context, tool_call)
    }
}
