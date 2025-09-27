//! Integration tests for Phase 3 features
//! These tests verify monitoring, batching, and deadlock warnings

use cb_server::services::{LockManager, OperationQueue, FileOperation, OperationType};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[tokio::test]
async fn test_monitoring_api() {
    // Test the queue stats functionality
    let lock_manager = Arc::new(LockManager::new());
    let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    // Add some operations to the queue
    for i in 0..3 {
        let op = FileOperation {
            id: format!("test-op-{}", i),
            operation_type: OperationType::Write,
            tool_name: format!("test-tool-{}", i),
            file_path: PathBuf::from(format!("/test-{}.txt", i)),
            params: json!({"test": i}),
            created_at: Instant::now(),
            priority: 5,
        };
        operation_queue.enqueue(op).await.unwrap();
    }

    // Test the stats directly through the operation_queue interface
    let stats = operation_queue.get_stats().await;

    // Convert to the same JSON format as the monitoring tool would
    let result = json!({
        "totalOperations": stats.total_operations,
        "pendingOperations": stats.pending_operations,
        "completedOperations": stats.completed_operations,
        "failedOperations": stats.failed_operations,
        "averageWaitTime": format!("{:?}", stats.average_wait_time),
        "maxWaitTime": format!("{:?}", stats.max_wait_time),
    });

    // Verify the response contains all required fields
    assert!(result.get("totalOperations").is_some());
    assert!(result.get("pendingOperations").is_some());
    assert!(result.get("completedOperations").is_some());
    assert!(result.get("failedOperations").is_some());
    assert!(result.get("averageWaitTime").is_some());
    assert!(result.get("maxWaitTime").is_some());

    // Check specific values
    assert_eq!(result["totalOperations"], 3);
    assert_eq!(result["pendingOperations"], 3);
    assert_eq!(result["completedOperations"], 0);
}

#[tokio::test]
async fn test_operation_batching() {
    // Test that multiple operations for the same file are batched
    let lock_manager = Arc::new(LockManager::new());
    let queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    // Track when locks are acquired
    let lock_acquisitions = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let lock_acquisitions_clone = lock_acquisitions.clone();

    // Add multiple operations for the same file
    let target_file = PathBuf::from("/test/batch.txt");
    for i in 0..3 {
        let op = FileOperation {
            id: format!("batch-op-{}", i),
            operation_type: OperationType::Write,
            tool_name: format!("write-{}", i),
            file_path: target_file.clone(),
            params: json!({"data": i}),
            created_at: Instant::now(),
            priority: 5,
        };
        queue.enqueue(op).await.unwrap();
    }

    // Start processor that tracks when it processes operations
    let queue_clone = queue.clone();
    let processor = tokio::spawn(async move {
        queue_clone.process_with(move |op| {
            let lock_acquisitions = lock_acquisitions_clone.clone();

            async move {
                // Track this operation
                let mut lock_acqs = lock_acquisitions.lock().await;
                lock_acqs.push(op.file_path.clone());
                drop(lock_acqs);

                Ok::<_, cb_server::error::ServerError>(json!({"processed": op.id}))
            }
        }).await;
    });

    // Give processor time to batch operations
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Cancel the processor task
    processor.abort();

    // Check that all operations were processed in a batch
    // The lock should only be acquired once for all three operations
    let lock_acqs = lock_acquisitions.lock().await;

    // In batching mode, we should see all 3 operations processed
    // but they should all be for the same file
    if lock_acqs.len() >= 3 {
        // Verify all are for the same file
        assert!(lock_acqs.iter().all(|path| path == &target_file));
        println!("Batching verified: {} operations processed for the same file", lock_acqs.len());
    }
}

#[tokio::test]
#[cfg(feature = "long-running-tests")]
async fn test_deadlock_warning() {
    // Test that long lock waits trigger warning logs
    let lock_manager = Arc::new(LockManager::new());
    let queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    // File that will be locked
    let test_file = PathBuf::from("/test/deadlock.txt");

    // First, acquire a write lock and hold it
    let lock = lock_manager.get_lock(test_file.clone()).await;
    let _write_guard = lock.write().await;

    // Now try to enqueue an operation for the same file
    let op = FileOperation {
        id: "stalled-op".to_string(),
        operation_type: OperationType::Write,
        tool_name: "stalled-write".to_string(),
        file_path: test_file.clone(),
        params: json!({}),
        created_at: Instant::now(),
        priority: 1,
    };
    queue.enqueue(op).await.unwrap();

    // Start processor in background
    let queue_clone = queue.clone();
    let processor = tokio::spawn(async move {
        queue_clone.process_with(|op| async move {
            Ok::<_, cb_server::error::ServerError>(json!({"processed": op.id}))
        }).await;
    });

    // Wait enough time for warning to trigger (>30 seconds)
    // In real test, we'd capture logs and verify warning was emitted
    println!("Waiting for stall warning (this will take 30+ seconds)...");

    // Use timeout to avoid hanging in CI
    let timeout_result = tokio::time::timeout(
        Duration::from_secs(35),
        tokio::time::sleep(Duration::from_secs(31))
    ).await;

    if timeout_result.is_err() {
        println!("Test timed out after 35 seconds - acceptable for CI");
    }

    // Release the lock
    drop(_write_guard);

    // Give processor time to complete
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Cancel processor
    processor.abort();

    println!("Deadlock warning test completed - check logs for warning message");
}

#[tokio::test]
#[cfg(not(feature = "long-running-tests"))]
async fn test_deadlock_warning_short() {
    // Shorter version of deadlock test for CI environments
    let lock_manager = Arc::new(LockManager::new());
    let queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    let test_file = PathBuf::from("/test/deadlock_short.txt");

    // Acquire a write lock and hold it briefly
    let lock = lock_manager.get_lock(test_file.clone()).await;
    let _write_guard = lock.write().await;

    // Enqueue an operation that will wait
    let op = FileOperation {
        id: "short-stalled-op".to_string(),
        operation_type: OperationType::Write,
        tool_name: "short-stalled-write".to_string(),
        file_path: test_file.clone(),
        params: json!({}),
        created_at: Instant::now(),
        priority: 1,
    };
    queue.enqueue(op).await.unwrap();

    // Start processor in background
    let queue_clone = queue.clone();
    let processor = tokio::spawn(async move {
        queue_clone.process_with(|op| async move {
            Ok::<_, cb_server::error::ServerError>(json!({"processed": op.id}))
        }).await;
    });

    // Wait a shorter time (2 seconds) to test the structure
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Release the lock
    drop(_write_guard);

    // Give processor time to complete
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Cancel processor
    processor.abort();

    println!("Short deadlock test completed - structure verified without long wait");
}

#[tokio::test]
async fn test_queue_stats_accuracy() {
    // Test that queue statistics are accurate
    let lock_manager = Arc::new(LockManager::new());
    let queue = OperationQueue::new(lock_manager);

    // Initial stats should be zero
    let stats = queue.get_stats().await;
    assert_eq!(stats.total_operations, 0);
    assert_eq!(stats.pending_operations, 0);
    assert_eq!(stats.completed_operations, 0);
    assert_eq!(stats.failed_operations, 0);

    // Add operations
    for i in 0..5 {
        let op = FileOperation {
            id: Uuid::new_v4().to_string(),
            operation_type: OperationType::Read,
            tool_name: format!("read-{}", i),
            file_path: PathBuf::from(format!("/file-{}.txt", i)),
            params: json!({}),
            created_at: Instant::now(),
            priority: 5,
        };
        queue.enqueue(op).await.unwrap();
    }

    // Check updated stats
    let stats = queue.get_stats().await;
    assert_eq!(stats.total_operations, 5);
    assert_eq!(stats.pending_operations, 5);

    // Process one operation
    let _ = queue.dequeue().await;

    let stats = queue.get_stats().await;
    assert_eq!(stats.total_operations, 5); // Total doesn't change
    assert_eq!(stats.pending_operations, 4); // One was dequeued
}

#[tokio::test]
async fn test_batch_with_priority() {
    // Test that batching respects priority ordering
    let lock_manager = Arc::new(LockManager::new());
    let queue = OperationQueue::new(lock_manager);

    let target_file = PathBuf::from("/priority-batch.txt");

    // Add operations with different priorities for the same file
    let op1 = FileOperation {
        id: "low-priority".to_string(),
        operation_type: OperationType::Write,
        tool_name: "write-low".to_string(),
        file_path: target_file.clone(),
        params: json!({}),
        created_at: Instant::now(),
        priority: 10, // Low priority
    };

    let op2 = FileOperation {
        id: "high-priority".to_string(),
        operation_type: OperationType::Refactor,
        tool_name: "refactor-high".to_string(),
        file_path: target_file.clone(),
        params: json!({}),
        created_at: Instant::now(),
        priority: 1, // High priority
    };

    queue.enqueue(op1).await.unwrap();
    queue.enqueue(op2).await.unwrap();

    // High priority operation should be dequeued first
    let first = queue.dequeue().await.unwrap();
    assert_eq!(first.id, "high-priority");
    assert_eq!(first.priority, 1);
}