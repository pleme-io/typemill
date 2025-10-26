//! Test helpers for integration tests

use crate::handlers::plugin_dispatcher::{AppState, PluginDispatcher};
use crate::services::operation_queue::OperationType;
use crate::services::{DefaultAstService, FileService, LockManager, OperationQueue};
use crate::workspaces::WorkspaceManager;
use mill_ast::AstCache;
use mill_config::AppConfig;
use mill_plugin_system::PluginManager;
use std::sync::Arc;

/// Spawn background worker to process file operations from the queue (test version)
fn spawn_test_worker(queue: Arc<OperationQueue>) {
    use tokio::fs;

    tokio::spawn(async move {
        eprintln!("DEBUG: Test worker started");
        queue
            .process_with(|op, stats| async move {
                eprintln!(
                    "DEBUG: Test worker processing: {:?} on {}",
                    op.operation_type,
                    op.file_path.display()
                );

                // Process the operation
                let result = match op.operation_type {
                    OperationType::CreateDir => {
                        fs::create_dir_all(&op.file_path).await.map_err(|e| {
                            mill_foundation::protocol::ApiError::Internal(format!(
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

                        // Write and explicitly sync to disk to avoid caching issues
                        let mut file = fs::File::create(&op.file_path).await.map_err(|e| {
                            mill_foundation::protocol::ApiError::Internal(format!(
                                "Failed to create file {}: {}",
                                op.file_path.display(),
                                e
                            ))
                        })?;

                        use tokio::io::AsyncWriteExt;
                        file.write_all(content.as_bytes()).await.map_err(|e| {
                            mill_foundation::protocol::ApiError::Internal(format!(
                                "Failed to write content to {}: {}",
                                op.file_path.display(),
                                e
                            ))
                        })?;

                        // CRITICAL: Sync file to disk BEFORE updating stats
                        file.sync_all().await.map_err(|e| {
                            mill_foundation::protocol::ApiError::Internal(format!(
                                "Failed to sync file {}: {}",
                                op.file_path.display(),
                                e
                            ))
                        })?;

                        eprintln!(
                            "DEBUG: Wrote {} bytes to {} (with sync)",
                            content.len(),
                            op.file_path.display()
                        );
                        Ok(())
                    }
                    OperationType::Delete => {
                        if op.file_path.exists() {
                            fs::remove_file(&op.file_path).await.map_err(|e| {
                                mill_foundation::protocol::ApiError::Internal(format!(
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
                                mill_foundation::protocol::ApiError::InvalidRequest(
                                    "Rename operation missing new_path".to_string(),
                                )
                            })?;
                        fs::rename(&op.file_path, new_path_str).await.map_err(|e| {
                            mill_foundation::protocol::ApiError::Internal(format!(
                                "Failed to rename file {} to {}: {}",
                                op.file_path.display(),
                                new_path_str,
                                e
                            ))
                        })
                    }
                    OperationType::Read
                    | OperationType::Format
                    | OperationType::Refactor
                    | OperationType::UpdateDependency => {
                        // These operations don't modify filesystem, just log
                        eprintln!(
                            "DEBUG: Skipping non-modifying operation: {:?}",
                            op.operation_type
                        );
                        Ok(())
                    }
                };

                // Update stats AFTER all I/O is complete (including sync_all)
                let mut stats_guard = stats.lock().await;
                match result {
                    Ok(_) => {
                        stats_guard.completed_operations += 1;
                        eprintln!(
                            "DEBUG: Operation completed, stats updated (completed={})",
                            stats_guard.completed_operations
                        );
                    }
                    Err(ref e) => {
                        stats_guard.failed_operations += 1;
                        eprintln!(
                            "DEBUG: Operation failed: {} (failed={})",
                            e, stats_guard.failed_operations
                        );
                    }
                }
                drop(stats_guard); // Explicitly release lock

                result.map(|_| serde_json::json!({"success": true}))
            })
            .await;
    });
}

/// Create a test dispatcher for integration tests with a custom project root
pub async fn create_test_dispatcher_with_root(
    project_root: std::path::PathBuf,
) -> PluginDispatcher {
    // Build language plugin registry (centralized)
    let plugin_registry = mill_services::services::build_language_plugin_registry();

    let ast_cache = Arc::new(AstCache::new());
    let ast_service = Arc::new(DefaultAstService::new(
        ast_cache.clone(),
        plugin_registry.clone(),
    ));
    let lock_manager = Arc::new(LockManager::new());
    let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    // Spawn operation queue worker to process file operations
    spawn_test_worker(operation_queue.clone());

    // Use default config for tests
    let config = AppConfig::default();

    let file_service = Arc::new(FileService::new(
        project_root.clone(),
        ast_cache.clone(),
        lock_manager.clone(),
        operation_queue.clone(),
        &config,
        plugin_registry.clone(),
    ));
    let planner = crate::services::planner::DefaultPlanner::new();
    let plugin_manager = Arc::new(PluginManager::new());
    let workflow_executor =
        crate::services::workflow_executor::DefaultWorkflowExecutor::new(plugin_manager.clone());
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
        language_plugins: mill_handlers::LanguagePluginRegistry::from_registry(plugin_registry),
    });

    PluginDispatcher::new(app_state, plugin_manager)
}

/// Create a test dispatcher with a temporary directory (for backward compatibility)
#[allow(clippy::expect_used)]
pub async fn create_test_dispatcher() -> PluginDispatcher {
    let temp_dir = std::env::temp_dir().join(format!("mill-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");
    create_test_dispatcher_with_root(temp_dir).await
}
