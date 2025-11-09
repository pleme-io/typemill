//! Rust LSP installer implementation

use async_trait::async_trait;
use mill_lang_common::lsp::{
    check_binary_in_path, decompress_gzip, download_file, get_cache_dir, make_executable,
    verify_checksum, Platform,
};
use mill_plugin_api::{LspInstaller, PluginApiError, PluginResult};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Rust LSP installer (rust-analyzer)
#[derive(Default)]
pub struct RustLspInstaller;

impl RustLspInstaller {
    pub const fn new() -> Self {
        Self
    }

    /// Get download URL for current platform
    fn get_download_url(platform: &Platform) -> PluginResult<String> {
        let base = "https://github.com/rust-lang/rust-analyzer/releases/download/2025-10-27";

        let filename = match (platform.os_string(), platform.arch_string()) {
            ("linux", "x86_64") => "rust-analyzer-x86_64-unknown-linux-gnu.gz",
            ("linux", "aarch64") => "rust-analyzer-aarch64-unknown-linux-gnu.gz",
            ("macos", "x86_64") => "rust-analyzer-x86_64-apple-darwin.gz",
            ("macos", "aarch64") => "rust-analyzer-aarch64-apple-darwin.gz",
            (os, arch) => {
                return Err(PluginApiError::not_supported(format!(
                    "rust-analyzer download not available for {}-{}",
                    os, arch
                )))
            }
        };

        Ok(format!("{}/{}", base, filename))
    }

    /// Get expected checksum for current platform
    fn get_checksum(platform: &Platform) -> PluginResult<String> {
        let checksum = match (platform.os_string(), platform.arch_string()) {
            ("linux", "x86_64") => {
                "001a0a999990247df48367d5a396fa30b093af4e44bf1be903a5636a1c78a25f"
            }
            ("linux", "aarch64") => {
                "5b47cbfc75b58c46553cf1d9d0f5e6b44157223289d6526a5d8879e73e163fc5"
            }
            ("macos", "x86_64") => {
                "af58238af1c6df60e5658e2dff1881b2fd1c8eb486351437b9479fc5af6e8581"
            }
            ("macos", "aarch64") => {
                "e2baa9d70672d4b58cb36d35f2975b7316814b7bcc1ded2eabbb59053be152a0"
            }
            (os, arch) => {
                return Err(PluginApiError::not_supported(format!(
                    "rust-analyzer checksum not available for {}-{}",
                    os, arch
                )))
            }
        };

        Ok(checksum.to_string())
    }
}

#[async_trait]
impl LspInstaller for RustLspInstaller {
    fn lsp_name(&self) -> &str {
        "rust-analyzer"
    }

    fn check_installed(&self) -> PluginResult<Option<PathBuf>> {
        // Check system PATH first
        if let Some(path) = check_binary_in_path("rust-analyzer") {
            debug!("Found rust-analyzer in PATH: {:?}", path);
            return Ok(Some(path));
        }

        // Check cache directory
        let cache_dir = get_cache_dir();
        let cached_path = cache_dir.join("rust-analyzer");
        if cached_path.exists() {
            debug!("Found rust-analyzer in cache: {:?}", cached_path);
            return Ok(Some(cached_path));
        }

        debug!("rust-analyzer not found");
        Ok(None)
    }

    async fn install_lsp(&self, cache_dir: &Path) -> PluginResult<PathBuf> {
        info!("Installing rust-analyzer from GitHub releases");

        let platform = Platform::current();
        let url = Self::get_download_url(&platform)?;
        let checksum = Self::get_checksum(&platform)?;

        // Download to temporary location
        let download_path = cache_dir.join("rust-analyzer.gz");
        download_file(&url, &download_path)
            .await
            .map_err(|e| PluginApiError::internal(format!("Download failed: {}", e)))?;

        // Verify checksum
        verify_checksum(&download_path, &checksum).map_err(|e| {
            PluginApiError::internal(format!("Checksum verification failed: {}", e))
        })?;

        // Decompress
        let binary_path = cache_dir.join("rust-analyzer");
        decompress_gzip(&download_path, &binary_path)
            .map_err(|e| PluginApiError::internal(format!("Decompression failed: {}", e)))?;

        // Make executable
        make_executable(&binary_path)
            .map_err(|e| PluginApiError::internal(format!("Failed to make executable: {}", e)))?;

        // Clean up compressed file
        std::fs::remove_file(&download_path).ok();

        info!("✅ Installed rust-analyzer to {:?}", binary_path);
        Ok(binary_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_download_url() {
        let platform = Platform::current();
        let url = RustLspInstaller::get_download_url(&platform);
        assert!(url.is_ok());
        assert!(url.unwrap().starts_with("https://github.com"));
    }

    #[test]
    fn test_get_checksum() {
        let platform = Platform::current();
        let checksum = RustLspInstaller::get_checksum(&platform);
        assert!(checksum.is_ok());
        assert_eq!(checksum.unwrap().len(), 64); // SHA256 is 64 hex chars
    }
}
