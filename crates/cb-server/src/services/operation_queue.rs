//! Operation queue for serializing file operations

use super::lock_manager::{LockManager, LockType};
use crate::{ServerError, ServerResult};
use serde_json::Value;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Notify};
use tokio::time::timeout;
use tracing::{debug, error, warn};

/// Warning timeout for lock acquisition (30 seconds)
const LOCK_ACQUISITION_WARNING_TIMEOUT: Duration = Duration::from_secs(30);

/// Type of file operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationType {
    Read,
    Write,
    Delete,
    Rename,
    Format,
    Refactor,
}

impl OperationType {
    /// Check if this operation modifies files
    pub fn is_write_operation(&self) -> bool {
        matches!(
            self,
            OperationType::Write
                | OperationType::Delete
                | OperationType::Rename
                | OperationType::Format
                | OperationType::Refactor
        )
    }

    /// Get the lock type needed for this operation
    pub fn lock_type(&self) -> LockType {
        if self.is_write_operation() {
            LockType::Write
        } else {
            LockType::Read
        }
    }
}

/// A queued file operation
#[derive(Debug)]
pub struct FileOperation {
    pub id: String,
    pub operation_type: OperationType,
    pub tool_name: String,
    pub file_path: PathBuf,
    pub params: Value,
    pub created_at: Instant,
    pub priority: u8, // 0 = highest priority
}

impl FileOperation {
    /// Create a new file operation
    pub fn new(
        tool_name: String,
        operation_type: OperationType,
        file_path: PathBuf,
        params: Value,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            operation_type,
            tool_name,
            file_path,
            params,
            created_at: Instant::now(),
            priority: 5, // Default medium priority
        }
    }

    /// Set the priority (0 = highest)
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Get the age of this operation
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }
}

/// Queue statistics
#[derive(Debug, Clone)]
pub struct QueueStats {
    pub total_operations: usize,
    pub pending_operations: usize,
    pub completed_operations: usize,
    pub failed_operations: usize,
    pub average_wait_time: Duration,
    pub max_wait_time: Duration,
}

/// Manages a queue of file operations
pub struct OperationQueue {
    /// Pending operations queue
    queue: Arc<Mutex<VecDeque<FileOperation>>>,
    /// Lock manager for file-level locking
    lock_manager: Arc<LockManager>,
    /// Notification for new operations
    notify: Arc<Notify>,
    /// Statistics
    stats: Arc<Mutex<QueueStatsInternal>>,
    /// Maximum queue size
    max_queue_size: usize,
    /// Operation timeout
    operation_timeout: Duration,
}

#[derive(Debug)]
struct QueueStatsInternal {
    total_operations: usize,
    completed_operations: usize,
    failed_operations: usize,
    total_wait_time: Duration,
    max_wait_time: Duration,
}

impl OperationQueue {
    /// Create a new operation queue
    pub fn new(lock_manager: Arc<LockManager>) -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            lock_manager,
            notify: Arc::new(Notify::new()),
            stats: Arc::new(Mutex::new(QueueStatsInternal {
                total_operations: 0,
                completed_operations: 0,
                failed_operations: 0,
                total_wait_time: Duration::ZERO,
                max_wait_time: Duration::ZERO,
            })),
            max_queue_size: 1000,
            operation_timeout: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Add an operation to the queue
    pub async fn enqueue(&self, operation: FileOperation) -> ServerResult<String> {
        let mut queue = self.queue.lock().await;

        // Check queue size limit
        if queue.len() >= self.max_queue_size {
            return Err(ServerError::runtime("Operation queue is full"));
        }

        let operation_id = operation.id.clone();
        debug!(
            "Enqueueing operation {}: {} on {}",
            operation_id,
            operation.tool_name,
            operation.file_path.display()
        );

        // Insert based on priority
        let priority = operation.priority;
        let mut insert_pos = queue.len();
        for (i, op) in queue.iter().enumerate() {
            if op.priority > priority {
                insert_pos = i;
                break;
            }
        }

        queue.insert(insert_pos, operation);

        // Update stats
        let mut stats = self.stats.lock().await;
        stats.total_operations += 1;

        // Notify processor of new operation
        self.notify.notify_one();

        Ok(operation_id)
    }

    /// Get the next operation from the queue
    pub async fn dequeue(&self) -> Option<FileOperation> {
        let mut queue = self.queue.lock().await;
        queue.pop_front()
    }

    /// Wait for and get the next operation
    pub async fn wait_for_operation(&self) -> Option<FileOperation> {
        loop {
            // Check for existing operations
            if let Some(op) = self.dequeue().await {
                return Some(op);
            }

            // Wait for notification of new operation
            self.notify.notified().await;
        }
    }

    /// Process operations with the given handler
    pub async fn process_with<F, Fut>(&self, mut handler: F)
    where
        F: FnMut(FileOperation) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ServerResult<Value>> + Send,
    {
        loop {
            if let Some(operation) = self.wait_for_operation().await {
                let wait_time = operation.age();
                let file_path = operation.file_path.clone();
                let lock_type = operation.operation_type.lock_type();

                // Update wait time stats
                {
                    let mut stats = self.stats.lock().await;
                    stats.total_wait_time += wait_time;
                    if wait_time > stats.max_wait_time {
                        stats.max_wait_time = wait_time;
                    }
                }

                // Check if operation has timed out
                if wait_time > self.operation_timeout {
                    warn!(operation_id = %operation.id, wait_time = ?wait_time, "Operation timed out");
                    let mut stats = self.stats.lock().await;
                    stats.failed_operations += 1;
                    continue;
                }

                // Acquire lock for the file
                debug!(lock_type = ?lock_type, file_path = %file_path.display(), "Acquiring lock");
                let file_lock = self.lock_manager.get_lock(&file_path).await;

                // Acquire the appropriate lock and process immediately
                match lock_type {
                    LockType::Read => {
                        // Try to acquire read lock with timeout warning
                        let _guard = match timeout(
                            LOCK_ACQUISITION_WARNING_TIMEOUT,
                            file_lock.read(),
                        )
                        .await
                        {
                            Ok(guard) => guard,
                            Err(_) => {
                                warn!(
                                    "Potential stall detected: Operation {} waiting >30s for read lock on {}",
                                    operation.id, file_path.display()
                                );
                                // Continue waiting for the lock (don't cancel)
                                file_lock.read().await
                            }
                        };
                        // Process the operation
                        debug!(
                            "Processing operation {}: {}",
                            operation.id, operation.tool_name
                        );
                        match handler(operation).await {
                            Ok(_result) => {
                                debug!("Operation completed successfully");
                                let mut stats = self.stats.lock().await;
                                stats.completed_operations += 1;
                            }
                            Err(e) => {
                                error!(error = %e, "Operation failed");
                                let mut stats = self.stats.lock().await;
                                stats.failed_operations += 1;
                            }
                        }
                    }
                    LockType::Write => {
                        // Batch processing: collect all operations for the same file
                        let mut batched_operations = vec![operation];

                        // Look for other operations targeting the same file
                        {
                            let mut queue = self.queue.lock().await;
                            let mut i = 0;
                            while i < queue.len() {
                                if queue[i].file_path == file_path {
                                    // Remove and add to batch
                                    if let Some(op) = queue.remove(i) {
                                        debug!(
                                            "Batching operation {} for file {}",
                                            op.id,
                                            file_path.display()
                                        );
                                        batched_operations.push(op);
                                    } else {
                                        // Index became invalid, skip and continue
                                        warn!("Failed to remove operation at index {}", i);
                                        i += 1;
                                    }
                                } else {
                                    i += 1;
                                }
                            }
                        }

                        // Process all batched operations under the same write lock
                        // Try to acquire write lock with timeout warning
                        let _guard = match timeout(
                            LOCK_ACQUISITION_WARNING_TIMEOUT,
                            file_lock.write(),
                        )
                        .await
                        {
                            Ok(guard) => guard,
                            Err(_) => {
                                warn!(
                                    "Potential stall detected: {} batched operations waiting >30s for write lock on {}",
                                    batched_operations.len(), file_path.display()
                                );
                                // Continue waiting for the lock (don't cancel)
                                file_lock.write().await
                            }
                        };
                        debug!(
                            "Processing {} batched operations for file {}",
                            batched_operations.len(),
                            file_path.display()
                        );

                        for batched_op in batched_operations {
                            debug!(
                                "Processing operation {}: {}",
                                batched_op.id, batched_op.tool_name
                            );
                            match handler(batched_op).await {
                                Ok(_result) => {
                                    debug!("Operation completed successfully");
                                    let mut stats = self.stats.lock().await;
                                    stats.completed_operations += 1;
                                }
                                Err(e) => {
                                    error!(error = %e, "Operation failed");
                                    let mut stats = self.stats.lock().await;
                                    stats.failed_operations += 1;
                                }
                            }
                        }
                    }
                };
            }
        }
    }

    /// Get current queue size
    pub async fn queue_size(&self) -> usize {
        self.queue.lock().await.len()
    }

    /// Check if queue is empty
    pub async fn is_empty(&self) -> bool {
        self.queue.lock().await.is_empty()
    }

    /// Get queue statistics
    pub async fn get_stats(&self) -> QueueStats {
        let stats = self.stats.lock().await;
        let pending = self.queue.lock().await.len();

        let average_wait_time = if stats.completed_operations > 0 {
            stats.total_wait_time / stats.completed_operations as u32
        } else {
            Duration::ZERO
        };

        QueueStats {
            total_operations: stats.total_operations,
            pending_operations: pending,
            completed_operations: stats.completed_operations,
            failed_operations: stats.failed_operations,
            average_wait_time,
            max_wait_time: stats.max_wait_time,
        }
    }

    /// Checks if the queue is idle (no pending operations and all operations processed).
    /// This is useful for CLI tools that need to wait for async operations to complete.
    pub async fn is_idle(&self) -> bool {
        let stats = self.get_stats().await;
        stats.pending_operations == 0
            && stats.total_operations == (stats.completed_operations + stats.failed_operations)
    }

    /// Clear all pending operations
    pub async fn clear(&self) {
        let mut queue = self.queue.lock().await;
        queue.clear();
    }

    /// Remove a specific operation from the queue
    pub async fn cancel_operation(&self, operation_id: &str) -> bool {
        let mut queue = self.queue.lock().await;
        let initial_len = queue.len();
        queue.retain(|op| op.id != operation_id);
        queue.len() < initial_len
    }

    /// Get all pending operations (for monitoring)
    pub async fn get_pending_operations(&self) -> Vec<(String, String, PathBuf, Duration)> {
        let queue = self.queue.lock().await;
        queue
            .iter()
            .map(|op| {
                (
                    op.id.clone(),
                    op.tool_name.clone(),
                    op.file_path.clone(),
                    op.age(),
                )
            })
            .collect()
    }
}

/// Transaction support for grouped operations
pub struct OperationTransaction {
    operations: Vec<FileOperation>,
    queue: Arc<OperationQueue>,
}

impl OperationTransaction {
    /// Create a new transaction
    pub fn new(queue: Arc<OperationQueue>) -> Self {
        Self {
            operations: Vec::new(),
            queue,
        }
    }

    /// Add an operation to the transaction
    pub fn add_operation(&mut self, operation: FileOperation) {
        self.operations.push(operation);
    }

    /// Commit all operations to the queue
    pub async fn commit(self) -> ServerResult<Vec<String>> {
        let mut operation_ids = Vec::new();

        for operation in self.operations {
            let id = self.queue.enqueue(operation).await?;
            operation_ids.push(id);
        }

        Ok(operation_ids)
    }

    /// Cancel the transaction (drop all operations)
    pub fn rollback(self) {
        // Operations are dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_enqueue_dequeue() {
        let lock_manager = Arc::new(LockManager::new());
        let queue = OperationQueue::new(lock_manager);

        let op = FileOperation::new(
            "test_tool".to_string(),
            OperationType::Read,
            PathBuf::from("/test.txt"),
            Value::Null,
        );

        let id = queue.enqueue(op).await.unwrap();
        assert!(!id.is_empty());

        let dequeued = queue.dequeue().await;
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().id, id);
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let lock_manager = Arc::new(LockManager::new());
        let queue = OperationQueue::new(lock_manager);

        // Add operations with different priorities
        let op1 = FileOperation::new(
            "tool1".to_string(),
            OperationType::Read,
            PathBuf::from("/file1.txt"),
            Value::Null,
        )
        .with_priority(5);

        let op2 = FileOperation::new(
            "tool2".to_string(),
            OperationType::Read,
            PathBuf::from("/file2.txt"),
            Value::Null,
        )
        .with_priority(1); // Higher priority

        let op3 = FileOperation::new(
            "tool3".to_string(),
            OperationType::Read,
            PathBuf::from("/file3.txt"),
            Value::Null,
        )
        .with_priority(10); // Lower priority

        queue.enqueue(op1).await.unwrap();
        queue.enqueue(op2).await.unwrap();
        queue.enqueue(op3).await.unwrap();

        // Should dequeue in priority order
        let first = queue.dequeue().await.unwrap();
        assert_eq!(first.priority, 1);

        let second = queue.dequeue().await.unwrap();
        assert_eq!(second.priority, 5);

        let third = queue.dequeue().await.unwrap();
        assert_eq!(third.priority, 10);
    }

    #[tokio::test]
    async fn test_cancel_operation() {
        let lock_manager = Arc::new(LockManager::new());
        let queue = OperationQueue::new(lock_manager);

        let op1 = FileOperation::new(
            "tool1".to_string(),
            OperationType::Read,
            PathBuf::from("/file1.txt"),
            Value::Null,
        );

        let op2 = FileOperation::new(
            "tool2".to_string(),
            OperationType::Read,
            PathBuf::from("/file2.txt"),
            Value::Null,
        );

        let id1 = queue.enqueue(op1).await.unwrap();
        let id2 = queue.enqueue(op2).await.unwrap();

        // Cancel first operation
        assert!(queue.cancel_operation(&id1).await);
        assert_eq!(queue.queue_size().await, 1);

        // Should only have second operation
        let remaining = queue.dequeue().await.unwrap();
        assert_eq!(remaining.id, id2);
    }

    #[tokio::test]
    async fn test_transaction() {
        let lock_manager = Arc::new(LockManager::new());
        let queue = Arc::new(OperationQueue::new(lock_manager));

        let mut transaction = OperationTransaction::new(queue.clone());

        transaction.add_operation(FileOperation::new(
            "tool1".to_string(),
            OperationType::Write,
            PathBuf::from("/file1.txt"),
            Value::Null,
        ));

        transaction.add_operation(FileOperation::new(
            "tool2".to_string(),
            OperationType::Write,
            PathBuf::from("/file2.txt"),
            Value::Null,
        ));

        let ids = transaction.commit().await.unwrap();
        assert_eq!(ids.len(), 2);
        assert_eq!(queue.queue_size().await, 2);
    }

    #[tokio::test]
    async fn test_stats() {
        let lock_manager = Arc::new(LockManager::new());
        let queue = OperationQueue::new(lock_manager);

        let op = FileOperation::new(
            "test".to_string(),
            OperationType::Read,
            PathBuf::from("/test.txt"),
            Value::Null,
        );

        queue.enqueue(op).await.unwrap();

        let stats = queue.get_stats().await;
        assert_eq!(stats.total_operations, 1);
        assert_eq!(stats.pending_operations, 1);
        assert_eq!(stats.completed_operations, 0);
    }
}
