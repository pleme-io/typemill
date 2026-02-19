//! System operations tool handler
//!
//! Handles: health_check, get_lsp_progress, notify_file_opened, notify_file_saved, notify_file_closed
//!
//! Note: Import optimization available via optimize_imports

use super::lsp_adapter::DirectLspAdapter;
use super::tools::{extensions::get_concrete_app_state, ToolHandler};
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_services::services::perf_metrics::snapshot_metrics;
use serde_json::{json, Value};
use std::path::PathBuf;
use tracing::{debug, info, warn};

// ============================================================================
// SystemHandler - Public Interface
// ============================================================================

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
    fn tool_names(&self) -> &[&str] {
        &[
            "health_check",
            "get_lsp_progress",
            "notify_file_opened",
            "notify_file_saved",
            "notify_file_closed",
        ]
    }

    async fn handle_tool_call(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling system operation");

        match tool_call.name.as_str() {
            "health_check" => self.handle_health_check(tool_call.clone(), context).await,
            "get_lsp_progress" => self.handle_get_lsp_progress(context).await,
            "notify_file_opened" => {
                self.handle_notify_file_opened(tool_call.clone(), context)
                    .await
            }
            "notify_file_saved" => {
                self.handle_notify_file_saved(tool_call.clone(), context)
                    .await
            }
            "notify_file_closed" => {
                self.handle_notify_file_closed(tool_call.clone(), context)
                    .await
            }

            _ => Err(ServerError::not_supported(format!(
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
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<Value> {
        info!("Handling health check request");

        let concrete_state = get_concrete_app_state(&context.app_state)?;
        let uptime_secs = concrete_state.start_time.elapsed().as_secs();
        let uptime_mins = uptime_secs / 60;
        let uptime_hours = uptime_mins / 60;

        // Get plugin count from plugin manager
        let plugin_count = context
            .plugin_manager
            .get_all_tool_definitions()
            .await
            .len();

        // Get detailed metrics and statistics
        let metrics = context.plugin_manager.get_metrics().await;
        let stats = context.plugin_manager.get_registry_statistics().await;

        // Get paused workflow count from executor
        let paused_workflows = concrete_state.workflow_executor.get_paused_workflow_count();

        // Get internal performance metrics snapshots
        let perf_metrics = snapshot_metrics();

        // Calculate success rate
        let success_rate = if metrics.total_requests > 0 {
            (metrics.successful_requests as f64 / metrics.total_requests as f64) * 100.0
        } else {
            0.0
        };

        Ok(json!({
            "status": "healthy",
            "uptime": {
                "seconds": uptime_secs,
                "minutes": uptime_mins,
                "hours": uptime_hours,
                "formatted": format!("{}h {}m {}s", uptime_hours, uptime_mins % 60, uptime_secs % 60)
            },
            "plugins": {
                "loaded": plugin_count,
                "total_plugins": stats.total_plugins,
                "supported_extensions": stats.supported_extensions,
                "supported_methods": stats.supported_methods,
                "average_methods_per_plugin": stats.average_methods_per_plugin
            },
            "metrics": {
                "total_requests": metrics.total_requests,
                "successful_requests": metrics.successful_requests,
                "failed_requests": metrics.failed_requests,
                "success_rate": format!("{:.2}%", success_rate),
                "average_processing_time_ms": metrics.average_processing_time_ms,
                "requests_per_plugin": metrics.requests_per_plugin,
                "processing_time_per_plugin": metrics.processing_time_per_plugin
            },
            "workflows": {
                "paused": paused_workflows
            },
            "performance": {
                "metrics": perf_metrics
            }
        }))
    }

    async fn handle_get_lsp_progress(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<Value> {
        debug!("Handling get_lsp_progress request");

        // Get the LSP adapter from context
        let lsp_adapter_guard = context.lsp_adapter.lock().await;
        let lsp_adapter = lsp_adapter_guard
            .as_ref()
            .ok_or_else(|| ServerError::internal("LSP adapter not initialized"))?;

        // Downcast to DirectLspAdapter to access progress methods
        let direct_adapter = lsp_adapter
            .as_any()
            .downcast_ref::<DirectLspAdapter>()
            .ok_or_else(|| ServerError::internal("LSP adapter is not a DirectLspAdapter"))?;

        // Get progress from all active LSP clients
        let progress = direct_adapter.get_all_lsp_progress().await;

        // Calculate summary
        let mut total_tasks = 0;
        let mut in_progress = 0;
        let mut completed = 0;

        for (_ext, tasks) in &progress {
            for (_, info) in tasks {
                total_tasks += 1;
                match info.status.as_str() {
                    "in_progress" => in_progress += 1,
                    "completed" => completed += 1,
                    _ => {}
                }
            }
        }

        let is_ready = in_progress == 0;

        Ok(json!({
            "ready": is_ready,
            "summary": {
                "totalTasks": total_tasks,
                "inProgress": in_progress,
                "completed": completed
            },
            "progress": progress
        }))
    }

    async fn handle_notify_file_opened(
        &self,
        tool_call: ToolCall,
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling notify_file_opened");

        let args = tool_call.arguments.unwrap_or(json!({}));
        let file_path_str = args
            .get("filePath")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::invalid_request("Missing 'file_path' parameter"))?;

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
        let app_config = mill_config::config::AppConfig::load()
            .map_err(|e| ServerError::internal(format!("Failed to load app config: {}", e)))?;
        let lsp_config = app_config.lsp;
        if lsp_config.mode == mill_config::config::LspMode::Off {
            return Err(ServerError::not_supported(
                "LSP is disabled (lsp.mode=off).",
            ));
        }

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
                        Err(ServerError::runtime(format!(
                            "Failed to notify LSP server: {}",
                            e
                        )))
                    }
                },
                Err(e) => {
                    warn!(
                        extension = %extension,
                        error = %e,
                        "Failed to get LSP client for extension"
                    );
                    Err(ServerError::runtime(format!(
                        "Failed to get LSP client: {}",
                        e
                    )))
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
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling notify_file_saved");

        let args = tool_call.arguments.unwrap_or(json!({}));
        let file_path_str = args
            .get("filePath")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::invalid_request("Missing 'file_path' parameter"))?;

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
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling notify_file_closed");

        let args = tool_call.arguments.unwrap_or(json!({}));
        let file_path_str = args
            .get("filePath")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::invalid_request("Missing 'file_path' parameter"))?;

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
