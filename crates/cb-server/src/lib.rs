//! cb-server: Core server implementation for Codeflow Buddy
//!
//! This crate implements the main server functionality including the MCP protocol
//! handlers, plugin system dispatcher, Language Server Protocol (LSP) client management,
//! authentication, file services with atomic operations, and various transport
//! mechanisms (stdio, WebSocket). It provides the runtime infrastructure for all
//! code intelligence and refactoring operations.

// Prevent technical debt accumulation
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

pub mod handlers;
pub mod services;
pub mod systems;
pub mod utils;

// Re-export workspaces from cb-core for backward compatibility
pub use cb_core::workspaces;

use crate::handlers::plugin_dispatcher::{AppState, PluginDispatcher};
use crate::services::{DefaultAstService, FileService, LockManager, OperationQueue};
pub use cb_api::{ApiError as ServerError, ApiResult as ServerResult, AstService, LspService};
use cb_ast::AstCache;
use cb_core::AppConfig;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::oneshot;

/// Server configuration options
#[derive(Debug, Clone)]
pub struct ServerOptions {
    /// Application configuration
    pub config: AppConfig,
    /// Enable debug mode
    pub debug: bool,
}

/// Handle to a running server
pub struct ServerHandle {
    shutdown_tx: oneshot::Sender<()>,
    _config: AppConfig,
    _dispatcher: Arc<PluginDispatcher>,
}

/// Bootstrap the server with given options
pub async fn bootstrap(options: ServerOptions) -> ServerResult<ServerHandle> {
    tracing::info!("Bootstrapping Codeflow Buddy server");

    // Validate configuration
    if options.config.server.port == 0 {
        return Err(ServerError::config("Invalid server port"));
    }

    // Get project root
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Create shared AST cache with configuration
    let cache_settings = cb_ast::CacheSettings::from_config(
        options.config.cache.enabled,
        options.config.cache.ttl_seconds,
        options.config.cache.max_size_bytes,
    );
    let ast_cache = Arc::new(AstCache::with_settings(cache_settings));

    // Create services
    let ast_service: Arc<dyn AstService> = Arc::new(DefaultAstService::new(ast_cache.clone()));
    let lock_manager = Arc::new(LockManager::new());
    let file_service = Arc::new(FileService::new(
        &project_root,
        ast_cache.clone(),
        lock_manager.clone(),
    ));
    let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    // Create planner
    let planner = crate::services::planner::DefaultPlanner::new();

    // Create plugin manager and workflow executor
    let plugin_manager = Arc::new(cb_plugins::PluginManager::new());

    // Register MCP proxy plugin if feature enabled
    #[cfg(feature = "mcp-proxy")]
    if let Some(external_mcp_config) = &options.config.external_mcp {
        use cb_mcp_proxy::McpProxyPlugin;
        use cb_plugins::LanguagePlugin;

        tracing::info!(
            servers_count = external_mcp_config.servers.len(),
            "Registering MCP proxy plugin"
        );

        let mut mcp_plugin = McpProxyPlugin::new(external_mcp_config.servers.clone());

        // Initialize the plugin BEFORE wrapping in Arc
        mcp_plugin.initialize().await.map_err(|e| {
            ServerError::plugin(format!("Failed to initialize MCP proxy plugin: {}", e))
        })?;

        plugin_manager
            .register_plugin("mcp-proxy", Arc::new(mcp_plugin))
            .await
            .map_err(|e| {
                ServerError::plugin(format!("Failed to register MCP proxy plugin: {}", e))
            })?;
    }

    let workflow_executor =
        crate::services::workflow_executor::DefaultWorkflowExecutor::new(plugin_manager.clone());

    // Create workspace manager for tracking connected containers
    let workspace_manager = Arc::new(cb_core::workspaces::WorkspaceManager::new());

    // Create application state
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

    // Create dispatcher
    let dispatcher = Arc::new(PluginDispatcher::new(app_state, plugin_manager));

    // Create shutdown channel
    let (shutdown_tx, _shutdown_rx) = oneshot::channel();

    tracing::info!("Server bootstrapped successfully");

    Ok(ServerHandle {
        shutdown_tx,
        _config: options.config,
        _dispatcher: dispatcher,
    })
}

impl ServerOptions {
    /// Create server options from app config
    pub fn from_config(config: AppConfig) -> Self {
        Self {
            config,
            debug: false,
        }
    }

    /// Enable debug mode
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }
}

/// Create AppState and PluginDispatcher with custom workspace manager
///
/// This is a helper function to reduce code duplication between
/// the standalone binary (main.rs) and the unified binary (apps/codebuddy).
pub async fn create_dispatcher_with_workspace(
    config: Arc<AppConfig>,
    workspace_manager: Arc<cb_core::workspaces::WorkspaceManager>,
) -> ServerResult<Arc<PluginDispatcher>> {
    // Get project root
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Create shared AST cache with configuration
    let cache_settings = cb_ast::CacheSettings::from_config(
        config.cache.enabled,
        config.cache.ttl_seconds,
        config.cache.max_size_bytes,
    );
    let ast_cache = Arc::new(AstCache::with_settings(cache_settings));

    // Create services
    let ast_service: Arc<dyn AstService> = Arc::new(DefaultAstService::new(ast_cache.clone()));
    let lock_manager = Arc::new(LockManager::new());
    let file_service = Arc::new(FileService::new(
        &project_root,
        ast_cache.clone(),
        lock_manager.clone(),
    ));
    let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    // Create planner
    let planner = crate::services::planner::DefaultPlanner::new();

    // Create plugin manager and workflow executor
    let plugin_manager = Arc::new(cb_plugins::PluginManager::new());

    // Register MCP proxy plugin if feature enabled
    #[cfg(feature = "mcp-proxy")]
    if let Some(external_mcp_config) = &config.external_mcp {
        use cb_mcp_proxy::McpProxyPlugin;
        use cb_plugins::LanguagePlugin;

        tracing::info!(
            servers_count = external_mcp_config.servers.len(),
            "Registering MCP proxy plugin"
        );

        let mut mcp_plugin = McpProxyPlugin::new(external_mcp_config.servers.clone());

        // Initialize the plugin BEFORE wrapping in Arc
        mcp_plugin.initialize().await.map_err(|e| {
            ServerError::plugin(format!("Failed to initialize MCP proxy plugin: {}", e))
        })?;

        plugin_manager
            .register_plugin("mcp-proxy", Arc::new(mcp_plugin))
            .await
            .map_err(|e| {
                ServerError::plugin(format!("Failed to register MCP proxy plugin: {}", e))
            })?;
    }

    let workflow_executor =
        crate::services::workflow_executor::DefaultWorkflowExecutor::new(plugin_manager.clone());

    // Start background processor for operation queue
    {
        let queue = operation_queue.clone();
        let file_svc = file_service.clone();
        tokio::spawn(async move {
            use std::path::Path;

            queue
                .process_with(move |op| {
                    let file_svc = file_svc.clone();
                    async move {
                        tracing::debug!(
                            operation_id = %op.id,
                            operation_type = ?op.operation_type,
                            file_path = %op.file_path.display(),
                            "Processing queued operation"
                        );

                        let result = match op.operation_type {
                            crate::services::OperationType::Write => {
                                let file_path = op.params.get("file_path")
                                    .and_then(|v| v.as_str())
                                    .ok_or_else(|| ServerError::runtime("Missing file_path parameter"))?;
                                let content = op.params.get("content")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                let dry_runnable = file_svc.write_file(Path::new(file_path), content, false).await?;
                                Ok(dry_runnable.result)
                            }
                            crate::services::OperationType::Delete => {
                                let file_path = op.params.get("file_path")
                                    .and_then(|v| v.as_str())
                                    .ok_or_else(|| ServerError::runtime("Missing file_path parameter"))?;
                                let dry_runnable = file_svc.delete_file(Path::new(file_path), false, false).await?;
                                Ok(dry_runnable.result)
                            }
                            crate::services::OperationType::Rename => {
                                let old_path = op.params.get("old_path")
                                    .and_then(|v| v.as_str())
                                    .ok_or_else(|| ServerError::runtime("Missing old_path parameter"))?;
                                let new_path = op.params.get("new_path")
                                    .and_then(|v| v.as_str())
                                    .ok_or_else(|| ServerError::runtime("Missing new_path parameter"))?;
                                let dry_runnable = file_svc.rename_file_with_imports(Path::new(old_path), Path::new(new_path), false).await?;
                                Ok(dry_runnable.result)
                            }
                            _ => {
                                Err(ServerError::runtime(format!("Unsupported operation type: {:?}", op.operation_type)))
                            }
                        };

                        match &result {
                            Ok(_) => {
                                tracing::info!(
                                    operation_id = %op.id,
                                    operation_type = ?op.operation_type,
                                    "Operation completed successfully"
                                );
                            }
                            Err(e) => {
                                tracing::error!(
                                    operation_id = %op.id,
                                    operation_type = ?op.operation_type,
                                    error = %e,
                                    "Operation failed"
                                );
                            }
                        }

                        result
                    }
                })
                .await;
        });
    }

    // Create application state
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

    // Create and return dispatcher
    Ok(Arc::new(PluginDispatcher::new(app_state, plugin_manager)))
}

impl ServerHandle {
    /// Start the server (async)
    pub async fn start(&self) -> ServerResult<()> {
        tracing::info!("Starting server...");

        // Note: The actual server implementation is in main.rs
        // This method exists for API compatibility but the real
        // server startup is handled by cb-transport layer

        tracing::info!("Server started successfully");
        Ok(())
    }

    /// Shutdown the server gracefully
    pub async fn shutdown(self) -> ServerResult<()> {
        tracing::info!("Shutting down server...");

        // Send shutdown signal
        if self.shutdown_tx.send(()).is_err() {
            tracing::warn!("Server already shut down");
        }

        // In a real implementation, this would:
        // 1. Stop accepting new connections
        // 2. Finish processing existing requests
        // 3. Clean up resources
        // 4. Unmount FUSE filesystem

        tracing::info!("Server shut down successfully");
        Ok(())
    }
}
