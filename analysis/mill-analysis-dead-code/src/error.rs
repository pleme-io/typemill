//! Error types for dead code analysis.

use thiserror::Error;

/// Errors that can occur during dead code analysis.
#[derive(Debug, Error)]
pub enum Error {
    /// LSP communication error.
    #[error("LSP error: {0}")]
    Lsp(String),

    /// File system error.
    #[error("File system error: {0}")]
    FileSystem(String),

    /// Path does not exist.
    #[error("Path does not exist: {0}")]
    PathNotFound(String),

    /// Analysis timeout.
    #[error("Analysis timed out")]
    Timeout,

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<mill_analysis_common::AnalysisError> for Error {
    fn from(err: mill_analysis_common::AnalysisError) -> Self {
        match err {
            mill_analysis_common::AnalysisError::LspError(msg) => Error::Lsp(msg),
            mill_analysis_common::AnalysisError::FileSystemError(msg) => Error::FileSystem(msg),
            mill_analysis_common::AnalysisError::ConfigError(msg) => Error::Internal(msg),
            mill_analysis_common::AnalysisError::Timeout(_) => Error::Timeout,
        }
    }
}
