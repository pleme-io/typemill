//! Shared dispatcher initialization factory
//!
//! Eliminates 3x duplication across CLI, stdio, WebSocket entry points

use cb_plugins::PluginManager;
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
    // Create AppState with all required services
    let app_state = crate::create_app_state(workspace_manager).await?;

    // Create plugin manager
    let plugin_manager = Arc::new(PluginManager::new());

    // Create dispatcher
    let dispatcher = Arc::new(PluginDispatcher::new(app_state, plugin_manager));

    // Initialize dispatcher (loads plugins, starts LSP servers)
    dispatcher
        .initialize()
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    Ok(dispatcher)
}
