//! Shared test utilities

use mill_handlers::handlers::plugin_dispatcher::AppState;
use std::sync::Arc;
use tempfile::TempDir;

pub async fn create_test_app_state() -> (Arc<AppState>, TempDir) {
    use mill_plugin_system::PluginManager;
    use mill_services::services::app_state_factory::create_services_bundle;
    use mill_workspaces::WorkspaceManager;

    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();
    let cache_settings = mill_ast::CacheSettings::default();
    let plugin_manager = Arc::new(PluginManager::new());
    let config = mill_config::AppConfig::default();

    // Build plugin registry for tests
    let plugin_registry =
        mill_services::services::registry_builder::build_language_plugin_registry();

    let services = create_services_bundle(
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
        language_plugins: mill_handlers::LanguagePluginRegistry::from_registry(plugin_registry),
    });

    (app_state, temp_dir)
}
