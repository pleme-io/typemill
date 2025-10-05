//! Navigation and intelligence tool handlers
//!
//! Handles: find_definition, find_references, find_implementations, find_type_definition,
//! get_document_symbols, search_workspace_symbols, get_hover, get_completions,
//! get_signature_help, get_diagnostics, prepare_call_hierarchy,
//! get_call_hierarchy_incoming_calls, get_call_hierarchy_outgoing_calls
//!
//! These tools are delegated to the LSP plugin system.

use super::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_plugins::PluginRequest;
use cb_protocol::ApiResult as ServerResult;
use serde_json::{json, Value};
use std::path::PathBuf;

pub struct NavigationHandler;

impl NavigationHandler {
    pub fn new() -> Self {
        Self
    }

    /// Handle workspace symbol search across all plugins
    async fn handle_workspace_symbols(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        use std::time::Instant;
        use tracing::debug;

        debug!("handle_workspace_symbols: Starting multi-plugin workspace search");

        let start_time = Instant::now();
        let args = tool_call.arguments.clone().unwrap_or(json!({}));

        // Get all registered plugins
        let plugin_names = context.plugin_manager.list_plugins().await;
        debug!(
            plugin_count = plugin_names.len(),
            plugins = ?plugin_names,
            "handle_workspace_symbols: Found registered plugins"
        );

        let mut all_symbols = Vec::new();
        let mut queried_plugins = Vec::new();

        // Query each plugin for workspace symbols
        for plugin_name in plugin_names {
            if let Some(plugin) = context.plugin_manager.get_plugin_by_name(&plugin_name).await {
                // Create a dummy file path with extension for this plugin
                // Use first extension from plugin's supported extensions
                let extensions = plugin.supported_extensions();
                let file_path = if let Some(ext) = extensions.first() {
                    PathBuf::from(format!("workspace.{}", ext))
                } else {
                    continue; // Skip plugins with no extensions
                };

                let mut request = PluginRequest::new("search_workspace_symbols".to_string(), file_path);
                request = request.with_params(args.clone());

                // Try to get symbols from this plugin
                match plugin.handle_request(request).await {
                    Ok(response) => {
                        debug!(
                            plugin = %plugin_name,
                            has_data = response.data.is_some(),
                            "Got response from plugin"
                        );
                        if let Some(data) = response.data {
                            if let Some(symbols) = data.as_array() {
                                debug!(
                                    plugin = %plugin_name,
                                    symbol_count = symbols.len(),
                                    "Found symbols from plugin"
                                );
                                all_symbols.extend(symbols.clone());
                                queried_plugins.push(plugin_name.clone());
                            } else {
                                debug!(
                                    plugin = %plugin_name,
                                    data_type = ?data,
                                    "Data is not an array"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        debug!(
                            plugin = %plugin_name,
                            error = %e,
                            "Plugin query failed"
                        );
                        // Plugin doesn't support workspace symbols or query failed
                        // Continue to next plugin
                    }
                }
            }
        }

        let processing_time = start_time.elapsed().as_millis() as u64;

        Ok(json!({
            "content": all_symbols,
            "plugin": format!("multi-plugin ({})", queried_plugins.join(", ")),
            "processing_time_ms": processing_time,
            "cached": false
        }))
    }

    fn convert_tool_call_to_plugin_request(
        &self,
        tool_call: &ToolCall,
    ) -> Result<PluginRequest, cb_protocol::ApiError> {
        let args = tool_call.arguments.clone().unwrap_or(json!({}));

        // Handle workspace-level operations that don't require a file path
        let file_path = match tool_call.name.as_str() {
            "search_workspace_symbols" => {
                // Use a dummy file path for workspace symbols
                PathBuf::from(".")
            }
            _ => {
                // Extract file path for file-specific operations
                let file_path_str =
                    args.get("file_path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            cb_protocol::ApiError::InvalidRequest(
                                "Missing file_path parameter".into(),
                            )
                        })?;
                PathBuf::from(file_path_str)
            }
        };

        let mut request = PluginRequest::new(tool_call.name.clone(), file_path);

        // Extract position if available
        if let (Some(line), Some(character)) = (
            args.get("line").and_then(|v| v.as_u64()),
            args.get("character").and_then(|v| v.as_u64()),
        ) {
            request = request.with_position(line as u32 - 1, character as u32);
        }

        // Extract range if available
        if let (Some(start_line), Some(start_char), Some(end_line), Some(end_char)) = (
            args.get("start_line").and_then(|v| v.as_u64()),
            args.get("start_character").and_then(|v| v.as_u64()),
            args.get("end_line").and_then(|v| v.as_u64()),
            args.get("end_character").and_then(|v| v.as_u64()),
        ) {
            request = request.with_range(
                start_line as u32 - 1,
                start_char as u32,
                end_line as u32 - 1,
                end_char as u32,
            );
        }

        // Set parameters
        request = request.with_params(args);

        Ok(request)
    }
}

#[async_trait]
impl ToolHandler for NavigationHandler {
    fn tool_names(&self) -> &[&str] {
        &[
            "find_definition",
            "find_references",
            "find_implementations",
            "find_type_definition",
            "get_document_symbols",
            "search_workspace_symbols",
            "get_hover",
            "get_completions",
            "get_signature_help",
            "get_diagnostics",
            "prepare_call_hierarchy",
            "get_call_hierarchy_incoming_calls",
            "get_call_hierarchy_outgoing_calls",
        ]
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        use tracing::debug;

        debug!(
            tool_name = %tool_call.name,
            "NavigationHandler::handle_tool_call called"
        );

        // Special handling for workspace symbols - query all plugins
        if tool_call.name == "search_workspace_symbols" {
            debug!("Routing to handle_workspace_symbols for multi-plugin query");
            return self.handle_workspace_symbols(context, tool_call).await;
        }

        // Convert to plugin request and delegate to plugin system
        let plugin_request = self.convert_tool_call_to_plugin_request(tool_call)?;

        match context.plugin_manager.handle_request(plugin_request).await {
            Ok(response) => Ok(json!({
                "content": response.data.unwrap_or(json!(null)),
                "plugin": response.metadata.plugin_name,
                "processing_time_ms": response.metadata.processing_time_ms,
                "cached": response.metadata.cached
            })),
            Err(err) => Err(cb_protocol::ApiError::Internal(format!(
                "Plugin request failed: {}",
                err
            ))),
        }
    }
}
