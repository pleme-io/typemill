//! System and health tool handlers
//!
//! Handles: health_check

use super::{ToolHandler, ToolHandlerContext};
use crate::handlers::compat::{ToolContext, ToolHandler as LegacyToolHandler};
use crate::handlers::system_handler::SystemHandler as LegacySystemHandler;
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::{ApiError, ApiResult as ServerResult};
use serde_json::{json, Value};

pub struct SystemHandler {
    legacy_handler: LegacySystemHandler,
}

impl SystemHandler {
    pub fn new() -> Self {
        Self {
            legacy_handler: LegacySystemHandler::new(),
        }
    }
}

#[async_trait]
impl ToolHandler for SystemHandler {
    fn tool_names(&self) -> &[&str] {
        &["health_check"]
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        if tool_call.name == "health_check" {
            // The new health_check combines the legacy health_check (plugins, etc.)
            // with the system status information.
            let legacy_context = ToolContext {
                user_id: context.user_id.clone(),
                app_state: context.app_state.clone(),
                plugin_manager: context.plugin_manager.clone(),
                lsp_adapter: context.lsp_adapter.clone(),
            };
            let mut health_report = self
                .legacy_handler
                .handle_tool(tool_call.clone(), &legacy_context)
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
