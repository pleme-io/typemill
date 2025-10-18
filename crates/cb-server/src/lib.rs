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

// Test helpers - available for integration tests
#[cfg(any(test, feature = "test-helpers"))]
pub mod test_helpers;

// Re-export workspaces from cb-core for backward compatibility
pub use codebuddy_core::workspaces;

// Re-export from new crates for backward compatibility
pub use cb_handlers::handlers;
pub use cb_services::services;

use codebuddy_core::AppConfig;
use cb_handlers::handlers::plugin_dispatcher::{AppState, PluginDispatcher};
pub use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult, AstService, LspService};
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

    // Use the app_state_factory to create services and app_state
    use cb_services::services::app_state_factory::create_services_bundle;
    #[cfg(feature = "mcp-proxy")]
    use cb_services::services::app_state_factory::register_mcp_proxy_if_enabled;

    let cache_settings = cb_ast::CacheSettings::from_config(
        options.config.cache.enabled,
        options.config.cache.ttl_seconds,
        options.config.cache.max_size_bytes,
    );

    let plugin_manager = Arc::new(codebuddy_plugin_system::PluginManager::new());

    // Register MCP proxy plugin if feature enabled
    #[cfg(feature = "mcp-proxy")]
    register_mcp_proxy_if_enabled(&plugin_manager, options.config.external_mcp.as_ref()).await?;

    // Build language plugin registry
    let plugin_registry = cb_services::services::registry_builder::build_language_plugin_registry();

    let services = create_services_bundle(
        &project_root,
        cache_settings,
        plugin_manager.clone(),
        &options.config,
        plugin_registry.clone(),
    )
    .await;

    let workspace_manager = Arc::new(codebuddy_core::workspaces::WorkspaceManager::new());

    // Create application state
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
        language_plugins: cb_handlers::LanguagePluginRegistry::from_registry(plugin_registry),
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
    workspace_manager: Arc<codebuddy_core::workspaces::WorkspaceManager>,
    plugin_registry: Arc<cb_plugin_api::PluginRegistry>,
) -> ServerResult<Arc<PluginDispatcher>> {
    // Get project root
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Use the app_state_factory to create services
    use cb_services::services::app_state_factory::create_services_bundle;
    #[cfg(feature = "mcp-proxy")]
    use cb_services::services::app_state_factory::register_mcp_proxy_if_enabled;

    let cache_settings = cb_ast::CacheSettings::from_config(
        config.cache.enabled,
        config.cache.ttl_seconds,
        config.cache.max_size_bytes,
    );

    let plugin_manager = Arc::new(codebuddy_plugin_system::PluginManager::new());

    // Register MCP proxy plugin if feature enabled
    #[cfg(feature = "mcp-proxy")]
    register_mcp_proxy_if_enabled(&plugin_manager, config.external_mcp.as_ref()).await?;

    let services = create_services_bundle(
        &project_root,
        cache_settings,
        plugin_manager.clone(),
        &config,
        plugin_registry.clone(),
    )
    .await;

    // Start background processor for operation queue
    {
        let queue = services.operation_queue.clone();
        tokio::spawn(async move {
            use cb_services::services::operation_queue::OperationType;
            use serde_json::Value;
            use std::path::Path;
            use tokio::fs;
            use tokio::io::AsyncWriteExt;

            queue
                .process_with(move |op, stats| async move {
                    tracing::debug!(
                        operation_id = %op.id,
                        operation_type = ?op.operation_type,
                        file_path = %op.file_path.display(),
                        "Executing queued operation"
                    );

                    let result = match op.operation_type {
                        OperationType::CreateDir => {
                            fs::create_dir_all(&op.file_path).await.map_err(|e| {
                                ServerError::internal(format!("Failed to create directory: {}", e))
                            })?;
                            Ok(Value::Null)
                        }
                        OperationType::CreateFile | OperationType::Write => {
                            let content = op
                                .params
                                .get("content")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");

                            // Write and explicitly sync to disk to avoid caching issues
                            let mut file = fs::File::create(&op.file_path).await.map_err(|e| {
                                ServerError::internal(format!("Failed to create file: {}", e))
                            })?;

                            file.write_all(content.as_bytes()).await.map_err(|e| {
                                ServerError::internal(format!("Failed to write content: {}", e))
                            })?;

                            // CRITICAL: Sync file to disk BEFORE updating stats
                            file.sync_all().await.map_err(|e| {
                                ServerError::internal(format!("Failed to sync file: {}", e))
                            })?;

                            Ok(Value::Null)
                        }
                        OperationType::Delete => {
                            if op.file_path.exists() {
                                fs::remove_file(&op.file_path).await.map_err(|e| {
                                    ServerError::internal(format!("Failed to delete file: {}", e))
                                })?;
                            }
                            Ok(Value::Null)
                        }
                        OperationType::Rename => {
                            let new_path_str = op
                                .params
                                .get("new_path")
                                .and_then(|v| v.as_str())
                                .ok_or_else(|| {
                                ServerError::internal("Missing 'new_path' parameter for Rename")
                            })?;
                            let new_path = Path::new(new_path_str);

                            fs::rename(&op.file_path, new_path).await.map_err(|e| {
                                ServerError::internal(format!("Failed to rename file: {}", e))
                            })?;
                            Ok(Value::Null)
                        }
                        _ => Err(ServerError::internal(format!(
                            "Unsupported operation type in worker: {:?}",
                            op.operation_type
                        ))),
                    };

                    // Update stats AFTER all I/O is complete (including sync_all)
                    let mut stats_guard = stats.lock().await;
                    match &result {
                        Ok(_) => {
                            stats_guard.completed_operations += 1;
                            tracing::info!(
                                operation_id = %op.id,
                                operation_type = ?op.operation_type,
                                completed = stats_guard.completed_operations,
                                "Operation executed successfully"
                            );
                        }
                        Err(e) => {
                            stats_guard.failed_operations += 1;
                            tracing::error!(
                                operation_id = %op.id,
                                operation_type = ?op.operation_type,
                                error = %e,
                                failed = stats_guard.failed_operations,
                                "Operation execution failed"
                            );
                        }
                    }
                    drop(stats_guard); // Explicitly release lock

                    result
                })
                .await;
        });
    }

    // Create application state
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
        language_plugins: cb_handlers::LanguagePluginRegistry::from_registry(plugin_registry),
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

        // Shutdown dispatcher (which shutdowns LSP clients)
        if let Err(e) = self._dispatcher.shutdown().await {
            tracing::warn!(
                error = %e,
                "Failed to shutdown dispatcher cleanly"
            );
        }

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