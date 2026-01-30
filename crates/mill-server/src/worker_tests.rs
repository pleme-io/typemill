use crate::spawn_operation_worker;
use mill_services::services::coordination::lock_manager::LockManager;
use mill_services::services::coordination::operation_queue::{OperationQueue, FileOperation, OperationType};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

#[tokio::test]
async fn test_worker_path_traversal_prevention() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path().to_path_buf();

    // Setup queue
    let lock_manager = Arc::new(LockManager::new());
    let queue = Arc::new(OperationQueue::new(lock_manager));

    // Spawn worker
    spawn_operation_worker(queue.clone(), root.clone());

    // 1. Attempt traversal with ../
    // Note: We use a relative path that tries to escape.
    // The worker joins it with root.
    let op = FileOperation::new(
        "malicious_tool".to_string(),
        OperationType::Write,
        PathBuf::from("../outside.txt"),
        json!({"content": "hacked"}),
    );

    queue.enqueue(op).await.unwrap();

    // Wait for processing
    // We use a simple sleep here as the worker runs in background
    tokio::time::sleep(Duration::from_millis(100)).await;

    let stats = queue.get_stats().await;
    assert_eq!(stats.failed_operations, 1, "Should have one failed operation");
    assert_eq!(stats.completed_operations, 0, "Should have zero completed operations");

    // Verify file was NOT created outside
    // Note: root is a temp dir, so parent is likely /tmp or similar.
    // We check if the file exists relative to the parent.
    // However, if we don't have write access to parent, the create would fail anyway.
    // Ideally we'd verify the worker rejected it *before* attempting create.
    // The failed_operations count confirms the worker rejected or failed.
    // But let's check the path if possible.
    if let Some(parent) = root.parent() {
        let outside_path = parent.join("outside.txt");
        if outside_path.exists() {
             // Clean up if it was created (it shouldn't be)
             let _ = std::fs::remove_file(&outside_path);
             panic!("File was created outside project root!");
        }
    }
}

#[tokio::test]
async fn test_worker_absolute_path_traversal() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path().to_path_buf();

    let lock_manager = Arc::new(LockManager::new());
    let queue = Arc::new(OperationQueue::new(lock_manager));

    spawn_operation_worker(queue.clone(), root.clone());

    // Attempt absolute path outside root
    // We create a separate temp dir to target
    let other_temp = TempDir::new().unwrap();
    let target_path = other_temp.path().join("hacked.txt");

    let op = FileOperation::new(
        "malicious_tool".to_string(),
        OperationType::Write,
        target_path.clone(),
        json!({"content": "hacked"}),
    );

    queue.enqueue(op).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let stats = queue.get_stats().await;
    assert_eq!(stats.failed_operations, 1, "Should have one failed operation");

    assert!(!target_path.exists(), "File should not be created at absolute path outside root");
}

#[tokio::test]
async fn test_worker_valid_operation() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path().to_path_buf();

    let lock_manager = Arc::new(LockManager::new());
    let queue = Arc::new(OperationQueue::new(lock_manager));

    spawn_operation_worker(queue.clone(), root.clone());

    let op = FileOperation::new(
        "good_tool".to_string(),
        OperationType::Write,
        PathBuf::from("inside.txt"),
        json!({"content": "safe"}),
    );

    queue.enqueue(op).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let stats = queue.get_stats().await;
    assert_eq!(stats.completed_operations, 1);

    let inside_path = root.join("inside.txt");
    assert!(inside_path.exists());
    let content = std::fs::read_to_string(inside_path).unwrap();
    assert_eq!(content, "safe");
}
