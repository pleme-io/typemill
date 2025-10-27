//! LSP Installer capability trait
//!
//! This trait allows language plugins to implement custom LSP installation logic.
//! Each plugin decides how to install its corresponding LSP server
//! (direct download, package manager, etc.)

use crate::PluginResult;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

/// LSP Installer capability
///
/// Plugins implement this trait to provide custom LSP installation logic.
/// The trait uses the Strategy pattern - each language decides:
/// - Whether the LSP is already installed
/// - How to install it (npm, pip, direct download, etc.)
/// - Where to cache the binary
///
/// # Example Implementation
///
/// ```rust,ignore
/// use mill_plugin_api::LspInstaller;
/// use mill_lang_common::lsp::{check_binary_in_path, install_npm_package};
///
/// #[async_trait]
/// impl LspInstaller for TypeScriptPlugin {
///     fn lsp_name(&self) -> &str {
///         "typescript-language-server"
///     }
///
///     fn check_installed(&self) -> PluginResult<Option<PathBuf>> {
///         Ok(check_binary_in_path("typescript-language-server"))
///     }
///
///     async fn install_lsp(&self, _cache_dir: &Path) -> PluginResult<PathBuf> {
///         install_npm_package("typescript-language-server", "typescript-language-server")
///             .await
///             .map_err(|e| PluginError::internal(e.to_string()))
///     }
/// }
/// ```
#[async_trait]
pub trait LspInstaller: Send + Sync {
    /// Get the name of the LSP server
    ///
    /// This is the binary name or package name for the LSP.
    /// Examples: "rust-analyzer", "typescript-language-server", "pylsp"
    fn lsp_name(&self) -> &str;

    /// Get the LSP version to install
    ///
    /// Default implementation returns "latest". Plugins can override
    /// to pin to specific versions.
    fn lsp_version(&self) -> &str {
        "latest"
    }

    /// Check if the LSP is already installed
    ///
    /// Returns the path to the installed binary if found, None otherwise.
    /// Implementations should check:
    /// 1. System PATH (via `which`)
    /// 2. Cache directory
    /// 3. Language-specific install locations
    ///
    /// # Returns
    ///
    /// - `Ok(Some(path))` if installed and available
    /// - `Ok(None)` if not installed
    /// - `Err(...)` if check failed (permissions, etc.)
    fn check_installed(&self) -> PluginResult<Option<PathBuf>>;

    /// Install the LSP server
    ///
    /// Implementations should:
    /// 1. Download/install the LSP binary
    /// 2. Verify checksums if applicable
    /// 3. Place binary in cache directory or system location
    /// 4. Make binary executable (Unix)
    /// 5. Return path to installed binary
    ///
    /// # Arguments
    ///
    /// * `cache_dir` - Directory to store downloaded binaries (e.g., ~/.mill/lsp)
    ///
    /// # Returns
    ///
    /// Path to the installed LSP binary
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Download fails
    /// - Checksum verification fails
    /// - Installation command fails (npm, pip, etc.)
    /// - Binary not found after installation
    async fn install_lsp(&self, cache_dir: &Path) -> PluginResult<PathBuf>;

    /// Ensure LSP is installed (convenience method)
    ///
    /// Checks if already installed, installs if not, returns path.
    /// This is the main entry point for consumers.
    ///
    /// # Arguments
    ///
    /// * `cache_dir` - Directory to store downloaded binaries
    ///
    /// # Returns
    ///
    /// Path to the LSP binary (installed or existing)
    async fn ensure_installed(&self, cache_dir: &Path) -> PluginResult<PathBuf> {
        if let Some(path) = self.check_installed()? {
            tracing::debug!(
                lsp = self.lsp_name(),
                path = ?path,
                "LSP already installed"
            );
            return Ok(path);
        }

        tracing::info!(lsp = self.lsp_name(), "Installing LSP server");
        self.install_lsp(cache_dir).await
    }
}
