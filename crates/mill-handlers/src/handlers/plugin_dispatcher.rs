//! Plugin-based MCP message dispatcher
//!
//! This is the new plugin-based dispatcher that replaces the monolithic
//! dispatcher with a flexible plugin system.
//!
//! ## Handler Registry
//!
//! The dispatcher registers 19 internal tools across multiple handlers:
//! - FileOperationHandler: 4 internal tools (create_file, delete_file, rename_file, rename_directory)
//! - FileToolsHandler: 3 internal tools (read_file, write_file, list_files)
//! - AdvancedToolsHandler: 2 internal tools (execute_edits, execute_batch)
//! - InternalNavigationHandler: 1 internal tool (get_document_symbols)
//! - LifecycleHandler: 3 internal tools (notify_file_opened, notify_file_saved, notify_file_closed)
//! - InternalEditingToolsHandler: 1 internal tool (rename_symbol_with_imports)
//! - InternalWorkspaceHandler: 1 internal tool (apply_workspace_edit)
//! - InternalIntelligenceHandler: 2 internal tools (get_completions, get_signature_help)
//! - WorkspaceToolsHandler: 2 internal tools (move_directory, update_dependencies)

use crate::register_handlers_with_logging;
use async_trait::async_trait;
use mill_foundation::core::model::mcp::{McpMessage, McpRequest, McpResponse, ToolCall};
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_foundation::protocol::AstService;
use mill_plugin_system::{LspAdapterPlugin, PluginManager};
use mill_services::services::planner::Planner;
use mill_services::services::workflow_executor::WorkflowExecutor;
use mill_transport::McpDispatcher;
use mill_workspaces::WorkspaceManager;
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
    pub file_service: Arc<mill_services::services::FileService>,
    /// Planner service for generating workflows from intents
    pub planner: Arc<dyn Planner>,
    /// Workflow executor for running planned workflows
    pub workflow_executor: Arc<dyn WorkflowExecutor>,
    /// Project root directory
    pub project_root: std::path::PathBuf,
    /// Lock manager for file-level locking
    pub lock_manager: Arc<mill_services::services::LockManager>,
    /// Operation queue for serializing file operations
    pub operation_queue: Arc<mill_services::services::OperationQueue>,
    /// Server start time for uptime calculation
    pub start_time: Instant,
    /// Workspace manager for tracking connected containers
    pub workspace_manager: Arc<WorkspaceManager>,
    /// Language plugin registry for dynamic language support
    pub language_plugins: crate::LanguagePluginRegistry,
}

impl AppState {
    /// Create a MoveService for unified move/rename planning
    ///
    /// This provides direct access to MoveService without going through FileService.
    /// Handlers should use this factory method instead of calling FileService wrappers.
    pub fn move_service(&self) -> mill_services::services::MoveService<'_> {
        mill_services::services::MoveService::new(
            &self.file_service.reference_updater,
            &self.language_plugins.inner,
            &self.project_root,
        )
    }

    /// Convert to mill_handler_api::AppState for use with trait-based handlers
    pub fn to_api_app_state(&self) -> Arc<mill_handler_api::AppState> {
        Arc::new(mill_handler_api::AppState {
            file_service: Arc::new(FileServiceWrapper(self.file_service.clone()))
                as Arc<dyn mill_handler_api::FileService>,
            language_plugins: Arc::new(LanguagePluginRegistryWrapper(self.language_plugins.clone()))
                as Arc<dyn mill_handler_api::LanguagePluginRegistry>,
            project_root: self.project_root.clone(),
            extensions: None, // Will be set by caller if needed
        })
    }
}

// Newtype wrappers to satisfy the orphan rule (external trait + external type)

/// Wrapper for FileService to implement mill_handler_api::FileService
pub struct FileServiceWrapper(pub Arc<mill_services::services::FileService>);

#[async_trait]
impl mill_handler_api::FileService for FileServiceWrapper {
    async fn read_file(
        &self,
        path: &std::path::Path,
    ) -> Result<String, mill_foundation::errors::MillError> {
        self.0.read_file(path).await
    }

    async fn list_files(
        &self,
        path: &std::path::Path,
        recursive: bool,
    ) -> Result<Vec<String>, mill_foundation::errors::MillError> {
        self.0.list_files(path, recursive).await
    }

    async fn write_file(
        &self,
        path: &std::path::Path,
        content: &str,
        dry_run: bool,
    ) -> Result<
        mill_foundation::core::dry_run::DryRunnable<serde_json::Value>,
        mill_foundation::errors::MillError,
    > {
        self.0.write_file(path, content, dry_run).await
    }

    async fn delete_file(
        &self,
        path: &std::path::Path,
        force: bool,
        dry_run: bool,
    ) -> Result<
        mill_foundation::core::dry_run::DryRunnable<serde_json::Value>,
        mill_foundation::errors::MillError,
    > {
        self.0.delete_file(path, force, dry_run).await
    }

    async fn create_file(
        &self,
        path: &std::path::Path,
        content: Option<&str>,
        overwrite: bool,
        dry_run: bool,
    ) -> Result<
        mill_foundation::core::dry_run::DryRunnable<serde_json::Value>,
        mill_foundation::errors::MillError,
    > {
        self.0.create_file(path, content, overwrite, dry_run).await
    }

    async fn rename_file_with_imports(
        &self,
        old_path: &std::path::Path,
        new_path: &std::path::Path,
        dry_run: bool,
        scan_scope: Option<mill_plugin_api::ScanScope>,
    ) -> Result<
        mill_foundation::core::dry_run::DryRunnable<serde_json::Value>,
        mill_foundation::errors::MillError,
    > {
        self.0
            .rename_file_with_imports(old_path, new_path, dry_run, scan_scope)
            .await
    }

    async fn rename_directory_with_imports(
        &self,
        old_path: &std::path::Path,
        new_path: &std::path::Path,
        dry_run: bool,
        scan_scope: Option<mill_plugin_api::ScanScope>,
        details: bool,
    ) -> Result<
        mill_foundation::core::dry_run::DryRunnable<serde_json::Value>,
        mill_foundation::errors::MillError,
    > {
        self.0
            .rename_directory_with_imports(old_path, new_path, dry_run, scan_scope, details)
            .await
    }

    async fn list_files_with_pattern(
        &self,
        path: &std::path::Path,
        recursive: bool,
        pattern: Option<&str>,
    ) -> Result<Vec<String>, mill_foundation::errors::MillError> {
        self.0
            .list_files_with_pattern(path, recursive, pattern)
            .await
    }

    fn to_absolute_path_checked(
        &self,
        path: &std::path::Path,
    ) -> Result<std::path::PathBuf, mill_foundation::errors::MillError> {
        self.0.to_absolute_path_checked(path)
    }

    async fn apply_edit_plan(
        &self,
        plan: &mill_foundation::protocol::EditPlan,
    ) -> Result<mill_foundation::protocol::EditPlanResult, mill_foundation::errors::MillError> {
        self.0.apply_edit_plan(plan).await
    }
}

/// Wrapper for LanguagePluginRegistry to implement mill_handler_api::LanguagePluginRegistry
pub struct LanguagePluginRegistryWrapper(pub crate::LanguagePluginRegistry);

impl mill_handler_api::LanguagePluginRegistry for LanguagePluginRegistryWrapper {
    fn get_plugin(&self, extension: &str) -> Option<&dyn mill_plugin_api::LanguagePlugin> {
        self.0.get_plugin(extension)
    }

    fn supported_extensions(&self) -> Vec<String> {
        self.0.supported_extensions()
    }

    fn get_plugin_for_manifest(
        &self,
        file_path: &std::path::Path,
    ) -> Option<&dyn mill_plugin_api::LanguagePlugin> {
        // Extract filename from path
        let filename = file_path.file_name()?.to_str()?;
        self.0.get_plugin_for_manifest(filename)
    }

    fn inner(&self) -> &dyn std::any::Any {
        &self.0.inner as &dyn std::any::Any
    }
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
    pub fn operation_queue(&self) -> Arc<mill_services::services::OperationQueue> {
        self.app_state.operation_queue.clone()
    }

    /// Initializes the plugin system.
    #[instrument(skip(self))]
    pub async fn initialize(&self) -> ServerResult<()> {
        debug!("PluginDispatcher::initialize() called");
        self.initialized.get_or_try_init(|| async {
            debug!("Inside initialize - loading app config");
            info!("Initializing plugin system with DirectLspAdapter (bypassing hard-coded mappings)");

            // Use the injected language plugin registry from AppState
            // This ensures we use the same registry that was built at the application layer
            let plugin_registry = self.app_state.language_plugins.inner.clone();

            // Get LSP configuration from app config
            let app_config = mill_config::config::AppConfig::load()
                .map_err(|e| {
                    error!(error = %e, "Failed to load app config");
                    ServerError::internal(format!("Failed to load app config: {}", e))
                })?;
            debug!("App config loaded successfully");
            let lsp_config = app_config.lsp;

            let all_extensions: Vec<String> = lsp_config
                .servers
                .iter()
                .flat_map(|s| s.extensions.clone())
                .collect();

            let unified_lsp_adapter = Arc::new(DirectLspAdapter::new(
                lsp_config.clone(),
                all_extensions,
                "unified-lsp-direct".to_string(),
            ));

            {
                let mut stored_adapter = self.lsp_adapter.lock().await;
                *stored_adapter = Some(unified_lsp_adapter.clone());
                debug!("Stored unified LSP adapter for all tool handlers");
            }

            let mut registered_plugins = 0;
            for server_config in &lsp_config.servers {
                if server_config.extensions.is_empty() {
                    warn!(command = ?server_config.command, "LSP server config has no extensions, skipping");
                    continue;
                }

                let lsp_adapter = unified_lsp_adapter.clone();
                let primary_extension = &server_config.extensions[0];

                // Use generic LSP adapter for all languages - no hardcoded routing needed
                // The plugin name is derived from the primary extension
                let plugin_name = format!("{}-lsp", primary_extension);
                let plugin = Arc::new(LspAdapterPlugin::new(
                    plugin_name.clone(),
                    server_config.extensions.clone(),
                    lsp_adapter,
                ));

                self.plugin_manager
                    .register_plugin(&plugin_name, plugin)
                    .await
                    .map_err(|e| {
                        ServerError::internal(format!("Failed to register {} plugin: {}", plugin_name, e))
                    })?;

                registered_plugins += 1;
            }

            // Register System Tools plugin for workspace-level operations
            let system_plugin = Arc::new(mill_plugin_system::system_tools_plugin::SystemToolsPlugin::new(
                plugin_registry.clone(),
            ));
            self.plugin_manager
                .register_plugin("system", system_plugin)
                .await
                .map_err(|e| ServerError::internal(format!("Failed to register System tools plugin: {}", e)))?;
            registered_plugins += 1;

            info!(
                total_plugins = registered_plugins,
                "Plugin system initialized successfully"
            );

            {
                use super::tools::{
                    AdvancedToolsHandler, FileToolsHandler,
                    InternalEditingToolsHandler, InternalIntelligenceHandler, InternalNavigationHandler,
                    InternalWorkspaceHandler, LifecycleHandler, NavigationHandler,
                    SystemToolsHandler, WorkspaceToolsHandler, WorkspaceCreateHandler, WorkspaceExtractDepsHandler,
                    WorkspaceUpdateMembersHandler,
                };
                use super::workspace::FindReplaceHandler;
                use super::FileOperationHandler;

                let mut registry = self.tool_registry.lock().await;
                register_handlers_with_logging!(registry, {
                    SystemToolsHandler => "SystemToolsHandler with 1 tool (health_check)",
                    FileOperationHandler => "FileOperationHandler with 4 file operations (create_file, delete_file, rename_file, rename_directory)",
                    FileToolsHandler => "FileToolsHandler with 3 utility tools (read_file, write_file, list_files)",
                    AdvancedToolsHandler => "AdvancedToolsHandler with 2 INTERNAL tools (execute_edits, execute_batch)",
                    NavigationHandler => "NavigationHandler with 8 tools (find_definition, find_references, find_implementations, find_type_definition, search_symbols, get_symbol_info, get_diagnostics, get_call_hierarchy)",
                    InternalNavigationHandler => "InternalNavigationHandler with 1 INTERNAL tool (get_document_symbols)",
                    LifecycleHandler => "LifecycleHandler with 3 INTERNAL tools (notify_file_opened, notify_file_saved, notify_file_closed)",
                    InternalEditingToolsHandler => "InternalEditingToolsHandler with 1 INTERNAL tool (rename_symbol_with_imports)",
                    InternalWorkspaceHandler => "InternalWorkspaceHandler with 1 INTERNAL tool (apply_workspace_edit)",
                    InternalIntelligenceHandler => "InternalIntelligenceHandler with 2 INTERNAL tools (get_completions, get_signature_help)",
                    WorkspaceToolsHandler => "WorkspaceToolsHandler with 2 INTERNAL tools (move_directory, update_dependencies)",
                    WorkspaceCreateHandler => "WorkspaceCreateHandler with 1 tool (workspace.create_package)",
                    WorkspaceExtractDepsHandler => "WorkspaceExtractDepsHandler with 1 tool (workspace.extract_dependencies)",
                    WorkspaceUpdateMembersHandler => "WorkspaceUpdateMembersHandler with 1 tool (workspace.update_members)",
                    FindReplaceHandler => "FindReplaceHandler with 1 tool (workspace.find_replace)"
                });

                // Register refactoring handlers (feature-gated)
                // Each handler supports unified API with dryRun option:
                // - dryRun: true (default) - Preview mode, returns plan
                // - dryRun: false - Execute mode, applies changes
                use std::sync::Arc;

                #[cfg(feature = "refactor-rename")]
                {
                    use super::RenameHandler;
                    let rename_handler = Arc::new(RenameHandler::new());
                    register_handlers_with_logging!(registry, @arc {
                        rename_handler => "Unified rename handler (supports dryRun)"
                    });
                }
                #[cfg(feature = "refactor-extract")]
                {
                    use super::ExtractHandler;
                    let extract_handler = Arc::new(ExtractHandler::new());
                    register_handlers_with_logging!(registry, @arc {
                        extract_handler => "Unified extract handler (supports dryRun)"
                    });
                }
                #[cfg(feature = "refactor-inline")]
                {
                    use super::InlineHandler;
                    let inline_handler = Arc::new(InlineHandler::new());
                    register_handlers_with_logging!(registry, @arc {
                        inline_handler => "Unified inline handler (supports dryRun)"
                    });
                }
                #[cfg(feature = "refactor-move")]
                {
                    use super::MoveHandler;
                    let move_handler = Arc::new(MoveHandler::new());
                    register_handlers_with_logging!(registry, @arc {
                        move_handler => "Unified move handler (supports dryRun)"
                    });
                }
                #[cfg(feature = "refactor-reorder")]
                {
                    use super::ReorderHandler;
                    let reorder_handler = Arc::new(ReorderHandler::new());
                    register_handlers_with_logging!(registry, @arc {
                        reorder_handler => "Unified reorder handler (supports dryRun)"
                    });
                }
                #[cfg(feature = "refactor-transform")]
                {
                    use super::TransformHandler;
                    let transform_handler = Arc::new(TransformHandler::new());
                    register_handlers_with_logging!(registry, @arc {
                        transform_handler => "Unified transform handler (supports dryRun)"
                    });
                }
                #[cfg(feature = "refactor-delete")]
                {
                    use super::DeleteHandler;
                    let delete_handler = Arc::new(DeleteHandler::new());
                    register_handlers_with_logging!(registry, @arc {
                        delete_handler => "Unified delete handler (supports dryRun)"
                    });
                }
            }

            Ok::<(), ServerError>(())
        }).await?;

        Ok(())
    }

    /// Dispatches an MCP message using the plugin system.
    #[instrument(skip(self, message, session_info), fields(request_id = %uuid::Uuid::new_v4()))]
    pub async fn dispatch(
        &self,
        message: McpMessage,
        session_info: &mill_transport::SessionInfo,
    ) -> ServerResult<McpMessage> {
        self.initialize().await?;

        match message {
            McpMessage::Request(request) => self.handle_request(request, session_info).await,
            McpMessage::Response(response) => Ok(McpMessage::Response(response)),
            McpMessage::Notification(_) => Ok(McpMessage::Response(McpResponse {
                jsonrpc: "2.0".to_string(),
                id: None,
                result: Some(json!({"status": "ok"})),
                error: None,
            })),
            _ => Err(ServerError::not_supported("Unknown message type")),
        }
    }

    /// Handle an MCP request using plugins
    #[instrument(skip(self, request, session_info), fields(method = %request.method))]
    async fn handle_request(
        &self,
        request: McpRequest,
        session_info: &mill_transport::SessionInfo,
    ) -> ServerResult<McpMessage> {
        let response = match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params).await?,
            "initialized" | "notifications/initialized" => self.handle_initialized().await?,
            "tools/list" => self.handle_list_tools().await?,
            "tools/call" => self.handle_tool_call(request.params, session_info).await?,
            _ => {
                return Err(ServerError::not_supported(format!(
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
    #[instrument(skip(self, params, session_info))]
    async fn handle_tool_call(
        &self,
        params: Option<Value>,
        session_info: &mill_transport::SessionInfo,
    ) -> ServerResult<Value> {
        let start_time = Instant::now();

        let params = params.ok_or_else(|| ServerError::invalid_request("Missing params"))?;

        let tool_call: ToolCall = serde_json::from_value(params)
            .map_err(|e| ServerError::invalid_request(format!("Invalid tool call: {}", e)))?;

        let tool_name = tool_call.name.clone();

        // Create concrete context first
        let concrete_context = super::tools::ToolHandlerContext {
            user_id: session_info.user_id.clone(),
            app_state: self.app_state.clone(),
            plugin_manager: self.plugin_manager.clone(),
            lsp_adapter: self.lsp_adapter.clone(),
        };

        // Convert to trait-based context for handler compatibility
        let api_context = concrete_context.to_api_context().await;

        let result = self
            .tool_registry
            .lock()
            .await
            .handle_tool(tool_call, &api_context)
            .await;

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
                "name": "mill",
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

    pub fn plugin_manager(&self) -> &PluginManager {
        &self.plugin_manager
    }

    pub async fn is_method_supported(&self, file_path: &std::path::Path, method: &str) -> bool {
        self.initialize().await.is_ok()
            && self
                .plugin_manager
                .is_method_supported(file_path, method)
                .await
    }

    pub async fn get_supported_extensions(&self) -> Vec<String> {
        if self.initialize().await.is_ok() {
            self.plugin_manager.get_supported_extensions().await
        } else {
            Vec::new()
        }
    }

    /// Gracefully shutdown the dispatcher and all LSP clients
    pub async fn shutdown(&self) -> ServerResult<()> {
        debug!("PluginDispatcher shutting down");

        // Shutdown LSP adapter if it exists
        if let Some(adapter) = self.lsp_adapter.lock().await.as_ref() {
            match adapter.shutdown().await {
                Ok(_) => {
                    debug!("LSP adapter shutdown successfully");
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        "Failed to shutdown LSP adapter cleanly, some clients may not have shutdown"
                    );
                }
            }
        }

        Ok(())
    }
}

impl Drop for PluginDispatcher {
    fn drop(&mut self) {
        // Attempt to shutdown LSP adapter when dispatcher is dropped
        // This is best-effort - we spawn a task to avoid blocking Drop
        let lsp_adapter = self.lsp_adapter.clone();

        tokio::spawn(async move {
            if let Some(adapter) = lsp_adapter.lock().await.as_ref() {
                match adapter.shutdown().await {
                    Ok(_) => {
                        debug!("LSP adapter shutdown successfully from PluginDispatcher drop");
                    }
                    Err(e) => {
                        warn!(
                            error = %e,
                            "Failed to shutdown LSP adapter from PluginDispatcher drop"
                        );
                    }
                }
            }
        });
    }
}

#[async_trait]
impl McpDispatcher for PluginDispatcher {
    async fn dispatch(
        &self,
        message: McpMessage,
        session_info: &mill_transport::SessionInfo,
    ) -> mill_foundation::errors::MillResult<McpMessage> {
        self.dispatch(message, session_info)
            .await
            .map_err(|e| mill_foundation::errors::MillError::internal(e.to_string()))
    }
}

/// Create a test dispatcher for testing purposes
pub async fn create_test_dispatcher() -> PluginDispatcher {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();

    let cache_settings = mill_ast::CacheSettings::default();
    let plugin_manager = Arc::new(PluginManager::new());
    let config = mill_config::AppConfig::default();

    // Build plugin registry for tests
    let plugin_registry =
        mill_services::services::registry_builder::build_language_plugin_registry(vec![]);

    let services = mill_services::services::app_state_factory::create_services_bundle(
        &project_root,
        cache_settings,
        plugin_manager.clone(),
        &config,
        plugin_registry.clone(),
    )
    .await;

    let workspace_manager = Arc::new(WorkspaceManager::new());

    let app_state = Arc::new(AppState {
        ast_service: services.ast_service,
        file_service: services.file_service,
        planner: services.planner,
        workflow_executor: services.workflow_executor,
        project_root,
        lock_manager: services.lock_manager,
        operation_queue: services.operation_queue,
        start_time: std::time::Instant::now(),
        workspace_manager,
        language_plugins: crate::LanguagePluginRegistry::from_registry(plugin_registry),
    });

    PluginDispatcher::new(app_state, plugin_manager)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_app_state() -> Arc<AppState> {
        let temp_dir = TempDir::new().unwrap();

        // Build plugin registry for tests
        let plugin_registry =
            mill_services::services::registry_builder::build_language_plugin_registry(vec![]);
        let language_plugins = crate::LanguagePluginRegistry::from_registry(plugin_registry);
        let ast_cache = Arc::new(mill_ast::AstCache::new());
        let ast_service = Arc::new(mill_services::services::DefaultAstService::new(
            ast_cache.clone(),
            language_plugins.inner.clone(),
        ));
        let project_root = temp_dir.path().to_path_buf();
        let lock_manager = Arc::new(mill_services::services::LockManager::new());
        let operation_queue = Arc::new(mill_services::services::OperationQueue::new(
            lock_manager.clone(),
        ));
        let config = mill_config::AppConfig::default();
        let file_service = Arc::new(mill_services::services::FileService::new(
            project_root.clone(),
            ast_cache.clone(),
            lock_manager.clone(),
            operation_queue.clone(),
            &config,
            language_plugins.inner.clone(),
        ));
        let planner = mill_services::services::planner::DefaultPlanner::new();
        let plugin_manager = Arc::new(PluginManager::new());
        let workflow_executor =
            mill_services::services::workflow_executor::DefaultWorkflowExecutor::new(
                plugin_manager,
            );
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
            language_plugins,
        })
    }

    #[tokio::test]
    async fn test_plugin_dispatcher_initialization() {
        let app_state = create_test_app_state().await;
        let plugin_manager = Arc::new(PluginManager::new());
        let dispatcher = PluginDispatcher::new(app_state, plugin_manager);

        assert!(dispatcher.initialize().await.is_ok());
        let plugins = dispatcher.plugin_manager.list_plugins().await;
        assert!(!plugins.is_empty());
    }

    #[tokio::test]
    async fn test_tools_list() {
        let app_state = create_test_app_state().await;
        let plugin_manager = Arc::new(PluginManager::new());
        let dispatcher = PluginDispatcher::new(app_state, plugin_manager);

        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "tools/list".to_string(),
            params: None,
        };
        let session_info = mill_transport::SessionInfo::default();

        let response = dispatcher
            .dispatch(McpMessage::Request(request), &session_info)
            .await
            .unwrap();

        if let McpMessage::Response(resp) = response {
            assert!(resp.result.is_some());
            let result = resp.result.unwrap();
            assert!(result["tools"].is_array());
        } else {
            panic!("Expected Response message");
        }
    }
}
