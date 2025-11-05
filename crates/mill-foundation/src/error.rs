//! Error handling for cb-core and the broader Codeflow Buddy system

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// Core error type used throughout the Codeflow Buddy system
#[deprecated(since = "0.3.0", note = "Use MillError instead")]
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum CoreError {
    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid data: {message}")]
    InvalidData { message: String },

    #[error("Operation not supported: {operation}")]
    NotSupported { operation: String },

    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    #[error("Permission denied: {operation}")]
    PermissionDenied { operation: String },

    #[error("Timeout occurred during: {operation}")]
    Timeout { operation: String },

    #[error("Internal error: {message}")]
    Internal { message: String },
}

#[allow(deprecated)]
impl CoreError {
    /// Create a new configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Create a new invalid data error
    pub fn invalid_data(message: impl Into<String>) -> Self {
        Self::InvalidData {
            message: message.into(),
        }
    }

    /// Create a new not supported error
    pub fn not_supported(operation: impl Into<String>) -> Self {
        Self::NotSupported {
            operation: operation.into(),
        }
    }

    /// Create a new not found error
    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound {
            resource: resource.into(),
        }
    }

    /// Create a new permission denied error
    pub fn permission_denied(operation: impl Into<String>) -> Self {
        Self::PermissionDenied {
            operation: operation.into(),
        }
    }

    /// Create a new timeout error
    pub fn timeout(operation: impl Into<String>) -> Self {
        Self::Timeout {
            operation: operation.into(),
        }
    }

    /// Create a new internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }
}

/// Result type alias for convenience
#[allow(deprecated)]
#[deprecated(since = "0.3.0", note = "Use MillResult instead")]
pub type CoreResult<T> = Result<T, CoreError>;

// ============================================================================
// Standardized API Error Response
// ============================================================================

/// Standardized error codes for API responses
pub mod error_codes {
    /// Internal server error (500)
    pub const E1000_INTERNAL_SERVER_ERROR: &str = "E1000";
    /// Invalid request parameters (400)
    pub const E1001_INVALID_REQUEST: &str = "E1001";
    /// File not found (404)
    pub const E1002_FILE_NOT_FOUND: &str = "E1002";
    /// LSP server error
    pub const E1003_LSP_ERROR: &str = "E1003";
    /// Operation timeout
    pub const E1004_TIMEOUT: &str = "E1004";
    /// Permission denied (403)
    pub const E1005_PERMISSION_DENIED: &str = "E1005";
    /// Resource not found (404)
    pub const E1006_RESOURCE_NOT_FOUND: &str = "E1006";
    /// Operation not supported
    pub const E1007_NOT_SUPPORTED: &str = "E1007";
    /// Invalid data format
    pub const E1008_INVALID_DATA: &str = "E1008";
}

/// Standardized API error structure for MCP tool responses
///
/// This struct provides a consistent error format across all MCP tools,
/// making it easier for clients to parse and handle errors programmatically.
///
/// # Fields
/// - `code`: Machine-readable error code (e.g., "E1000")
/// - `message`: Human-readable error message
/// - `details`: Optional additional context (file paths, line numbers, etc.)
///
/// # Example
/// ```rust,no_run
/// use mill_foundation::error::ApiError;
/// use serde_json::json;
///
/// let error = ApiError {
///     code: "E1002".to_string(),
///     message: "File does not exist".to_string(),
///     details: Some(json!({"path": "/path/to/missing/file.rs"})),
///     suggestion: Some("Check that the file path is correct and the file exists".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    /// Machine-readable error code
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Optional additional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
    /// Optional actionable suggestion for fixing the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl ApiError {
    /// Create a new API error
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            suggestion: None,
        }
    }

    /// Create a new API error with details
    pub fn with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: Value,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: Some(details),
            suggestion: None,
        }
    }

    /// Add details to an existing error
    pub fn details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Add a suggestion to an existing error
    pub fn suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Create a new API error with a suggestion
    pub fn with_suggestion(
        code: impl Into<String>,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            suggestion: Some(suggestion.into()),
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)?;
        if let Some(details) = &self.details {
            write!(f, " (details: {})", details)?;
        }
        if let Some(suggestion) = &self.suggestion {
            write!(f, "\nSuggestion: {}", suggestion)?;
        }
        Ok(())
    }
}

impl std::error::Error for ApiError {}

/// Convert CoreError to ApiError
#[allow(deprecated)]
impl From<CoreError> for ApiError {
    fn from(err: CoreError) -> Self {
        use error_codes::*;

        match err {
            CoreError::Config { message } => ApiError::new(E1001_INVALID_REQUEST, message),
            CoreError::Io(e) => {
                ApiError::new(E1000_INTERNAL_SERVER_ERROR, format!("I/O error: {}", e))
            }
            CoreError::Json(e) => ApiError::new(E1008_INVALID_DATA, format!("JSON error: {}", e)),
            CoreError::InvalidData { message } => ApiError::new(E1008_INVALID_DATA, message),
            CoreError::NotSupported { operation } => ApiError::new(
                E1007_NOT_SUPPORTED,
                format!("Operation not supported: {}", operation),
            ),
            CoreError::NotFound { resource } => ApiError::new(
                E1006_RESOURCE_NOT_FOUND,
                format!("Resource not found: {}", resource),
            ),
            CoreError::PermissionDenied { operation } => ApiError::new(
                E1005_PERMISSION_DENIED,
                format!("Permission denied: {}", operation),
            ),
            CoreError::Timeout { operation } => {
                ApiError::new(E1004_TIMEOUT, format!("Timeout: {}", operation))
            }
            CoreError::Internal { message } => ApiError::new(E1000_INTERNAL_SERVER_ERROR, message),
        }
    }
}
