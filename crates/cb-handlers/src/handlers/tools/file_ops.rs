//! File operations tool handlers
//!
//! Handles: create_file, read_file, write_file, delete_file, rename_file, list_files

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::compat::{ToolContext, ToolHandler as LegacyToolHandler};
use crate::handlers::file_operation_handler::FileOperationHandler as LegacyFileHandler;
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::ApiResult as ServerResult;
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
    fn tool_names(&self) -> &[&str] {
        &[
            "create_file",
            "read_file",
            "write_file",
            "delete_file",
            "rename_file",
            "list_files",
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
        self.legacy_handler
            .handle_tool(tool_call.clone(), &legacy_context)
            .await
    }
}
