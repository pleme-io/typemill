//! Plugin-based MCP message dispatcher
//!
//! This is the new plugin-based dispatcher that replaces the monolithic
//! dispatcher with a flexible plugin system.

use crate::error::{ServerError, ServerResult};
use crate::handlers::AppState;
use cb_core::model::mcp::{McpMessage, McpRequest, McpResponse, ToolCall};
use cb_plugins::{
    PluginManager, LspAdapterPlugin, LspService, PluginRequest, PluginError
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, OnceCell};
use tracing::{debug, error, info, instrument};

/// Plugin-based MCP dispatcher
pub struct PluginDispatcher {
    /// Plugin manager for handling requests
    plugin_manager: Arc<PluginManager>,
    /// Application state for accessing services
    app_state: Arc<AppState>,
    /// Initialization flag
    initialized: OnceCell<()>,
}

/// Direct LSP adapter that bypasses the old LSP manager and its hard-coded mappings
struct DirectLspAdapter {
    /// LSP clients by extension
    lsp_clients: Arc<Mutex<HashMap<String, Arc<crate::systems::lsp::LspClient>>>>,
    /// LSP configuration
    config: cb_core::config::LspConfig,
    /// Supported file extensions
    extensions: Vec<String>,
    /// Adapter name
    name: String,
}

impl DirectLspAdapter {
    fn new(config: cb_core::config::LspConfig, extensions: Vec<String>, name: String) -> Self {
        Self {
            lsp_clients: Arc::new(Mutex::new(HashMap::new())),
            config,
            extensions,
            name,
        }
    }

    /// Get or create an LSP client for the given extension
    async fn get_or_create_client(&self, extension: &str) -> Result<Arc<crate::systems::lsp::LspClient>, String> {
        // Check if client already exists
        {
            let clients = self.lsp_clients.lock().await;
            if let Some(client) = clients.get(extension) {
                return Ok(client.clone());
            }
        }

        // Find server config for this extension
        let server_config = self.config
            .servers
            .iter()
            .find(|server| server.extensions.contains(&extension.to_string()))
            .ok_or_else(|| format!("No LSP server configured for extension: {}", extension))?
            .clone();

        // Create new LSP client
        let client = crate::systems::lsp::LspClient::new(server_config)
            .await
            .map_err(|e| format!("Failed to create LSP client: {}", e))?;

        let client = Arc::new(client);

        // Store the client
        {
            let mut clients = self.lsp_clients.lock().await;
            clients.insert(extension.to_string(), client.clone());
        }

        Ok(client)
    }

    /// Extract file extension from LSP params
    fn extract_extension_from_params(&self, params: &Value) -> Option<String> {
        // Try to get from textDocument.uri
        if let Some(uri) = params.get("textDocument")?.get("uri")?.as_str() {
            if uri.starts_with("file://") {
                let path = uri.trim_start_matches("file://");
                return std::path::Path::new(path).extension()?.to_str().map(|s| s.to_string());
            }
        }
        None
    }
}

#[async_trait]
impl LspService for DirectLspAdapter {
    async fn request(&self, method: &str, params: Value) -> Result<Value, String> {
        // Extract extension from params
        let extension = self.extract_extension_from_params(&params)
            .ok_or_else(|| "Could not extract file extension from params".to_string())?;

        // Get appropriate LSP client
        let client = self.get_or_create_client(&extension).await?;

        // Send LSP method DIRECTLY to client (bypassing old manager and its hard-coded mappings!)
        client.send_request(method, params)
            .await
            .map_err(|e| format!("LSP request failed: {}", e))
    }

    fn supports_extension(&self, extension: &str) -> bool {
        self.extensions.contains(&extension.to_string())
    }

    fn service_name(&self) -> String {
        self.name.clone()
    }
}

impl PluginDispatcher {
    /// Create a new plugin dispatcher
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self {
            plugin_manager: Arc::new(PluginManager::new()),
            app_state,
            initialized: OnceCell::new(),
        }
    }

    /// Initialize the plugin system with default plugins
    #[instrument(skip(self))]
    pub async fn initialize(&self) -> ServerResult<()> {
        self.initialized.get_or_try_init(|| async {
            info!("Initializing plugin system with DirectLspAdapter (bypassing hard-coded mappings)");

            // Get LSP configuration from app config
            let app_config = cb_core::config::AppConfig::load()
                .map_err(|e| ServerError::Internal(format!("Failed to load app config: {}", e)))?;
            let lsp_config = app_config.lsp;

            // Register TypeScript/JavaScript plugin with DirectLspAdapter
            let ts_lsp_adapter = Arc::new(DirectLspAdapter::new(
                lsp_config.clone(),
                vec!["ts".to_string(), "tsx".to_string(), "js".to_string(), "jsx".to_string()],
                "typescript-lsp-direct".to_string(),
            ));
            let ts_plugin = Arc::new(LspAdapterPlugin::typescript(ts_lsp_adapter));
            self.plugin_manager
                .register_plugin("typescript", ts_plugin)
                .await
                .map_err(|e| ServerError::Internal(format!("Failed to register TypeScript plugin: {}", e)))?;

            // Register Python plugin with DirectLspAdapter
            let py_lsp_adapter = Arc::new(DirectLspAdapter::new(
                lsp_config.clone(),
                vec!["py".to_string(), "pyi".to_string()],
                "python-lsp-direct".to_string(),
            ));
            let py_plugin = Arc::new(LspAdapterPlugin::python(py_lsp_adapter));
            self.plugin_manager
                .register_plugin("python", py_plugin)
                .await
                .map_err(|e| ServerError::Internal(format!("Failed to register Python plugin: {}", e)))?;

            // Register Go plugin with DirectLspAdapter
            let go_lsp_adapter = Arc::new(DirectLspAdapter::new(
                lsp_config.clone(),
                vec!["go".to_string()],
                "go-lsp-direct".to_string(),
            ));
            let go_plugin = Arc::new(LspAdapterPlugin::go(go_lsp_adapter));
            self.plugin_manager
                .register_plugin("go", go_plugin)
                .await
                .map_err(|e| ServerError::Internal(format!("Failed to register Go plugin: {}", e)))?;

            // Register Rust plugin with DirectLspAdapter
            let rust_lsp_adapter = Arc::new(DirectLspAdapter::new(
                lsp_config,
                vec!["rs".to_string()],
                "rust-lsp-direct".to_string(),
            ));
            let rust_plugin = Arc::new(LspAdapterPlugin::rust(rust_lsp_adapter));
            self.plugin_manager
                .register_plugin("rust", rust_plugin)
                .await
                .map_err(|e| ServerError::Internal(format!("Failed to register Rust plugin: {}", e)))?;

            info!("Plugin system initialized successfully with 4 language plugins");
            Ok::<(), ServerError>(())
        }).await?;

        Ok(())
    }

    /// Dispatch an MCP message using the plugin system
    #[instrument(skip(self, message))]
    pub async fn dispatch(&self, message: McpMessage) -> ServerResult<McpMessage> {
        // Ensure initialization
        self.initialize().await?;

        match message {
            McpMessage::Request(request) => self.handle_request(request).await,
            McpMessage::Response(response) => Ok(McpMessage::Response(response)),
            McpMessage::Notification(notification) => {
                debug!("Received notification: {:?}", notification);
                Ok(McpMessage::Response(McpResponse {
                    id: None,
                    result: Some(json!({"status": "ok"})),
                    error: None,
                }))
            }
            _ => Err(ServerError::Unsupported("Unknown message type".into())),
        }
    }

    /// Handle an MCP request using plugins
    #[instrument(skip(self, request))]
    async fn handle_request(&self, request: McpRequest) -> ServerResult<McpMessage> {
        debug!("Handling request: {:?}", request.method);

        let response = match request.method.as_str() {
            "tools/list" => self.handle_list_tools().await?,
            "tools/call" => self.handle_tool_call(request.params).await?,
            _ => {
                return Err(ServerError::Unsupported(format!(
                    "Unknown method: {}",
                    request.method
                )))
            }
        };

        Ok(McpMessage::Response(McpResponse {
            id: request.id,
            result: Some(response),
            error: None,
        }))
    }

    /// Handle tools/list request using plugin capabilities
    #[instrument(skip(self))]
    async fn handle_list_tools(&self) -> ServerResult<Value> {
        let all_capabilities = self.plugin_manager.get_all_capabilities().await;
        let all_metadata = self.plugin_manager.get_all_metadata().await;

        let mut tools = Vec::new();

        // Collect tools from all plugins
        for (plugin_name, capabilities) in all_capabilities {
            let metadata = all_metadata.get(&plugin_name);

            // Add standard tools based on capabilities
            if capabilities.navigation.go_to_definition {
                tools.push(self.create_tool_description(
                    "find_definition",
                    "Find the definition of a symbol",
                    &plugin_name,
                    metadata,
                ));
            }

            if capabilities.navigation.find_references {
                tools.push(self.create_tool_description(
                    "find_references",
                    "Find all references to a symbol",
                    &plugin_name,
                    metadata,
                ));
            }

            if capabilities.editing.rename {
                tools.push(self.create_tool_description(
                    "rename_symbol",
                    "Rename a symbol throughout the codebase",
                    &plugin_name,
                    metadata,
                ));
            }

            if capabilities.intelligence.hover {
                tools.push(self.create_tool_description(
                    "get_hover",
                    "Get hover information for a symbol",
                    &plugin_name,
                    metadata,
                ));
            }

            if capabilities.intelligence.completions {
                tools.push(self.create_tool_description(
                    "get_completions",
                    "Get code completions at a position",
                    &plugin_name,
                    metadata,
                ));
            }

            // Add more capabilities as needed...

            // Add custom capabilities
            for custom_method in capabilities.custom.keys() {
                tools.push(self.create_tool_description(
                    custom_method,
                    &format!("Custom {} method", custom_method),
                    &plugin_name,
                    metadata,
                ));
            }
        }

        // Remove duplicates (multiple plugins might support the same tool)
        tools.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
        tools.dedup_by(|a, b| a["name"] == b["name"]);

        Ok(json!({ "tools": tools }))
    }

    /// Create a tool description for the tools/list response
    fn create_tool_description(
        &self,
        tool_name: &str,
        description: &str,
        plugin_name: &str,
        metadata: Option<&cb_plugins::PluginMetadata>,
    ) -> Value {
        let mut tool = json!({
            "name": tool_name,
            "description": description,
            "plugin": plugin_name,
            "parameters": {
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to the file"
                    }
                },
                "required": ["file_path"]
            }
        });

        // Add plugin metadata if available
        if let Some(meta) = metadata {
            tool["plugin_info"] = json!({
                "name": meta.name,
                "version": meta.version,
                "author": meta.author,
                "description": meta.description
            });
        }

        // Add method-specific parameters
        match tool_name {
            "find_definition" | "find_references" => {
                tool["parameters"]["properties"]["line"] = json!({
                    "type": "number",
                    "description": "Line number (1-indexed)"
                });
                tool["parameters"]["properties"]["character"] = json!({
                    "type": "number",
                    "description": "Character position (0-indexed)"
                });
            }
            "rename_symbol" => {
                tool["parameters"]["properties"]["new_name"] = json!({
                    "type": "string",
                    "description": "New name for the symbol"
                });
                tool["parameters"]["required"].as_array_mut().unwrap().push(json!("new_name"));
            }
            "search_workspace_symbols" => {
                tool["parameters"]["properties"]["query"] = json!({
                    "type": "string",
                    "description": "Search query"
                });
                tool["parameters"]["required"].as_array_mut().unwrap().push(json!("query"));
            }
            _ => {}
        }

        tool
    }

    /// Handle tools/call request using plugins
    #[instrument(skip(self, params))]
    async fn handle_tool_call(&self, params: Option<Value>) -> ServerResult<Value> {
        let params = params.ok_or_else(|| ServerError::InvalidRequest("Missing params".into()))?;

        let tool_call: ToolCall = serde_json::from_value(params)
            .map_err(|e| ServerError::InvalidRequest(format!("Invalid tool call: {}", e)))?;

        debug!("Calling tool '{}' with plugin system", tool_call.name);

        // Convert MCP tool call to plugin request
        let plugin_request = self.convert_tool_call_to_plugin_request(tool_call)?;

        // Handle the request through the plugin system
        let start_time = Instant::now();
        match self.plugin_manager.handle_request(plugin_request).await {
            Ok(response) => {
                let processing_time = start_time.elapsed().as_millis();
                debug!("Plugin request processed in {}ms", processing_time);

                Ok(json!({
                    "content": response.data.unwrap_or(json!(null)),
                    "plugin": response.metadata.plugin_name,
                    "processing_time_ms": response.metadata.processing_time_ms,
                    "cached": response.metadata.cached
                }))
            }
            Err(err) => {
                error!("Plugin request failed: {}", err);
                Err(self.convert_plugin_error_to_server_error(err))
            }
        }
    }

    /// Convert MCP tool call to plugin request
    fn convert_tool_call_to_plugin_request(&self, tool_call: ToolCall) -> ServerResult<PluginRequest> {
        let args = tool_call.arguments.unwrap_or(json!({}));

        // Extract file path
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing file_path parameter".into()))?;

        let mut request = PluginRequest::new(tool_call.name, PathBuf::from(file_path));

        // Extract position if available
        if let (Some(line), Some(character)) = (
            args.get("line").and_then(|v| v.as_u64()),
            args.get("character").and_then(|v| v.as_u64()),
        ) {
            request = request.with_position(line as u32 - 1, character as u32); // Convert to 0-indexed
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
            ); // Convert to 0-indexed
        }

        // Set parameters
        request = request.with_params(args);

        Ok(request)
    }

    /// Convert plugin error to server error
    fn convert_plugin_error_to_server_error(&self, error: PluginError) -> ServerError {
        match error {
            PluginError::PluginNotFound { file, method } => {
                ServerError::Unsupported(format!("No plugin found for file '{}' and method '{}'", file, method))
            }
            PluginError::MethodNotSupported { method, plugin } => {
                ServerError::Unsupported(format!("Method '{}' not supported by plugin '{}'", method, plugin))
            }
            PluginError::PluginRequestFailed { plugin, message } => {
                ServerError::Internal(format!("Plugin '{}' failed: {}", plugin, message))
            }
            PluginError::ConfigurationError { message } => {
                ServerError::InvalidRequest(format!("Configuration error: {}", message))
            }
            _ => ServerError::Internal(format!("Plugin error: {}", error)),
        }
    }

    /// Get plugin manager for advanced operations
    pub fn plugin_manager(&self) -> &PluginManager {
        &self.plugin_manager
    }

    /// Check if a method is supported for a file
    pub async fn is_method_supported(&self, file_path: &std::path::Path, method: &str) -> bool {
        self.initialize().await.is_ok() &&
        self.plugin_manager.is_method_supported(file_path, method).await
    }

    /// Get supported file extensions
    pub async fn get_supported_extensions(&self) -> Vec<String> {
        if self.initialize().await.is_ok() {
            self.plugin_manager.get_supported_extensions().await
        } else {
            Vec::new()
        }
    }

    /// Get plugin statistics for monitoring
    pub async fn get_plugin_statistics(&self) -> ServerResult<Value> {
        self.initialize().await?;

        let registry_stats = self.plugin_manager.get_registry_statistics().await;
        let metrics = self.plugin_manager.get_metrics().await;
        let plugins = self.plugin_manager.list_plugins().await;

        Ok(json!({
            "registry": {
                "total_plugins": registry_stats.total_plugins,
                "supported_extensions": registry_stats.supported_extensions,
                "supported_methods": registry_stats.supported_methods,
                "average_methods_per_plugin": registry_stats.average_methods_per_plugin
            },
            "metrics": {
                "total_requests": metrics.total_requests,
                "successful_requests": metrics.successful_requests,
                "failed_requests": metrics.failed_requests,
                "average_processing_time_ms": metrics.average_processing_time_ms,
                "requests_per_plugin": metrics.requests_per_plugin,
                "processing_time_per_plugin": metrics.processing_time_per_plugin
            },
            "plugins": plugins
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::LspManager;
    use cb_core::config::LspConfig;
    use tempfile::TempDir;

    fn create_test_app_state() -> Arc<AppState> {
        let lsp_config = LspConfig::default();
        let lsp_manager = Arc::new(LspManager::new(lsp_config));
        let temp_dir = TempDir::new().unwrap();
        let file_service = Arc::new(crate::services::FileService::new(temp_dir.path().to_path_buf()));
        let project_root = temp_dir.path().to_path_buf();
        let lock_manager = Arc::new(crate::services::LockManager::new());
        let operation_queue = Arc::new(crate::services::OperationQueue::new(lock_manager.clone()));

        Arc::new(AppState {
            lsp: lsp_manager,
            file_service,
            project_root,
            lock_manager,
            operation_queue,
        })
    }

    #[tokio::test]
    async fn test_plugin_dispatcher_initialization() {
        let app_state = create_test_app_state();
        let dispatcher = PluginDispatcher::new(app_state);

        // Initialize should succeed
        assert!(dispatcher.initialize().await.is_ok());

        // Should have registered plugins
        let plugins = dispatcher.plugin_manager.list_plugins().await;
        assert!(!plugins.is_empty());
        assert!(plugins.contains(&"typescript".to_string()));
        assert!(plugins.contains(&"python".to_string()));
    }

    #[tokio::test]
    async fn test_tools_list() {
        let app_state = create_test_app_state();
        let dispatcher = PluginDispatcher::new(app_state);

        let request = McpRequest {
            id: Some(json!(1)),
            method: "tools/list".to_string(),
            params: None,
        };

        let response = dispatcher.dispatch(McpMessage::Request(request)).await.unwrap();

        if let McpMessage::Response(resp) = response {
            assert!(resp.result.is_some());
            let result = resp.result.unwrap();
            assert!(result["tools"].is_array());

            let tools = result["tools"].as_array().unwrap();
            assert!(!tools.is_empty());

            // Should have common tools
            let tool_names: Vec<&str> = tools
                .iter()
                .filter_map(|t| t["name"].as_str())
                .collect();
            assert!(tool_names.contains(&"find_definition"));
        } else {
            panic!("Expected Response message");
        }
    }

    #[tokio::test]
    async fn test_method_support_checking() {
        let app_state = create_test_app_state();
        let dispatcher = PluginDispatcher::new(app_state);

        assert!(dispatcher.initialize().await.is_ok());

        // TypeScript file should support find_definition
        let ts_file = std::path::Path::new("test.ts");
        assert!(dispatcher.is_method_supported(ts_file, "find_definition").await);

        // Unknown extension should not be supported
        let unknown_file = std::path::Path::new("test.unknown");
        assert!(!dispatcher.is_method_supported(unknown_file, "find_definition").await);
    }

    #[tokio::test]
    async fn test_plugin_statistics() {
        let app_state = create_test_app_state();
        let dispatcher = PluginDispatcher::new(app_state);

        let stats = dispatcher.get_plugin_statistics().await.unwrap();

        assert!(stats["registry"]["total_plugins"].as_u64().unwrap() > 0);
        assert!(stats["registry"]["supported_extensions"].as_u64().unwrap() > 0);
        assert!(stats["plugins"].is_array());
    }
}