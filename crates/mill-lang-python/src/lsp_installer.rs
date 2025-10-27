//! Python LSP installer implementation

use async_trait::async_trait;
use mill_lang_common::lsp::{check_binary_in_path, install_pip_package};
use mill_plugin_api::{LspInstaller, PluginError, PluginResult};
use std::path::{Path, PathBuf};
use tracing::debug;

/// Python LSP installer (pylsp)
#[derive(Default)]
pub struct PythonLspInstaller;

impl PythonLspInstaller {
    pub const fn new() -> Self {
        Self
    }
}

#[async_trait]
impl LspInstaller for PythonLspInstaller {
    fn lsp_name(&self) -> &str {
        "pylsp"
    }

    fn check_installed(&self) -> PluginResult<Option<PathBuf>> {
        // Python LSP is installed via pip/pipx, so check PATH
        Ok(check_binary_in_path("pylsp"))
    }

    async fn install_lsp(&self, _cache_dir: &Path) -> PluginResult<PathBuf> {
        debug!("Installing pylsp via pip/pipx");

        install_pip_package("python-lsp-server", "pylsp")
            .await
            .map_err(|e| PluginError::internal(format!("pip install failed: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_installed() {
        let installer = PythonLspInstaller::new();
        // Should not error, might return Some or None depending on system
        let result = installer.check_installed();
        assert!(result.is_ok());
    }

    #[test]
    fn test_lsp_name() {
        let installer = PythonLspInstaller::new();
        assert_eq!(installer.lsp_name(), "pylsp");
    }
}
