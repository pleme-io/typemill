//! Plugin-based MCP message dispatcher
//!
//! This is the new plugin-based dispatcher that replaces the monolithic
//! dispatcher with a flexible plugin system.

use crate::register_handlers_with_logging;
use async_trait::async_trait;
use cb_core::model::mcp::{McpMessage, McpRequest, McpResponse, ToolCall};
use cb_core::workspaces::WorkspaceManager;
use cb_plugins::{LspAdapterPlugin, PluginManager};
use cb_protocol::AstService;
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
use cb_services::services::planner::Planner;
use cb_services::services::workflow_executor::WorkflowExecutor;
use cb_transport::McpDispatcher;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, OnceCell};
use tracing::{debug, error, info, instrument, warn};

use super::lsp_adapter::DirectLspAdapter;

/// Application state containing services
#[derive(Clone)]
pub struct AppState {
    /// AST service for code analysis and parsing
    pub ast_service: Arc<dyn AstService>,
    /// File service for file operations with import awareness
    pub file_service: Arc<cb_services::services::FileService>,
    /// Planner service for generating workflows from intents
    pub planner: Arc<dyn Planner>,
    /// Workflow executor for running planned workflows
    pub workflow_executor: Arc<dyn WorkflowExecutor>,
    /// Project root directory
    pub project_root: std::path::PathBuf,
    /// Lock manager for file-level locking
    pub lock_manager: Arc<cb_services::services::LockManager>,
    /// Operation queue for serializing file operations
    pub operation_queue: Arc<cb_services::services::OperationQueue>,
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
    /// Tool handler registry for automatic routing (public for testing)
    pub tool_registry: Arc<Mutex<super::tool_registry::ToolRegistry>>,
    /// Initialization flag
    initialized: OnceCell<()>,
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

    /// Returns a reference to the operation queue.
    /// This is useful for CLI tools that need to wait for async operations to complete.
    pub fn operation_queue(&self) -> Arc<cb_services::services::OperationQueue> {
        self.app_state.operation_queue.clone()
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
                use super::tools::*;
                let mut registry = self.tool_registry.lock().await;

                // Register all handlers using the unified ToolHandler trait
                // Using the declarative macro for clean, maintainable registration
                register_handlers_with_logging!(registry, {
                    SystemHandler => "SystemHandler with 3 tools (health_check, web_fetch, system_status)",
                    LifecycleHandler => "LifecycleHandler with 3 tools (notify_file_opened, notify_file_saved, notify_file_closed)",
                    WorkspaceHandler => "WorkspaceHandler with 5 tools (rename_directory, analyze_imports, find_dead_code, update_dependencies, extract_module_to_package)",
                    AdvancedHandler => "AdvancedHandler with 2 tools (apply_edits, batch_execute)",
                    FileOpsHandler => "FileOpsHandler with 6 tools (create_file, read_file, write_file, delete_file, rename_file, list_files)",
                    EditingHandler => "EditingHandler with 10 tools (rename_symbol, rename_symbol_strict, rename_symbol_with_imports, organize_imports, fix_imports, get_code_actions, format_document, extract_function, extract_variable, inline_variable)",
                    NavigationHandler => "NavigationHandler with 13 tools (find_definition, find_references, find_implementations, find_type_definition, get_document_symbols, search_workspace_symbols, get_hover, get_completions, get_signature_help, get_diagnostics, prepare_call_hierarchy, get_call_hierarchy_incoming_calls, get_call_hierarchy_outgoing_calls)",
                });
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

    /// Handle tools/call request using the unified tool registry
    ///
    /// This function serves as the main entry point for all tool executions.
    /// All tools are now handled by the unified tool registry, which delegates
    /// to the appropriate handler based on the tool name.
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
        debug!(tool_name = %tool_name, "Dispatching tool call to unified registry");

        // Create the context all handlers now expect
        let context = super::tools::ToolHandlerContext {
            app_state: self.app_state.clone(),
            plugin_manager: self.plugin_manager.clone(),
            lsp_adapter: self.lsp_adapter.clone(),
        };

        // Directly dispatch to the tool_registry. It now handles ALL tools.
        let result = self
            .tool_registry
            .lock()
            .await
            .handle_tool(tool_call, &context)
            .await;

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

    /// Public method for benchmarking tool dispatch
    ///
    /// This method provides access to the tool dispatch logic for performance benchmarking
    pub async fn benchmark_tool_call(&self, params: Option<Value>) -> ServerResult<Value> {
        self.handle_tool_call(params).await
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
}

#[async_trait]
impl McpDispatcher for PluginDispatcher {
    async fn dispatch(&self, message: McpMessage) -> cb_protocol::ApiResult<McpMessage> {
        self.dispatch(message)
            .await
            .map_err(|e| cb_protocol::ApiError::internal(e.to_string()))
    }
}

/// Create a test dispatcher for testing purposes
/// This is exposed publicly to support integration tests
pub fn create_test_dispatcher() -> PluginDispatcher {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let ast_cache = Arc::new(cb_ast::AstCache::new());
    let ast_service = Arc::new(cb_services::services::DefaultAstService::new(
        ast_cache.clone(),
    ));
    let project_root = temp_dir.path().to_path_buf();
    let lock_manager = Arc::new(cb_services::services::LockManager::new());
    let operation_queue = Arc::new(cb_services::services::OperationQueue::new(
        lock_manager.clone(),
    ));
    let file_service = Arc::new(cb_services::services::FileService::new(
        project_root.clone(),
        ast_cache.clone(),
        lock_manager.clone(),
        operation_queue.clone(),
    ));
    let planner = cb_services::services::planner::DefaultPlanner::new();
    let plugin_manager = Arc::new(PluginManager::new());
    let workflow_executor = cb_services::services::workflow_executor::DefaultWorkflowExecutor::new(
        plugin_manager.clone(),
    );
    let workspace_manager = Arc::new(WorkspaceManager::new());

    let app_state = Arc::new(AppState {
        ast_service,
        file_service,
        planner,
        workflow_executor,
        project_root,
        lock_manager,
        operation_queue,
        start_time: std::time::Instant::now(),
        workspace_manager,
    });

    PluginDispatcher::new(app_state, plugin_manager)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_app_state() -> Arc<AppState> {
        let temp_dir = TempDir::new().unwrap();
        let ast_cache = Arc::new(cb_ast::AstCache::new());
        let ast_service = Arc::new(cb_services::services::DefaultAstService::new(
            ast_cache.clone(),
        ));
        let project_root = temp_dir.path().to_path_buf();
        let lock_manager = Arc::new(cb_services::services::LockManager::new());
        let operation_queue = Arc::new(cb_services::services::OperationQueue::new(
            lock_manager.clone(),
        ));
        let file_service = Arc::new(cb_services::services::FileService::new(
            project_root.clone(),
            ast_cache.clone(),
            lock_manager.clone(),
            operation_queue.clone(),
        ));
        let planner = cb_services::services::planner::DefaultPlanner::new();
        let plugin_manager = Arc::new(PluginManager::new());
        let workflow_executor =
            cb_services::services::workflow_executor::DefaultWorkflowExecutor::new(plugin_manager);
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
