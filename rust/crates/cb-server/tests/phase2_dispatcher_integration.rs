//! Phase 2 integration tests for concurrent reads, priorities, and transactions - OBSOLETE
//! These tests were for the old McpDispatcher which has been replaced by the plugin system
#![cfg(skip_integration_tests)] // Tests disabled - need rewrite for plugin architecture

use super::*;
// NOTE: McpDispatcher no longer exists - replaced by PluginDispatcher
// use crate::handlers::{McpDispatcher, AppState};
use crate::systems::LspManager;
use crate::services::operation_queue::OperationTransaction;
use cb_core::config::LspConfig;
use cb_core::model::mcp::{ToolCall};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, timeout, Instant};
use serde_json::json;
use std::path::PathBuf;
use futures_util::future;

/// Create test app state with lock manager and operation queue
fn create_phase2_test_app_state() -> Arc<AppState> {
    let lsp_config = LspConfig::default();
    let lsp_manager = Arc::new(LspManager::new(lsp_config));
    let file_service = Arc::new(crate::services::FileService::new(std::path::PathBuf::from("/tmp")));
    let project_root = std::path::PathBuf::from("/tmp");
    let lock_manager = Arc::new(LockManager::new());
    let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    Arc::new(AppState {
        lsp: lsp_manager,
        file_service,
        project_root,
        lock_manager,
        operation_queue,
    })
}

/// Test that multiple read operations can execute concurrently
#[tokio::test]
async fn test_concurrent_read_operations() {
    let app_state = create_phase2_test_app_state();
    let mut dispatcher = McpDispatcher::new(app_state.clone());

    // Register a mock read tool
    dispatcher.register_tool("test_read_tool".to_string(), |_app_state, args| async move {
        // Simulate some processing time
        sleep(Duration::from_millis(50)).await;
        Ok(json!({"read_result": args.get("file_path").unwrap_or(&json!("unknown"))}))
    });

    let file_path = "/test/concurrent_read.txt";

    // Create multiple concurrent read operations
    let mut handles = Vec::new();
    let start_time = Instant::now();

    for i in 0..5 {
        let dispatcher_clone = &dispatcher;
        let handle = tokio::spawn(async move {
            let tool_call = ToolCall {
                name: "test_read_tool".to_string(),
                arguments: Some(json!({
                    "file_path": file_path,
                    "operation_id": i
                })),
            };

            dispatcher_clone.handle_tool_call_for_test(Some(json!(tool_call))).await
        });
        handles.push(handle);
    }

    // All operations should complete
    let results: Vec<_> = future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    let elapsed = start_time.elapsed();

    // All operations should succeed
    assert_eq!(results.len(), 5);
    for result in &results {
        assert!(result.is_ok());
    }

    // Since operations run concurrently, total time should be closer to 50ms than 250ms (5 * 50ms)
    // Allow some margin for test execution overhead
    assert!(elapsed < Duration::from_millis(150), "Operations took too long: {:?}, expected concurrent execution", elapsed);
}

/// Test that write operations respect priority ordering
#[tokio::test]
async fn test_priority_ordering() {
    let app_state = create_phase2_test_app_state();
    let operation_queue = app_state.operation_queue.clone();

    let file_path = PathBuf::from("/test/priority_test.txt");

    // Create operations with different priorities
    let operations = vec![
        ("format_tool", OperationType::Format),      // Priority 10 (low)
        ("write_tool", OperationType::Write),        // Priority 5 (medium)
        ("refactor_tool", OperationType::Refactor),  // Priority 1 (high)
        ("delete_tool", OperationType::Delete),      // Priority 3 (high)
        ("rename_tool", OperationType::Rename),      // Priority 2 (high)
    ];

    // Enqueue operations
    for (tool_name, op_type) in operations {
        let priority = match op_type {
            OperationType::Format => 10,
            OperationType::Write => 5,
            OperationType::Delete => 3,
            OperationType::Rename => 2,
            OperationType::Refactor => 1,
            OperationType::Read => 5,
        };

        let operation = FileOperation::new(
            tool_name.to_string(),
            op_type,
            file_path.clone(),
            json!({"test": "data"})
        ).with_priority(priority);

        operation_queue.enqueue(operation).await.unwrap();
    }

    // Check that operations are ordered by priority
    let pending = operation_queue.get_pending_operations().await;
    assert_eq!(pending.len(), 5);

    // First operation should be refactor_tool (priority 1)
    assert!(pending[0].1.contains("refactor_tool"));
    // Second operation should be rename_tool (priority 2)
    assert!(pending[1].1.contains("rename_tool"));
    // Third operation should be delete_tool (priority 3)
    assert!(pending[2].1.contains("delete_tool"));
    // Fourth operation should be write_tool (priority 5)
    assert!(pending[3].1.contains("write_tool"));
    // Last operation should be format_tool (priority 10)
    assert!(pending[4].1.contains("format_tool"));
}

/// Test that read operations bypass the queue and execute immediately
#[tokio::test]
async fn test_read_bypass_queue() {
    let app_state = create_phase2_test_app_state();
    let mut dispatcher = McpDispatcher::new(app_state.clone());

    // Register a mock read tool
    dispatcher.register_tool("bypass_read_tool".to_string(), |_app_state, _args| async move {
        Ok(json!({"status": "read_executed"}))
    });

    let start_time = Instant::now();

    // Execute read operation
    let tool_call = ToolCall {
        name: "bypass_read_tool".to_string(),
        arguments: Some(json!({
            "file_path": "/test/bypass.txt"
        })),
    };

    let result = dispatcher.handle_tool_call_for_test(Some(json!(tool_call))).await.unwrap();
    let elapsed = start_time.elapsed();

    // Operation should succeed
    assert!(result["content"]["status"].as_str().unwrap() == "read_executed");

    // Should execute quickly (not queued)
    assert!(elapsed < Duration::from_millis(50), "Read operation took too long: {:?}", elapsed);

    // Queue should be empty (read operations bypass the queue)
    let stats = app_state.operation_queue.get_stats().await;
    assert_eq!(stats.pending_operations, 0);
    assert_eq!(stats.total_operations, 0);
}

/// Test transaction support for refactoring operations
#[tokio::test]
async fn test_refactoring_transactions() {
    let app_state = create_phase2_test_app_state();
    let mut dispatcher = McpDispatcher::new(app_state.clone());

    // Register a mock refactor tool
    dispatcher.register_tool("rename_symbol".to_string(), |_app_state, args| async move {
        Ok(json!({
            "status": "refactored",
            "symbol": args.get("symbol").unwrap_or(&json!("unknown"))
        }))
    });

    let initial_stats = app_state.operation_queue.get_stats().await;
    assert_eq!(initial_stats.pending_operations, 0);

    // Execute refactoring operation
    let tool_call = ToolCall {
        name: "rename_symbol".to_string(),
        arguments: Some(json!({
            "file_path": "/test/refactor.ts",
            "symbol": "oldName",
            "new_name": "newName"
        })),
    };

    let result = dispatcher.handle_tool_call_for_test(Some(json!(tool_call))).await.unwrap();

    // Operation should succeed
    assert!(result["content"]["status"].as_str().unwrap() == "refactored");

    // Multiple operations should be enqueued (transaction creates multiple file operations)
    let final_stats = app_state.operation_queue.get_stats().await;
    assert!(final_stats.total_operations >= 3, "Expected at least 3 operations from transaction, got {}", final_stats.total_operations);

    // All operations should have high priority (refactor = priority 1)
    let pending = app_state.operation_queue.get_pending_operations().await;
    for (_, tool_name, _, _) in pending {
        assert!(tool_name.contains("rename_symbol_file_operation"));
    }
}

/// Test that write operations wait for read locks to be released
#[tokio::test]
async fn test_read_write_lock_coordination() {
    let app_state = create_phase2_test_app_state();
    let lock_manager = app_state.lock_manager.clone();
    let file_path = PathBuf::from("/test/coordination.txt");

    // Acquire a read lock
    let file_lock = lock_manager.get_lock(&file_path).await;
    let _read_guard = file_lock.read().await;

    // Try to acquire a write lock (should not succeed immediately)
    let write_attempt = timeout(
        Duration::from_millis(10),
        file_lock.write()
    ).await;

    // Write should timeout because read lock is held
    assert!(write_attempt.is_err(), "Write lock should not be acquired while read lock is held");

    // Drop read lock
    drop(_read_guard);

    // Now write lock should be acquirable
    let write_result = timeout(
        Duration::from_millis(10),
        file_lock.write()
    ).await;

    assert!(write_result.is_ok(), "Write lock should be acquired after read lock is released");
}

/// Test concurrent read operations on the same file
#[tokio::test]
async fn test_multiple_concurrent_reads_same_file() {
    let app_state = create_phase2_test_app_state();
    let lock_manager = app_state.lock_manager.clone();
    let file_path = PathBuf::from("/test/multi_read.txt");

    let file_lock = lock_manager.get_lock(&file_path).await;

    // Acquire multiple read locks concurrently
    let start_time = Instant::now();
    let mut handles = Vec::new();

    for i in 0..10 {
        let lock_clone = file_lock.clone();
        let handle = tokio::spawn(async move {
            let _guard = lock_clone.read().await;
            sleep(Duration::from_millis(20)).await;
            i
        });
        handles.push(handle);
    }

    let results: Vec<_> = future::join_all(handles).await;
    let elapsed = start_time.elapsed();

    // All operations should complete successfully
    assert_eq!(results.len(), 10);
    for (i, result) in results.iter().enumerate() {
        assert_eq!(result.as_ref().unwrap(), &i);
    }

    // Should complete in roughly 20ms (concurrent) rather than 200ms (sequential)
    assert!(elapsed < Duration::from_millis(100), "Concurrent reads took too long: {:?}", elapsed);
}

/// Test operation cancellation during priority reordering
#[tokio::test]
async fn test_priority_with_cancellation() {
    let app_state = create_phase2_test_app_state();
    let operation_queue = app_state.operation_queue.clone();

    let file_path = PathBuf::from("/test/cancel_priority.txt");

    // Enqueue low priority operation
    let low_priority_op = FileOperation::new(
        "format_tool".to_string(),
        OperationType::Format,
        file_path.clone(),
        json!({"test": "data"})
    ).with_priority(10);

    let low_priority_id = operation_queue.enqueue(low_priority_op).await.unwrap();

    // Enqueue high priority operation
    let high_priority_op = FileOperation::new(
        "refactor_tool".to_string(),
        OperationType::Refactor,
        file_path.clone(),
        json!({"test": "data"})
    ).with_priority(1);

    let _high_priority_id = operation_queue.enqueue(high_priority_op).await.unwrap();

    // Verify high priority is first
    let pending = operation_queue.get_pending_operations().await;
    assert_eq!(pending.len(), 2);
    assert!(pending[0].1.contains("refactor_tool"));
    assert!(pending[1].1.contains("format_tool"));

    // Cancel the low priority operation
    let cancelled = operation_queue.cancel_operation(&low_priority_id).await;
    assert!(cancelled);

    // Only high priority operation should remain
    let remaining = operation_queue.get_pending_operations().await;
    assert_eq!(remaining.len(), 1);
    assert!(remaining[0].1.contains("refactor_tool"));
}

/// Test that transaction rollback works (operations not committed if transaction is dropped)
#[tokio::test]
async fn test_transaction_rollback() {
    let app_state = create_phase2_test_app_state();
    let operation_queue = app_state.operation_queue.clone();

    let initial_count = operation_queue.queue_size().await;

    {
        // Create transaction but don't commit
        let mut transaction = OperationTransaction::new(operation_queue.clone());

        let operation = FileOperation::new(
            "rollback_test".to_string(),
            OperationType::Write,
            PathBuf::from("/test/rollback.txt"),
            json!({"test": "data"})
        );

        transaction.add_operation(operation);

        // Transaction is dropped here without commit
    }

    // Queue should be unchanged
    let final_count = operation_queue.queue_size().await;
    assert_eq!(initial_count, final_count);
}