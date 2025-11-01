//! Types for validation configuration and results.

use serde::{Deserialize, Serialize};

/// Configuration for post-apply validation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationConfig {
    /// Enable post-operation validation
    pub enabled: bool,
    /// Command to run for validation
    pub command: String,
    /// Action on failure
    pub on_failure: ValidationFailureAction,
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

/// Action to take when validation fails
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "PascalCase")]
pub enum ValidationFailureAction {
    /// Just report the error
    #[default]
    Report,
    /// Rollback the operation using git
    Rollback,
    /// Ask the user what to do
    Interactive,
}

/// Result of running a validation command
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            command: "cargo check".to_string(),
            on_failure: ValidationFailureAction::Report,
            timeout_seconds: 60,
            working_dir: None,
            fail_on_stderr: false,
        }
    }
}
