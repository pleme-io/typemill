//! mill-server: Core server implementation for TypeMill
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

#[cfg(test)]
mod worker_tests;

// Re-export workspaces from mill-workspaces for backward compatibility
pub use mill_workspaces as workspaces;

// Re-export from new crates for backward compatibility
pub use mill_handlers::handlers;
pub use mill_services::services;

use mill_config::AppConfig;
pub use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
pub use mill_foundation::protocol::{AstService, LspService};
use mill_handlers::handlers::plugin_dispatcher::{AppState, PluginDispatcher};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::oneshot;

/// Server configuration options
#[derive(Clone)]
pub struct ServerOptions {
    /// Application configuration
    pub config: AppConfig,
    /// Enable debug mode
    pub debug: bool,
    /// Optional pre-built language plugin registry (for dependency injection)
    /// If None, will build registry automatically using all available plugins
    pub plugin_registry: Option<Arc<mill_plugin_api::PluginDiscovery>>,
}

impl std::fmt::Debug for ServerOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerOptions")
            .field("config", &self.config)
            .field("debug", &self.debug)
            .field(
                "plugin_registry",
                &self.plugin_registry.as_ref().map(|_| "<PluginDiscovery>"),
            )
            .finish()
    }
}

/// Handle to a running server
pub struct ServerHandle {
    shutdown_tx: oneshot::Sender<()>,
    _config: AppConfig,
    _dispatcher: Arc<PluginDispatcher>,
}

/// Bootstrap the server with given options
pub async fn bootstrap(options: ServerOptions) -> ServerResult<ServerHandle> {
    tracing::info!("Bootstrapping TypeMill server");

    // Validate configuration
    if options.config.server.port == 0 {
        return Err(ServerError::config("Invalid server port"));
    }

    // Get project root
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Use the app_state_factory to create services and app_state
    use mill_services::services::app_state_factory::create_services_bundle;
    #[cfg(feature = "mcp-proxy")]
    use mill_services::services::app_state_factory::register_mcp_proxy_if_enabled;

    let cache_settings = mill_ast::CacheSettings::from_config(
        options.config.cache.enabled,
        options.config.cache.ttl_seconds,
        options.config.cache.max_size_bytes,
    );

    let plugin_manager = Arc::new(mill_plugin_system::PluginManager::new());

    // Register MCP proxy plugin if feature enabled
    #[cfg(feature = "mcp-proxy")]
    register_mcp_proxy_if_enabled(&plugin_manager, options.config.external_mcp.as_ref()).await?;

    // Use injected plugin registry or build one
    let plugin_registry = options.plugin_registry.unwrap_or_else(|| {
        tracing::debug!("No plugin registry injected, building default registry (empty)");
        mill_services::services::registry_builder::build_language_plugin_registry(vec![])
    });

    let services = create_services_bundle(
        &project_root,
        cache_settings,
        plugin_manager.clone(),
        &options.config,
        plugin_registry.clone(),
    )
    .await;

    let workspace_manager = Arc::new(mill_workspaces::WorkspaceManager::new());

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
        language_plugins: mill_handlers::LanguagePluginRegistry::from_registry(plugin_registry),
        lsp_mode: options.config.lsp.mode,
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
    ///
    /// By default, this does not inject a plugin registry - the server will build
    /// one automatically. For dependency injection, use `with_plugin_registry()`.
    pub fn from_config(config: AppConfig) -> Self {
        Self {
            config,
            debug: false,
            plugin_registry: None,
        }
    }

    /// Set a pre-built plugin registry (for dependency injection)
    ///
    /// This allows the application layer to control which language plugins are loaded.
    ///
    /// # Example
    /// ```no_run
    /// use mill_server::ServerOptions;
    /// use mill_services::services::registry_builder::build_language_plugin_registry;
    /// use mill_config::AppConfig;
    ///
    /// # let config = AppConfig::default();
    /// let registry = build_language_plugin_registry(vec![]);
    /// let options = ServerOptions::from_config(config)
    ///     .with_plugin_registry(registry);
    /// ```
    pub fn with_plugin_registry(mut self, registry: Arc<mill_plugin_api::PluginDiscovery>) -> Self {
        self.plugin_registry = Some(registry);
        self
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
/// the standalone binary (main.rs) and the unified binary (apps/mill).
pub async fn create_dispatcher_with_workspace(
    config: Arc<AppConfig>,
    workspace_manager: Arc<mill_workspaces::WorkspaceManager>,
    plugin_registry: Arc<mill_plugin_api::PluginDiscovery>,
) -> ServerResult<Arc<PluginDispatcher>> {
    // Get project root
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Use the app_state_factory to create services
    use mill_services::services::app_state_factory::create_services_bundle;
    #[cfg(feature = "mcp-proxy")]
    use mill_services::services::app_state_factory::register_mcp_proxy_if_enabled;

    let cache_settings = mill_ast::CacheSettings::from_config(
        config.cache.enabled,
        config.cache.ttl_seconds,
        config.cache.max_size_bytes,
    );

    let plugin_manager = Arc::new(mill_plugin_system::PluginManager::new());

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
    spawn_operation_worker(services.operation_queue.clone(), project_root.clone());

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
        language_plugins: mill_handlers::LanguagePluginRegistry::from_registry(plugin_registry),
        lsp_mode: config.lsp.mode,
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
        // server startup is handled by mill-transport layer

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

/// Convert path to absolute and verify it's within project root
async fn validate_path(
    project_root: &std::path::Path,
    path: &std::path::Path,
) -> ServerResult<PathBuf> {
    use tokio::fs;

    let canonical_root = fs::canonicalize(project_root).await.map_err(|e| {
        ServerError::internal(format!(
            "Failed to canonicalize project root {:?}: {}",
            project_root, e
        ))
    })?;

    // Convert to absolute
    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    };

    // Try to canonicalize the full path if it exists
    // We use fs::metadata as a way to check existence async
    let canonical = if fs::metadata(&abs_path).await.is_ok() {
        fs::canonicalize(&abs_path).await.map_err(|e| {
            ServerError::invalid_request(format!(
                "Path canonicalization failed for {:?}: {}",
                abs_path, e
            ))
        })?
    } else {
        // Path doesn't exist - find first existing ancestor and build from there
        let mut current = abs_path.clone();
        let mut components_to_add = Vec::new();

        // Walk up until we find an existing directory
        // Loop bound: preventing infinite loop if root is missing (though unlikely if project_root exists)
        loop {
            if fs::metadata(&current).await.is_ok() {
                break;
            }

            if let Some(filename) = current.file_name() {
                components_to_add.push(filename.to_os_string());
                if let Some(parent) = current.parent() {
                    current = parent.to_path_buf();
                } else {
                    // Reached root without finding existing path
                    return Err(ServerError::invalid_request(format!(
                        "Cannot validate path: no existing ancestor found for {:?}",
                        abs_path
                    )));
                }
            } else {
                return Err(ServerError::invalid_request(format!(
                    "Invalid path: no filename component in {:?}",
                    current
                )));
            }
        }

        // Canonicalize the existing ancestor
        let mut canonical = fs::canonicalize(&current).await.map_err(|e| {
            ServerError::invalid_request(format!(
                "Path canonicalization failed for {:?}: {}",
                current, e
            ))
        })?;

        // Add back the non-existing components
        for component in components_to_add.iter().rev() {
            canonical = canonical.join(component);
        }

        canonical
    };

    // Verify containment within project root
    if !canonical.starts_with(&canonical_root) {
        return Err(ServerError::permission_denied(format!(
            "Path traversal detected: {:?} escapes project root {:?}",
            path, project_root
        )));
    }

    Ok(canonical)
}

/// Spawn a worker to process file operations in the background
pub fn spawn_operation_worker(
    queue: Arc<mill_services::services::OperationQueue>,
    project_root: PathBuf,
) {
    tokio::spawn(async move {
        use mill_services::services::OperationType;
        use serde_json::Value;
        use std::path::Path;
        use tokio::fs;
        use tokio::io::AsyncWriteExt;

        queue
            .process_with(move |op, stats| {
                let project_root = project_root.clone();
                async move {
                    tracing::debug!(
                        operation_id = %op.id,
                        operation_type = ?op.operation_type,
                        file_path = %op.file_path.display(),
                        "Executing queued operation"
                    );

                    // Security check: Validate path before any operation
                    let valid_path = match validate_path(&project_root, &op.file_path).await {
                        Ok(p) => p,
                        Err(e) => {
                            let mut stats_guard = stats.lock().await;
                            stats_guard.failed_operations += 1;
                            tracing::error!(
                                operation_id = %op.id,
                                error = %e,
                                "Security check failed: Path traversal prevented"
                            );
                            return Err(e);
                        }
                    };

                    // Use valid_path instead of op.file_path for subsequent operations
                    let result = match op.operation_type {
                        OperationType::CreateDir => {
                            fs::create_dir_all(&valid_path).await.map_err(|e| {
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
                            let mut file = fs::File::create(&valid_path).await.map_err(|e| {
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
                            if valid_path.exists() {
                                fs::remove_file(&valid_path).await.map_err(|e| {
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

                            // Also validate new_path
                            let valid_new_path = validate_path(&project_root, new_path).await?;

                            fs::rename(&valid_path, valid_new_path).await.map_err(|e| {
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
                }
            })
            .await;
    });
}
