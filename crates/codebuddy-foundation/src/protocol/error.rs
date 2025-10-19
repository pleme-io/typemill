//! API error types for the codebuddy system

use crate::error::{error_codes, ApiError as CoreApiError};
use serde::Serialize;
use thiserror::Error;

/// Core API operation errors
#[derive(Error, Debug, Serialize)]
#[non_exhaustive]
#[serde(tag = "type", content = "details")]
pub enum ApiError {
    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("Bootstrap error: {message}")]
    Bootstrap { message: String },

    #[error("Runtime error: {message}")]
    Runtime { message: String },

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Unsupported operation: {0}")]
    Unsupported(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("IO error: {0}")]
    #[serde(serialize_with = "serialize_io_error")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    #[serde(serialize_with = "serialize_json_error")]
    Serialization(#[from] serde_json::Error),

    #[error("Parse error: {message}")]
    Parse { message: String },

    #[error("LSP error: {0}")]
    Lsp(String),

    #[error("AST error: {0}")]
    Ast(String),

    #[error("Plugin error: {0}")]
    Plugin(String),
}

impl ApiError {
    /// Get the error category for structured logging and alerting
    pub fn category(&self) -> &'static str {
        match self {
            ApiError::Config { .. } => "config_error",
            ApiError::Bootstrap { .. } => "bootstrap_error",
            ApiError::Runtime { .. } => "runtime_error",
            ApiError::InvalidRequest(_) => "invalid_request",
            ApiError::Unsupported(_) => "unsupported_operation",
            ApiError::Auth(_) => "authentication_error",
            ApiError::NotFound(_) => "not_found",
            ApiError::AlreadyExists(_) => "already_exists",
            ApiError::Internal(_) => "internal_error",
            ApiError::Io(_) => "io_error",
            ApiError::Serialization(_) => "serialization_error",
            ApiError::Parse { .. } => "parse_error",
            ApiError::Lsp(_) => "lsp_error",
            ApiError::Ast(_) => "ast_error",
            ApiError::Plugin(_) => "plugin_error",
        }
    }

    /// Check if this is a client error (4xx-style)
    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            ApiError::InvalidRequest(_)
                | ApiError::Unsupported(_)
                | ApiError::Auth(_)
                | ApiError::NotFound(_)
                | ApiError::AlreadyExists(_)
        )
    }

    /// Check if this is a server error (5xx-style)
    pub fn is_server_error(&self) -> bool {
        !self.is_client_error()
    }

    /// Create a new configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Create a new bootstrap error
    pub fn bootstrap(message: impl Into<String>) -> Self {
        Self::Bootstrap {
            message: message.into(),
        }
    }

    /// Create a new runtime error
    pub fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime {
            message: message.into(),
        }
    }

    /// Create a new internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    /// Create a new LSP error
    pub fn lsp(message: impl Into<String>) -> Self {
        Self::Lsp(message.into())
    }

    /// Create a new AST error
    pub fn ast(message: impl Into<String>) -> Self {
        Self::Ast(message.into())
    }

    /// Create a new plugin error
    pub fn plugin(message: impl Into<String>) -> Self {
        Self::Plugin(message.into())
    }

    /// Convert to standardized API error response
    ///
    /// This converts the internal error enum to a structured error format
    /// suitable for API responses with error codes and optional details.
    pub fn to_api_response(&self) -> CoreApiError {
        use error_codes::*;

        match self {
            ApiError::Config { message } => CoreApiError::new(E1001_INVALID_REQUEST, message),
            ApiError::Bootstrap { message } => CoreApiError::new(
                E1000_INTERNAL_SERVER_ERROR,
                format!("Bootstrap error: {}", message),
            ),
            ApiError::Runtime { message } => CoreApiError::new(
                E1000_INTERNAL_SERVER_ERROR,
                format!("Runtime error: {}", message),
            ),
            ApiError::InvalidRequest(msg) => CoreApiError::new(E1001_INVALID_REQUEST, msg),
            ApiError::Unsupported(msg) => CoreApiError::new(E1007_NOT_SUPPORTED, msg),
            ApiError::Auth(msg) => CoreApiError::new(E1005_PERMISSION_DENIED, msg),
            ApiError::NotFound(msg) => CoreApiError::new(E1002_FILE_NOT_FOUND, msg),
            ApiError::AlreadyExists(msg) => CoreApiError::new(
                E1001_INVALID_REQUEST,
                format!("Resource already exists: {}", msg),
            ),
            ApiError::Internal(msg) => CoreApiError::new(E1000_INTERNAL_SERVER_ERROR, msg),
            ApiError::Io(e) => {
                CoreApiError::new(E1000_INTERNAL_SERVER_ERROR, format!("I/O error: {}", e))
            }
            ApiError::Serialization(e) => {
                CoreApiError::new(E1008_INVALID_DATA, format!("Serialization error: {}", e))
            }
            ApiError::Parse { message } => CoreApiError::new(E1008_INVALID_DATA, message),
            ApiError::Lsp(msg) => CoreApiError::new(E1003_LSP_ERROR, msg),
            ApiError::Ast(msg) => {
                CoreApiError::new(E1000_INTERNAL_SERVER_ERROR, format!("AST error: {}", msg))
            }
            ApiError::Plugin(msg) => CoreApiError::new(
                E1000_INTERNAL_SERVER_ERROR,
                format!("Plugin error: {}", msg),
            ),
        }
    }
}

/// Convert from crate::error::CoreError to ApiError
impl From<crate::error::CoreError> for ApiError {
    fn from(error: crate::error::CoreError) -> Self {
        match error {
            crate::error::CoreError::Config { message } => ApiError::Config { message },
            crate::error::CoreError::NotFound { resource } => ApiError::NotFound(resource),
            crate::error::CoreError::InvalidData { message } => {
                ApiError::InvalidRequest(message)
            }
            crate::error::CoreError::Internal { message } => ApiError::Internal(message),
            crate::error::CoreError::NotSupported { operation } => {
                ApiError::Unsupported(operation)
            }
            _ => ApiError::Internal(error.to_string()),
        }
    }
}

/// Convert from crate::model::mcp::McpError to ApiError
impl From<crate::model::mcp::McpError> for ApiError {
    fn from(error: crate::model::mcp::McpError) -> Self {
        ApiError::InvalidRequest(error.message)
    }
}

/// Result type alias for API operations
pub type ApiResult<T> = Result<T, ApiError>;

/// Macro for logging errors with automatic category extraction
#[macro_export]
macro_rules! log_error {
    ($err:expr, $msg:literal) => {
        tracing::error!(
            error_category = $err.category(),
            error = %$err,
            is_client_error = $err.is_client_error(),
            $msg
        )
    };
    ($err:expr, $msg:literal, $($field:ident = $value:expr),* $(,)?) => {
        tracing::error!(
            error_category = $err.category(),
            error = %$err,
            is_client_error = $err.is_client_error(),
            $($field = $value,)*
            $msg
        )
    };
}

/// Helper function to serialize std::io::Error as a string
fn serialize_io_error<S>(error: &std::io::Error, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&error.to_string())
}

/// Helper function to serialize serde_json::Error as a string
fn serialize_json_error<S>(error: &serde_json::Error, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&error.to_string())
}
