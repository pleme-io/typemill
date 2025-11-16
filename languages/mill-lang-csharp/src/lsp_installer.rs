//! LSP Installer for the C# language server (csharp-ls)

use async_trait::async_trait;
use mill_plugin_api::{LspInstaller, PluginApiError, PluginResult};
use std::path::PathBuf;
use tokio::process::Command as TokioCommand;
use tracing::{error, info};

#[derive(Default)]
pub struct CsharpLspInstaller;

impl CsharpLspInstaller {
    /// Creates a new C# LSP installer instance.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl LspInstaller for CsharpLspInstaller {
    fn lsp_name(&self) -> &str {
        "csharp-ls"
    }

    fn check_installed(&self) -> PluginResult<Option<PathBuf>> {
        which::which("csharp-ls")
            .map(Some)
            .map_err(|e| PluginApiError::internal(format!("csharp-ls not found: {}", e)))
    }

    async fn install_lsp(&self, _install_dir: &std::path::Path) -> PluginResult<PathBuf> {
        info!("Installing csharp-ls via dotnet tool...");
        let output = TokioCommand::new("dotnet")
            .args(["tool", "install", "--global", "csharp-ls"])
            .output()
            .await
            .map_err(|e| {
                PluginApiError::internal(format!("Failed to execute dotnet command: {}", e))
            })?;

        if output.status.success() {
            info!("csharp-ls installed successfully.");
            // After installation, find the path
            self.check_installed()?.ok_or_else(|| {
                PluginApiError::internal("Failed to find csharp-ls after installation.".to_string())
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Failed to install csharp-ls: {}", stderr);
            Err(PluginApiError::internal(format!(
                "Failed to install csharp-ls: {}",
                stderr
            )))
        }
    }
}
