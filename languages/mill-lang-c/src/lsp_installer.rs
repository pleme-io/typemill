use mill_plugin_api::{LspInstaller, PluginError, PluginResult};
use async_trait::async_trait;
use std::path::{Path, PathBuf};

pub struct CLspInstaller;

#[async_trait]
impl LspInstaller for CLspInstaller {
    fn lsp_name(&self) -> &str {
        "clangd"
    }

    fn check_installed(&self) -> PluginResult<Option<PathBuf>> {
        if let Ok(path) = which::which("clangd") {
            Ok(Some(path))
        } else {
            Ok(None)
        }
    }

    async fn install_lsp(&self, _install_dir: &Path) -> PluginResult<PathBuf> {
        Err(PluginError::not_supported(
            "Automatic installation of clangd is not supported. Please install it using your system's package manager."
        ))
    }
}