//! Intent specification types for workflow automation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Intent specification for automated workflows
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IntentSpec {
    /// Intent name/identifier
    pub name: String,
    /// Intent arguments as JSON value
    pub arguments: serde_json::Value,
    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<IntentMetadata>,
}

/// Metadata for intent specifications
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IntentMetadata {
    /// Source of the intent (e.g., "user", "automated", "system")
    pub source: String,
    /// Optional correlation ID for tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    /// Timestamp when the intent was created
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
    /// Priority level (1-10, where 10 is highest)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,
    /// Additional context information
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub context: HashMap<String, serde_json::Value>,
}

/// Intent execution result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IntentResult {
    /// Success status
    pub success: bool,
    /// Result message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Result data (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Error details (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<IntentError>,
    /// Execution metrics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<IntentMetrics>,
}

/// Intent execution error
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IntentError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Additional error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Retry information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_info: Option<IntentRetryInfo>,
}

/// Retry information for failed intents
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IntentRetryInfo {
    /// Number of retry attempts made
    pub attempts: u32,
    /// Maximum number of retries allowed
    pub max_attempts: u32,
    /// Delay before next retry in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_delay_ms: Option<u64>,
    /// Exponential backoff multiplier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backoff_multiplier: Option<f64>,
}

/// Intent execution metrics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IntentMetrics {
    /// Execution start time
    pub start_time: chrono::DateTime<chrono::Utc>,
    /// Execution end time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Resource usage metrics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_usage: Option<IntentResourceUsage>,
}

/// Resource usage metrics for intent execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IntentResourceUsage {
    /// CPU time used in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_time_ms: Option<u64>,
    /// Memory usage in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_bytes: Option<u64>,
    /// Number of file operations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_operations: Option<u64>,
    /// Number of network requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_requests: Option<u64>,
}

/// Intent execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum IntentStatus {
    /// Intent is pending execution
    Pending,
    /// Intent is currently being executed
    Running,
    /// Intent completed successfully
    Completed,
    /// Intent failed with error
    Failed,
    /// Intent was cancelled
    Cancelled,
    /// Intent execution timed out
    TimedOut,
    /// Intent is waiting for retry
    Retrying,
}

/// Intent execution context
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IntentContext {
    /// Unique execution ID
    pub execution_id: String,
    /// Intent specification
    pub intent: IntentSpec,
    /// Current status
    pub status: IntentStatus,
    /// Execution result (if completed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<IntentResult>,
    /// Parent execution ID (for sub-intents)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_execution_id: Option<String>,
    /// Child execution IDs
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub child_execution_ids: Vec<String>,
}

impl IntentSpec {
    /// Create a new intent specification
    pub fn new(name: impl Into<String>, arguments: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            arguments,
            metadata: None,
        }
    }

    /// Create a new intent specification with metadata
    pub fn with_metadata(
        name: impl Into<String>,
        arguments: serde_json::Value,
        metadata: IntentMetadata,
    ) -> Self {
        Self {
            name: name.into(),
            arguments,
            metadata: Some(metadata),
        }
    }

    /// Get the intent name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the intent arguments
    pub fn arguments(&self) -> &serde_json::Value {
        &self.arguments
    }

    /// Get the intent metadata
    pub fn metadata(&self) -> Option<&IntentMetadata> {
        self.metadata.as_ref()
    }

    /// Get the correlation ID if available
    pub fn correlation_id(&self) -> Option<&str> {
        self.metadata
            .as_ref()
            .and_then(|m| m.correlation_id.as_deref())
    }

    /// Get the source if available
    pub fn source(&self) -> Option<&str> {
        self.metadata.as_ref().map(|m| m.source.as_str())
    }

    /// Get the priority if available
    pub fn priority(&self) -> Option<u8> {
        self.metadata.as_ref().and_then(|m| m.priority)
    }
}

impl IntentMetadata {
    /// Create new metadata with source
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            correlation_id: None,
            timestamp: Some(chrono::Utc::now()),
            priority: None,
            context: HashMap::new(),
        }
    }

    /// Set correlation ID
    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = Some(priority.clamp(1, 10));
        self
    }

    /// Add context value
    pub fn with_context(
        mut self,
        key: impl Into<String>,
        value: serde_json::Value,
    ) -> Self {
        self.context.insert(key.into(), value);
        self
    }
}

impl IntentResult {
    /// Create a successful result
    pub fn success() -> Self {
        Self {
            success: true,
            message: None,
            data: None,
            error: None,
            metrics: None,
        }
    }

    /// Create a successful result with data
    pub fn success_with_data(data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: None,
            data: Some(data),
            error: None,
            metrics: None,
        }
    }

    /// Create a failed result
    pub fn failure(error: IntentError) -> Self {
        Self {
            success: false,
            message: None,
            data: None,
            error: Some(error),
            metrics: None,
        }
    }

    /// Create a failed result with message
    pub fn failure_with_message(error: IntentError, message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: Some(message.into()),
            data: None,
            error: Some(error),
            metrics: None,
        }
    }

    /// Add metrics to the result
    pub fn with_metrics(mut self, metrics: IntentMetrics) -> Self {
        self.metrics = Some(metrics);
        self
    }
}

impl IntentError {
    /// Create a new intent error
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            retry_info: None,
        }
    }

    /// Add error details
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Add retry information
    pub fn with_retry_info(mut self, retry_info: IntentRetryInfo) -> Self {
        self.retry_info = Some(retry_info);
        self
    }
}

impl IntentContext {
    /// Create a new intent context
    pub fn new(execution_id: impl Into<String>, intent: IntentSpec) -> Self {
        Self {
            execution_id: execution_id.into(),
            intent,
            status: IntentStatus::Pending,
            result: None,
            parent_execution_id: None,
            child_execution_ids: Vec::new(),
        }
    }

    /// Update the status
    pub fn with_status(mut self, status: IntentStatus) -> Self {
        self.status = status;
        self
    }

    /// Add result
    pub fn with_result(mut self, result: IntentResult) -> Self {
        self.result = Some(result);
        self
    }

    /// Set parent execution ID
    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_execution_id = Some(parent_id.into());
        self
    }

    /// Add child execution ID
    pub fn add_child(&mut self, child_id: impl Into<String>) {
        self.child_execution_ids.push(child_id.into());
    }
}

impl Default for IntentStatus {
    fn default() -> Self {
        Self::Pending
    }
}