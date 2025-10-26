//! Client error types

use mill_foundation::core::CoreError;
use thiserror::Error;

/// Client operation errors
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ClientError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Transport error: {0}")]
    TransportError(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Request error: {0}")]
    RequestError(String),

    #[error("Timeout error: {0}")]
    TimeoutError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("I/O error: {0}")]
    IoError(String),

    #[error("Core error: {0}")]
    Core(#[from] CoreError),
}

impl ClientError {
    /// Create a new configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::ConfigError(message.into())
    }

    /// Create a new connection error
    pub fn connection(message: impl Into<String>) -> Self {
        Self::ConnectionError(message.into())
    }

    /// Create a new transport error
    pub fn transport(message: impl Into<String>) -> Self {
        Self::TransportError(message.into())
    }

    /// Create a new protocol error
    pub fn protocol(message: impl Into<String>) -> Self {
        Self::ProtocolError(message.into())
    }

    /// Create a new authentication error
    pub fn authentication(message: impl Into<String>) -> Self {
        Self::AuthError(message.into())
    }

    /// Create a new request error
    pub fn request(message: impl Into<String>) -> Self {
        Self::RequestError(message.into())
    }

    /// Create a new timeout error
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::TimeoutError(message.into())
    }

    /// Create a new serialization error
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::SerializationError(message.into())
    }

    /// Create a new I/O error
    pub fn io(message: impl Into<String>) -> Self {
        Self::IoError(message.into())
    }
}

impl From<ClientError> for CoreError {
    fn from(err: ClientError) -> Self {
        match err {
            ClientError::Core(core_err) => core_err,
            _ => CoreError::internal(format!("Client error: {}", err)),
        }
    }
}

/// Result type alias for client operations
pub type ClientResult<T> = Result<T, ClientError>;
