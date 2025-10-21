//! Internal-only intelligence tool handlers
//!
//! Handles: get_completions, get_signature_help

use super::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use codebuddy_foundation::core::model::mcp::ToolCall;
use codebuddy_foundation::protocol::{ApiError, ApiResult as ServerResult};
use codebuddy_plugin_system::PluginRequest;
use serde_json::{json, Value};
use std::path::PathBuf;

fn to_api_error(plugin_error: codebuddy_plugin_system::PluginError) -> ApiError {
    match plugin_error {
        codebuddy_plugin_system::PluginError::MethodNotSupported { method, plugin } => {
            ApiError::Unsupported(format!(
                "Method '{}' not supported by plugin '{}'",
                method, plugin
            ))
        }
        codebuddy_plugin_system::PluginError::SerializationError { message } => {
            ApiError::Parse { message }
        }
        e => ApiError::Internal(e.to_string()),
    }
}

pub struct InternalIntelligenceHandler;

impl InternalIntelligenceHandler {
    pub fn new() -> Self {
        Self
    }

    fn convert_tool_call_to_plugin_request(
        &self,
        tool_call: &ToolCall,
    ) -> Result<PluginRequest, ApiError> {
        let args = tool_call.arguments.clone().unwrap_or_default();

        let file_path_str = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ApiError::InvalidRequest("Missing file_path parameter".into()))?;

        let file_path = PathBuf::from(file_path_str);

        let mut request = PluginRequest::new(tool_call.name.clone(), file_path);

        if let (Some(line), Some(character)) = (
            args.get("line").and_then(|v| v.as_u64()),
            args.get("character").and_then(|v| v.as_u64()),
        ) {
            request = request.with_position(line.saturating_sub(1) as u32, character as u32);
        }

        request = request.with_params(args);

        Ok(request)
    }
}

#[async_trait]
impl ToolHandler for InternalIntelligenceHandler {
    fn tool_names(&self) -> &[&str] {
        &["get_completions", "get_signature_help"]
    }

    fn is_internal(&self) -> bool {
        true
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        let plugin_request = self.convert_tool_call_to_plugin_request(tool_call)?;

        context
            .plugin_manager
            .handle_request(plugin_request)
            .await
            .map_err(to_api_error)
            .map(|response| {
                json!({
                    "content": response.data.unwrap_or(json!(null)),
                    "plugin": response.metadata.plugin_name,
                    "processing_time_ms": response.metadata.processing_time_ms,
                    "cached": response.metadata.cached
                })
            })
    }
}
