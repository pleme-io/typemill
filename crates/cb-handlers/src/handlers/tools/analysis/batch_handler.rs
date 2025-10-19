//! Tool handler for the `analyze.batch` command
use super::super::{ToolHandler, ToolHandlerContext};
use crate::handlers::tools::analysis::batch::{
    run_batch_analysis, AnalysisQuery, BatchAnalysisRequest,
};
use async_trait::async_trait;
use codebuddy_core::model::mcp::ToolCall;
use codebuddy_foundation::protocol::{ ApiError as ServerError , ApiResult as ServerResult };
use serde::Deserialize;
use serde_json::{json, Value};

/// Tool handler for `analyze.batch`
pub struct BatchAnalysisHandler;

impl BatchAnalysisHandler {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Deserialize)]
struct BatchAnalysisArgs {
    queries: Vec<AnalysisQuery>,
}

#[async_trait]
impl ToolHandler for BatchAnalysisHandler {
    fn tool_names(&self) -> &[&str] {
        &["analyze.batch"]
    }

    fn is_internal(&self) -> bool {
        false
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        let args: BatchAnalysisArgs = serde_json::from_value(
            tool_call.arguments.clone().unwrap_or(json!({})),
        )
        .map_err(|e| {
            ServerError::InvalidRequest(format!("Invalid arguments for analyze.batch: {}", e))
        })?;

        let request = BatchAnalysisRequest {
            queries: args.queries,
            config: None, // Config support can be added later
        };

        let result = run_batch_analysis(request, context)
            .await
            .map_err(|e| ServerError::Internal(format!("Batch analysis failed: {}", e)))?;

        serde_json::to_value(result)
            .map_err(|e| ServerError::Internal(format!("Failed to serialize batch result: {}", e)))
    }
}