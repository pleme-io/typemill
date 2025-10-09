use super::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};

pub mod complexity;
pub mod hotspots;
pub mod refactoring;
pub mod unused_imports;

#[cfg(test)]
mod tests;

pub struct AnalysisHandler;

impl AnalysisHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ToolHandler for AnalysisHandler {
    fn tool_names(&self) -> &[&str] {
        &[
            "find_unused_imports",
            "analyze_complexity",
            "suggest_refactoring",
            "analyze_project_complexity",
            "find_complexity_hotspots",
        ]
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<serde_json::Value> {
        match tool_call.name.as_str() {
            "find_unused_imports" => {
                unused_imports::handle_find_unused_imports(context, tool_call).await
            }
            "analyze_complexity" => complexity::handle_analyze_complexity(context, tool_call).await,
            "suggest_refactoring" => {
                refactoring::handle_suggest_refactoring(context, tool_call).await
            }
            "analyze_project_complexity" => {
                hotspots::handle_analyze_project_complexity(context, tool_call).await
            }
            "find_complexity_hotspots" => {
                hotspots::handle_find_complexity_hotspots(context, tool_call).await
            }
            _ => Err(ServerError::InvalidRequest(format!(
                "Unknown analysis tool: {}",
                tool_call.name
            ))),
        }
    }
}