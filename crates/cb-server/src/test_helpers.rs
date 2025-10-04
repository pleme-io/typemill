//! Test helpers for integration tests

use crate::handlers::plugin_dispatcher::{AppState, PluginDispatcher};
use crate::services::{DefaultAstService, FileService, LockManager, OperationQueue};
use crate::workspaces::WorkspaceManager;
use cb_ast::AstCache;
use cb_core::AppConfig;
use cb_plugins::PluginManager;
use std::sync::Arc;

/// Create a test dispatcher for integration tests
///
/// Note: The dispatcher will use a temporary directory that will be cleaned up when dropped
pub fn create_test_dispatcher() -> PluginDispatcher {
    // Use a temporary directory that won't be cleaned up during the test
    let temp_dir = std::env::temp_dir().join(format!("codebuddy-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    let ast_cache = Arc::new(AstCache::new());
    let ast_service = Arc::new(DefaultAstService::new(ast_cache.clone()));
    let project_root = temp_dir;
    let lock_manager = Arc::new(LockManager::new());
    let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    // Use default config for tests
    let config = AppConfig::default();

    let file_service = Arc::new(FileService::new(
        project_root.clone(),
        ast_cache.clone(),
        lock_manager.clone(),
        operation_queue.clone(),
        &config,
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
        language_plugins: cb_handlers::LanguagePluginRegistry::new(),
    });

    PluginDispatcher::new(app_state, plugin_manager)
}
