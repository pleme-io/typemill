//! System operations tool handler
//!
//! Handles: health_check, notify_file_opened, notify_file_saved,
//!          notify_file_closed, find_dead_code, fix_imports

use super::plugin_dispatcher::DirectLspAdapter;
use super::tool_handler::{ToolContext, ToolHandler};
use crate::{ServerError, ServerResult};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_plugins::PluginRequest;
use serde_json::{json, Value};
use std::path::PathBuf;
use tracing::{debug, info, warn};

pub struct SystemHandler;

impl SystemHandler {
    pub fn new() -> Self {
        Self
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
        ]
    }

    async fn handle_tool(
        &self,
        tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling system operation");

        match tool_call.name.as_str() {
            "health_check" => self.handle_health_check(tool_call, context).await,
            "notify_file_opened" => self.handle_notify_file_opened(tool_call, context).await,
            "notify_file_saved" => self.handle_notify_file_saved(tool_call, context).await,
            "notify_file_closed" => self.handle_notify_file_closed(tool_call, context).await,
            "find_dead_code" => self.handle_find_dead_code(tool_call, context).await,
            _ => Err(ServerError::Unsupported(format!(
                "Unknown system operation: {}",
                tool_call.name
            ))),
        }
    }
}

impl SystemHandler {
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
        let plugin_count = context.plugin_manager.get_all_tool_definitions().await.len();

        // Get paused workflow count from executor
        let paused_workflows = context.app_state.workflow_executor.get_paused_workflow_count();

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

    async fn handle_find_dead_code(
        &self,
        tool_call: ToolCall,
        _context: &ToolContext,
    ) -> ServerResult<Value> {
        let start_time = std::time::Instant::now();
        let args = tool_call.arguments.unwrap_or(json!({}));
        let workspace_path = args
            .get("workspace_path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        debug!(workspace_path = %workspace_path, "Handling find_dead_code request");

        // Load LSP configuration
        let app_config = cb_core::config::AppConfig::load()
            .map_err(|e| ServerError::Internal(format!("Failed to load config: {}", e)))?;

        // Run dead code analysis
        let config = crate::handlers::dead_code::AnalysisConfig::default();
        let dead_symbols = crate::handlers::dead_code::analyze_dead_code(
            app_config.lsp,
            workspace_path,
            config,
        )
        .await?;

        // Format response with complete stats
        let dead_symbols_json: Vec<Value> = dead_symbols
            .iter()
            .map(|s| {
                json!({
                    "name": s.name,
                    "kind": s.kind,
                    "file": s.file_path,
                    "line": s.line,
                    "column": s.column,
                    "referenceCount": s.reference_count,
                })
            })
            .collect();

        let files_analyzed = dead_symbols
            .iter()
            .map(|s| s.file_path.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len();

        Ok(json!({
            "workspacePath": workspace_path,
            "deadSymbols": dead_symbols_json,
            "analysisStats": {
                "filesAnalyzed": files_analyzed,
                "symbolsAnalyzed": dead_symbols_json.len(),
                "deadSymbolsFound": dead_symbols.len(),
                "analysisDurationMs": start_time.elapsed().as_millis(),
            }
        }))
    }

}
