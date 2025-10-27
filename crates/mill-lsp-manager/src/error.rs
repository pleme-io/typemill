// ! Error types for LSP manager

use thiserror::Error;

#[derive(Error, Debug)]
pub enum LspError {
    #[error("LSP server '{0}' not found in registry")]
    LspNotFound(String),

    #[error("No platform configuration for {os}/{arch}")]
    PlatformNotSupported { os: String, arch: String },

    #[error("Failed to load registry: {0}")]
    RegistryLoadFailed(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Checksum verification failed (expected: {expected}, got: {actual})")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Insecure URL (must use HTTPS): {0}")]
    InsecureUrl(String),

    #[error("Download too large: {0} bytes (max: {1} bytes)")]
    DownloadTooLarge(u64, u64),

    #[error("Decompression failed: {0}")]
    DecompressionFailed(String),

    #[error("Invalid LSP name (contains path separators): {0}")]
    InvalidLspName(String),

    #[error("Runtime '{0}' not found (required for this LSP)")]
    RuntimeNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, LspError>;
