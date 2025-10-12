//! Error types for graph analysis operations.

use thiserror::Error;

/// A generic error type for graph operations.
#[derive(Debug, Error)]
pub enum GraphError {
    /// Represents an I/O error during graph persistence.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Represents a serialization or deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Represents a node not being found in the graph.
    #[error("Node not found: {0}")]
    NodeNotFound(String),
}
