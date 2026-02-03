//! File operations tool handlers
//!
//! Handles basic file utilities: read_file, write_file, list_files
//!
//! NOTE: Refactoring file operations (create, delete, move/rename) have been
//! removed in favor of the Unified Refactoring API with dryRun:
//! - Use `delete` with options.dryRun: false for file deletion
//! - Use `move` with options.dryRun: false for file moves
//! - File creation is typically part of extract/move operations

use super::ToolHandler;
use crate::handlers::file_operation_handler::FileOperationHandler;
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::MillResult as ServerResult;
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
        #[cfg(feature = "heavy-tests")]
        {
            &["read_file", "write_file", "list_files", "create_file"]
        }
        #[cfg(not(feature = "heavy-tests"))]
        {
            &["read_file", "write_file", "list_files"]
        }
    }

    async fn handle_tool_call(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // Delegate to FileOperationHandler for basic file utilities
        self.file_op_handler
            .handle_tool_call(context, tool_call)
            .await
    }
}
