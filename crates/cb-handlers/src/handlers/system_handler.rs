//! System operations tool handler
//!
//! Handles: health_check, notify_file_opened, notify_file_saved,
//!          notify_file_closed, fix_imports
//!
//! Note: find_dead_code has been moved to analysis_handler.rs

use super::compat::{ToolContext, ToolHandler};
use super::lsp_adapter::DirectLspAdapter;
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
use serde_json::{json, Value};
use std::path::PathBuf;
use tracing::{debug, error, info, warn};

// ============================================================================
// SystemHandler - Public Interface
// ============================================================================

pub struct SystemHandler {
    dependency_handler: super::dependency_handler::DependencyHandler,
    analysis_handler: super::analysis_handler::AnalysisHandler,
}

impl SystemHandler {
    pub fn new() -> Self {
        Self {
            dependency_handler: super::dependency_handler::DependencyHandler::new(),
            analysis_handler: super::analysis_handler::AnalysisHandler::new(),
        }
    }
}

impl Default for SystemHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for SystemHandler {
    fn supported_tools(&self) -> Vec<&'static str> {
        vec![
            "health_check",
            "notify_file_opened",
            "notify_file_saved",
            "notify_file_closed",
            "find_dead_code",
            "analyze_imports",
            "update_dependencies",
        ]
    }

    async fn handle_tool(&self, tool_call: ToolCall, context: &ToolContext) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling system operation");

        match tool_call.name.as_str() {
            "health_check" => self.handle_health_check(tool_call, context).await,
            "notify_file_opened" => self.handle_notify_file_opened(tool_call, context).await,
            "notify_file_saved" => self.handle_notify_file_saved(tool_call, context).await,
            "notify_file_closed" => self.handle_notify_file_closed(tool_call, context).await,

            // Delegate to AnalysisHandler
            "find_dead_code" => {
                self.analysis_handler.handle_tool(tool_call, context).await
            }

            // Delegate to DependencyHandler
            "update_dependencies" => {
                self.dependency_handler.handle_tool(tool_call, context).await
            }

            // Delegate to plugin system (SystemToolsPlugin handles this)
            "analyze_imports" => {
                self.delegate_to_plugin_system(tool_call, context).await
            }

            _ => Err(ServerError::Unsupported(format!(
                "Unknown system operation: {}",
                tool_call.name
            ))),
        }
    }
}

impl SystemHandler {
    /// Delegate tool call to the plugin system
    /// Used for tools that are implemented in plugins (e.g., SystemToolsPlugin)
    async fn delegate_to_plugin_system(
        &self,
        tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value> {
        use cb_plugins::PluginRequest;
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
                        response.error.map(|e| e.to_string())
                            .unwrap_or_else(|| "Plugin request failed".to_string())
                    ))
                }
            }
            Err(e) => {
                error!(error = %e, tool = %tool_call.name, "Plugin request failed");
                Err(ServerError::Internal(format!(
                    "Failed to execute {}: {}",
                    tool_call.name, e
                )))
            }
        }
    }

    async fn handle_health_check(
        &self,
        _tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value> {
        info!("Handling health check request");

        let uptime_secs = context.app_state.start_time.elapsed().as_secs();
        let uptime_mins = uptime_secs / 60;
        let uptime_hours = uptime_mins / 60;

        // Get plugin count from plugin manager
        let plugin_count = context
            .plugin_manager
            .get_all_tool_definitions()
            .await
            .len();

        // Get paused workflow count from executor
        let paused_workflows = context
            .app_state
            .workflow_executor
            .get_paused_workflow_count();

        Ok(json!({
            "status": "healthy",
            "uptime": {
                "seconds": uptime_secs,
                "minutes": uptime_mins,
                "hours": uptime_hours,
                "formatted": format!("{}h {}m {}s", uptime_hours, uptime_mins % 60, uptime_secs % 60)
            },
            "plugins": {
                "loaded": plugin_count
            },
            "workflows": {
                "paused": paused_workflows
            }
        }))
    }

    async fn handle_notify_file_opened(
        &self,
        tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling notify_file_opened");

        let args = tool_call.arguments.unwrap_or(json!({}));
        let file_path_str = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'file_path' parameter".into()))?;

        let file_path = PathBuf::from(file_path_str);

        // Trigger plugin lifecycle hooks for all plugins that can handle this file
        if let Err(e) = context
            .plugin_manager
            .trigger_file_open_hooks(&file_path)
            .await
        {
            warn!(
                file_path = %file_path.display(),
                error = %e,
                "Failed to trigger plugin hooks (continuing)"
            );
        }

        // Get file extension to determine which LSP adapter to notify
        let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        // Load LSP config to create a temporary DirectLspAdapter for notification
        let app_config = cb_core::config::AppConfig::load()
            .map_err(|e| ServerError::Internal(format!("Failed to load app config: {}", e)))?;
        let lsp_config = app_config.lsp;

        // Find the server config for this extension
        if let Some(_server_config) = lsp_config
            .servers
            .iter()
            .find(|server| server.extensions.contains(&extension.to_string()))
        {
            // Create a temporary DirectLspAdapter to handle the notification
            let adapter = DirectLspAdapter::new(
                lsp_config,
                vec![extension.to_string()],
                format!("temp-{}-notifier", extension),
            );

            // Get or create LSP client and notify
            match adapter.get_or_create_client(extension).await {
                Ok(client) => match client.notify_file_opened(&file_path).await {
                    Ok(()) => {
                        debug!(
                            file_path = %file_path.display(),
                            "Successfully notified LSP server about file"
                        );
                        Ok(json!({
                            "success": true,
                            "message": format!("Notified LSP server about file: {}", file_path.display())
                        }))
                    }
                    Err(e) => {
                        warn!(
                            file_path = %file_path.display(),
                            error = %e,
                            "Failed to notify LSP server about file"
                        );
                        Err(ServerError::Runtime {
                            message: format!("Failed to notify LSP server: {}", e),
                        })
                    }
                },
                Err(e) => {
                    warn!(
                        extension = %extension,
                        error = %e,
                        "Failed to get LSP client for extension"
                    );
                    Err(ServerError::Runtime {
                        message: format!("Failed to get LSP client: {}", e),
                    })
                }
            }
        } else {
            debug!(extension = %extension, "No LSP server configured for extension");
            Ok(json!({
                "success": true,
                "message": format!("No LSP server configured for extension '{}'", extension)
            }))
        }
    }

    async fn handle_notify_file_saved(
        &self,
        tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling notify_file_saved");

        let args = tool_call.arguments.unwrap_or(json!({}));
        let file_path_str = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'file_path' parameter".into()))?;

        let file_path = PathBuf::from(file_path_str);

        // Trigger plugin lifecycle hooks for all plugins that can handle this file
        if let Err(e) = context
            .plugin_manager
            .trigger_file_save_hooks(&file_path)
            .await
        {
            warn!(
                file_path = %file_path.display(),
                error = %e,
                "Failed to trigger plugin save hooks (continuing)"
            );
        }

        debug!(
            file_path = %file_path.display(),
            "File saved notification processed"
        );

        Ok(json!({
            "success": true,
            "message": format!("Notified about saved file: {}", file_path.display())
        }))
    }

    async fn handle_notify_file_closed(
        &self,
        tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling notify_file_closed");

        let args = tool_call.arguments.unwrap_or(json!({}));
        let file_path_str = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'file_path' parameter".into()))?;

        let file_path = PathBuf::from(file_path_str);

        // Trigger plugin lifecycle hooks for all plugins that can handle this file
        if let Err(e) = context
            .plugin_manager
            .trigger_file_close_hooks(&file_path)
            .await
        {
            warn!(
                file_path = %file_path.display(),
                error = %e,
                "Failed to trigger plugin close hooks (continuing)"
            );
        }

        debug!(
            file_path = %file_path.display(),
            "File closed notification processed"
        );

        Ok(json!({
            "success": true,
            "message": format!("Notified about closed file: {}", file_path.display())
        }))
    }
}
