//! Integration tests for monitoring tools

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::handlers::McpDispatcher;
    use crate::state::AppState;
    use crate::services::{FileService, SymbolService, EditingService, ImportService};
    use crate::systems::lsp::MockLspService;
    use crate::systems::operation_queue::{OperationQueue, QueueStats};
    use std::sync::Arc;
    use serde_json::{json, Value};
    use std::time::Duration;

    /// Create a test AppState with mock services
    fn create_test_app_state() -> Arc<AppState> {
        let mock_lsp = MockLspService::new();
        Arc::new(AppState {
            file_service: Arc::new(FileService::new()),
            symbol_service: Arc::new(SymbolService::new(Arc::new(mock_lsp.clone()))),
            editing_service: Arc::new(EditingService::new(Arc::new(mock_lsp.clone()))),
            import_service: Arc::new(ImportService::new()),
            lsp_service: Arc::new(mock_lsp),
            operation_queue: Arc::new(OperationQueue::new()),
        })
    }

    #[tokio::test]
    async fn test_get_queue_stats_empty_queue() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::monitoring::register_tools(&mut dispatcher);

        let args = json!({});

        let result = dispatcher.call_tool_for_test("server/getQueueStats", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["totalOperations"], 0);
        assert_eq!(response["pendingOperations"], 0);
        assert_eq!(response["completedOperations"], 0);
        assert_eq!(response["failedOperations"], 0);
        assert!(response["averageWaitTime"].is_string());
        assert!(response["maxWaitTime"].is_string());
    }

    #[tokio::test]
    async fn test_get_queue_stats_with_operations() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        // Simulate some queue activity
        let stats = QueueStats {
            total_operations: 10,
            pending_operations: 2,
            completed_operations: 7,
            failed_operations: 1,
            average_wait_time: Duration::from_millis(250),
            max_wait_time: Duration::from_millis(500),
        };

        // Mock the queue stats
        app_state.operation_queue.set_mock_stats(stats).await;

        super::super::monitoring::register_tools(&mut dispatcher);

        let args = json!({});

        let result = dispatcher.call_tool_for_test("server/getQueueStats", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["totalOperations"], 10);
        assert_eq!(response["pendingOperations"], 2);
        assert_eq!(response["completedOperations"], 7);
        assert_eq!(response["failedOperations"], 1);

        // Check that wait times are formatted properly
        let avg_wait = response["averageWaitTime"].as_str().unwrap();
        let max_wait = response["maxWaitTime"].as_str().unwrap();
        assert!(avg_wait.contains("250ms") || avg_wait.contains("250"));
        assert!(max_wait.contains("500ms") || max_wait.contains("500"));
    }

    #[tokio::test]
    async fn test_get_queue_stats_concurrent_access() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::monitoring::register_tools(&mut dispatcher);

        // Execute multiple concurrent requests to test thread safety
        let tasks = (0..5).map(|_| {
            dispatcher.call_tool_for_test("server/getQueueStats", json!({}))
        }).collect::<Vec<_>>();

        let results = futures::future::join_all(tasks).await;

        // All requests should succeed
        for result in results {
            assert!(result.is_ok());
            let response = result.unwrap();
            assert!(response["totalOperations"].is_number());
            assert!(response["pendingOperations"].is_number());
            assert!(response["completedOperations"].is_number());
            assert!(response["failedOperations"].is_number());
        }
    }

    #[tokio::test]
    async fn test_get_queue_stats_response_format() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::monitoring::register_tools(&mut dispatcher);

        let args = json!({});

        let result = dispatcher.call_tool_for_test("server/getQueueStats", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();

        // Verify all expected fields are present
        assert!(response.get("totalOperations").is_some());
        assert!(response.get("pendingOperations").is_some());
        assert!(response.get("completedOperations").is_some());
        assert!(response.get("failedOperations").is_some());
        assert!(response.get("averageWaitTime").is_some());
        assert!(response.get("maxWaitTime").is_some());

        // Verify camelCase formatting
        assert!(response.get("total_operations").is_none()); // snake_case should not exist
        assert!(response.get("pending_operations").is_none());
        assert!(response.get("completed_operations").is_none());
        assert!(response.get("failed_operations").is_none());
    }

    #[tokio::test]
    async fn test_get_queue_stats_large_numbers() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        // Test with large operation counts
        let stats = QueueStats {
            total_operations: 1_000_000,
            pending_operations: 50_000,
            completed_operations: 949_500,
            failed_operations: 500,
            average_wait_time: Duration::from_secs(2),
            max_wait_time: Duration::from_secs(10),
        };

        app_state.operation_queue.set_mock_stats(stats).await;

        super::super::monitoring::register_tools(&mut dispatcher);

        let args = json!({});

        let result = dispatcher.call_tool_for_test("server/getQueueStats", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["totalOperations"], 1_000_000);
        assert_eq!(response["pendingOperations"], 50_000);
        assert_eq!(response["completedOperations"], 949_500);
        assert_eq!(response["failedOperations"], 500);
    }

    #[tokio::test]
    async fn test_get_queue_stats_zero_wait_times() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        // Test with zero wait times (immediate processing)
        let stats = QueueStats {
            total_operations: 5,
            pending_operations: 0,
            completed_operations: 5,
            failed_operations: 0,
            average_wait_time: Duration::from_nanos(0),
            max_wait_time: Duration::from_nanos(0),
        };

        app_state.operation_queue.set_mock_stats(stats).await;

        super::super::monitoring::register_tools(&mut dispatcher);

        let args = json!({});

        let result = dispatcher.call_tool_for_test("server/getQueueStats", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["totalOperations"], 5);
        assert_eq!(response["pendingOperations"], 0);
        assert_eq!(response["completedOperations"], 5);
        assert_eq!(response["failedOperations"], 0);
    }

    #[tokio::test]
    async fn test_get_queue_stats_invalid_args() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::monitoring::register_tools(&mut dispatcher);

        // Test with unexpected arguments (should be ignored gracefully)
        let args = json!({
            "unexpected_field": "value",
            "another_field": 123
        });

        let result = dispatcher.call_tool_for_test("server/getQueueStats", args).await;
        assert!(result.is_ok()); // Should still work, ignoring extra args

        let response = result.unwrap();
        assert!(response["totalOperations"].is_number());
    }

    #[tokio::test]
    async fn test_get_queue_stats_consistency() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::monitoring::register_tools(&mut dispatcher);

        // Make multiple calls and verify consistency
        let result1 = dispatcher.call_tool_for_test("server/getQueueStats", json!({})).await;
        let result2 = dispatcher.call_tool_for_test("server/getQueueStats", json!({})).await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let response1 = result1.unwrap();
        let response2 = result2.unwrap();

        // Stats should be consistent between calls (assuming no operations in between)
        assert_eq!(response1["totalOperations"], response2["totalOperations"]);
        assert_eq!(response1["pendingOperations"], response2["pendingOperations"]);
        assert_eq!(response1["completedOperations"], response2["completedOperations"]);
        assert_eq!(response1["failedOperations"], response2["failedOperations"]);
    }
}