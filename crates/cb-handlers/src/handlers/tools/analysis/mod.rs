use super::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};

pub mod batch;
pub mod config;
pub mod dead_code;
pub mod dependencies;
pub mod documentation;
pub mod engine;
pub mod project;
pub mod quality;
pub mod structure;
pub mod tests_handler;

pub use batch::{BatchAnalysisRequest, BatchAnalysisResult, BatchError};
pub use config::{AnalysisConfig, CategoryConfig, ConfigError};
pub use dead_code::DeadCodeHandler;
pub use dependencies::DependenciesHandler;
pub use documentation::DocumentationHandler;
pub use quality::QualityHandler;
pub use structure::StructureHandler;
pub use tests_handler::TestsHandler;

pub struct AnalysisHandler;

impl AnalysisHandler {
    pub fn new() -> Self {
        Self
    }

    /// Delegate tool call to the plugin system
    /// Used for tools that are implemented in plugins (e.g., SystemToolsPlugin)
    async fn delegate_to_plugin_system(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<serde_json::Value> {
        use cb_plugins::PluginRequest;
        use serde_json::json;
        use std::path::PathBuf;

        // Extract file_path from arguments
        let args = tool_call.arguments.clone().unwrap_or(json!({}));
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".")); // Default for workspace operations

        // Create plugin request
        let plugin_request = PluginRequest {
            method: tool_call.name.clone(),
            file_path,
            position: None,
            range: None,
            params: args,
            request_id: None,
        };

        // Get the SystemToolsPlugin directly by name
        let system_plugin = context
            .plugin_manager
            .get_plugin_by_name("system")
            .await
            .ok_or_else(|| ServerError::Internal("SystemToolsPlugin not found".to_string()))?;

        // Call the plugin directly
        match system_plugin.handle_request(plugin_request).await {
            Ok(response) => {
                if response.success {
                    Ok(response.data.unwrap_or(json!(null)))
                } else {
                    Err(ServerError::Internal(
                        response
                            .error
                            .map(|e| e.to_string())
                            .unwrap_or_else(|| "Plugin request failed".to_string()),
                    ))
                }
            }
            Err(e) => {
                tracing::error!(error = %e, tool = %tool_call.name, "Plugin request failed");
                Err(ServerError::Internal(format!(
                    "Failed to execute {}: {}",
                    tool_call.name, e
                )))
            }
        }
    }
}

#[async_trait]
impl ToolHandler for AnalysisHandler {
    fn tool_names(&self) -> &[&str] {
        &[
            "analyze_project",
            "analyze_imports",
        ]
    }

    fn is_internal(&self) -> bool {
        // Legacy analysis tools are internal - replaced by Unified Analysis API
        // - analyze_project → analyze.quality("maintainability") (workspace aggregator)
        // - analyze_imports → analyze.dependencies("imports") (plugin-native graphs)
        true
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<serde_json::Value> {
        match tool_call.name.as_str() {
            "analyze_project" => project::handle_analyze_project(context, tool_call).await,
            "analyze_imports" => {
                // Delegate to the plugin system (SystemToolsPlugin handles this)
                self.delegate_to_plugin_system(context, tool_call).await
            }
            _ => Err(ServerError::InvalidRequest(format!(
                "Unknown analysis tool: {}",
                tool_call.name
            ))),
        }
    }
}
