//! LSP installer for Eclipse JDT Language Server (jdtls)
//!
//! This module provides functionality for detecting and installing
//! the Eclipse JDT Language Server for Java language support.

use async_trait::async_trait;
use mill_plugin_api::{LspInstaller, PluginApiError, PluginResult};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::debug;

/// Java LSP installer for Eclipse JDT LS
#[derive(Default, Clone)]
pub struct JavaLspInstaller;

#[async_trait]
impl LspInstaller for JavaLspInstaller {
    fn lsp_name(&self) -> &str {
        "jdtls"
    }

    fn check_installed(&self) -> PluginResult<Option<PathBuf>> {
        debug!("Checking for jdtls installation");

        // Check standard locations in order of preference
        let check_locations = vec![
            // 1. Check PATH
            check_in_path(),
            // 2. Check common LSP manager locations
            check_mason_installation(),
            // 3. Check mill's own cache
            check_mill_cache(),
            // 4. Check system-wide installations
            check_system_installation(),
        ];

        if let Some(path) = check_locations.into_iter().flatten().next() {
            debug!(path = %path.display(), "Found jdtls installation");
            return Ok(Some(path));
        }

        debug!("jdtls not found in any standard location");
        Ok(None)
    }

    async fn install_lsp(&self, cache_dir: &Path) -> PluginResult<PathBuf> {
        debug!(cache_dir = %cache_dir.display(), "Installing jdtls");

        // Create installation directory
        let install_dir = cache_dir.join("jdtls");
        std::fs::create_dir_all(&install_dir).map_err(|e| {
            PluginApiError::internal(format!("Failed to create install directory: {}", e))
        })?;

        // For now, return instructions for manual installation
        // Automatic installation would require downloading from GitHub releases
        // and extracting the archive
        let instructions = format!(
            r#"Eclipse JDT Language Server (jdtls) installation required.

Please install jdtls using one of these methods:

1. Using mason.nvim (Neovim users):
   :MasonInstall jdtls

2. Using VSCode:
   Install the "Language Support for Java(TM) by Red Hat" extension

3. Manual download:
   - Download from: https://github.com/eclipse/eclipse.jdt.ls
   - Extract to: {}
   - Make sure the launcher script is executable

4. Using system package manager:
   - Arch Linux: pacman -S jdtls
   - Homebrew: brew install jdtls

After installation, jdtls should be available in your PATH."#,
            install_dir.display()
        );

        Err(PluginApiError::not_supported(instructions))
    }
}

/// Check if jdtls is in the system PATH
fn check_in_path() -> Option<PathBuf> {
    // Try to find jdtls in PATH
    if let Ok(output) = Command::new("which").arg("jdtls").output() {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() {
                return Some(PathBuf::from(path_str));
            }
        }
    }

    // Windows alternative
    if cfg!(target_os = "windows") {
        if let Ok(output) = Command::new("where").arg("jdtls").output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    // Take the first line (where can return multiple results)
                    if let Some(first_path) = path_str.lines().next() {
                        return Some(PathBuf::from(first_path));
                    }
                }
            }
        }
    }

    None
}

/// Check mason.nvim installation location
fn check_mason_installation() -> Option<PathBuf> {
    // Mason.nvim typically installs to ~/.local/share/nvim/mason/bin/jdtls
    if let Some(home) = std::env::var_os("HOME") {
        let mason_path = PathBuf::from(home).join(".local/share/nvim/mason/bin/jdtls");
        if mason_path.exists() {
            return Some(mason_path);
        }
    }

    // XDG_DATA_HOME alternative
    if let Some(xdg_data) = std::env::var_os("XDG_DATA_HOME") {
        let mason_path = PathBuf::from(xdg_data).join("nvim/mason/bin/jdtls");
        if mason_path.exists() {
            return Some(mason_path);
        }
    }

    None
}

/// Check mill's own cache directory
fn check_mill_cache() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("HOME") {
        let cache_path = PathBuf::from(home).join(".mill/lsp/jdtls/jdtls");
        if cache_path.exists() {
            return Some(cache_path);
        }
    }
    None
}

/// Check system-wide installation locations
fn check_system_installation() -> Option<PathBuf> {
    let system_locations = vec![
        PathBuf::from("/usr/local/bin/jdtls"),
        PathBuf::from("/usr/bin/jdtls"),
        PathBuf::from("/opt/jdtls/bin/jdtls"),
    ];

    system_locations
        .into_iter()
        .find(|location| location.exists())
}

