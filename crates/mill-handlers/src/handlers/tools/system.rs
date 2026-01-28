//! System and health tool handlers
//!
//! Handles: health_check

use super::{extensions::get_concrete_app_state, ToolHandler};
use crate::handlers::system_handler::SystemHandler;
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
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

    fn is_internal(&self) -> bool {
        // health_check is now internal - use workspace action:verify_project instead
        true
    }

    async fn handle_tool_call(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
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
                let concrete_state = get_concrete_app_state(&context.app_state)?;
                obj.insert(
                    "system_status".to_string(),
                    json!({
                        "status": "ok",
                        "uptime_seconds": concrete_state.start_time.elapsed().as_secs(),
                        "message": "System is operational"
                    }),
                );
            }

            return Ok(health_report);
        }

        Err(ServerError::invalid_request(format!(
            "Unknown system tool: {}",
            tool_call.name
        )))
    }
}
