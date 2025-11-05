//! From implementations for standard library types

use super::MillError;
#[allow(deprecated)]
use crate::protocol::error::ApiError;

impl From<std::io::Error> for MillError {
    fn from(err: std::io::Error) -> Self {
        MillError::Io {
            message: err.to_string(),
            path: None,
            source: Some(err),
        }
    }
}

impl From<serde_json::Error> for MillError {
    fn from(err: serde_json::Error) -> Self {
        MillError::Json {
            message: err.to_string(),
            source: Some(err),
        }
    }
}

#[allow(deprecated)]
impl From<ApiError> for MillError {
    fn from(err: ApiError) -> Self {
        match err {
            // Struct variants
            ApiError::Config { message } => MillError::Config {
                message,
                source: None,
            },
            ApiError::Bootstrap { message } => MillError::Bootstrap {
                message,
                source: None,
            },
            ApiError::Runtime { message } => MillError::Runtime {
                message,
                context: None,
            },
            ApiError::Parse { message } => MillError::Parse {
                message,
                file: None,
                line: None,
                column: None,
            },
            // Tuple variants
            ApiError::InvalidRequest(msg) => MillError::InvalidRequest {
                message: msg,
                parameter: None,
            },
            ApiError::Unsupported(msg) => MillError::NotSupported {
                operation: msg,
                reason: None,
            },
            ApiError::Auth(msg) => MillError::Auth {
                message: msg,
                method: None,
            },
            ApiError::NotFound(msg) => MillError::NotFound {
                resource: msg,
                resource_type: None,
            },
            ApiError::AlreadyExists(msg) => MillError::AlreadyExists {
                resource: msg,
                resource_type: None,
            },
            ApiError::Internal(msg) => MillError::Internal {
                message: msg,
                source: None,
            },
            ApiError::Lsp(msg) => MillError::Lsp {
                message: msg,
                server: None,
                method: None,
            },
            ApiError::Ast(msg) => MillError::Ast {
                message: msg,
                operation: None,
            },
            ApiError::Plugin(msg) => MillError::Plugin {
                plugin: "unknown".to_string(),
                message: msg,
                operation: None,
            },
            // From variants (convert to Io/Json with embedded error info)
            ApiError::Io(io_err) => MillError::Io {
                message: io_err.to_string(),
                path: None,
                source: Some(io_err),
            },
            ApiError::Serialization(json_err) => MillError::Json {
                message: json_err.to_string(),
                source: Some(json_err),
            },
        }
    }
}
