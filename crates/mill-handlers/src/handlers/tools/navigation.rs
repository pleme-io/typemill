//! Navigation and intelligence tool handlers
//!
//! Handles: find_definition, find_references, find_implementations, find_type_definition,
//! get_document_symbols, search_workspace_symbols, get_hover, get_completions,
//! get_signature_help, get_diagnostics, prepare_call_hierarchy,
//! get_call_hierarchy_incoming_calls, get_call_hierarchy_outgoing_calls
//!
//! These tools are delegated to the LSP plugin system.

use super::ToolHandler;
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_plugin_system::PluginRequest;
use serde_json::{json, Value};
use std::path::PathBuf;

pub struct NavigationHandler;

impl NavigationHandler {
    pub fn new() -> Self {
        Self
    }

    async fn handle_find_symbol(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        let args = tool_call.arguments.clone().unwrap_or(json!({}));
        let symbol_name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::invalid_request("Missing 'name' parameter"))?;
        let file_path = args.get("filePath").and_then(|v| v.as_str());

        let mut symbols = Vec::new();

        if let Some(path) = file_path {
            // Document symbols
            let mut request =
                PluginRequest::new("get_document_symbols".to_string(), PathBuf::from(path));
            request = request.with_params(args.clone());

            if let Ok(response) = context.plugin_manager.handle_request(request).await {
                if let Some(data) = response.data {
                    if let Some(arr) = data.as_array() {
                        symbols.extend(arr.clone());
                    }
                }
            }
        } else {
            // Workspace search
            // Reuse handle_search_symbols logic but pass 'query' = symbol_name
            let mut search_args = args.as_object().cloned().unwrap_or_default();
            search_args.insert("query".to_string(), json!(symbol_name));

            let search_call = ToolCall {
                name: "search_symbols".to_string(),
                arguments: Some(Value::Object(search_args)),
            };

            let result = self.handle_search_symbols(context, &search_call).await?;
            if let Some(content) = result.get("content").and_then(|v| v.as_array()) {
                symbols.extend(content.clone());
            }
        }

        // Filter symbols by name (fuzzy match is usually already done by search, but we refine it)
        let filtered: Vec<Value> = symbols
            .into_iter()
            .filter(|s| {
                s.get("name")
                    .and_then(|n| n.as_str())
                    .map(|n| n.contains(symbol_name))
                    .unwrap_or(false)
            })
            .collect();

        Ok(json!(filtered))
    }

    async fn handle_find_referencing_symbols(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // First find the symbol
        let find_symbol_call = ToolCall {
            name: "find_symbol".to_string(),
            arguments: tool_call.arguments.clone(),
        };
        let symbols_json = self.handle_find_symbol(context, &find_symbol_call).await?;
        let symbols = symbols_json
            .as_array()
            .ok_or_else(|| ServerError::internal("Invalid response from find_symbol"))?;

        if symbols.is_empty() {
            return Ok(json!([]));
        }

        let mut all_references = Vec::new();

        for symbol in symbols {
            // Try to extract location and file URI
            let location_opt = symbol.get("location");
            // LSP SymbolInformation has location: { uri: "...", range: ... }
            // DocumentSymbol has range, selectionRange, but NO URI (implicit in file path)

            // If we have uri in location
            let uri_opt = location_opt.and_then(|l| l.get("uri").and_then(|v| v.as_str()));

            // If we have file path from args (if find_symbol was called with filePath)
            let args_file_path = tool_call
                .arguments
                .as_ref()
                .and_then(|a| a.get("filePath").and_then(|v| v.as_str()));

            let final_uri = uri_opt.or(args_file_path);

            if let Some(uri) = final_uri {
                let range_opt = location_opt
                    .and_then(|l| l.get("range"))
                    .or_else(|| symbol.get("selectionRange")) // DocumentSymbol
                    .or_else(|| symbol.get("range")); // DocumentSymbol

                let (line, col) = if let Some(range) = range_opt {
                    (
                        range
                            .get("start")
                            .and_then(|s| s.get("line"))
                            .and_then(|v| v.as_u64()),
                        range
                            .get("start")
                            .and_then(|s| s.get("character"))
                            .and_then(|v| v.as_u64()),
                    )
                } else {
                    // Internal Symbol
                    (
                        location_opt.and_then(|l| l.get("line").and_then(|v| v.as_u64())),
                        location_opt.and_then(|l| l.get("column").and_then(|v| v.as_u64())),
                    )
                };

                if let (Some(l), Some(c)) = (line, col) {
                    let mut ref_request =
                        PluginRequest::new("find_references".to_string(), PathBuf::from(uri));
                    ref_request = ref_request.with_position(l as u32, c as u32);

                    if let Ok(response) = context.plugin_manager.handle_request(ref_request).await {
                        if let Some(data) = response.data {
                            if let Some(arr) = data.as_array() {
                                all_references.extend(arr.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok(json!(all_references))
    }

    /// Find a representative file in the workspace with the given extension
    fn find_representative_file(
        workspace_path: &std::path::Path,
        extension: &str,
    ) -> Option<PathBuf> {
        use std::fs;

        // First, try to find a file in common source directories
        let common_dirs = ["src", "lib", "packages", "apps", "."];

        for dir in common_dirs {
            let search_path = if dir == "." {
                workspace_path.to_path_buf()
            } else {
                workspace_path.join(dir)
            };

            if search_path.is_dir() {
                // Look for files with the target extension
                if let Ok(entries) = fs::read_dir(&search_path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(ext) = path.extension() {
                                if ext == extension {
                                    return Some(path);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fall back to recursive search (limited depth)
        Self::find_file_recursive(workspace_path, extension, 3)
    }

    fn find_file_recursive(
        dir: &std::path::Path,
        extension: &str,
        max_depth: u32,
    ) -> Option<PathBuf> {
        use std::fs;

        if max_depth == 0 {
            return None;
        }

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                // Skip hidden directories and node_modules
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') || name == "node_modules" || name == "target" {
                        continue;
                    }
                }

                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == extension {
                            return Some(path);
                        }
                    }
                } else if path.is_dir() {
                    if let Some(found) = Self::find_file_recursive(&path, extension, max_depth - 1)
                    {
                        return Some(found);
                    }
                }
            }
        }

        None
    }

    /// Handle workspace symbol search across all plugins
    async fn handle_search_symbols(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        use std::time::Instant;
        use tracing::{debug, warn};

        debug!("handle_search_symbols: Starting multi-plugin workspace search");

        let start_time = Instant::now();
        let args = tool_call.arguments.clone().unwrap_or(json!({}));

        // Get workspace path from args or use current directory
        let workspace_path = args
            .get("workspacePath")
            .or_else(|| args.get("workspace_path"))
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        // Get all registered plugins
        let plugin_names = context.plugin_manager.list_plugins().await;
        debug!(
            plugin_count = plugin_names.len(),
            plugins = ?plugin_names,
            workspace = %workspace_path.display(),
            "handle_search_symbols: Found registered plugins"
        );

        let mut all_symbols = Vec::new();
        let mut queried_plugins = Vec::new();
        let mut warnings = Vec::new();

        // Query each plugin for workspace symbols
        for plugin_name in plugin_names {
            if let Some(plugin) = context
                .plugin_manager
                .get_plugin_by_name(&plugin_name)
                .await
            {
                // Get supported extensions for this plugin
                let extensions = plugin.supported_extensions();
                let ext = match extensions.first() {
                    Some(e) => e,
                    None => continue, // Skip plugins with no extensions
                };

                // Find a real file in the workspace with this extension
                // This is necessary to establish project context for LSP servers like TypeScript
                let file_path = match Self::find_representative_file(&workspace_path, ext) {
                    Some(path) => path,
                    None => {
                        debug!(
                            plugin = %plugin_name,
                            extension = %ext,
                            "No files found with extension, skipping plugin"
                        );
                        continue;
                    }
                };

                debug!(
                    plugin = %plugin_name,
                    representative_file = %file_path.display(),
                    "Found representative file for plugin"
                );

                // Use the internal plugin method name with the real file path
                let mut request =
                    PluginRequest::new("search_workspace_symbols".to_string(), file_path);
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
                        warn!(
                            plugin = %plugin_name,
                            error = %e,
                            "Plugin query failed"
                        );
                        warnings.push(format!("{}: {}", plugin_name, e));
                    }
                }
            }
        }

        let processing_time = start_time.elapsed().as_millis() as u64;

        let mut result = json!({
            "content": all_symbols,
            "plugin": format!("multi-plugin ({})", queried_plugins.join(", ")),
            "processing_time_ms": processing_time,
            "cached": false
        });

        if !warnings.is_empty() {
            result["warnings"] = json!(warnings);
        }

        Ok(result)
    }

    fn convert_tool_call_to_plugin_request(
        &self,
        tool_call: &ToolCall,
    ) -> Result<PluginRequest, ServerError> {
        let args = tool_call.arguments.clone().unwrap_or(json!({}));

        // Handle workspace-level operations that don't require a file path
        let file_path = match tool_call.name.as_str() {
            "search_symbols" => {
                // Use a dummy file path for workspace symbols
                PathBuf::from(".")
            }
            _ => {
                // Extract file path for file-specific operations
                // Accept both camelCase (filePath) and snake_case (file_path) for compatibility
                let file_path_str = args
                    .get("filePath")
                    .or_else(|| args.get("file_path"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ServerError::invalid_request("Missing 'filePath' parameter"))?;
                PathBuf::from(file_path_str)
            }
        };

        let mut request = PluginRequest::new(tool_call.name.clone(), file_path);

        // Extract position if available
        // Validate that if position parameters are present, they must be valid numbers
        if let Some(line_value) = args.get("line") {
            let line = line_value.as_u64().ok_or_else(|| {
                ServerError::invalid_request(format!(
                    "Invalid type for 'line' parameter: expected number, got {:?}",
                    line_value
                ))
            })?;
            let character_value = args.get("character").ok_or_else(|| {
                ServerError::invalid_request(
                    "Missing 'character' parameter (required when 'line' is present)",
                )
            })?;
            let character = character_value.as_u64().ok_or_else(|| {
                ServerError::invalid_request(format!(
                    "Invalid type for 'character' parameter: expected number, got {:?}",
                    character_value
                ))
            })?;
            request = request.with_position(line.saturating_sub(1) as u32, character as u32);
        } else if args.get("character").is_some() {
            return Err(ServerError::invalid_request(
                "Missing 'line' parameter (required when 'character' is present)",
            ));
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
            "search_symbols",
            "find_symbol",
            "find_referencing_symbols",
            "get_symbol_info",
            "get_diagnostics",
            "get_call_hierarchy",
        ]
    }

    fn is_internal(&self) -> bool {
        // Legacy navigation tools - now internal, use inspect_code/search_code instead
        true
    }

    async fn handle_tool_call(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        use tracing::debug;

        debug!(
            tool_name = %tool_call.name,
            "NavigationHandler::handle_tool_call called"
        );

        if tool_call.name == "find_symbol" {
            return self.handle_find_symbol(context, tool_call).await;
        }

        if tool_call.name == "find_referencing_symbols" {
            return self
                .handle_find_referencing_symbols(context, tool_call)
                .await;
        }

        let mut call = tool_call.clone();

        // Handle tool name mappings for internal plugins
        if call.name == "get_symbol_info" {
            call.name = "get_hover".to_string();
        }

        if call.name == "get_call_hierarchy" {
            let args = call.arguments.clone().unwrap_or(json!({}));
            let hierarchy_type = args.get("type").and_then(|v| v.as_str());

            call.name = match hierarchy_type {
                Some("incoming") => "get_call_hierarchy_incoming_calls".to_string(),
                Some("outgoing") => "get_call_hierarchy_outgoing_calls".to_string(),
                _ => "prepare_call_hierarchy".to_string(),
            };
        }

        // Special handling for workspace symbols - query all plugins
        if tool_call.name == "search_symbols" {
            debug!("Routing to handle_search_symbols for multi-plugin query");
            return self.handle_search_symbols(context, tool_call).await;
        }

        // Convert to plugin request and delegate to plugin system
        let plugin_request = self.convert_tool_call_to_plugin_request(&call)?;

        match context.plugin_manager.handle_request(plugin_request).await {
            Ok(response) => {
                let result = json!({
                    "content": response.data.unwrap_or(json!(null)),
                    "plugin": response.metadata.plugin_name,
                    "processing_time_ms": response.metadata.processing_time_ms,
                    "cached": response.metadata.cached
                });

                // Enhance find_references with cross-file discovery
                if call.name == "find_references" {
                    let args = call.arguments.clone().unwrap_or(json!({}));
                    if let (Some(file_path), Some(line), Some(character)) = (
                        args.get("filePath").and_then(|v| v.as_str()),
                        args.get("line").and_then(|v| v.as_u64()),
                        args.get("character").and_then(|v| v.as_u64()),
                    ) {
                        let path = PathBuf::from(file_path);
                        match super::cross_file_references::enhance_find_references(
                            result.clone(),
                            &path,
                            line as u32,
                            character as u32,
                            context,
                        )
                        .await
                        {
                            Ok(enhanced) => return Ok(enhanced),
                            Err(e) => {
                                debug!(error = %e, "Cross-file enhancement failed, returning original");
                                // Fall through to return original result
                            }
                        }
                    }
                }

                Ok(result)
            }
            Err(err) => Err(ServerError::internal(format!(
                "Plugin request failed: {}",
                err
            ))),
        }
    }
}

/// Internal navigation handler for symbol queries
/// These are replaced by the Unified Analysis API
pub struct InternalNavigationHandler;

impl InternalNavigationHandler {
    pub fn new() -> Self {
        Self
    }

    fn convert_tool_call_to_plugin_request(
        &self,
        tool_call: &ToolCall,
    ) -> Result<PluginRequest, ServerError> {
        let args = tool_call.arguments.clone().unwrap_or(json!({}));
        let file_path_str = args
            .get("filePath")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::invalid_request("Missing file_path parameter"))?;

        let file_path = PathBuf::from(file_path_str);
        let request = PluginRequest::new(tool_call.name.clone(), file_path);
        Ok(request.with_params(args))
    }
}

#[async_trait]
impl ToolHandler for InternalNavigationHandler {
    fn tool_names(&self) -> &[&str] {
        &["get_document_symbols"]
    }

    fn is_internal(&self) -> bool {
        // get_document_symbols is internal-only
        true
    }

    async fn handle_tool_call(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        let plugin_request = self.convert_tool_call_to_plugin_request(tool_call)?;

        match context.plugin_manager.handle_request(plugin_request).await {
            Ok(response) => Ok(json!({
                "content": response.data.unwrap_or(json!(null)),
                "plugin": response.metadata.plugin_name,
                "processing_time_ms": response.metadata.processing_time_ms,
                "cached": response.metadata.cached
            })),
            Err(err) => Err(ServerError::internal(format!(
                "Plugin request failed: {}",
                err
            ))),
        }
    }
}
