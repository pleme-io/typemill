//! LSP Progress Notification Support
//!
//! This module implements support for LSP `$/progress` notifications, enabling
//! the client to track long-running server operations like rust-analyzer's
//! workspace indexing.
//!
//! ## Architecture
//!
//! The `ProgressManager` component tracks active progress tasks and provides
//! async coordination primitives for waiting on task completion. It uses:
//!
//! - `DashMap` for lock-free concurrent progress state tracking
//! - `tokio::sync::broadcast` for fan-out notification of progress updates
//! - State machine tracking: InProgress → Completed/Failed
//!
//! ## Usage
//!
//! ```rust,no_run
//! use mill_lsp::progress::{ProgressManager, ProgressToken};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let manager = ProgressManager::new();
//!
//! // Wait for rust-analyzer indexing
//! let token = ProgressToken::String("rustAnalyzer/Indexing".to_string());
//! manager.wait_for_completion(&token, Duration::from_secs(30)).await?;
//! # Ok(())
//! # }
//! ```

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::broadcast;
use tracing::{debug, warn};

/// LSP Progress token (string or integer)
///
/// From LSP spec: `type ProgressToken = integer | string;`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProgressToken {
    String(String),
    Number(i32),
}

impl std::fmt::Display for ProgressToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProgressToken::String(s) => write!(f, "{}", s),
            ProgressToken::Number(n) => write!(f, "{}", n),
        }
    }
}

/// Work done progress value from LSP `$/progress` notification
///
/// From LSP spec: `export type WorkDoneProgressValue = WorkDoneProgressBegin | WorkDoneProgressReport | WorkDoneProgressEnd`
///
/// Internal implementation detail for progress tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum WorkDoneProgressValue {
    /// Progress has started
    #[serde(rename = "begin")]
    Begin {
        /// Mandatory title of the progress operation
        title: String,
        /// Optional, more detailed message
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        /// Optional progress percentage (0-100)
        #[serde(skip_serializing_if = "Option::is_none")]
        percentage: Option<u32>,
        /// Whether operation can be cancelled
        #[serde(skip_serializing_if = "Option::is_none")]
        cancellable: Option<bool>,
    },
    /// Progress update
    #[serde(rename = "report")]
    Report {
        /// Optional, more detailed message
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        /// Optional progress percentage (0-100)
        #[serde(skip_serializing_if = "Option::is_none")]
        percentage: Option<u32>,
        /// Whether operation can be cancelled
        #[serde(skip_serializing_if = "Option::is_none")]
        cancellable: Option<bool>,
    },
    /// Progress has completed
    #[serde(rename = "end")]
    End {
        /// Optional final message
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
}

/// Parameters for `$/progress` notification
///
/// Internal implementation detail for progress tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressParams {
    /// The progress token provided by the server
    pub token: ProgressToken,
    /// The progress value
    pub value: WorkDoneProgressValue,
}

/// Internal progress state tracked by ProgressManager
#[derive(Debug, Clone)]
pub enum ProgressState {
    /// Task is in progress
    InProgress {
        title: String,
        message: Option<String>,
        percentage: Option<u32>,
    },
    /// Task completed successfully
    Completed { message: Option<String> },
    /// Task failed (timeout or error)
    Failed { reason: String },
}

/// Errors that can occur during progress tracking
#[derive(Debug, Error)]
pub enum ProgressError {
    #[error("Progress tracking timed out after {0:?}")]
    Timeout(Duration),

    #[error("Progress broadcast channel closed")]
    ChannelClosed,

    #[error("Task failed: {0}")]
    TaskFailed(String),
}

/// Manages LSP progress notifications and provides async coordination
///
/// The ProgressManager tracks active progress tasks and enables waiting
/// for task completion with timeout support.
///
/// ## Thread Safety
///
/// ProgressManager is thread-safe and can be shared across async tasks
/// using `Arc<ProgressManager>` or via `.clone()`.
#[derive(Clone)]
pub struct ProgressManager {
    /// Active progress tasks by token
    tasks: Arc<DashMap<ProgressToken, ProgressState>>,

    /// Broadcast channel for progress updates (token, state)
    /// Channel size of 100 should be sufficient for progress notifications
    updates_tx: broadcast::Sender<(ProgressToken, ProgressState)>,
}

impl ProgressManager {
    /// Creates a new ProgressManager
    pub fn new() -> Self {
        let (updates_tx, _) = broadcast::channel(100);
        Self {
            tasks: Arc::new(DashMap::new()),
            updates_tx,
        }
    }

    /// Handles a `$/progress` notification from the LSP server
    ///
    /// Updates internal state and broadcasts the update to waiting tasks.
    pub fn handle_notification(&self, params: ProgressParams) {
        let token = params.token;
        let value = params.value;

        let new_state = match value {
            WorkDoneProgressValue::Begin {
                title,
                message,
                percentage,
                ..
            } => {
                debug!(
                    token = %token,
                    title = %title,
                    message = ?message,
                    percentage = ?percentage,
                    "Progress started"
                );
                ProgressState::InProgress {
                    title,
                    message,
                    percentage,
                }
            }
            WorkDoneProgressValue::Report {
                message,
                percentage,
                ..
            } => {
                debug!(
                    token = %token,
                    message = ?message,
                    percentage = ?percentage,
                    "Progress update"
                );

                // Get existing state to preserve title
                if let Some(entry) = self.tasks.get(&token) {
                    match entry.value() {
                        ProgressState::InProgress { title, .. } => ProgressState::InProgress {
                            title: title.clone(),
                            message,
                            percentage,
                        },
                        _ => {
                            // Unexpected state transition
                            warn!(
                                token = %token,
                                "Received progress report for task not in progress"
                            );
                            return;
                        }
                    }
                } else {
                    // Report without begin - log warning but continue
                    warn!(
                        token = %token,
                        "Received progress report for unknown task"
                    );
                    ProgressState::InProgress {
                        title: "Unknown".to_string(),
                        message,
                        percentage,
                    }
                }
            }
            WorkDoneProgressValue::End { message } => {
                debug!(
                    token = %token,
                    message = ?message,
                    "Progress completed"
                );
                ProgressState::Completed { message }
            }
        };

        // Update state
        self.tasks.insert(token.clone(), new_state.clone());

        // Broadcast update (ignore send errors - no receivers is fine)
        let _ = self.updates_tx.send((token, new_state));
    }

    /// Waits for a progress task to complete
    ///
    /// Returns immediately if the task is already completed.
    /// Returns `Err(ProgressError::Timeout)` if timeout is reached.
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// # use mill_lsp::progress::{ProgressManager, ProgressToken};
    /// # use std::time::Duration;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = ProgressManager::new();
    /// let token = ProgressToken::String("rustAnalyzer/Indexing".to_string());
    ///
    /// manager.wait_for_completion(&token, Duration::from_secs(30)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn wait_for_completion(
        &self,
        token: &ProgressToken,
        timeout: Duration,
    ) -> Result<(), ProgressError> {
        // Already completed?
        if let Some(entry) = self.tasks.get(token) {
            match entry.value() {
                ProgressState::Completed { .. } => return Ok(()),
                ProgressState::Failed { reason } => {
                    return Err(ProgressError::TaskFailed(reason.clone()));
                }
                ProgressState::InProgress { .. } => {
                    // Continue to wait
                }
            }
        }

        // Subscribe to updates
        let mut rx = self.updates_tx.subscribe();
        let target_token = token.clone();

        let result = tokio::time::timeout(timeout, async move {
            loop {
                match rx.recv().await {
                    Ok((token, state)) if token == target_token => match state {
                        ProgressState::Completed { .. } => return Ok(()),
                        ProgressState::Failed { reason } => {
                            return Err(ProgressError::TaskFailed(reason));
                        }
                        ProgressState::InProgress { .. } => continue,
                    },
                    Ok(_) => continue, // Different token
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        // We missed some messages but can continue
                        debug!("Progress notification lagged - checking current state");

                        // Check current state directly
                        if let Some(entry) = self.tasks.get(&target_token) {
                            match entry.value() {
                                ProgressState::Completed { .. } => return Ok(()),
                                ProgressState::Failed { reason } => {
                                    return Err(ProgressError::TaskFailed(reason.clone()));
                                }
                                ProgressState::InProgress { .. } => continue,
                            }
                        }
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        return Err(ProgressError::ChannelClosed);
                    }
                }
            }
        })
        .await;

        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(ProgressError::Timeout(timeout)),
        }
    }

    /// Checks if a progress task is completed
    pub fn is_completed(&self, token: &ProgressToken) -> bool {
        self.tasks
            .get(token)
            .map(|entry| matches!(entry.value(), ProgressState::Completed { .. }))
            .unwrap_or(false)
    }

    /// Gets the current state of a progress task
    pub fn get_state(&self, token: &ProgressToken) -> Option<ProgressState> {
        self.tasks.get(token).map(|entry| entry.value().clone())
    }

    /// Removes a completed task from tracking
    ///
    /// This is useful for cleanup after waiting for a task to complete.
    pub fn remove_task(&self, token: &ProgressToken) {
        self.tasks.remove(token);
    }

    /// Gets all active progress tasks
    pub fn active_tasks(&self) -> Vec<(ProgressToken, ProgressState)> {
        self.tasks
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// Subscribe to progress updates
    ///
    /// Returns a receiver that will receive all progress updates.
    /// Useful for displaying progress to users.
    pub fn subscribe(&self) -> broadcast::Receiver<(ProgressToken, ProgressState)> {
        self.updates_tx.subscribe()
    }

    /// Waits for any indexing-related progress to complete
    ///
    /// This waits for common indexing tokens like:
    /// - "rustAnalyzer/Indexing"
    /// - "rustAnalyzer/Roots Scanned"
    /// - TypeScript server project loading
    ///
    /// Returns progress updates via callback as they occur.
    pub async fn wait_for_indexing_with_progress<F>(
        &self,
        timeout: Duration,
        mut on_progress: F,
    ) -> Result<(), ProgressError>
    where
        F: FnMut(&str, Option<u32>), // (message, percentage)
    {
        let mut rx = self.updates_tx.subscribe();
        let start = std::time::Instant::now();

        // Known indexing-related tokens
        let indexing_tokens = [
            "rustAnalyzer/Indexing",
            "rustAnalyzer/Roots Scanned",
            "rustAnalyzer/cargo check",
        ];

        let mut active_indexing: std::collections::HashSet<ProgressToken> =
            std::collections::HashSet::new();

        loop {
            if start.elapsed() >= timeout {
                if active_indexing.is_empty() {
                    // No indexing was ever started, consider it done
                    return Ok(());
                }
                return Err(ProgressError::Timeout(timeout));
            }

            let remaining = timeout.saturating_sub(start.elapsed());

            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Ok((token, state))) => {
                    let token_str = token.to_string();
                    let is_indexing = indexing_tokens.iter().any(|t| {
                        token_str.contains(t) || token_str.to_lowercase().contains("index")
                    });

                    if is_indexing {
                        match &state {
                            ProgressState::InProgress {
                                title,
                                message,
                                percentage,
                            } => {
                                active_indexing.insert(token.clone());
                                let msg = message.as_deref().unwrap_or(title.as_str());
                                on_progress(msg, *percentage);
                            }
                            ProgressState::Completed { message } => {
                                active_indexing.remove(&token);
                                if let Some(msg) = message {
                                    on_progress(msg, Some(100));
                                }
                                // Check if all indexing is done
                                if active_indexing.is_empty() {
                                    // Give a small delay for any follow-up tasks
                                    tokio::time::sleep(Duration::from_millis(100)).await;
                                    // Check if new tasks started
                                    let current_tasks = self.active_tasks();
                                    let still_indexing = current_tasks.iter().any(|(t, s)| {
                                        let t_str = t.to_string();
                                        matches!(s, ProgressState::InProgress { .. })
                                            && indexing_tokens.iter().any(|it| t_str.contains(it))
                                    });
                                    if !still_indexing {
                                        return Ok(());
                                    }
                                }
                            }
                            ProgressState::Failed { reason } => {
                                active_indexing.remove(&token);
                                on_progress(&format!("Failed: {}", reason), None);
                            }
                        }
                    }
                }
                Ok(Err(broadcast::error::RecvError::Lagged(_))) => {
                    // Missed some messages, continue
                    continue;
                }
                Ok(Err(broadcast::error::RecvError::Closed)) => {
                    return Err(ProgressError::ChannelClosed);
                }
                Err(_) => {
                    // Timeout in recv - check if we're done
                    if active_indexing.is_empty() {
                        return Ok(());
                    }
                    return Err(ProgressError::Timeout(timeout));
                }
            }
        }
    }
}

impl Default for ProgressManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_progress_completion_workflow() {
        let manager = ProgressManager::new();
        let token = ProgressToken::String("test/indexing".to_string());

        // Start waiting in background
        let manager_clone = manager.clone();
        let token_clone = token.clone();
        let wait_task = tokio::spawn(async move {
            manager_clone
                .wait_for_completion(&token_clone, Duration::from_secs(5))
                .await
        });

        // Give the wait task time to subscribe
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Send begin notification
        manager.handle_notification(ProgressParams {
            token: token.clone(),
            value: WorkDoneProgressValue::Begin {
                title: "Indexing".to_string(),
                message: None,
                percentage: Some(0),
                cancellable: Some(false),
            },
        });

        // Send report notification
        manager.handle_notification(ProgressParams {
            token: token.clone(),
            value: WorkDoneProgressValue::Report {
                message: Some("Processing files".to_string()),
                percentage: Some(50),
                cancellable: Some(false),
            },
        });

        // Send end notification
        manager.handle_notification(ProgressParams {
            token: token.clone(),
            value: WorkDoneProgressValue::End {
                message: Some("Complete".to_string()),
            },
        });

        // Wait task should complete
        let result = wait_task.await.unwrap();
        assert!(result.is_ok());
        assert!(manager.is_completed(&token));
    }

    #[tokio::test]
    async fn test_wait_timeout() {
        let manager = ProgressManager::new();
        let token = ProgressToken::String("test/never-completes".to_string());

        // Start a task that never completes
        manager.handle_notification(ProgressParams {
            token: token.clone(),
            value: WorkDoneProgressValue::Begin {
                title: "Long task".to_string(),
                message: None,
                percentage: None,
                cancellable: None,
            },
        });

        // Wait should timeout
        let result = manager
            .wait_for_completion(&token, Duration::from_millis(100))
            .await;

        assert!(matches!(result, Err(ProgressError::Timeout(_))));
    }

    #[tokio::test]
    async fn test_already_completed() {
        let manager = ProgressManager::new();
        let token = ProgressToken::String("test/already-done".to_string());

        // Complete task immediately
        manager.handle_notification(ProgressParams {
            token: token.clone(),
            value: WorkDoneProgressValue::Begin {
                title: "Quick task".to_string(),
                message: None,
                percentage: None,
                cancellable: None,
            },
        });

        manager.handle_notification(ProgressParams {
            token: token.clone(),
            value: WorkDoneProgressValue::End { message: None },
        });

        // Wait should return immediately
        let result = manager
            .wait_for_completion(&token, Duration::from_secs(1))
            .await;

        assert!(result.is_ok());
    }
}
