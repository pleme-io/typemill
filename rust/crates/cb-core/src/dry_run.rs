//! Dry run utilities for preview mode operations
//!
//! This module provides a centralized pattern for implementing dry-run/preview mode
//! across all tools that modify the file system or project state.

use serde::{Deserialize, Serialize};
use std::future::Future;

/// Wrapper for operations that support dry-run mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DryRunnable<T> {
    /// Whether this was a dry run (preview only)
    pub dry_run: bool,
    /// The operation result
    pub result: T,
}

impl<T: Serialize> DryRunnable<T> {
    /// Create a new dry-runnable operation result
    pub fn new(dry_run: bool, result: T) -> Self {
        Self { dry_run, result }
    }

    /// Convert to JSON response with dry_run indicator
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "dry_run": self.dry_run,
            "result": self.result
        })
    }
}

/// Execute an operation with dry-run support
///
/// If dry_run=true, executes `preview_fn`, otherwise executes `execute_fn`.
/// This ensures a clean separation between preview and execution logic.
///
/// # Example
///
/// ```ignore
/// let result = execute_with_dry_run(
///     dry_run,
///     || async {
///         // Preview logic - analyze what would change
///         Ok(calculate_changes())
///     },
///     || async {
///         // Execution logic - actually modify files
///         modify_files().await?;
///         Ok(get_results())
///     },
/// ).await?;
/// ```
pub async fn execute_with_dry_run<T, PFut, EFut>(
    dry_run: bool,
    preview_fn: impl FnOnce() -> PFut,
    execute_fn: impl FnOnce() -> EFut,
) -> Result<DryRunnable<T>, Box<dyn std::error::Error + Send + Sync>>
where
    PFut: Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>>,
    EFut: Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>>,
    T: Serialize,
{
    let result = if dry_run {
        preview_fn().await?
    } else {
        execute_fn().await?
    };

    Ok(DryRunnable::new(dry_run, result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dry_run_executes_preview() {
        let result = execute_with_dry_run(
            true,
            || async { Ok("preview") },
            || async { Ok("execute") },
        )
        .await
        .unwrap();

        assert!(result.dry_run);
        assert_eq!(result.result, "preview");
    }

    #[tokio::test]
    async fn test_no_dry_run_executes_real() {
        let result = execute_with_dry_run(
            false,
            || async { Ok("preview") },
            || async { Ok("execute") },
        )
        .await
        .unwrap();

        assert!(!result.dry_run);
        assert_eq!(result.result, "execute");
    }

    #[test]
    fn test_dry_runnable_to_json() {
        let result = DryRunnable::new(true, "test_data");
        let json = result.to_json();

        assert_eq!(json["dry_run"], true);
        assert_eq!(json["result"], "test_data");
    }
}
