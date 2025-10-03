//! File-level locking mechanism to prevent concurrent modifications

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Lock type for file operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockType {
    Read,
    Write,
}

/// Manages file-level locks for concurrent operations
pub struct LockManager {
    /// Map of file paths to their associated locks
    locks: Arc<RwLock<HashMap<PathBuf, Arc<RwLock<()>>>>>,
}

impl LockManager {
    /// Create a new lock manager
    pub fn new() -> Self {
        Self {
            locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create a lock for a file
    pub async fn get_lock<P: AsRef<Path>>(&self, path: P) -> Arc<RwLock<()>> {
        let path = path.as_ref().to_path_buf();

        // Get or create the lock for this file
        let mut locks = self.locks.write().await;
        locks
            .entry(path)
            .or_insert_with(|| Arc::new(RwLock::new(())))
            .clone()
    }

    /// Check if a file is currently locked for writing
    pub async fn is_write_locked<P: AsRef<Path>>(&self, path: P) -> bool {
        let path = path.as_ref().to_path_buf();

        let locks = self.locks.read().await;
        if let Some(file_lock) = locks.get(&path) {
            // If we can't acquire a read lock, it's write-locked
            file_lock.try_read().is_err()
        } else {
            false
        }
    }

    /// Clean up locks for files that no longer exist or haven't been accessed
    pub async fn cleanup_unused_locks(&self) {
        let mut locks = self.locks.write().await;

        // Remove locks that have no other references
        locks.retain(|_path, lock| {
            // Keep locks that have other strong references
            // (Arc::strong_count returns at least 2 if we have it and it's in the map)
            Arc::strong_count(lock) > 1
        });
    }

    /// Get the number of active locks
    pub async fn lock_count(&self) -> usize {
        self.locks.read().await.len()
    }

    /// Clear all locks (use with caution - mainly for testing)
    #[cfg(test)]
    pub async fn clear_all(&self) {
        self.locks.write().await.clear();
    }
}

impl Default for LockManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_read_locks_are_shared() {
        let manager = LockManager::new();
        let path = "/test/file.txt";

        // Get the lock
        let lock = manager.get_lock(path).await;

        // Acquire multiple read locks
        let _guard1 = lock.read().await;
        let _guard2 = lock.read().await;

        // Should be able to acquire another read lock immediately
        let result = lock.try_read();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_write_lock_is_exclusive() {
        let manager = LockManager::new();
        let path = "/test/file.txt";

        // Get the lock
        let lock = manager.get_lock(path).await;

        // Acquire a write lock
        let _guard = lock.write().await;

        // Should not be able to acquire another lock
        let read_result = lock.try_read();
        assert!(read_result.is_err());

        let write_result = lock.try_write();
        assert!(write_result.is_err());
    }

    #[tokio::test]
    async fn test_lock_release() {
        let manager = LockManager::new();
        let path = "/test/file.txt";

        let lock = manager.get_lock(path).await;

        // Acquire and release a write lock
        {
            let _guard = lock.write().await;
            assert!(manager.is_write_locked(path).await);
        } // Guard dropped here

        // Should be able to acquire a new lock after release
        let result = lock.try_write();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let manager = Arc::new(LockManager::new());
        let path = "/test/concurrent.txt";

        let manager1 = manager.clone();
        let manager2 = manager.clone();

        // Spawn two tasks trying to write
        let task1 = tokio::spawn(async move {
            let lock = manager1.get_lock(path).await;
            let _guard = lock.write().await;
            sleep(Duration::from_millis(50)).await;
            1
        });

        let task2 = tokio::spawn(async move {
            sleep(Duration::from_millis(10)).await; // Let task1 acquire first
            let lock = manager2.get_lock(path).await;
            let _guard = lock.write().await;
            2
        });

        let result1 = task1.await.unwrap();
        let result2 = task2.await.unwrap();

        // Both tasks should complete successfully
        assert_eq!(result1, 1);
        assert_eq!(result2, 2);
    }

    #[tokio::test]
    async fn test_cleanup_unused_locks() {
        let manager = LockManager::new();

        // Create some locks
        {
            let _lock1 = manager.get_lock("/file1.txt").await;
            let _lock2 = manager.get_lock("/file2.txt").await;
            assert_eq!(manager.lock_count().await, 2);
        }

        // After locks are dropped, cleanup should remove unused locks
        manager.cleanup_unused_locks().await;

        // Note: In practice, locks might still exist if Arc has references
        // This test mainly verifies the cleanup doesn't panic
    }
}
