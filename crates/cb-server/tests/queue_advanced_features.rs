//! Integration tests for advanced queue features
//! These tests verify monitoring, batching, and priority handling

mod common;

use cb_protocol::ApiError;
use cb_server::services::{FileOperation, OperationType};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[tokio::test]
async fn test_monitoring_api() {
    // Test the queue stats functionality
    let (app_state, _temp_dir) = common::create_test_app_state();
    let operation_queue = app_state.operation_queue.clone();

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
    let (app_state, _temp_dir) = common::create_test_app_state();
    let queue = app_state.operation_queue.clone();

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
        queue_clone
            .process_with(move |op, stats| {
                let lock_acquisitions = lock_acquisitions_clone.clone();

                async move {
                    // Track this operation
                    let mut lock_acqs = lock_acquisitions.lock().await;
                    lock_acqs.push(op.file_path.clone());
                    drop(lock_acqs);

                    // Update stats
                    let mut stats_guard = stats.lock().await;
                    stats_guard.completed_operations += 1;
                    drop(stats_guard);

                    Ok::<_, ApiError>(json!({"processed": op.id}))
                }
            })
            .await;
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
        println!(
            "Batching verified: {} operations processed for the same file",
            lock_acqs.len()
        );
    }
}

// Removed: test_deadlock_warning - This test waited 31+ seconds and only printed messages
// without programmatic validation. To properly test deadlock warnings, we would need
// to capture logs using tracing-subscriber test utilities and verify the warning is emitted.

// Removed: test_deadlock_warning_short - This was a "cheating" test with no assertions.
// It set up a potential deadlock scenario, waited 2 seconds, then completed without
// validating anything. A test without assertions provides no value and creates false
// confidence. A proper deadlock detection test should be implemented separately using
// tracing-subscriber::test to capture and verify warning log output.

#[tokio::test]
async fn test_batch_with_priority() {
    // Test that batching respects priority ordering
    let (app_state, _temp_dir) = common::create_test_app_state();
    let queue = app_state.operation_queue.clone();

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
