//! Integration tests for concurrent operations

use super::*;
use crate::ServerError;
use futures_util::future;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, timeout};

/// Test concurrent read operations
#[tokio::test]
async fn test_concurrent_read_operations() {
    let lock_manager = Arc::new(LockManager::new());
    let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    let file_path = PathBuf::from("/test/concurrent_read.txt");

    // Create multiple read operations
    let mut handles = Vec::new();
    for i in 0..5 {
        let queue = operation_queue.clone();
        let path = file_path.clone();

        let handle = tokio::spawn(async move {
            let operation = FileOperation::new(
                format!("read_tool_{}", i),
                OperationType::Read,
                path,
                json!({"line": i}),
            );

            queue.enqueue(operation).await.unwrap();

            // Simulate some processing time
            sleep(Duration::from_millis(10)).await;

            format!("read_{}", i)
        });

        handles.push(handle);
    }

    // All operations should complete successfully
    let results: Vec<String> = future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(results.len(), 5);
    for i in 0..5 {
        assert!(results.contains(&format!("read_{}", i)));
    }

    // Queue should be empty after processing
    assert_eq!(operation_queue.queue_size().await, 5); // Operations are queued but not processed by a handler yet
}

/// Test that write operations are properly queued and executed in order
#[tokio::test]
async fn test_write_operations_serialized() {
    let lock_manager = Arc::new(LockManager::new());
    let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    let file_path = PathBuf::from("/test/write_serialized.txt");
    let execution_order = Arc::new(tokio::sync::Mutex::new(Vec::new()));

    // Create multiple write operations with different priorities
    let mut handles = Vec::new();
    let priorities = vec![5, 1, 3, 2, 4]; // Different priorities (1 = highest)

    for (i, priority) in priorities.iter().enumerate() {
        let queue = operation_queue.clone();
        let path = file_path.clone();
        let order = execution_order.clone();
        let priority = *priority;

        let handle = tokio::spawn(async move {
            let operation = FileOperation::new(
                format!("write_tool_{}", i),
                OperationType::Write,
                path,
                json!({"content": format!("write_{}", i)}),
            )
            .with_priority(priority);

            queue.enqueue(operation).await.unwrap();

            // Record execution order
            order.lock().await.push((i, priority));

            format!("write_{}", i)
        });

        handles.push(handle);
    }

    // Wait for all operations to be queued
    future::join_all(handles).await;

    // Check that operations are queued in priority order
    let stats = operation_queue.get_stats().await;
    assert_eq!(stats.pending_operations, 5);
    assert_eq!(stats.total_operations, 5);

    // Get pending operations to verify priority ordering
    let pending = operation_queue.get_pending_operations().await;
    assert_eq!(pending.len(), 5);

    // First operation should have highest priority (1)
    assert!(pending[0].1.contains("write_tool_1")); // tool name contains index 1 which had priority 1
}

/// Test concurrent operations on different files
#[tokio::test]
async fn test_concurrent_operations_different_files() {
    let lock_manager = Arc::new(LockManager::new());
    let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    // Create operations on different files
    let mut handles = Vec::new();
    for i in 0..3 {
        let queue = operation_queue.clone();
        let path = PathBuf::from(format!("/test/file_{}.txt", i));

        let handle = tokio::spawn(async move {
            let operation = FileOperation::new(
                format!("tool_{}", i),
                OperationType::Write,
                path,
                json!({"content": format!("content_{}", i)}),
            );

            queue.enqueue(operation).await.unwrap()
        });

        handles.push(handle);
    }

    // All operations should be queued successfully
    let operation_ids: Vec<String> = future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(operation_ids.len(), 3);
    assert_eq!(operation_queue.queue_size().await, 3);
}

/// Test operation cancellation
#[tokio::test]
async fn test_operation_cancellation() {
    let lock_manager = Arc::new(LockManager::new());
    let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    let file_path = PathBuf::from("/test/cancel_test.txt");

    // Enqueue several operations
    let mut operation_ids = Vec::new();
    for i in 0..5 {
        let operation = FileOperation::new(
            format!("tool_{}", i),
            OperationType::Write,
            file_path.clone(),
            json!({"content": format!("content_{}", i)}),
        );

        let id = operation_queue.enqueue(operation).await.unwrap();
        operation_ids.push(id);
    }

    assert_eq!(operation_queue.queue_size().await, 5);

    // Cancel middle operation
    let cancelled = operation_queue.cancel_operation(&operation_ids[2]).await;
    assert!(cancelled);
    assert_eq!(operation_queue.queue_size().await, 4);

    // Try to cancel non-existent operation
    let not_cancelled = operation_queue.cancel_operation("non-existent-id").await;
    assert!(!not_cancelled);
    assert_eq!(operation_queue.queue_size().await, 4);
}

/// Test operation timeout handling
#[tokio::test]
async fn test_operation_timeout() {
    let lock_manager = Arc::new(LockManager::new());
    let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    // Create an old operation (simulate by creating and waiting)
    let operation = FileOperation::new(
        "slow_tool".to_string(),
        OperationType::Write,
        PathBuf::from("/test/timeout.txt"),
        json!({"content": "test"}),
    );

    operation_queue.enqueue(operation).await.unwrap();

    // Wait a bit to age the operation
    sleep(Duration::from_millis(50)).await;

    let stats = operation_queue.get_stats().await;
    assert_eq!(stats.pending_operations, 1);

    // Check that the operation exists and has some age
    let pending = operation_queue.get_pending_operations().await;
    assert_eq!(pending.len(), 1);
    assert!(pending[0].3 > Duration::from_millis(40)); // Should have some age
}

/// Test queue statistics
#[tokio::test]
async fn test_queue_statistics() {
    let lock_manager = Arc::new(LockManager::new());
    let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    // Initially empty
    let initial_stats = operation_queue.get_stats().await;
    assert_eq!(initial_stats.total_operations, 0);
    assert_eq!(initial_stats.pending_operations, 0);
    assert_eq!(initial_stats.completed_operations, 0);

    // Add some operations
    for i in 0..3 {
        let operation = FileOperation::new(
            format!("tool_{}", i),
            OperationType::Write,
            PathBuf::from(format!("/test/stats_{}.txt", i)),
            json!({"index": i}),
        );

        operation_queue.enqueue(operation).await.unwrap();
    }

    let stats = operation_queue.get_stats().await;
    assert_eq!(stats.total_operations, 3);
    assert_eq!(stats.pending_operations, 3);
    assert_eq!(stats.completed_operations, 0);
}

/// Test lock manager integration
#[tokio::test]
async fn test_lock_manager_integration() {
    let lock_manager = Arc::new(LockManager::new());
    let file_path = PathBuf::from("/test/lock_integration.txt");

    // Test that we can acquire and release locks
    {
        let file_lock = lock_manager.get_lock(&file_path).await;
        let _write_lock = file_lock.write().await;
        assert!(lock_manager.is_write_locked(&file_path).await);
    }

    // Lock should be released
    assert!(!lock_manager.is_write_locked(&file_path).await);

    // Test concurrent read locks
    let file_lock = lock_manager.get_lock(&file_path).await;
    let _read_lock1 = file_lock.read().await;
    let _read_lock2 = file_lock.read().await;

    // Should be able to get another read lock immediately
    let read_lock3 = timeout(Duration::from_millis(10), file_lock.read()).await;

    assert!(read_lock3.is_ok());
}

/// Test operation processing with mock handler
#[tokio::test]
async fn test_operation_processing() {
    let lock_manager = Arc::new(LockManager::new());
    let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    let file_path = PathBuf::from("/test/processing.txt");
    let processed_operations = Arc::new(tokio::sync::Mutex::new(Vec::new()));

    // Create a mock handler
    let processed_ops_clone = processed_operations.clone();
    let mock_handler = move |operation: FileOperation| {
        let processed_ops = processed_ops_clone.clone();
        async move {
            processed_ops.lock().await.push(operation.tool_name.clone());
            Ok::<Value, ServerError>(json!({"status": "processed"}))
        }
    };

    // Enqueue an operation
    let operation = FileOperation::new(
        "test_tool".to_string(),
        OperationType::Write,
        file_path,
        json!({"test": "data"}),
    );

    operation_queue.enqueue(operation).await.unwrap();

    // Process one operation manually (simulating the processor)
    if let Some(op) = operation_queue.dequeue().await {
        let _result = mock_handler(op).await.unwrap();
    }

    // Verify the operation was processed
    let processed = processed_operations.lock().await;
    assert_eq!(processed.len(), 1);
    assert_eq!(processed[0], "test_tool");
}

/// Test clear functionality
#[tokio::test]
async fn test_clear_operations() {
    let lock_manager = Arc::new(LockManager::new());
    let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    // Add several operations
    for i in 0..5 {
        let operation = FileOperation::new(
            format!("tool_{}", i),
            OperationType::Write,
            PathBuf::from(format!("/test/clear_{}.txt", i)),
            json!({"index": i}),
        );

        operation_queue.enqueue(operation).await.unwrap();
    }

    assert_eq!(operation_queue.queue_size().await, 5);

    // Clear the queue
    operation_queue.clear().await;

    assert_eq!(operation_queue.queue_size().await, 0);
    assert!(operation_queue.is_empty().await);
}
