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
