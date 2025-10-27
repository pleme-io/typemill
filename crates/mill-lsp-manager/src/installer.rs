//! Package manager installation for LSP servers

use crate::error::{LspError, Result};
use std::path::PathBuf;
use tracing::{debug, info};

/// Install an npm package globally
pub async fn install_npm_package(package_name: &str, binary_name: &str) -> Result<PathBuf> {
    info!("Installing npm package: {}", package_name);

    // Check if npm is available
    if which::which("npm").is_err() {
        return Err(LspError::RuntimeNotFound(
            "npm (Node.js package manager)".to_string(),
        ));
    }

    // Run npm install -g
    let status = tokio::process::Command::new("npm")
        .args(&["install", "-g", package_name])
        .status()
        .await
        .map_err(|e| LspError::DownloadFailed(format!("Failed to run npm: {}", e)))?;

    if !status.success() {
        return Err(LspError::DownloadFailed(format!(
            "npm install failed with exit code: {:?}",
            status.code()
        )));
    }

    debug!("npm install completed successfully");

    // Find the installed binary
    let binary_path = which::which(binary_name).map_err(|_| {
        LspError::DownloadFailed(format!(
            "Binary '{}' not found after npm install. Check npm global bin directory.",
            binary_name
        ))
    })?;

    info!("✅ Installed {} via npm to {:?}", package_name, binary_path);
    Ok(binary_path)
}

/// Install a pip package (user install, not global)
pub async fn install_pip_package(package_name: &str, binary_name: &str) -> Result<PathBuf> {
    info!("Installing pip package: {}", package_name);

    // Try pip3 first, then pip
    let pip_cmd = if which::which("pip3").is_ok() {
        "pip3"
    } else if which::which("pip").is_ok() {
        "pip"
    } else {
        return Err(LspError::RuntimeNotFound(
            "pip or pip3 (Python package manager)".to_string(),
        ));
    };

    // Run pip install --user (avoid requiring sudo)
    let status = tokio::process::Command::new(pip_cmd)
        .args(&["install", "--user", package_name])
        .status()
        .await
        .map_err(|e| LspError::DownloadFailed(format!("Failed to run {}: {}", pip_cmd, e)))?;

    if !status.success() {
        return Err(LspError::DownloadFailed(format!(
            "{} install failed with exit code: {:?}",
            pip_cmd,
            status.code()
        )));
    }

    debug!("{} install completed successfully", pip_cmd);

    // Find the installed binary
    let binary_path = which::which(binary_name).map_err(|_| {
        LspError::DownloadFailed(format!(
            "Binary '{}' not found after pip install. \
             Ensure Python user bin directory is in PATH (e.g., ~/.local/bin or %APPDATA%\\Python\\Scripts)",
            binary_name
        ))
    })?;

    info!("✅ Installed {} via pip to {:?}", package_name, binary_path);
    Ok(binary_path)
}

/// Get npm package name from LSP command name
pub fn get_npm_package_name(command: &str) -> &str {
    // Most LSP servers have same name as command
    // Add special cases as needed
    match command {
        "typescript-language-server" => "typescript-language-server",
        _ => command,
    }
}

/// Get pip package name from LSP command name
pub fn get_pip_package_name(command: &str) -> &str {
    // Map command names to Python package names
    match command {
        "pylsp" => "python-lsp-server",
        "pyls" => "python-language-server", // Legacy name
        _ => command,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_name_mapping() {
        assert_eq!(get_npm_package_name("typescript-language-server"), "typescript-language-server");
        assert_eq!(get_pip_package_name("pylsp"), "python-lsp-server");
    }

    #[test]
    fn test_npm_not_available() {
        // This test assumes npm is not available in test environment
        // If npm is installed, it will actually try to install
        // In CI, we can control this with env vars
    }
}
