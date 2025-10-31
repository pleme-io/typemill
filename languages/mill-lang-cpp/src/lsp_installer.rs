use mill_plugin_api::{lsp_installer::LspInstaller, PluginResult, PluginError};
use async_trait::async_trait;
use std::path::{Path, PathBuf};

pub struct CppLspInstaller;

#[async_trait]
impl LspInstaller for CppLspInstaller {
    fn lsp_name(&self) -> &str {
        "clangd"
    }

    fn check_installed(&self) -> PluginResult<Option<PathBuf>> {
        match which::which("clangd") {
            Ok(path) => Ok(Some(path)),
            Err(_) => Ok(None),
        }
    }

    async fn install_lsp(&self, _install_dir: &Path) -> PluginResult<PathBuf> {
        println!("Please install clangd using your system's package manager.");
        println!("For example:");
        println!("  - Debian/Ubuntu: sudo apt-get install clangd");
        println!("  - Fedora: sudo dnf install clangd");
        println!("  - Arch: sudo pacman -S clangd");
        println!("  - macOS: brew install llvm");
        // This is not a real installation, so we just return an error.
        Err(PluginError::not_supported("Automatic installation of clangd"))
    }
}