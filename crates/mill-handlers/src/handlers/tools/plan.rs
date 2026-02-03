//! Plan tools handler
//!
//! Handles: apply_plan

use super::ToolHandler;
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_foundation::protocol::RefactorPlan;
use mill_services::services::planning::executor::{ExecutionOptions, PlanExecutor};
use serde::Deserialize;
use serde_json::Value;

pub struct PlanToolsHandler;

impl PlanToolsHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PlanToolsHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplyPlanParams {
    plan: Option<RefactorPlan>,
    options: Option<ExecutionOptions>,
}

#[async_trait]
impl ToolHandler for PlanToolsHandler {
    fn tool_names(&self) -> &[&str] {
        &["apply_plan"]
    }

    async fn handle_tool_call(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        if tool_call.name != "apply_plan" {
            return Err(ServerError::invalid_request(format!(
                "Unknown plan tool: {}",
                tool_call.name
            )));
        }

        let args = tool_call
            .arguments
            .clone()
            .unwrap_or(serde_json::Value::Null);

        let (plan_value, options) = if let Ok(params) =
            serde_json::from_value::<ApplyPlanParams>(args.clone())
        {
            if let Some(plan) = params.plan {
                (serde_json::to_value(plan).unwrap_or(Value::Null), params.options)
            } else {
                (args, params.options)
            }
        } else {
            (args, None)
        };

        let plan: RefactorPlan = serde_json::from_value(plan_value).map_err(|e| {
            ServerError::invalid_request(format!(
                "Failed to parse refactor plan JSON: {}",
                e
            ))
        })?;

        let concrete_state = super::extensions::get_concrete_app_state(&context.app_state)?;
        let executor = PlanExecutor::new(concrete_state.file_service.clone());
        let result = executor.execute_plan(plan, options.unwrap_or_default()).await?;

        Ok(serde_json::to_value(result).unwrap_or(Value::Null))
    }
}
