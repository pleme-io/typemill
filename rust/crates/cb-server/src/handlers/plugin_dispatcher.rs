//! Plugin-based MCP message dispatcher
//!
//! This is the new plugin-based dispatcher that replaces the monolithic
//! dispatcher with a flexible plugin system.

use crate::error::{ServerError, ServerResult};
use crate::interfaces::AstService;
use crate::mcp_tools;
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
use tracing::{debug, error, info, instrument, warn};

/// Application state containing services
#[derive(Clone)]
pub struct AppState {
    /// AST service for code analysis and parsing
    pub ast_service: Arc<dyn AstService>,
    /// File service for file operations with import awareness
    pub file_service: Arc<crate::services::FileService>,
    /// Project root directory
    pub project_root: std::path::PathBuf,
    /// Lock manager for file-level locking
    pub lock_manager: Arc<crate::services::LockManager>,
    /// Operation queue for serializing file operations
    pub operation_queue: Arc<crate::services::OperationQueue>,
}

/// Plugin-based MCP dispatcher
pub struct PluginDispatcher {
    /// Plugin manager for handling requests
    plugin_manager: Arc<PluginManager>,
    /// Application state for file operations and services beyond LSP
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
    fn extract_extension_from_params(&self, params: &Value, method: &str) -> Option<String> {
        // For workspace-level operations, return the first supported extension
        // since they don't operate on specific files
        match method {
            "workspace/symbol" => {
                // Workspace symbol search - use TypeScript client as default
                if self.extensions.contains(&"ts".to_string()) {
                    return Some("ts".to_string());
                } else if !self.extensions.is_empty() {
                    return Some(self.extensions[0].clone());
                }
                return None;
            }
            _ => {
                // For file-specific operations, extract from textDocument.uri
                if let Some(uri) = params.get("textDocument")?.get("uri")?.as_str() {
                    if uri.starts_with("file://") {
                        let path = uri.trim_start_matches("file://");
                        return std::path::Path::new(path).extension()?.to_str().map(|s| s.to_string());
                    }
                }
                None
            }
        }
    }
}

#[async_trait]
impl LspService for DirectLspAdapter {
    async fn request(&self, method: &str, params: Value) -> Result<Value, String> {
        // Extract extension from params
        let extension = self.extract_extension_from_params(&params, method)
            .ok_or_else(|| format!("Could not extract file extension from params for method '{}'", method))?;

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
        debug!("PluginDispatcher::initialize() called");
        self.initialized.get_or_try_init(|| async {
            debug!("Inside initialize - loading app config");
            info!("Initializing plugin system with DirectLspAdapter (bypassing hard-coded mappings)");

            // Get LSP configuration from app config
            let app_config = cb_core::config::AppConfig::load()
                .map_err(|e| {
                    error!("Failed to load app config: {}", e);
                    ServerError::Internal(format!("Failed to load app config: {}", e))
                })?;
            debug!("App config loaded successfully");
            let lsp_config = app_config.lsp;

            // Dynamically register plugins based on configured LSP servers
            let mut registered_plugins = 0;
            for server_config in &lsp_config.servers {
                if server_config.extensions.is_empty() {
                    warn!("LSP server config has no extensions, skipping: {:?}", server_config.command);
                    continue;
                }

                // Create a DirectLspAdapter for this server
                let adapter_name = format!("{}-lsp-direct", server_config.extensions.join("-"));
                debug!("Creating LSP adapter for extensions: {:?}", server_config.extensions);

                let lsp_adapter = Arc::new(DirectLspAdapter::new(
                    lsp_config.clone(),
                    server_config.extensions.clone(),
                    adapter_name.clone(),
                ));

                // Determine plugin type based on primary extension
                let primary_extension = &server_config.extensions[0];
                let (plugin_name, plugin) = match primary_extension.as_str() {
                    "ts" | "tsx" | "js" | "jsx" => {
                        debug!("Creating TypeScript plugin for extensions: {:?}", server_config.extensions);
                        ("typescript".to_string(), Arc::new(LspAdapterPlugin::typescript(lsp_adapter)))
                    }
                    "py" | "pyi" => {
                        debug!("Creating Python plugin for extensions: {:?}", server_config.extensions);
                        ("python".to_string(), Arc::new(LspAdapterPlugin::python(lsp_adapter)))
                    }
                    "go" => {
                        debug!("Creating Go plugin for extensions: {:?}", server_config.extensions);
                        ("go".to_string(), Arc::new(LspAdapterPlugin::go(lsp_adapter)))
                    }
                    "rs" => {
                        debug!("Creating Rust plugin for extensions: {:?}", server_config.extensions);
                        ("rust".to_string(), Arc::new(LspAdapterPlugin::rust(lsp_adapter)))
                    }
                    _ => {
                        // Generic plugin for unknown languages
                        debug!("Creating generic plugin for extensions: {:?}", server_config.extensions);
                        let generic_name = format!("{}-generic", primary_extension);
                        (generic_name.clone(), Arc::new(LspAdapterPlugin::new(
                            generic_name,
                            server_config.extensions.clone(),
                            lsp_adapter,
                        )))
                    }
                };

                debug!("Registering {} plugin for extensions: {:?}", plugin_name, server_config.extensions);
                self.plugin_manager
                    .register_plugin(&plugin_name, plugin)
                    .await
                    .map_err(|e| {
                        error!("Failed to register {} plugin: {}", plugin_name, e);
                        ServerError::Internal(format!("Failed to register {} plugin: {}", plugin_name, e))
                    })?;

                registered_plugins += 1;
                debug!("{} plugin registered successfully", plugin_name);
            }

            // Register System Tools plugin for workspace-level operations
            let system_plugin = Arc::new(cb_plugins::system_tools_plugin::SystemToolsPlugin::new());
            self.plugin_manager
                .register_plugin("system", system_plugin)
                .await
                .map_err(|e| ServerError::Internal(format!("Failed to register System tools plugin: {}", e)))?;
            registered_plugins += 1;

            info!("Plugin system initialized successfully with {} plugins ({} language + 1 system)",
                  registered_plugins, registered_plugins - 1);
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
                    jsonrpc: "2.0".to_string(),
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
            "initialize" => self.handle_initialize(request.params).await?,
            "initialized" | "notifications/initialized" => self.handle_initialized().await?,
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
                    jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(response),
            error: None,
        }))
    }

    /// Handle tools/list request using Rust-native tool definitions
    #[instrument(skip(self))]
    async fn handle_list_tools(&self) -> ServerResult<Value> {
        let tools = mcp_tools::get_tool_definitions();
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
                if let Some(array) = tool["parameters"]["required"].as_array_mut() {
                    array.push(json!("new_name"));
                } else {
                    warn!("Could not add 'new_name' to required parameters for rename_symbol");
                }
            }
            "search_workspace_symbols" => {
                tool["parameters"]["properties"]["query"] = json!({
                    "type": "string",
                    "description": "Search query"
                });
                if let Some(array) = tool["parameters"]["required"].as_array_mut() {
                    array.push(json!("query"));
                } else {
                    warn!("Could not add 'query' to required parameters for search_workspace_symbols");
                }
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

        // Check if this is a file operation that needs app_state services
        if self.is_file_operation(&tool_call.name) {
            return self.handle_file_operation(tool_call).await;
        }

        // Check if this is an LSP notification tool
        if tool_call.name == "notify_file_opened" {
            return self.handle_notify_file_opened(tool_call).await;
        }

        // Check if this is the AST-powered refactoring tool
        if tool_call.name == "rename_symbol_with_imports" {
            return self.handle_rename_symbol_with_imports(tool_call).await;
        }

        // Check if this is a system tool
        if self.is_system_tool(&tool_call.name) {
            return self.handle_system_tool(tool_call).await;
        }

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

        // Handle workspace-level operations that don't require a file path
        let file_path = match tool_call.name.as_str() {
            "search_workspace_symbols" => {
                // Use a dummy file path for workspace symbols
                PathBuf::from(".")
            }
            _ => {
                // Extract file path for file-specific operations
                let file_path_str = args
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ServerError::InvalidRequest("Missing file_path parameter".into()))?;
                PathBuf::from(file_path_str)
            }
        };

        let mut request = PluginRequest::new(tool_call.name, file_path);

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

    /// Handle MCP initialize request
    async fn handle_initialize(&self, _params: Option<Value>) -> ServerResult<Value> {
        debug!("Handling MCP initialize request");

        // Return server capabilities - using latest protocol version
        Ok(json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "codeflow-buddy",
                "version": "0.1.0"
            }
        }))
    }

    /// Handle MCP initialized notification
    async fn handle_initialized(&self) -> ServerResult<Value> {
        debug!("Handling MCP initialized notification");

        // Server is ready - return empty response for notification
        Ok(json!({}))
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

    /// Check if a tool name represents a file operation
    fn is_file_operation(&self, tool_name: &str) -> bool {
        matches!(tool_name, "rename_file" | "create_file" | "delete_file" | "rename_directory")
    }

    /// Check if a tool name represents a system tool
    fn is_system_tool(&self, tool_name: &str) -> bool {
        matches!(tool_name, "list_files" | "analyze_imports" | "find_dead_code")
    }

    /// Handle system tools through the plugin system
    async fn handle_system_tool(&self, tool_call: ToolCall) -> ServerResult<Value> {
        debug!("Handling system tool: {}", tool_call.name);

        // Create a plugin request for system tools
        // System tools don't require a file_path, so use a dummy path
        let mut request = PluginRequest::new(
            tool_call.name.clone(),
            PathBuf::from("."), // Dummy path for system tools
        );

        // Pass through all arguments as params
        request.params = tool_call.arguments.unwrap_or(json!({}));

        // Route to the system plugin
        let start_time = Instant::now();
        match self.plugin_manager.handle_request(request).await {
            Ok(response) => {
                let processing_time = start_time.elapsed().as_millis();
                debug!("System tool processed in {}ms", processing_time);

                Ok(json!({
                    "content": response.data.unwrap_or(json!(null)),
                    "plugin": response.metadata.plugin_name,
                    "processing_time_ms": response.metadata.processing_time_ms,
                }))
            }
            Err(e) => {
                warn!("System tool error: {}", e);
                Err(ServerError::Runtime {
                    message: format!("Tool '{}' failed: {}", tool_call.name, e),
                })
            }
        }
    }

    /// Handle LSP file notification tool
    async fn handle_notify_file_opened(&self, tool_call: ToolCall) -> ServerResult<Value> {
        debug!("Handling notify_file_opened: {}", tool_call.name);

        let args = tool_call.arguments.unwrap_or(json!({}));
        let file_path_str = args.get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'file_path' parameter".into()))?;

        let file_path = PathBuf::from(file_path_str);

        // Get file extension to determine which LSP adapter to notify
        let extension = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        // Load LSP config to create a temporary DirectLspAdapter for notification
        let app_config = cb_core::config::AppConfig::load()
            .map_err(|e| ServerError::Internal(format!("Failed to load app config: {}", e)))?;
        let lsp_config = app_config.lsp;

        // Find the server config for this extension
        if let Some(_server_config) = lsp_config.servers.iter()
            .find(|server| server.extensions.contains(&extension.to_string())) {

            // Create a temporary DirectLspAdapter to handle the notification
            let adapter = DirectLspAdapter::new(
                lsp_config,
                vec![extension.to_string()],
                format!("temp-{}-notifier", extension),
            );

            // Get or create LSP client and notify
            match adapter.get_or_create_client(extension).await {
                Ok(client) => {
                    match client.notify_file_opened(&file_path).await {
                        Ok(()) => {
                            debug!("Successfully notified LSP server about file: {}", file_path.display());
                            Ok(json!({
                                "success": true,
                                "message": format!("Notified LSP server about file: {}", file_path.display())
                            }))
                        }
                        Err(e) => {
                            warn!("Failed to notify LSP server about file {}: {}", file_path.display(), e);
                            Err(ServerError::Runtime {
                                message: format!("Failed to notify LSP server: {}", e),
                            })
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to get LSP client for extension '{}': {}", extension, e);
                    Err(ServerError::Runtime {
                        message: format!("Failed to get LSP client: {}", e),
                    })
                }
            }
        } else {
            debug!("No LSP server configured for extension '{}'", extension);
            Ok(json!({
                "success": true,
                "message": format!("No LSP server configured for extension '{}'", extension)
            }))
        }
    }

    /// Handle rename_symbol_with_imports tool using AST service
    async fn handle_rename_symbol_with_imports(&self, tool_call: ToolCall) -> ServerResult<Value> {
        debug!("Handling rename_symbol_with_imports: {}", tool_call.name);

        let args = tool_call.arguments.unwrap_or(json!({}));
        let file_path_str = args.get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'file_path' parameter".into()))?;

        let old_name = args.get("old_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'old_name' parameter".into()))?;

        let new_name = args.get("new_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'new_name' parameter".into()))?;

        let file_path = std::path::Path::new(file_path_str);

        debug!("Planning refactor to rename '{}' to '{}' in file: {}", old_name, new_name, file_path.display());

        // Create an IntentSpec for the rename operation
        let intent = cb_core::model::IntentSpec::new(
            "rename_symbol_with_imports",
            json!({
                "oldName": old_name,
                "newName": new_name,
                "sourceFile": file_path_str
            })
        );

        // Use the AST service to plan the refactoring
        match self.app_state.ast_service.plan_refactor(&intent, file_path).await {
            Ok(edit_plan) => {
                debug!("Successfully planned refactor for {} files", edit_plan.edits.len());
                Ok(json!({
                    "success": true,
                    "message": format!("Successfully planned rename of '{}' to '{}' affecting {} edits across {} files",
                                     old_name, new_name, edit_plan.edits.len(), edit_plan.metadata.impact_areas.len()),
                    "edit_plan": edit_plan
                }))
            }
            Err(e) => {
                error!("Failed to plan refactor for '{}' -> '{}': {}", old_name, new_name, e);
                Err(ServerError::Runtime {
                    message: format!("Failed to plan refactor: {}", e),
                })
            }
        }
    }

    /// Handle file operations using app_state services
    async fn handle_file_operation(&self, tool_call: ToolCall) -> ServerResult<Value> {
        debug!("Handling file operation: {}", tool_call.name);

        match tool_call.name.as_str() {
            "rename_file" => {
                let args = tool_call.arguments
                    .ok_or_else(|| ServerError::InvalidRequest("Missing arguments for rename_file".into()))?;
                let old_path = args.get("old_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ServerError::InvalidRequest("Missing 'old_path' parameter".into()))?;
                let new_path = args.get("new_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ServerError::InvalidRequest("Missing 'new_path' parameter".into()))?;
                let dry_run = args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false);

                let result = self.app_state.file_service
                    .rename_file_with_imports(
                        std::path::Path::new(old_path),
                        std::path::Path::new(new_path),
                        dry_run
                    ).await?;

                let imports_updated = result.import_updates
                    .as_ref()
                    .map(|r| r.imports_updated)
                    .unwrap_or(0);
                let files_affected = result.import_updates
                    .as_ref()
                    .map(|r| r.files_updated)
                    .unwrap_or(0);

                Ok(json!({
                    "success": true,
                    "old_path": old_path,
                    "new_path": new_path,
                    "imports_updated": imports_updated,
                    "files_affected": files_affected
                }))
            }
            "create_file" => {
                let args = tool_call.arguments
                    .ok_or_else(|| ServerError::InvalidRequest("Missing arguments for create_file".into()))?;
                let file_path = args.get("file_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ServerError::InvalidRequest("Missing 'file_path' parameter".into()))?;
                let content = args.get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                tokio::fs::write(file_path, content).await
                    .map_err(|e| ServerError::Internal(format!("Failed to create file: {}", e)))?;

                Ok(json!({
                    "success": true,
                    "file_path": file_path,
                    "created": true
                }))
            }
            "delete_file" => {
                let args = tool_call.arguments
                    .ok_or_else(|| ServerError::InvalidRequest("Missing arguments for delete_file".into()))?;
                let file_path = args.get("file_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ServerError::InvalidRequest("Missing 'file_path' parameter".into()))?;

                tokio::fs::remove_file(file_path).await
                    .map_err(|e| ServerError::Internal(format!("Failed to delete file: {}", e)))?;

                Ok(json!({
                    "success": true,
                    "file_path": file_path,
                    "deleted": true
                }))
            }
            _ => Err(ServerError::Unsupported(format!("File operation '{}' not implemented", tool_call.name)))
        }
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
    use tempfile::TempDir;

    fn create_test_app_state() -> Arc<AppState> {
        let temp_dir = TempDir::new().unwrap();
        let ast_service = Arc::new(crate::services::DefaultAstService::new());
        let file_service = Arc::new(crate::services::FileService::new(temp_dir.path().to_path_buf()));
        let project_root = temp_dir.path().to_path_buf();
        let lock_manager = Arc::new(crate::services::LockManager::new());
        let operation_queue = Arc::new(crate::services::OperationQueue::new(lock_manager.clone()));

        Arc::new(AppState {
            ast_service,
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
            jsonrpc: "2.0".to_string(),
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