//! System and health tool handlers
//!
//! Handles: health_check

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::system_handler::SystemHandler;
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::protocol::{ApiError, ApiResult as ServerResult};
use serde_json::{json, Value};

pub struct SystemToolsHandler {
    system_handler: SystemHandler,
}

impl SystemToolsHandler {
    pub fn new() -> Self {
        Self {
            system_handler: SystemHandler::new(),
        }
    }
}

#[async_trait]
impl ToolHandler for SystemToolsHandler {
    fn tool_names(&self) -> &[&str] {
        &["health_check"]
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        if tool_call.name == "health_check" {
            // Health check combines plugin status
            // with system information.
            let mut health_report = self
                .system_handler
                .handle_tool_call(context, tool_call)
                .await?;

            if let Some(obj) = health_report.as_object_mut() {
                obj.insert(
                    "system_status".to_string(),
                    json!({
                        "status": "ok",
                        "uptime_seconds": context.app_state.start_time.elapsed().as_secs(),
                        "message": "System is operational"
                    }),
                );
            }

            return Ok(health_report);
        }

        Err(ApiError::InvalidRequest(format!(
            "Unknown system tool: {}",
            tool_call.name
        )))
    }
}
