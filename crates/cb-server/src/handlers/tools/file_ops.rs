//! File operations tool handlers
//!
//! Handles: create_file, read_file, write_file, delete_file, rename_file, list_files

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::file_operation_handler::FileOperationHandler as LegacyFileHandler;
use crate::handlers::tool_handler::{ToolContext, ToolHandler as LegacyToolHandler};
use crate::ServerResult;
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use serde_json::Value;

pub struct FileOpsHandler {
    legacy_handler: LegacyFileHandler,
}

impl FileOpsHandler {
    pub fn new() -> Self {
        Self {
            legacy_handler: LegacyFileHandler::new(),
        }
    }
}

#[async_trait]
impl ToolHandler for FileOpsHandler {
    fn supported_tools(&self) -> &[&'static str] {
        &[
            "create_file",
            "read_file",
            "write_file",
            "delete_file",
            "rename_file",
            "list_files",
        ]
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
