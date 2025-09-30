//! Plugin-based MCP message dispatcher
//!
//! This is the new plugin-based dispatcher that replaces the monolithic
//! dispatcher with a flexible plugin system.

use crate::services::planner::Planner;
use crate::services::workflow_executor::WorkflowExecutor;
use crate::{ServerError, ServerResult};
use async_trait::async_trait;
use cb_api::AstService;
use cb_core::model::mcp::{McpMessage, McpRequest, McpResponse, ToolCall};
use cb_plugins::{LspAdapterPlugin, LspService, PluginError, PluginManager, PluginRequest};
use cb_transport::McpDispatcher;
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
    /// Planner service for generating workflows from intents
    pub planner: Arc<dyn Planner>,
    /// Workflow executor for running planned workflows
    pub workflow_executor: Arc<dyn WorkflowExecutor>,
    /// Project root directory
    pub project_root: std::path::PathBuf,
    /// Lock manager for file-level locking
    pub lock_manager: Arc<crate::services::LockManager>,
    /// Operation queue for serializing file operations
    pub operation_queue: Arc<crate::services::OperationQueue>,
    /// Server start time for uptime calculation
    pub start_time: Instant,
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
    async fn get_or_create_client(
        &self,
        extension: &str,
    ) -> Result<Arc<crate::systems::lsp::LspClient>, String> {
        // Check if client already exists
        {
            let clients = self.lsp_clients.lock().await;
            if let Some(client) = clients.get(extension) {
                return Ok(client.clone());
            }
        }

        // Find server config for this extension
        let server_config = self
            .config
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

    /// Query all active LSP servers for workspace symbols and merge results
    async fn query_all_servers_for_workspace_symbols(
        &self,
        params: Value,
    ) -> Result<Value, String> {
        const MAX_WORKSPACE_SYMBOLS: usize = 10_000;
        let mut all_symbols = Vec::new();
        let mut queried_servers = Vec::new();

        // Query each supported extension's LSP server
        for extension in &self.extensions {
            // Get or create client for this extension
            match self.get_or_create_client(extension).await {
                Ok(client) => {
                    // Send workspace/symbol request to this server
                    match client.send_request("workspace/symbol", params.clone()).await {
                        Ok(response) => {
                            // Extract symbols from response
                            if let Some(symbols) = response.as_array() {
                                debug!(
                                    extension = %extension,
                                    symbol_count = symbols.len(),
                                    "Got workspace symbols from LSP server"
                                );
                                all_symbols.extend_from_slice(symbols);
                                queried_servers.push(extension.clone());

                                // Prevent unbounded symbol collection
                                if all_symbols.len() >= MAX_WORKSPACE_SYMBOLS {
                                    debug!(
                                        symbol_count = all_symbols.len(),
                                        "Reached maximum workspace symbol limit, stopping collection"
                                    );
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            // Log error but continue with other servers
                            warn!(
                                extension = %extension,
                                error = %e,
                                "Failed to get workspace symbols from LSP server"
                            );
                        }
                    }
                }
                Err(e) => {
                    // Log error but continue with other servers
                    warn!(
                        extension = %extension,
                        error = %e,
                        "Failed to create LSP client for workspace symbol search"
                    );
                }
            }
        }

        if all_symbols.is_empty() {
            return Ok(json!([]));
        }

        debug!(
            total_symbols = all_symbols.len(),
            servers = ?queried_servers,
            "Merged workspace symbols from multiple LSP servers"
        );

        Ok(json!(all_symbols))
    }

    /// Extract file extension from LSP params
    fn extract_extension_from_params(&self, params: &Value, method: &str) -> Option<String> {
        // For workspace-level operations, no longer needed since we handle them specially
        match method {
            "workspace/symbol" => {
                // This path should not be reached anymore - handled in request() method
                warn!("extract_extension_from_params called for workspace/symbol - should be handled specially");
                None
            }
            _ => {
                // For file-specific operations, extract from textDocument.uri
                if let Some(uri) = params.get("textDocument")?.get("uri")?.as_str() {
                    if uri.starts_with("file://") {
                        let path = uri.trim_start_matches("file://");
                        return std::path::Path::new(path)
                            .extension()?
                            .to_str()
                            .map(|s| s.to_string());
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
        // Special handling for workspace/symbol - query ALL active LSP servers
        if method == "workspace/symbol" {
            return self.query_all_servers_for_workspace_symbols(params).await;
        }

        // Extract extension from params for file-specific operations
        let extension = self
            .extract_extension_from_params(&params, method)
            .ok_or_else(|| {
                format!(
                    "Could not extract file extension from params for method '{}'",
                    method
                )
            })?;

        // Get appropriate LSP client
        let client = self.get_or_create_client(&extension).await?;

        // Send LSP method DIRECTLY to client (bypassing old manager and its hard-coded mappings!)
        client
            .send_request(method, params)
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
    pub fn new(app_state: Arc<AppState>, plugin_manager: Arc<PluginManager>) -> Self {
        Self {
            plugin_manager,
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
                    error!(error = %e, "Failed to load app config");
                    ServerError::Internal(format!("Failed to load app config: {}", e))
                })?;
            debug!("App config loaded successfully");
            let lsp_config = app_config.lsp;

            // Dynamically register plugins based on configured LSP servers
            let mut registered_plugins = 0;
            for server_config in &lsp_config.servers {
                if server_config.extensions.is_empty() {
                    warn!(command = ?server_config.command, "LSP server config has no extensions, skipping");
                    continue;
                }

                // Create a DirectLspAdapter for this server
                let adapter_name = format!("{}-lsp-direct", server_config.extensions.join("-"));
                debug!(extensions = ?server_config.extensions, "Creating LSP adapter");

                let lsp_adapter = Arc::new(DirectLspAdapter::new(
                    lsp_config.clone(),
                    server_config.extensions.clone(),
                    adapter_name.clone(),
                ));

                // Determine plugin type based on primary extension
                let primary_extension = &server_config.extensions[0];
                let (plugin_name, plugin) = match primary_extension.as_str() {
                    "ts" | "tsx" | "js" | "jsx" => {
                        debug!(extensions = ?server_config.extensions, "Creating TypeScript plugin");
                        ("typescript".to_string(), Arc::new(LspAdapterPlugin::typescript(lsp_adapter)))
                    }
                    "py" | "pyi" => {
                        debug!(extensions = ?server_config.extensions, "Creating Python plugin");
                        ("python".to_string(), Arc::new(LspAdapterPlugin::python(lsp_adapter)))
                    }
                    "go" => {
                        debug!(extensions = ?server_config.extensions, "Creating Go plugin");
                        ("go".to_string(), Arc::new(LspAdapterPlugin::go(lsp_adapter)))
                    }
                    "rs" => {
                        debug!(extensions = ?server_config.extensions, "Creating Rust plugin");
                        ("rust".to_string(), Arc::new(LspAdapterPlugin::rust(lsp_adapter)))
                    }
                    _ => {
                        // Generic plugin for unknown languages
                        debug!(extensions = ?server_config.extensions, "Creating generic plugin");
                        let generic_name = format!("{}-generic", primary_extension);
                        (generic_name.clone(), Arc::new(LspAdapterPlugin::new(
                            generic_name,
                            server_config.extensions.clone(),
                            lsp_adapter,
                        )))
                    }
                };

                debug!(plugin_name = %plugin_name, extensions = ?server_config.extensions, "Registering plugin");
                self.plugin_manager
                    .register_plugin(&plugin_name, plugin)
                    .await
                    .map_err(|e| {
                        error!(plugin_name = %plugin_name, error = %e, "Failed to register plugin");
                        ServerError::Internal(format!("Failed to register {} plugin: {}", plugin_name, e))
                    })?;

                registered_plugins += 1;
                debug!(plugin_name = %plugin_name, "Plugin registered successfully");
            }

            // Register System Tools plugin for workspace-level operations
            let system_plugin = Arc::new(cb_plugins::system_tools_plugin::SystemToolsPlugin::new());
            self.plugin_manager
                .register_plugin("system", system_plugin)
                .await
                .map_err(|e| ServerError::Internal(format!("Failed to register System tools plugin: {}", e)))?;
            registered_plugins += 1;

            info!(
                total_plugins = registered_plugins,
                language_plugins = registered_plugins - 1,
                "Plugin system initialized successfully"
            );
            Ok::<(), ServerError>(())
        }).await?;

        Ok(())
    }

    /// Dispatch an MCP message using the plugin system
    #[instrument(skip(self, message), fields(request_id = %uuid::Uuid::new_v4()))]
    pub async fn dispatch(&self, message: McpMessage) -> ServerResult<McpMessage> {
        // Ensure initialization
        self.initialize().await?;

        match message {
            McpMessage::Request(request) => self.handle_request(request).await,
            McpMessage::Response(response) => Ok(McpMessage::Response(response)),
            McpMessage::Notification(notification) => {
                debug!(
                    notification_method = %notification.method,
                    "Received notification"
                );
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
    #[instrument(skip(self, request), fields(method = %request.method))]
    async fn handle_request(&self, request: McpRequest) -> ServerResult<McpMessage> {
        debug!(
            method = %request.method,
            has_params = request.params.is_some(),
            "Handling request"
        );

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

    /// Handle tools/list request using plugin-provided tool definitions
    #[instrument(skip(self))]
    async fn handle_list_tools(&self) -> ServerResult<Value> {
        let tools = self.plugin_manager.get_all_tool_definitions().await;
        Ok(json!({ "tools": tools }))
    }

    /// Handle tools/call request using plugins
    #[instrument(skip(self, params))]
    async fn handle_tool_call(&self, params: Option<Value>) -> ServerResult<Value> {
        let params = params.ok_or_else(|| ServerError::InvalidRequest("Missing params".into()))?;

        let tool_call: ToolCall = serde_json::from_value(params)
            .map_err(|e| ServerError::InvalidRequest(format!("Invalid tool call: {}", e)))?;

        debug!(tool_name = %tool_call.name, "Calling tool with plugin system");

        // Check if this is an intent to be planned into a workflow
        if tool_call.name == "achieve_intent" {
            return self.handle_achieve_intent(tool_call).await;
        }

        // Check if this is a health check request
        if tool_call.name == "health_check" {
            return self.handle_health_check().await;
        }

        // Check if this is a file operation that needs app_state services
        if self.is_file_operation(&tool_call.name) {
            return self.handle_file_operation(tool_call).await;
        }

        // Check if this is an LSP notification tool
        if tool_call.name == "notify_file_opened" {
            return self.handle_notify_file_opened(tool_call).await;
        }

        if tool_call.name == "notify_file_saved" {
            return self.handle_notify_file_saved(tool_call).await;
        }

        if tool_call.name == "notify_file_closed" {
            return self.handle_notify_file_closed(tool_call).await;
        }

        // Check if this is the AST-powered refactoring tool
        if tool_call.name == "rename_symbol_with_imports" {
            return self.handle_rename_symbol_with_imports(tool_call).await;
        }

        // Check if this is the edit plan application tool
        if tool_call.name == "apply_edits" {
            return self.handle_apply_edits(tool_call).await;
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
                debug!(
                    processing_time_ms = processing_time,
                    "Plugin request processed"
                );

                Ok(json!({
                    "content": response.data.unwrap_or(json!(null)),
                    "plugin": response.metadata.plugin_name,
                    "processing_time_ms": response.metadata.processing_time_ms,
                    "cached": response.metadata.cached
                }))
            }
            Err(err) => {
                error!(error = %err, "Plugin request failed");
                Err(self.convert_plugin_error_to_server_error(err))
            }
        }
    }

    /// Convert MCP tool call to plugin request
    fn convert_tool_call_to_plugin_request(
        &self,
        tool_call: ToolCall,
    ) -> ServerResult<PluginRequest> {
        let args = tool_call.arguments.unwrap_or(json!({}));

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
                            ServerError::InvalidRequest("Missing file_path parameter".into())
                        })?;
                PathBuf::from(file_path_str)
            }
        };

        let mut request = PluginRequest::new(tool_call.name, file_path);

        // Extract position if available
        // If line or character are provided, they must be valid numbers
        let line_opt = match args.get("line") {
            Some(val) if !val.is_null() => {
                Some(val.as_u64().ok_or_else(|| {
                    ServerError::InvalidRequest(format!(
                        "Parameter 'line' must be a number, got: {}",
                        val
                    ))
                })?)
            }
            _ => None,
        };

        let character_opt = match args.get("character") {
            Some(val) if !val.is_null() => {
                Some(val.as_u64().ok_or_else(|| {
                    ServerError::InvalidRequest(format!(
                        "Parameter 'character' must be a number, got: {}",
                        val
                    ))
                })?)
            }
            _ => None,
        };

        if let (Some(line), Some(character)) = (line_opt, character_opt) {
            request = request.with_position(line as u32 - 1, character as u32); // Convert to 0-indexed
        }

        // Extract range if available
        // If range parameters are provided, they must be valid numbers
        let start_line_opt = match args.get("start_line") {
            Some(val) if !val.is_null() => {
                Some(val.as_u64().ok_or_else(|| {
                    ServerError::InvalidRequest(format!(
                        "Parameter 'start_line' must be a number, got: {}",
                        val
                    ))
                })?)
            }
            _ => None,
        };

        let start_char_opt = match args.get("start_character") {
            Some(val) if !val.is_null() => {
                Some(val.as_u64().ok_or_else(|| {
                    ServerError::InvalidRequest(format!(
                        "Parameter 'start_character' must be a number, got: {}",
                        val
                    ))
                })?)
            }
            _ => None,
        };

        let end_line_opt = match args.get("end_line") {
            Some(val) if !val.is_null() => {
                Some(val.as_u64().ok_or_else(|| {
                    ServerError::InvalidRequest(format!(
                        "Parameter 'end_line' must be a number, got: {}",
                        val
                    ))
                })?)
            }
            _ => None,
        };

        let end_char_opt = match args.get("end_character") {
            Some(val) if !val.is_null() => {
                Some(val.as_u64().ok_or_else(|| {
                    ServerError::InvalidRequest(format!(
                        "Parameter 'end_character' must be a number, got: {}",
                        val
                    ))
                })?)
            }
            _ => None,
        };

        if let (Some(start_line), Some(start_char), Some(end_line), Some(end_char)) =
            (start_line_opt, start_char_opt, end_line_opt, end_char_opt)
        {
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
                "name": "codebuddy",
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
            PluginError::PluginNotFound { file, method } => ServerError::Unsupported(format!(
                "No plugin found for file '{}' and method '{}'",
                file, method
            )),
            PluginError::MethodNotSupported { method, plugin } => ServerError::Unsupported(
                format!("Method '{}' not supported by plugin '{}'", method, plugin),
            ),
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
        matches!(
            tool_name,
            "rename_file" | "create_file" | "delete_file" | "read_file" | "write_file"
        )
    }

    /// Check if a tool name represents a system tool
    fn is_system_tool(&self, tool_name: &str) -> bool {
        matches!(
            tool_name,
            "list_files"
                | "analyze_imports"
                | "find_dead_code"
                | "update_dependencies"
                | "rename_directory"
                | "extract_function"
                | "inline_variable"
                | "extract_variable"
                | "fix_imports"
        )
    }

    /// Handle system tools through the plugin system
    async fn handle_system_tool(&self, tool_call: ToolCall) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling system tool");

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
                debug!(
                    processing_time_ms = processing_time,
                    "System tool processed"
                );

                Ok(json!({
                    "content": response.data.unwrap_or(json!(null)),
                    "plugin": response.metadata.plugin_name,
                    "processing_time_ms": response.metadata.processing_time_ms,
                }))
            }
            Err(e) => {
                warn!(error = %e, "System tool error");
                Err(ServerError::Runtime {
                    message: format!("Tool '{}' failed: {}", tool_call.name, e),
                })
            }
        }
    }

    /// Handle LSP file notification tool
    async fn handle_notify_file_opened(&self, tool_call: ToolCall) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling notify_file_opened");

        let args = tool_call.arguments.unwrap_or(json!({}));
        let file_path_str = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'file_path' parameter".into()))?;

        let file_path = PathBuf::from(file_path_str);

        // Trigger plugin lifecycle hooks for all plugins that can handle this file
        if let Err(e) = self
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

    /// Handle LSP file saved notification tool
    async fn handle_notify_file_saved(&self, tool_call: ToolCall) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling notify_file_saved");

        let args = tool_call.arguments.unwrap_or(json!({}));
        let file_path_str = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'file_path' parameter".into()))?;

        let file_path = PathBuf::from(file_path_str);

        // Trigger plugin lifecycle hooks for all plugins that can handle this file
        if let Err(e) = self
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

    /// Handle LSP file closed notification tool
    async fn handle_notify_file_closed(&self, tool_call: ToolCall) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling notify_file_closed");

        let args = tool_call.arguments.unwrap_or(json!({}));
        let file_path_str = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'file_path' parameter".into()))?;

        let file_path = PathBuf::from(file_path_str);

        // Trigger plugin lifecycle hooks for all plugins that can handle this file
        if let Err(e) = self
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

    /// Handle health_check tool call by reporting server status.
    async fn handle_health_check(&self) -> ServerResult<Value> {
        info!("Handling health check request");

        let uptime_secs = self.app_state.start_time.elapsed().as_secs();
        let uptime_mins = uptime_secs / 60;
        let uptime_hours = uptime_mins / 60;

        // Get plugin count from plugin manager
        let plugin_count = self.plugin_manager.get_all_tool_definitions().await.len();

        // Get paused workflow count from executor
        let paused_workflows = self.app_state.workflow_executor.get_paused_workflow_count();

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

    /// Handle achieve_intent tool call by using the Planner service.
    async fn handle_achieve_intent(&self, tool_call: ToolCall) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Planning or resuming workflow");

        let args = tool_call.arguments.ok_or_else(|| {
            ServerError::InvalidRequest("Missing arguments for achieve_intent".into())
        })?;

        // Check if this is a workflow resume request
        if let Some(workflow_id) = args.get("workflow_id").and_then(|v| v.as_str()) {
            info!(workflow_id = %workflow_id, "Resuming paused workflow");

            let resume_data = args.get("resume_data").cloned();

            return self
                .app_state
                .workflow_executor
                .resume_workflow(workflow_id, resume_data)
                .await;
        }

        // Otherwise, plan a new workflow
        let intent_value = args
            .get("intent")
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'intent' parameter".into()))?;

        let intent: cb_core::model::workflow::Intent = serde_json::from_value(intent_value.clone())
            .map_err(|e| ServerError::InvalidRequest(format!("Invalid intent format: {}", e)))?;

        // Check if we should execute the workflow
        let execute = args
            .get("execute")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Check if dry-run mode is requested
        let dry_run = args
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        match self.app_state.planner.plan_for_intent(&intent) {
            Ok(workflow) => {
                info!(
                    intent = %intent.name,
                    workflow_name = %workflow.name,
                    steps = workflow.steps.len(),
                    complexity = workflow.metadata.complexity,
                    execute = execute,
                    dry_run = dry_run,
                    "Successfully planned workflow"
                );

                // If execute is true, run the workflow
                if execute {
                    debug!(dry_run = dry_run, "Executing workflow");
                    match self
                        .app_state
                        .workflow_executor
                        .execute_workflow(&workflow, dry_run)
                        .await
                    {
                        Ok(result) => {
                            info!(
                                workflow_name = %workflow.name,
                                dry_run = dry_run,
                                "Workflow executed successfully"
                            );
                            Ok(result)
                        }
                        Err(e) => {
                            error!(
                                workflow_name = %workflow.name,
                                error = %e,
                                "Workflow execution failed"
                            );
                            Err(e)
                        }
                    }
                } else {
                    // Just return the plan
                    Ok(json!({
                        "success": true,
                        "workflow": workflow,
                    }))
                }
            }
            Err(e) => {
                error!(intent = %intent.name, error = %e, "Failed to plan workflow for intent");
                Err(ServerError::Runtime { message: e })
            }
        }
    }

    /// Handle rename_symbol_with_imports tool using AST service
    async fn handle_rename_symbol_with_imports(&self, tool_call: ToolCall) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling rename_symbol_with_imports");

        let args = tool_call.arguments.unwrap_or(json!({}));
        let file_path_str = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'file_path' parameter".into()))?;

        let old_name = args
            .get("old_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'old_name' parameter".into()))?;

        let new_name = args
            .get("new_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'new_name' parameter".into()))?;

        let file_path = std::path::Path::new(file_path_str);

        debug!(
            old_name = %old_name,
            new_name = %new_name,
            file_path = %file_path.display(),
            "Planning refactor to rename symbol"
        );

        // Create an IntentSpec for the rename operation
        let intent = cb_core::model::IntentSpec::new(
            "rename_symbol_with_imports",
            json!({
                "oldName": old_name,
                "newName": new_name,
                "sourceFile": file_path_str
            }),
        );

        // Use the AST service to plan the refactoring
        match self
            .app_state
            .ast_service
            .plan_refactor(&intent, file_path)
            .await
        {
            Ok(edit_plan) => {
                debug!(
                    files_count = edit_plan.edits.len(),
                    "Successfully planned refactor"
                );
                Ok(json!({
                    "success": true,
                    "message": format!("Successfully planned rename of '{}' to '{}' affecting {} edits across {} files",
                                     old_name, new_name, edit_plan.edits.len(), edit_plan.metadata.impact_areas.len()),
                    "edit_plan": edit_plan
                }))
            }
            Err(e) => {
                error!(
                    old_name = %old_name,
                    new_name = %new_name,
                    error = %e,
                    "Failed to plan refactor"
                );
                Err(ServerError::Runtime {
                    message: format!("Failed to plan refactor: {}", e),
                })
            }
        }
    }

    /// Handle apply_edits tool using FileService
    async fn handle_apply_edits(&self, tool_call: ToolCall) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling apply_edits");

        let args = tool_call.arguments.unwrap_or(json!({}));
        let edit_plan_value = args
            .get("edit_plan")
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'edit_plan' parameter".into()))?;

        // Parse the EditPlan from the JSON value
        let edit_plan: cb_api::EditPlan = serde_json::from_value(edit_plan_value.clone())
            .map_err(|e| ServerError::InvalidRequest(format!("Invalid edit_plan format: {}", e)))?;

        debug!(
            source_file = %edit_plan.source_file,
            edits_count = edit_plan.edits.len(),
            dependency_updates_count = edit_plan.dependency_updates.len(),
            "Applying edit plan"
        );

        // Apply the edit plan using the FileService
        match self
            .app_state
            .file_service
            .apply_edit_plan(&edit_plan)
            .await
        {
            Ok(result) => {
                if result.success {
                    info!(
                        modified_files_count = result.modified_files.len(),
                        "Successfully applied edit plan"
                    );
                    Ok(json!({
                        "success": true,
                        "message": format!("Successfully applied edit plan to {} files",
                                         result.modified_files.len()),
                        "result": result
                    }))
                } else {
                    warn!(errors = ?result.errors, "Edit plan applied with errors");
                    Ok(json!({
                        "success": false,
                        "message": format!("Edit plan completed with errors: {}",
                                         result.errors.as_ref()
                                              .map(|e| e.join("; "))
                                              .unwrap_or_else(|| "Unknown errors".to_string())),
                        "result": result
                    }))
                }
            }
            Err(e) => {
                error!(error = %e, "Failed to apply edit plan");
                Err(ServerError::Runtime {
                    message: format!("Failed to apply edit plan: {}", e),
                })
            }
        }
    }

    /// Handle file operations using app_state services
    async fn handle_file_operation(&self, tool_call: ToolCall) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling file operation");

        match tool_call.name.as_str() {
            "rename_file" => {
                let args = tool_call.arguments.ok_or_else(|| {
                    ServerError::InvalidRequest("Missing arguments for rename_file".into())
                })?;
                let old_path = args
                    .get("old_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ServerError::InvalidRequest("Missing 'old_path' parameter".into())
                    })?;
                let new_path = args
                    .get("new_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ServerError::InvalidRequest("Missing 'new_path' parameter".into())
                    })?;
                let dry_run = args
                    .get("dry_run")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let result = self
                    .app_state
                    .file_service
                    .rename_file_with_imports(
                        std::path::Path::new(old_path),
                        std::path::Path::new(new_path),
                        dry_run,
                    )
                    .await?;

                let imports_updated = result
                    .import_updates
                    .as_ref()
                    .map(|r| r.imports_updated)
                    .unwrap_or(0);
                let files_affected = result
                    .import_updates
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
                let args = tool_call.arguments.ok_or_else(|| {
                    ServerError::InvalidRequest("Missing arguments for create_file".into())
                })?;
                let file_path =
                    args.get("file_path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            ServerError::InvalidRequest("Missing 'file_path' parameter".into())
                        })?;
                let content = args.get("content").and_then(|v| v.as_str());
                let overwrite = args.get("overwrite").and_then(|v| v.as_bool()).unwrap_or(false);

                // Use FileService for proper locking and cache invalidation
                self.app_state
                    .file_service
                    .create_file(std::path::Path::new(file_path), content, overwrite)
                    .await?;

                Ok(json!({
                    "success": true,
                    "file_path": file_path,
                    "created": true
                }))
            }
            "delete_file" => {
                let args = tool_call.arguments.ok_or_else(|| {
                    ServerError::InvalidRequest("Missing arguments for delete_file".into())
                })?;
                let file_path =
                    args.get("file_path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            ServerError::InvalidRequest("Missing 'file_path' parameter".into())
                        })?;
                let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);

                // Use FileService for proper locking and cache invalidation
                self.app_state
                    .file_service
                    .delete_file(std::path::Path::new(file_path), force)
                    .await?;

                Ok(json!({
                    "success": true,
                    "file_path": file_path,
                    "deleted": true
                }))
            }
            "read_file" => {
                let args = tool_call.arguments.ok_or_else(|| {
                    ServerError::InvalidRequest("Missing arguments for read_file".into())
                })?;
                let file_path =
                    args.get("file_path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            ServerError::InvalidRequest("Missing 'file_path' parameter".into())
                        })?;

                // Use FileService for proper locking
                let content = self.app_state
                    .file_service
                    .read_file(std::path::Path::new(file_path))
                    .await?;

                Ok(json!({
                    "success": true,
                    "file_path": file_path,
                    "content": content
                }))
            }
            "write_file" => {
                let args = tool_call.arguments.ok_or_else(|| {
                    ServerError::InvalidRequest("Missing arguments for write_file".into())
                })?;
                let file_path =
                    args.get("file_path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            ServerError::InvalidRequest("Missing 'file_path' parameter".into())
                        })?;
                let content = args.get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ServerError::InvalidRequest("Missing 'content' parameter".into())
                    })?;

                // Use FileService for proper locking and cache invalidation
                self.app_state
                    .file_service
                    .write_file(std::path::Path::new(file_path), content)
                    .await?;

                Ok(json!({
                    "success": true,
                    "file_path": file_path,
                    "written": true
                }))
            }
            _ => Err(ServerError::Unsupported(format!(
                "File operation '{}' not implemented",
                tool_call.name
            ))),
        }
    }

    /// Check if a method is supported for a file
    pub async fn is_method_supported(&self, file_path: &std::path::Path, method: &str) -> bool {
        self.initialize().await.is_ok()
            && self
                .plugin_manager
                .is_method_supported(file_path, method)
                .await
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

#[async_trait]
impl McpDispatcher for PluginDispatcher {
    async fn dispatch(&self, message: McpMessage) -> cb_api::ApiResult<McpMessage> {
        self.dispatch(message)
            .await
            .map_err(|e| cb_api::ApiError::internal(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_app_state() -> Arc<AppState> {
        let temp_dir = TempDir::new().unwrap();
        let ast_cache = Arc::new(cb_ast::AstCache::new());
        let ast_service = Arc::new(crate::services::DefaultAstService::new(ast_cache.clone()));
        let project_root = temp_dir.path().to_path_buf();
        let lock_manager = Arc::new(crate::services::LockManager::new());
        let file_service = Arc::new(crate::services::FileService::new(
            project_root.clone(),
            ast_cache.clone(),
            lock_manager.clone(),
        ));
        let operation_queue = Arc::new(crate::services::OperationQueue::new(lock_manager.clone()));
        let planner = crate::services::planner::DefaultPlanner::new();
        let plugin_manager = Arc::new(PluginManager::new());
        let workflow_executor =
            crate::services::workflow_executor::DefaultWorkflowExecutor::new(plugin_manager);

        Arc::new(AppState {
            ast_service,
            file_service,
            planner,
            workflow_executor,
            project_root,
            lock_manager,
            operation_queue,
            start_time: std::time::Instant::now(),
        })
    }

    #[tokio::test]
    async fn test_plugin_dispatcher_initialization() {
        let app_state = create_test_app_state();
        let plugin_manager = Arc::new(PluginManager::new());
        let dispatcher = PluginDispatcher::new(app_state, plugin_manager);

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
        let plugin_manager = Arc::new(PluginManager::new());
        let dispatcher = PluginDispatcher::new(app_state, plugin_manager);

        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "tools/list".to_string(),
            params: None,
        };

        let response = dispatcher
            .dispatch(McpMessage::Request(request))
            .await
            .unwrap();

        if let McpMessage::Response(resp) = response {
            assert!(resp.result.is_some());
            let result = resp.result.unwrap();
            assert!(result["tools"].is_array());

            let tools = result["tools"].as_array().unwrap();
            assert!(!tools.is_empty());

            // Should have common tools
            let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
            assert!(tool_names.contains(&"find_definition"));
        } else {
            panic!("Expected Response message");
        }
    }

    #[tokio::test]
    async fn test_method_support_checking() {
        let app_state = create_test_app_state();
        let plugin_manager = Arc::new(PluginManager::new());
        let dispatcher = PluginDispatcher::new(app_state, plugin_manager);

        assert!(dispatcher.initialize().await.is_ok());

        // TypeScript file should support find_definition
        let ts_file = std::path::Path::new("test.ts");
        assert!(
            dispatcher
                .is_method_supported(ts_file, "find_definition")
                .await
        );

        // Unknown extension should not be supported
        let unknown_file = std::path::Path::new("test.unknown");
        assert!(
            !dispatcher
                .is_method_supported(unknown_file, "find_definition")
                .await
        );
    }

    #[tokio::test]
    async fn test_plugin_statistics() {
        let app_state = create_test_app_state();
        let plugin_manager = Arc::new(PluginManager::new());
        let dispatcher = PluginDispatcher::new(app_state, plugin_manager);

        let stats = dispatcher.get_plugin_statistics().await.unwrap();

        assert!(stats["registry"]["total_plugins"].as_u64().unwrap() > 0);
        assert!(stats["registry"]["supported_extensions"].as_u64().unwrap() > 0);
        assert!(stats["plugins"].is_array());
    }
}
