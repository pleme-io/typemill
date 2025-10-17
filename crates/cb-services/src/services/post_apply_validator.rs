//! Post-apply validation service for refactoring plans
//!
//! Executes external validation commands (e.g., "cargo check") after applying
//! a refactoring plan to verify the changes didn't break the codebase.

use cb_protocol::{ApiError, ApiResult as ServerResult};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tokio::process::Command;
use tracing::debug;

/// Service for running post-apply validation commands
///
/// This service executes external commands with timeout support to verify
/// that refactoring changes are valid (e.g., code still compiles).
pub struct PostApplyValidator;

impl PostApplyValidator {
    /// Create a new post-apply validator
    pub fn new() -> Self {
        Self
    }

    /// Run a validation command and return the result
    ///
    /// Executes the specified command with a timeout and captures stdout/stderr.
    ///
    /// # Arguments
    ///
    /// * `config` - Validation configuration including command, timeout, working directory
    ///
    /// # Returns
    ///
    /// A ValidationResult with exit code, output, and duration.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The command times out
    /// - The command cannot be executed (not found, permission denied, etc.)
    pub async fn run_validation(
        &self,
        config: &ValidationConfig,
    ) -> ServerResult<ValidationResult> {
        let start = Instant::now();

        let working_dir = config
            .working_dir
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or(".");

        debug!(
            command = %config.command,
            working_dir = %working_dir,
            timeout_seconds = config.timeout_seconds,
            "Running validation command"
        );

        // Run command with timeout
        let output = tokio::time::timeout(
            tokio::time::Duration::from_secs(config.timeout_seconds),
            Command::new("sh")
                .arg("-c")
                .arg(&config.command)
                .current_dir(working_dir)
                .output(),
        )
        .await
        .map_err(|_| {
            ApiError::Internal(format!(
                "Validation command timed out after {} seconds",
                config.timeout_seconds
            ))
        })?
        .map_err(|e| {
            ApiError::Internal(format!("Failed to execute validation command: {}", e))
        })?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let passed = output.status.success() && (!config.fail_on_stderr || stderr.is_empty());

        debug!(
            exit_code,
            duration_ms,
            passed,
            stderr_len = stderr.len(),
            "Validation command completed"
        );

        Ok(ValidationResult {
            passed,
            command: config.command.clone(),
            exit_code,
            stdout,
            stderr,
            duration_ms,
        })
    }
}

impl Default for PostApplyValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for post-apply validation
#[derive(Debug, Clone, Deserialize)]
pub struct ValidationConfig {
    /// Command to run for validation (e.g., "cargo check --workspace")
    pub command: String,
    /// Timeout in seconds (default: 60)
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    /// Working directory for command execution (default: project root)
    #[serde(default)]
    pub working_dir: Option<String>,
    /// Fail validation if stderr is non-empty (default: false, since many tools write warnings to stderr)
    #[serde(default)]
    pub fail_on_stderr: bool,
}

fn default_timeout() -> u64 {
    60
}

/// Result of running a validation command
#[derive(Debug, Clone, Serialize)]
pub struct ValidationResult {
    /// Whether validation passed
    pub passed: bool,
    /// Command that was executed
    pub command: String,
    /// Exit code from command
    pub exit_code: i32,
    /// Standard output from command
    pub stdout: String,
    /// Standard error from command
    pub stderr: String,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validation_successful_command() {
        let validator = PostApplyValidator::new();

        let config = ValidationConfig {
            command: "echo 'success'".to_string(),
            timeout_seconds: 5,
            working_dir: None,
            fail_on_stderr: false,
        };

        let result = validator.run_validation(&config).await.unwrap();

        assert!(result.passed);
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("success"));
        assert!(result.duration_ms > 0);
    }

    #[tokio::test]
    async fn test_validation_failed_command() {
        let validator = PostApplyValidator::new();

        let config = ValidationConfig {
            command: "exit 1".to_string(),
            timeout_seconds: 5,
            working_dir: None,
            fail_on_stderr: false,
        };

        let result = validator.run_validation(&config).await.unwrap();

        assert!(!result.passed);
        assert_eq!(result.exit_code, 1);
    }

    #[tokio::test]
    async fn test_validation_stderr_handling() {
        let validator = PostApplyValidator::new();

        // Test with fail_on_stderr = false (default)
        let config = ValidationConfig {
            command: "echo 'error' >&2".to_string(),
            timeout_seconds: 5,
            working_dir: None,
            fail_on_stderr: false,
        };

        let result = validator.run_validation(&config).await.unwrap();
        assert!(result.passed); // Passes even with stderr

        // Test with fail_on_stderr = true
        let config_strict = ValidationConfig {
            command: "echo 'error' >&2".to_string(),
            timeout_seconds: 5,
            working_dir: None,
            fail_on_stderr: true,
        };

        let result_strict = validator.run_validation(&config_strict).await.unwrap();
        assert!(!result_strict.passed); // Fails due to stderr
        assert!(result_strict.stderr.contains("error"));
    }

    #[tokio::test]
    async fn test_validation_timeout() {
        let validator = PostApplyValidator::new();

        let config = ValidationConfig {
            command: "sleep 10".to_string(),
            timeout_seconds: 1, // Very short timeout
            working_dir: None,
            fail_on_stderr: false,
        };

        let result = validator.run_validation(&config).await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("timed out"));
    }

    #[tokio::test]
    async fn test_validation_command_not_found() {
        let validator = PostApplyValidator::new();

        let config = ValidationConfig {
            command: "nonexistent_command_12345".to_string(),
            timeout_seconds: 5,
            working_dir: None,
            fail_on_stderr: false,
        };

        let result = validator.run_validation(&config).await.unwrap();

        // Command not found returns exit code 127 via shell
        assert!(!result.passed);
        assert_eq!(result.exit_code, 127); // Shell's "command not found" exit code
        assert!(result.stderr.contains("not found") || result.stderr.contains("command"));
    }

    #[tokio::test]
    async fn test_validation_captures_duration() {
        let validator = PostApplyValidator::new();

        let config = ValidationConfig {
            command: "sleep 0.1 && echo 'done'".to_string(),
            timeout_seconds: 5,
            working_dir: None,
            fail_on_stderr: false,
        };

        let result = validator.run_validation(&config).await.unwrap();

        assert!(result.passed);
        assert!(result.duration_ms >= 100); // At least 100ms for sleep
        assert!(result.stdout.contains("done"));
    }
}
