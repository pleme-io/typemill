//! Plugin-based MCP message dispatcher
//!
//! This is the new plugin-based dispatcher that replaces the monolithic
//! dispatcher with a flexible plugin system.

use crate::services::planner::Planner;
use crate::services::workflow_executor::WorkflowExecutor;
use crate::workspaces::WorkspaceManager;
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
use std::time::{Duration, Instant};
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
    /// Workspace manager for tracking connected containers
    pub workspace_manager: Arc<WorkspaceManager>,
}

/// Plugin-based MCP dispatcher
pub struct PluginDispatcher {
    /// Plugin manager for handling requests
    plugin_manager: Arc<PluginManager>,
    /// Application state for file operations and services beyond LSP
    app_state: Arc<AppState>,
    /// LSP adapter for refactoring operations
    lsp_adapter: Arc<Mutex<Option<Arc<DirectLspAdapter>>>>,
    /// Tool handler registry for automatic routing
    tool_registry: Arc<Mutex<super::tool_registry::ToolRegistry>>,
    /// Initialization flag
    initialized: OnceCell<()>,
}

/// Direct LSP adapter that bypasses the old LSP manager and its hard-coded mappings
pub struct DirectLspAdapter {
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
    pub fn new(config: cb_core::config::LspConfig, extensions: Vec<String>, name: String) -> Self {
        Self {
            lsp_clients: Arc::new(Mutex::new(HashMap::new())),
            config,
            extensions,
            name,
        }
    }

    /// Get or create an LSP client for the given extension
    pub async fn get_or_create_client(
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
    /// Creates a new instance of the `PluginDispatcher`.
    ///
    /// # Arguments
    ///
    /// * `app_state` - The shared application state containing all services (AST, file operations, etc.)
    /// * `plugin_manager` - The manager responsible for routing requests to registered plugins
    ///
    /// # Returns
    ///
    /// A new `PluginDispatcher` instance ready to handle MCP requests
    pub fn new(app_state: Arc<AppState>, plugin_manager: Arc<PluginManager>) -> Self {
        Self {
            plugin_manager,
            app_state,
            lsp_adapter: Arc::new(Mutex::new(None)),
            tool_registry: Arc::new(Mutex::new(super::tool_registry::ToolRegistry::new())),
            initialized: OnceCell::new(),
        }
    }

    /// Initializes the plugin system by loading LSP configurations and registering
    /// all necessary language and system plugins.
    ///
    /// This function is called lazily on the first dispatch and uses a `OnceCell` to ensure
    /// it only runs once. It loads LSP server configurations from the application config,
    /// creates DirectLspAdapter instances for each configured server, and registers them
    /// along with the SystemToolsPlugin.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If initialization succeeds
    /// * `Err(ServerError)` - If configuration loading or plugin registration fails
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

                // Store the first LSP adapter for refactoring operations
                {
                    let mut stored_adapter = self.lsp_adapter.lock().await;
                    if stored_adapter.is_none() {
                        *stored_adapter = Some(lsp_adapter.clone());
                        debug!("Stored LSP adapter for refactoring operations");
                    }
                }

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

            // Register tool handlers for non-LSP operations
            {
                let mut registry = self.tool_registry.lock().await;
                registry.register(Arc::new(super::file_operation_handler::FileOperationHandler::new()));
                debug!("Registered FileOperationHandler with 7 tools");

                registry.register(Arc::new(super::workflow_handler::WorkflowHandler::new()));
                debug!("Registered WorkflowHandler with 2 tools");

                registry.register(Arc::new(super::system_handler::SystemHandler::new()));
                debug!("Registered SystemHandler with 5 tools");

                registry.register(Arc::new(super::refactoring_handler::RefactoringHandler::new()));
                debug!("Registered RefactoringHandler with 5 tools");
            }

            Ok::<(), ServerError>(())
        }).await?;

        Ok(())
    }

    /// Dispatches an MCP message using the plugin system.
    ///
    /// This is the main entry point for processing MCP messages. It ensures the plugin
    /// system is initialized, then routes requests to the appropriate handlers and returns
    /// responses or notifications unchanged.
    ///
    /// # Arguments
    ///
    /// * `message` - The MCP message to process (Request, Response, or Notification)
    ///
    /// # Returns
    ///
    /// * `Ok(McpMessage)` - The response message or echoed notification
    /// * `Err(ServerError)` - If initialization fails or the request cannot be handled
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
    ///
    /// This function serves as the main entry point for all tool executions, routing them
    /// based on their type (e.g., file operation, refactoring, LSP, system tools).
    /// It provides comprehensive telemetry including execution duration and success/failure status.
    ///
    /// # Arguments
    ///
    /// * `params` - Optional JSON value containing the tool call parameters, must include tool name and arguments
    ///
    /// # Returns
    ///
    /// Returns a JSON value containing the tool execution result, or an error if the tool call fails
    #[instrument(skip(self, params))]
    async fn handle_tool_call(&self, params: Option<Value>) -> ServerResult<Value> {
        let start_time = Instant::now();

        let params = params.ok_or_else(|| ServerError::InvalidRequest("Missing params".into()))?;

        let tool_call: ToolCall = serde_json::from_value(params)
            .map_err(|e| ServerError::InvalidRequest(format!("Invalid tool call: {}", e)))?;

        let tool_name = tool_call.name.clone();
        debug!(tool_name = %tool_name, "Calling tool with plugin system");

        // Execute the appropriate handler based on tool type
        // Try tool registry first (for file operations, workflows, system tools, refactoring)
        let result = {
            let registry = self.tool_registry.lock().await;
            if registry.has_tool(&tool_name) {
                let context = super::tool_handler::ToolContext {
                    app_state: self.app_state.clone(),
                    plugin_manager: self.plugin_manager.clone(),
                    lsp_adapter: self.lsp_adapter.clone(),
                };
                drop(registry); // Release lock before async operation
                self.tool_registry.lock().await.handle_tool(tool_call, &context).await
            } else {
                // Fall back to plugin system for LSP operations only
                let plugin_request = self.convert_tool_call_to_plugin_request(tool_call)?;

                let plugin_start = Instant::now();
                match self.plugin_manager.handle_request(plugin_request).await {
                    Ok(response) => {
                        let processing_time = plugin_start.elapsed().as_millis();
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
        };

        // Log telemetry
        let duration = start_time.elapsed();
        match &result {
            Ok(_) => {
                info!(
                    tool_name = %tool_name,
                    duration_ms = duration.as_millis() as u64,
                    status = "success",
                    "Tool call completed"
                );
            }
            Err(e) => {
                error!(
                    tool_name = %tool_name,
                    duration_ms = duration.as_millis() as u64,
                    status = "error",
                    error = %e,
                    "Tool call failed"
                );
            }
        }

        result
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

    /// Returns a reference to the plugin manager for advanced operations.
    ///
    /// This provides access to the underlying plugin manager, allowing callers to
    /// perform operations like querying plugin capabilities, getting statistics,
    /// or accessing plugin-specific functionality.
    ///
    /// # Returns
    ///
    /// A reference to the `PluginManager` instance
    pub fn plugin_manager(&self) -> &PluginManager {
        &self.plugin_manager
    }

    /// Escape a shell argument for safe execution
    fn escape_shell_arg(arg: &str) -> String {
        // Replace single quotes with '\'' to safely escape for sh -c
        arg.replace('\'', "'\\''")
    }

    /// Execute a command in a remote workspace via its agent
    async fn execute_remote_command(
        workspace_manager: &WorkspaceManager,
        workspace_id: &str,
        command: &str,
    ) -> ServerResult<String> {
        debug!(
            workspace_id = %workspace_id,
            command = %command,
            "Executing remote command"
        );

        // Look up workspace
        let workspace = workspace_manager
            .get(workspace_id)
            .ok_or_else(|| {
                ServerError::InvalidRequest(format!("Workspace '{}' not found", workspace_id))
            })?;

        // Build agent URL
        let agent_url = format!("{}/execute", workspace.agent_url);

        // Create HTTP client with timeout
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| {
                error!(error = %e, "Failed to create HTTP client");
                ServerError::Internal("HTTP client error".into())
            })?;

        // Execute command via agent
        let response = client
            .post(&agent_url)
            .json(&json!({ "command": command }))
            .send()
            .await
            .map_err(|e| {
                error!(
                    workspace_id = %workspace_id,
                    agent_url = %agent_url,
                    error = %e,
                    "Failed to send command to workspace agent"
                );
                ServerError::Internal(format!("Failed to reach workspace agent: {}", e))
            })?;

        // Check response status
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!(
                workspace_id = %workspace_id,
                status = %status,
                error = %error_text,
                "Agent returned error"
            );
            return Err(ServerError::Internal(format!(
                "Agent error ({}): {}",
                status, error_text
            )));
        }

        // Parse response
        #[derive(serde::Deserialize)]
        struct ExecuteResponse {
            exit_code: i32,
            stdout: String,
            stderr: String,
        }

        let result: ExecuteResponse = response.json().await.map_err(|e| {
            error!(
                workspace_id = %workspace_id,
                error = %e,
                "Failed to parse agent response"
            );
            ServerError::Internal("Invalid response from workspace agent".into())
        })?;

        // Check exit code
        if result.exit_code != 0 {
            error!(
                workspace_id = %workspace_id,
                exit_code = result.exit_code,
                stderr = %result.stderr,
                "Command failed in workspace"
            );
            return Err(ServerError::Internal(format!(
                "Command failed with exit code {}: {}",
                result.exit_code, result.stderr
            )));
        }

        debug!(
            workspace_id = %workspace_id,
            stdout_len = result.stdout.len(),
            "Command executed successfully"
        );

        Ok(result.stdout)
    }


    /// Checks if a specific LSP method is supported for the given file.
    ///
    /// This queries the plugin system to determine if any registered plugin can handle
    /// the specified method for the file type indicated by the file path's extension.
    ///
    /// # Arguments
    ///
    /// * `file_path` - The path to the file to check
    /// * `method` - The LSP method name (e.g., "textDocument/definition")
    ///
    /// # Returns
    ///
    /// `true` if the method is supported for this file type, `false` otherwise
    pub async fn is_method_supported(&self, file_path: &std::path::Path, method: &str) -> bool {
        self.initialize().await.is_ok()
            && self
                .plugin_manager
                .is_method_supported(file_path, method)
                .await
    }

    /// Returns a list of all file extensions supported by registered plugins.
    ///
    /// This aggregates the supported extensions from all registered language plugins,
    /// which is useful for determining which file types the system can process.
    ///
    /// # Returns
    ///
    /// A vector of file extension strings (e.g., `["rs", "ts", "py"]`), or an empty
    /// vector if initialization fails
    pub async fn get_supported_extensions(&self) -> Vec<String> {
        if self.initialize().await.is_ok() {
            self.plugin_manager.get_supported_extensions().await
        } else {
            Vec::new()
        }
    }

    /// Returns comprehensive statistics about the plugin system for monitoring and debugging.
    ///
    /// This aggregates statistics from the plugin registry (total plugins, supported extensions,
    /// methods per plugin) and individual plugin metrics (cache hits, processing times, etc.).
    ///
    /// # Returns
    ///
    /// * `Ok(Value)` - A JSON object containing registry statistics, plugin metrics, and plugin list
    /// * `Err(ServerError)` - If initialization fails
    ///
    /// # JSON Structure
    ///
    /// ```json
    /// {
    ///   "registry": {
    ///     "total_plugins": 3,
    ///     "supported_extensions": ["rs", "ts", "py"],
    ///     "supported_methods": ["textDocument/definition", ...],
    ///     "average_methods_per_plugin": 15.2
    ///   },
    ///   "metrics": { ... },
    ///   "plugins": [...]
    /// }
    /// ```
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

    /// Handle find_dead_code tool using the dedicated analyzer module
    async fn handle_find_dead_code(&self, tool_call: ToolCall) -> ServerResult<Value> {
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

    /// Handle fix_imports by delegating to LSP's organize_imports
    ///
    /// This tool analyzes and fixes import statements in a file by removing unused imports,
    /// organizing import order, and applying language-specific formatting conventions.
    ///
    /// # How it Works
    ///
    /// - **Dry-run mode** (`dry_run: true`): Returns a preview message without making changes
    /// - **Normal mode** (`dry_run: false`): Delegates to LSP's `organize_imports` code action
    ///   which performs semantic analysis to identify and remove all types of unused imports:
    ///   - Named imports (e.g., `import { unused } from 'module'`)
    ///   - Default imports (e.g., `import Unused from 'module'`)
    ///   - Namespace imports (e.g., `import * as unused from 'module'`)
    ///   - Side-effect imports (e.g., `import './unused.css'`)
    ///
    /// # Requirements
    ///
    /// - Requires an LSP server that supports the `source.organizeImports` code action
    /// - TypeScript Language Server provides full support for this feature
    ///
    /// # Parameters
    ///
    /// - `file_path`: Absolute path to the file to fix
    /// - `dry_run`: Optional boolean (default: false) - if true, returns preview without changes
    ///
    /// # Returns
    ///
    /// Returns a JSON object with:
    /// - `operation`: "fix_imports"
    /// - `file_path`: The file that was processed
    /// - `dry_run`: Whether this was a dry-run
    /// - `modified`: Whether the file was actually modified
    /// - `status`: "preview" (dry-run) or "fixed" (actual changes)
    /// - `lsp_response`: The response from organize_imports (when not dry-run)
    async fn handle_fix_imports(&self, tool_call: ToolCall) -> ServerResult<Value> {
        let args = tool_call.arguments.unwrap_or(json!({}));

        // Extract parameters from fix_imports call
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("file_path is required".to_string()))?;

        let dry_run = args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false);

        debug!(file_path = %file_path, dry_run = dry_run, "Handling fix_imports via organize_imports");

        if dry_run {
            // For dry-run mode, just return a preview message
            return Ok(json!({
                "operation": "fix_imports",
                "file_path": file_path,
                "dry_run": true,
                "modified": false,
                "status": "preview",
                "message": "Dry run mode - set dry_run: false to apply import organization"
            }));
        }

        // For actual fixes, delegate to organize_imports via LSP
        // Create a new tool call for organize_imports
        let organize_imports_call = ToolCall {
            name: "organize_imports".to_string(),
            arguments: Some(json!({
                "file_path": file_path
            })),
        };

        // Convert to plugin request and dispatch through LSP adapter
        let plugin_request = self.convert_tool_call_to_plugin_request(organize_imports_call)?;

        match self.plugin_manager.handle_request(plugin_request).await {
            Ok(response) => {
                // Wrap LSP response in fix_imports format
                Ok(json!({
                    "operation": "fix_imports",
                    "file_path": file_path,
                    "dry_run": false,
                    "modified": true,
                    "status": "fixed",
                    "lsp_response": response
                }))
            }
            Err(e) => {
                Err(ServerError::internal(format!(
                    "Failed to organize imports: {}",
                    e
                )))
            }
        }
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
        let workspace_manager = Arc::new(WorkspaceManager::new());

        Arc::new(AppState {
            ast_service,
            file_service,
            planner,
            workflow_executor,
            project_root,
            lock_manager,
            operation_queue,
            start_time: std::time::Instant::now(),
            workspace_manager,
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
