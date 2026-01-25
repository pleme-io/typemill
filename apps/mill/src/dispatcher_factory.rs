//! Shared dispatcher initialization factory
//!
//! Eliminates duplication across CLI, stdio, WebSocket entry points

use mill_server::handlers::plugin_dispatcher::PluginDispatcher;
use mill_server::workspaces::WorkspaceManager;
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
    let config =
        mill_config::config::AppConfig::load().map_err(|e| std::io::Error::other(e.to_string()))?;

    // Build plugin registry using mill-services
    // We now pass the plugin instances to the service layer for registration
    let plugins = mill_plugin_bundle::all_plugins();
    // Use mill_server re-export to access services since mill-services is not a direct dep of apps/mill
    let plugin_registry =
        mill_server::services::registry_builder::build_language_plugin_registry(plugins);

    // Create dispatcher using shared library function (reduces duplication)
    let dispatcher = mill_server::create_dispatcher_with_workspace(
        Arc::new(config),
        workspace_manager,
        plugin_registry,
    )
    .await
    .map_err(|e| std::io::Error::other(e.to_string()))?;

    // Initialize dispatcher (loads plugins, starts LSP servers)
    dispatcher
        .initialize()
        .await
        .map_err(|e| std::io::Error::other(e.to_string()))?;

    Ok(dispatcher)
}
