//! Factory for creating AppState instances
//! Consolidates duplicate initialization logic

#![allow(unexpected_cfgs)]

use super::*;
use cb_ast::AstCache;
use std::path::PathBuf;
use std::sync::Arc;

/// Bundle of core services used by AppState
pub struct ServicesBundle {
    pub ast_service: Arc<dyn codebuddy_foundation::protocol::AstService>,
    pub file_service: Arc<FileService>,
    pub lock_manager: Arc<LockManager>,
    pub operation_queue: Arc<OperationQueue>,
    pub planner: Arc<dyn planner::Planner>,
    pub workflow_executor: Arc<dyn workflow_executor::WorkflowExecutor>,
}

/// Create services bundle with default configuration
///
/// # Parameters
/// - `plugin_registry`: Pre-built plugin registry (injected by the application layer)
pub async fn create_services_bundle(
    project_root: &PathBuf,
    cache_settings: cb_ast::CacheSettings,
    plugin_manager: Arc<codebuddy_plugin_system::PluginManager>,
    config: &codebuddy_config::AppConfig,
    plugin_registry: Arc<cb_plugin_api::PluginRegistry>,
) -> ServicesBundle {
    // Plugin registry is now injected by the caller (dependency injection)

    let ast_cache = Arc::new(AstCache::with_settings(cache_settings));
    let ast_service = Arc::new(DefaultAstService::new(
        ast_cache.clone(),
        plugin_registry.clone(),
    ));
    let lock_manager = Arc::new(LockManager::new());
    let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    // Spawn operation queue worker to process file operations
    spawn_operation_worker(operation_queue.clone(), plugin_manager.clone());

    let file_service = Arc::new(FileService::new(
        project_root,
        ast_cache.clone(),
        lock_manager.clone(),
        operation_queue.clone(),
        config,
        plugin_registry,
    ));
    let planner = planner::DefaultPlanner::new();
    let workflow_executor = workflow_executor::DefaultWorkflowExecutor::new(plugin_manager);

    ServicesBundle {
        ast_service,
        file_service,
        lock_manager,
        operation_queue,
        planner,
        workflow_executor,
    }
}

/// Spawn background worker to process file operations from the queue
fn spawn_operation_worker(
    queue: Arc<super::operation_queue::OperationQueue>,
    plugin_manager: Arc<codebuddy_plugin_system::PluginManager>,
) {
    use super::operation_queue::OperationType;
    use tokio::fs;

    tokio::spawn(async move {
        tracing::info!("Operation queue worker started");
        queue
            .process_with(move |op, stats| {
                let plugin_manager = plugin_manager.clone();
                async move {
                    tracing::info!(
                        op_type = ?op.operation_type,
                        file_path = %op.file_path.display(),
                        "Processing queued operation"
                    );

                    // Process the operation
                    let result = match op.operation_type {
                        OperationType::CreateDir => {
                            fs::create_dir_all(&op.file_path).await.map_err(|e| {
                                codebuddy_foundation::protocol::ApiError::Internal(format!(
                                    "Failed to create directory {}: {}",
                                    op.file_path.display(),
                                    e
                                ))
                            })
                        }
                        OperationType::CreateFile | OperationType::Write => {
                            let content = op
                                .params
                                .get("content")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");

                            let mut file = fs::File::create(&op.file_path).await.map_err(|e| {
                                codebuddy_foundation::protocol::ApiError::Internal(format!(
                                    "Failed to create file {}: {}",
                                    op.file_path.display(),
                                    e
                                ))
                            })?;

                            use tokio::io::AsyncWriteExt;
                            file.write_all(content.as_bytes()).await.map_err(|e| {
                                codebuddy_foundation::protocol::ApiError::Internal(format!(
                                    "Failed to write content to {}: {}",
                                    op.file_path.display(),
                                    e
                                ))
                            })?;

                            file.sync_all().await.map_err(|e| {
                                codebuddy_foundation::protocol::ApiError::Internal(format!(
                                    "Failed to sync file {}: {}",
                                    op.file_path.display(),
                                    e
                                ))
                            })?;

                            Ok(())
                        }
                        OperationType::Delete => {
                            if op.file_path.exists() {
                                fs::remove_file(&op.file_path).await.map_err(|e| {
                                    codebuddy_foundation::protocol::ApiError::Internal(format!(
                                        "Failed to delete file {}: {}",
                                        op.file_path.display(),
                                        e
                                    ))
                                })
                            } else {
                                Ok(())
                            }
                        }
                        OperationType::Rename => {
                            let new_path_str = op
                                .params
                                .get("new_path")
                                .and_then(|v| v.as_str())
                                .ok_or_else(|| {
                                codebuddy_foundation::protocol::ApiError::InvalidRequest(
                                    "Rename operation missing new_path".to_string(),
                                )
                            })?;
                            fs::rename(&op.file_path, new_path_str).await.map_err(|e| {
                                codebuddy_foundation::protocol::ApiError::Internal(format!(
                                    "Failed to rename file {} to {}: {}",
                                    op.file_path.display(),
                                    new_path_str,
                                    e
                                ))
                            })
                        }
                        OperationType::UpdateDependency => {
                            use codebuddy_plugin_system::protocol::PluginRequest;
                            let request =
                                PluginRequest::new("update_dependency", op.file_path.clone())
                                    .with_params(op.params.clone());

                            plugin_manager
                                .handle_request(request)
                                .await
                                .map(|_| ())
                                .map_err(|e| codebuddy_foundation::protocol::ApiError::Plugin(e.to_string()))
                        }
                        OperationType::Read | OperationType::Format | OperationType::Refactor => {
                            tracing::trace!(
                                op_type = ?op.operation_type,
                                path = %op.file_path.display(),
                                "Operation queued"
                            );
                            Ok(())
                        }
                    };

                    let mut stats_guard = stats.lock().await;
                    match result {
                        Ok(_) => {
                            stats_guard.completed_operations += 1;
                        }
                        Err(ref e) => {
                            stats_guard.failed_operations += 1;
                            tracing::error!(error = %e, "Operation failed");
                        }
                    }
                    drop(stats_guard);

                    result.map(|_| serde_json::json!({"success": true}))
                }
            })
            .await;
    });
}

/// Register MCP proxy plugin if feature is enabled
#[cfg(feature = "mcp-proxy")]
pub async fn register_mcp_proxy_if_enabled(
    plugin_manager: &Arc<codebuddy_plugin_system::PluginManager>,
    external_mcp_config: Option<&codebuddy_config::config::ExternalMcpConfig>,
) -> Result<(), codebuddy_foundation::protocol::ApiError> {
    if let Some(config) = external_mcp_config {
        use codebuddy_plugin_system::mcp::McpProxyPlugin;
        use codebuddy_plugin_system::LanguagePlugin;

        tracing::info!(
            servers_count = config.servers.len(),
            "Registering MCP proxy plugin"
        );

        let mut plugin = McpProxyPlugin::new(config.servers.clone());
        plugin.initialize().await.map_err(|e| {
            codebuddy_foundation::protocol::ApiError::plugin(format!("Failed to initialize MCP proxy plugin: {}", e))
        })?;

        plugin_manager
            .register_plugin("mcp-proxy", Arc::new(plugin))
            .await
            .map_err(|e| {
                codebuddy_foundation::protocol::ApiError::plugin(format!("Failed to register MCP proxy plugin: {}", e))
            })?;
    }
    Ok(())
}
