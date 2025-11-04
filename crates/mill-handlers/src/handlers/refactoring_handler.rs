//! Refactoring handler stub
//!
//! This is a temporary stub to satisfy compilation requirements.
//! Individual refactoring operations are handled by specific tool handlers.

use crate::handlers::tools::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use serde_json::Value;

pub struct RefactoringHandler;

impl RefactoringHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ToolHandler for RefactoringHandler {
    fn tool_names(&self) -> &[&str] {
        &[] // No tools - this is a stub
    }

    fn is_internal(&self) -> bool {
        true
    }

    async fn handle_tool_call(
        &self,
        _context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        Err(ServerError::invalid_request(format!(
            "RefactoringHandler stub does not handle tool: {}",
            tool_call.name
        )))
    }
}
