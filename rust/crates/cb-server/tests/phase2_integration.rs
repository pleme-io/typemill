//! Integration tests for Phase 2 features
//! These tests verify the operation queue, lock manager, and transaction support

use cb_server::services::{LockManager, OperationQueue, FileOperation, OperationType};
use cb_server::services::operation_queue::OperationTransaction;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[tokio::test]
async fn test_concurrent_reads() {
    // Test that multiple read operations can proceed concurrently
    let lock_manager = Arc::new(LockManager::new());
    let test_file = PathBuf::from("/test/file.rs");

    let mut handles = vec![];

    // Spawn 5 concurrent readers
    for i in 0..5 {
        let lm = lock_manager.clone();
        let file = test_file.clone();

        handles.push(tokio::spawn(async move {
            let start = tokio::time::Instant::now();
            let lock = lm.get_lock(file).await;
            let _guard = lock.read().await;

            // Simulate some work
            tokio::time::sleep(Duration::from_millis(50)).await;

            let elapsed = start.elapsed();
            println!("Reader {} completed in {:?}", i, elapsed);
            elapsed
        }));
    }

    // All readers should complete in roughly the same time (concurrent execution)
    let mut results = vec![];
    for handle in handles {
        results.push(handle.await.unwrap());
    }

    // If they ran concurrently, the maximum time should be around 50ms
    // If they ran sequentially, it would be around 250ms
    let max_time = results.iter().max().unwrap();
    assert!(max_time.as_millis() < 100, "Reads should execute concurrently");
}

#[tokio::test]
async fn test_write_blocks_reads() {
    // Test that a write operation blocks concurrent reads
    let lock_manager = Arc::new(LockManager::new());
    let test_file = PathBuf::from("/test/file.rs");

    // Acquire write lock
    let write_lock = lock_manager.get_lock(test_file.clone()).await;
    let write_guard = write_lock.write().await;

    // Try to acquire read lock in another task
    let lm = lock_manager.clone();
    let file = test_file.clone();
    let read_task = tokio::spawn(async move {
        let start = tokio::time::Instant::now();
        let lock = lm.get_lock(file).await;
        let _guard = lock.read().await;
        start.elapsed()
    });

    // Hold write lock for 100ms
    tokio::time::sleep(Duration::from_millis(100)).await;
    drop(write_guard);

    // Read should only proceed after write is released
    let read_time = read_task.await.unwrap();
    assert!(read_time.as_millis() >= 95, "Read should wait for write to complete");
}

#[tokio::test]
async fn test_priority_ordering() {
    // Test that operations are processed in priority order
    let lock_manager = Arc::new(LockManager::new());
    let queue = OperationQueue::new(lock_manager);

    // Enqueue operations with different priorities
    let op1 = FileOperation {
        id: "write".to_string(),
        operation_type: OperationType::Write,
        tool_name: "write_file".to_string(),
        file_path: PathBuf::from("/test.txt"),
        params: json!({"content": "test"}),
        created_at: Instant::now(),
        priority: 5, // Write priority
    };
    queue.enqueue(op1).await.unwrap();

    let op2 = FileOperation {
        id: "refactor".to_string(),
        operation_type: OperationType::Refactor,
        tool_name: "rename_symbol".to_string(),
        file_path: PathBuf::from("/test.rs"),
        params: json!({"old": "foo", "new": "bar"}),
        created_at: Instant::now(),
        priority: 1, // Refactor priority
    };
    queue.enqueue(op2).await.unwrap();

    let op3 = FileOperation {
        id: "format".to_string(),
        operation_type: OperationType::Format,
        tool_name: "format_document".to_string(),
        file_path: PathBuf::from("/test.rs"),
        params: json!({}),
        created_at: Instant::now(),
        priority: 10, // Format priority
    };
    queue.enqueue(op3).await.unwrap();

    // Get pending operations
    let pending = queue.get_pending_operations().await;

    // Should be ordered by priority: Refactor (1), Write (5), Format (10)
    assert_eq!(pending.len(), 3);
    assert!(pending[0].1.contains("rename_symbol"));
    assert!(pending[1].1.contains("write_file"));
    assert!(pending[2].1.contains("format_document"));
}

#[tokio::test]
async fn test_operation_stats() {
    // Test operation queue statistics
    let lock_manager = Arc::new(LockManager::new());
    let queue = OperationQueue::new(lock_manager);

    // Initial stats should be empty
    let stats = queue.get_stats().await;
    assert_eq!(stats.total_operations, 0);
    assert_eq!(stats.pending_operations, 0);
    assert_eq!(stats.completed_operations, 0);

    // Add an operation
    let op = FileOperation {
        id: "test_op".to_string(),
        operation_type: OperationType::Write,
        tool_name: "test_tool".to_string(),
        file_path: PathBuf::from("/test.txt"),
        params: json!({}),
        created_at: Instant::now(),
        priority: 5,
    };
    queue.enqueue(op).await.unwrap();

    // Stats should reflect the pending operation
    let stats = queue.get_stats().await;
    assert_eq!(stats.total_operations, 1);
    assert_eq!(stats.pending_operations, 1);
    assert_eq!(stats.completed_operations, 0);
}

#[tokio::test]
async fn test_transaction_creation() {
    // Test that transactions group multiple file operations
    let lock_manager = Arc::new(LockManager::new());
    let queue = Arc::new(OperationQueue::new(lock_manager));

    // Start a transaction
    let mut transaction = OperationTransaction::new(queue.clone());

    // Add operations to the transaction
    let op1 = FileOperation {
        id: Uuid::new_v4().to_string(),
        operation_type: OperationType::Refactor,
        tool_name: "rename_symbol_file1".to_string(),
        file_path: PathBuf::from("/src/main.rs"),
        params: json!({"edit": "rename foo to bar"}),
        created_at: Instant::now(),
        priority: 1,
    };
    transaction.add_operation(op1);

    let op2 = FileOperation {
        id: Uuid::new_v4().to_string(),
        operation_type: OperationType::Refactor,
        tool_name: "rename_symbol_file2".to_string(),
        file_path: PathBuf::from("/src/lib.rs"),
        params: json!({"edit": "rename foo to bar"}),
        created_at: Instant::now(),
        priority: 1,
    };
    transaction.add_operation(op2);

    // Commit the transaction
    transaction.commit().await.unwrap();

    // Both operations should be in the queue
    let stats = queue.get_stats().await;
    assert_eq!(stats.total_operations, 2);
    assert_eq!(stats.pending_operations, 2);
}

#[tokio::test]
async fn test_clear_queue() {
    // Test clearing the operation queue
    let lock_manager = Arc::new(LockManager::new());
    let queue = OperationQueue::new(lock_manager);

    // Add multiple operations
    for i in 0..5 {
        let op = FileOperation {
            id: format!("op_{}", i),
            operation_type: OperationType::Write,
            tool_name: format!("tool_{}", i),
            file_path: PathBuf::from(format!("/file_{}.txt", i)),
            params: json!({}),
            created_at: Instant::now(),
            priority: 5,
        };
        queue.enqueue(op).await.unwrap();
    }

    // Verify operations are queued
    let stats = queue.get_stats().await;
    assert_eq!(stats.pending_operations, 5);

    // Clear the queue
    queue.clear().await;

    // Queue should be empty (pending operations cleared)
    let stats = queue.get_stats().await;
    assert_eq!(stats.pending_operations, 0);
    // Total operations tracks history, so it remains at 5
    assert_eq!(stats.total_operations, 5);
}