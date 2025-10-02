//! Shared dispatcher initialization factory
//!
//! Eliminates duplication across CLI, stdio, WebSocket entry points

use cb_server::handlers::plugin_dispatcher::PluginDispatcher;
use cb_server::workspaces::WorkspaceManager;
use std::sync::Arc;

/// Create and initialize a PluginDispatcher with all dependencies
pub async fn create_initialized_dispatcher() -> Result<Arc<PluginDispatcher>, std::io::Error> {
    let workspace_manager = Arc::new(WorkspaceManager::new());
    create_initialized_dispatcher_with_workspace(workspace_manager).await
}

/// Create dispatcher with custom workspace manager (for testing)
pub async fn create_initialized_dispatcher_with_workspace(
    workspace_manager: Arc<WorkspaceManager>,
) -> Result<Arc<PluginDispatcher>, std::io::Error> {
    // Load configuration
    let config = cb_core::config::AppConfig::load()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    // Create dispatcher using shared library function (reduces duplication)
    let dispatcher =
        cb_server::create_dispatcher_with_workspace(Arc::new(config), workspace_manager)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    // Initialize dispatcher (loads plugins, starts LSP servers)
    dispatcher
        .initialize()
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    Ok(dispatcher)
}
