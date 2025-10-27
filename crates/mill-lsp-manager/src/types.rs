//! Data structures for LSP server registry

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// LSP server registry containing all available LSP servers
#[derive(Debug, Clone, Deserialize)]
pub struct LspRegistry {
    /// Map of LSP server name to its configuration
    pub lsp: HashMap<String, LspConfig>,
}

/// Configuration for a single LSP server
#[derive(Debug, Clone, Deserialize)]
pub struct LspConfig {
    /// Languages this LSP server supports
    pub languages: Vec<String>,

    /// Command name to execute
    pub command: String,

    /// Arguments to test if LSP is installed
    #[serde(default)]
    pub test_args: Vec<String>,

    /// Runtime dependency (e.g., "node" for TypeScript LSP)
    #[serde(default)]
    pub runtime_required: Option<String>,

    /// Platform-specific download information
    pub platform: Vec<PlatformConfig>,
}

/// Platform-specific download configuration
#[derive(Debug, Clone, Deserialize)]
pub struct PlatformConfig {
    /// Operating system (linux, macos, windows)
    pub os: String,

    /// CPU architecture (x86_64, aarch64)
    pub arch: String,

    /// Download URL
    pub url: String,

    /// SHA256 checksum for verification
    pub sha256: String,

    /// Compression format (gzip, tar.gz, zip)
    pub compressed: String,
}

/// Current platform information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Platform {
    pub os: String,
    pub arch: String,
}

impl Platform {
    /// Detect the current platform
    pub fn current() -> Self {
        let os = if cfg!(target_os = "linux") {
            "linux"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else if cfg!(target_os = "windows") {
            "windows"
        } else {
            "unknown"
        };

        let arch = if cfg!(target_arch = "x86_64") {
            "x86_64"
        } else if cfg!(target_arch = "aarch64") {
            "aarch64"
        } else {
            "unknown"
        };

        Self {
            os: os.to_string(),
            arch: arch.to_string(),
        }
    }
}

/// Download progress information
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

/// LSP installation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallStatus {
    /// LSP is installed and ready
    Installed { path: std::path::PathBuf },

    /// LSP is not installed
    NotInstalled,

    /// LSP is installed but needs runtime (e.g., Node.js)
    NeedsRuntime { runtime: String },
}
