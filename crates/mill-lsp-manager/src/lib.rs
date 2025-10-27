//! LSP server auto-download and management for TypeMill
//!
//! This crate provides automatic LSP server installation with:
//! - Zero-configuration setup
//! - Automatic language detection
//! - Secure downloads with checksum verification
//! - Cross-platform support
//!
//! # Example
//!
//! ```no_run
//! use mill_lsp_manager::LspManager;
//! use std::path::Path;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let manager = LspManager::new()?;
//!
//! // Auto-detect and ensure LSPs are installed
//! let needed = manager.detect_needed_lsps(Path::new("."))?;
//! for lsp in needed {
//!     let path = manager.ensure_installed(&lsp).await?;
//!     println!("✅ {} installed at {}", lsp, path.display());
//! }
//! # Ok(())
//! # }
//! ```

mod cache;
mod detector;
mod downloader;
mod error;
mod registry;
mod types;
mod verifier;

pub use error::{LspError, Result};
pub use types::{InstallStatus, Platform};

use types::LspRegistry;
use std::path::{Path, PathBuf};
use tracing::info;

/// LSP manager for auto-downloading and managing LSP servers
pub struct LspManager {
    registry: LspRegistry,
}

impl LspManager {
    /// Create a new LSP manager
    pub fn new() -> Result<Self> {
        let registry = registry::load_registry()?;
        Ok(Self { registry })
    }

    /// Ensure an LSP server is installed (download if needed)
    ///
    /// Returns the path to the LSP binary
    pub async fn ensure_installed(&self, lsp_name: &str) -> Result<PathBuf> {
        info!("Ensuring {} is installed", lsp_name);

        // Get LSP configuration
        let config = self
            .registry
            .get(lsp_name)
            .ok_or_else(|| LspError::LspNotFound(lsp_name.to_string()))?;

        // Check if already installed in system PATH
        if config.test_system_install() {
            info!("{} found in system PATH", lsp_name);
            return Ok(PathBuf::from(&config.command));
        }

        // Check cache
        let cache_path = cache::lsp_binary_path(lsp_name)?;
        if cache_path.exists() {
            info!("{} found in cache", lsp_name);
            return Ok(cache_path);
        }

        // Download and install
        info!("{} not found, downloading...", lsp_name);
        self.download_and_install(lsp_name).await
    }

    /// Download and install an LSP server
    async fn download_and_install(&self, lsp_name: &str) -> Result<PathBuf> {
        let config = self
            .registry
            .get(lsp_name)
            .ok_or_else(|| LspError::LspNotFound(lsp_name.to_string()))?;

        // Check runtime dependency
        config.check_runtime()?;

        // Get platform-specific configuration
        let platform = Platform::current();
        let platform_config = config.get_platform_config(&platform)?;

        // Initialize cache
        cache::init_cache().await?;

        // Download to temporary location
        let temp_dir = std::env::temp_dir().join(format!("mill-lsp-{}", lsp_name));
        tokio::fs::create_dir_all(&temp_dir).await?;

        let temp_download = temp_dir.join("download");

        info!(
            "Downloading {} from {}",
            lsp_name, platform_config.url
        );

        downloader::download_file(&platform_config.url, &temp_download, &platform_config.sha256)
            .await?;

        // Decompress if needed
        let final_path = cache::lsp_binary_path(lsp_name)?;

        if platform_config.compressed != "none" {
            info!("Decompressing {}", lsp_name);
            downloader::decompress_file(&temp_download, &final_path, &platform_config.compressed)
                .await?;
        } else {
            tokio::fs::rename(&temp_download, &final_path).await?;
        }

        // Cleanup temp directory
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;

        info!("✅ {} installed successfully", lsp_name);
        Ok(final_path)
    }

    /// Check if an LSP is cached locally
    pub fn is_cached(&self, lsp_name: &str) -> bool {
        cache::is_cached(lsp_name)
    }

    /// Get LSPs needed for languages in a project
    pub fn detect_needed_lsps(&self, project_path: &Path) -> Result<Vec<String>> {
        Ok(detector::required_lsps(&self.registry, project_path))
    }

    /// Check installation status of an LSP
    pub fn check_status(&self, lsp_name: &str) -> Result<InstallStatus> {
        let config = self
            .registry
            .get(lsp_name)
            .ok_or_else(|| LspError::LspNotFound(lsp_name.to_string()))?;

        // Check system PATH
        if config.test_system_install() {
            return Ok(InstallStatus::Installed {
                path: PathBuf::from(&config.command),
            });
        }

        // Check cache
        let cache_path = cache::lsp_binary_path(lsp_name)?;
        if cache_path.exists() {
            return Ok(InstallStatus::Installed { path: cache_path });
        }

        // Check if needs runtime
        if let Some(runtime) = &config.runtime_required {
            if !command_exists(runtime) {
                return Ok(InstallStatus::NeedsRuntime {
                    runtime: runtime.to_string(),
                });
            }
        }

        Ok(InstallStatus::NotInstalled)
    }

    /// List all available LSP servers in the registry
    pub fn list_available(&self) -> Vec<&String> {
        self.registry.list_all()
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> Result<cache::CacheStats> {
        cache::cache_stats().await
    }
}

/// Check if a command exists in PATH
fn command_exists(cmd: &str) -> bool {
    which::which(cmd).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_manager() {
        let manager = LspManager::new().unwrap();
        let available = manager.list_available();
        assert!(!available.is_empty());
    }

    #[test]
    fn test_detect_needed_lsps() {
        let manager = LspManager::new().unwrap();
        let needed = manager.detect_needed_lsps(Path::new(".")).unwrap();
        // Should detect rust-analyzer for this Rust project
        assert!(!needed.is_empty());
    }
}
