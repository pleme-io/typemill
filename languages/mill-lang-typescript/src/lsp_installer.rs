//! TypeScript LSP installer implementation

use async_trait::async_trait;
use mill_lang_common::lsp::{check_binary_in_path, install_npm_package};
use mill_plugin_api::{LspInstaller, PluginApiError, PluginResult};
use std::path::{Path, PathBuf};
use tracing::debug;

/// TypeScript LSP installer (typescript-language-server)
#[derive(Default)]
pub struct TypeScriptLspInstaller;

impl TypeScriptLspInstaller {
    pub const fn new() -> Self {
        Self
    }
}

#[async_trait]
impl LspInstaller for TypeScriptLspInstaller {
    fn lsp_name(&self) -> &str {
        "typescript-language-server"
    }

    fn check_installed(&self) -> PluginResult<Option<PathBuf>> {
        // TypeScript LSP is always installed via npm, so check PATH
        Ok(check_binary_in_path("typescript-language-server"))
    }

    async fn install_lsp(&self, _cache_dir: &Path) -> PluginResult<PathBuf> {
        debug!("Installing typescript-language-server via npm");

        install_npm_package("typescript-language-server", "typescript-language-server")
            .await
            .map_err(|e| PluginApiError::internal(format!("npm install failed: {}", e)))
    }
}

