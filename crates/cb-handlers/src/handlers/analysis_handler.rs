//! Code analysis tool handler
//!
//! This module is reserved for deep static analysis tools.
//! This module provides deep static analysis tools via the unified analysis API.

use super::tools::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use codebuddy_foundation::core::model::mcp::ToolCall;
use codebuddy_foundation::protocol::{ApiError as ServerError, ApiResult as ServerResult};
use serde_json::Value;
use tracing::debug;

pub struct AnalysisHandler;

impl AnalysisHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AnalysisHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for AnalysisHandler {
    fn tool_names(&self) -> &[&str] {
        &[]
    }

    async fn handle_tool_call(
        &self,
        _context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling code analysis operation");

        Err(ServerError::Unsupported(format!(
            "Unknown analysis operation: {}",
            tool_call.name
        )))
    }
}
