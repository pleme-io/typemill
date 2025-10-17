//! Error types for MCP proxy

use thiserror::Error;

pub type McpProxyResult<T> = Result<T, McpProxyError>;

#[derive(Debug, Error)]
pub enum McpProxyError {
    #[error("Failed to spawn MCP server '{0}': {1}")]
    SpawnFailed(String, #[source] std::io::Error),

    #[error("MCP server error: {0}")]
    McpServerError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Server '{0}' not found")]
    ServerNotFound(String),

    #[error("Tool '{0}' not found on server '{1}'")]
    ToolNotFound(String, String),

    #[error("Plugin error: {0}")]
    PluginError(String),
}

impl McpProxyError {
    pub fn spawn_failed(name: &str, error: std::io::Error) -> Self {
        Self::SpawnFailed(name.to_string(), error)
    }
}
