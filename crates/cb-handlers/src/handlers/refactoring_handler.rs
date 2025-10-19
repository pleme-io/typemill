//! Refactoring handler stub
//!
//! This is a temporary stub to satisfy compilation requirements.
//! Individual refactoring operations are handled by specific tool handlers.

use crate::handlers::tools::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use codebuddy_core::model::mcp::ToolCall;
use codebuddy_foundation::protocol::{ ApiError , ApiResult as ServerResult };
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
        _context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        Err(ApiError::InvalidRequest(format!(
            "RefactoringHandler stub does not handle tool: {}",
            tool_call.name
        )))
    }
}