//! File operations tool handlers
//!
//! Handles basic file utilities: read_file, write_file, list_files
//!
//! NOTE: Refactoring file operations (create, delete, move/rename) have been
//! removed in favor of the Unified Refactoring API:
//! - Use `delete.plan("file", ...)` + `workspace.apply_edit()` for file deletion
//! - Use `move.plan("symbol", ...)` + `workspace.apply_edit()` for file moves
//! - File creation is typically part of extract/move operations

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::file_operation_handler::FileOperationHandler;
use async_trait::async_trait;
use codebuddy_foundation::core::model::mcp::ToolCall;
use codebuddy_foundation::protocol::ApiResult as ServerResult;
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
        &["read_file", "write_file", "list_files"]
    }

    fn is_internal(&self) -> bool {
        // Basic file I/O operations are internal - used by backend/workflows.
        // AI agents should use higher-level operations from the Unified Refactoring API.
        true
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // Delegate to FileOperationHandler for basic file utilities
        self.file_op_handler
            .handle_tool_call(context, tool_call)
            .await
    }
}
