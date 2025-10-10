//! File operations tool handlers
//!
//! Handles: create_file, read_file, write_file, delete_file, rename_file, list_files

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::file_operation_handler::FileOperationHandler;
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::ApiResult as ServerResult;
use serde_json::Value;

pub struct FileToolsHandler {
    file_op_handler: FileOperationHandler,
}

impl FileToolsHandler {
    pub fn new() -> Self {
        Self {
            file_op_handler: FileOperationHandler::new(),
        }
    }
}

#[async_trait]
impl ToolHandler for FileToolsHandler {
    fn tool_names(&self) -> &[&str] {
        &[
            "create_file",
            "read_file",
            "write_file",
            "delete_file",
            "move_file",
            "list_files",
        ]
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        if tool_call.name == "move_file" {
            let mut legacy_tool_call = tool_call.clone();
            legacy_tool_call.name = "rename_file".to_string();
            self.file_op_handler
                .handle_tool_call(context, &legacy_tool_call)
                .await
        } else {
            // FileOperationHandler now uses the new trait, so just delegate directly
            self.file_op_handler
                .handle_tool_call(context, tool_call)
                .await
        }
    }
}
