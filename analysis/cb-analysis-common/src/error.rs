// analysis/cb-analysis-common/src/error.rs

/// Common error type for analysis operations
#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("LSP communication failed: {0}")]
    LspError(String),

    #[error("File system error: {0}")]
    FileSystemError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Analysis timeout after {0}s")]
    Timeout(u64),
}
