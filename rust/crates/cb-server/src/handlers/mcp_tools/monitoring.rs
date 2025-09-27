//! Monitoring and observability tools

use crate::handlers::McpDispatcher;
use serde_json::json;

/// Register monitoring tools
pub fn register_tools(dispatcher: &mut McpDispatcher) {
    // server/getQueueStats tool - Read-only monitoring of queue statistics
    dispatcher.register_tool("server/getQueueStats".to_string(), |app_state, _args| async move {
        tracing::debug!("Getting queue statistics");

        // Get current queue statistics
        let stats = app_state.operation_queue.get_stats().await;

        // Convert to JSON format with camelCase keys as specified
        Ok(json!({
            "totalOperations": stats.total_operations,
            "pendingOperations": stats.pending_operations,
            "completedOperations": stats.completed_operations,
            "failedOperations": stats.failed_operations,
            "averageWaitTime": format!("{:?}", stats.average_wait_time),
            "maxWaitTime": format!("{:?}", stats.max_wait_time),
        }))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::AppState;
    use crate::services::{LockManager, OperationQueue, FileService};
    use crate::systems::LspManager;
    use cb_core::config::LspConfig;
    use std::sync::Arc;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_get_queue_stats() {
        // Create test app state
        let lsp_config = LspConfig::default();
        let lsp_manager = Arc::new(LspManager::new(lsp_config));
        let file_service = Arc::new(FileService::new(PathBuf::from("/tmp")));
        let project_root = PathBuf::from("/tmp");
        let lock_manager = Arc::new(LockManager::new());
        let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

        let app_state = Arc::new(AppState {
            lsp: lsp_manager,
            file_service,
            project_root,
            lock_manager,
            operation_queue,
        });

        let mut dispatcher = McpDispatcher::new(app_state.clone());
        register_tools(&mut dispatcher);

        // Call the tool
        let result = dispatcher.handle_tool_call(Some(json!({
            "name": "server/getQueueStats",
            "arguments": {}
        }))).await;

        assert!(result.is_ok());
        let response = result.unwrap();

        // Check that all required keys are present
        assert!(response.get("totalOperations").is_some());
        assert!(response.get("pendingOperations").is_some());
        assert!(response.get("completedOperations").is_some());
        assert!(response.get("failedOperations").is_some());
        assert!(response.get("averageWaitTime").is_some());
        assert!(response.get("maxWaitTime").is_some());
    }
}