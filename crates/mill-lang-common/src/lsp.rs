//! LSP installation utilities
//!
//! Shared primitives for language plugins to implement LSP server installation.
//! This module provides the foundational tools that each language plugin can use
//! to download, verify, and cache LSP server binaries.

use reqwest::Client;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Result type for LSP operations
pub type LspResult<T> = Result<T, LspError>;

/// Errors that can occur during LSP installation
#[derive(Debug, thiserror::Error)]
pub enum LspError {
    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Checksum verification failed: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Runtime not found: {0}")]
    RuntimeNotFound(String),

    #[error("Installation failed: {0}")]
    InstallationFailed(String),
}

/// Platform information for binary downloads
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Linux,
    MacOs,
    Windows,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    Aarch64,
}

impl Platform {
    /// Detect current platform
    pub fn current() -> Self {
        let os = if cfg!(target_os = "linux") {
            Os::Linux
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else if cfg!(target_os = "windows") {
            Os::Windows
        } else {
            panic!("Unsupported OS");
        };

        let arch = if cfg!(target_arch = "x86_64") {
            Arch::X86_64
        } else if cfg!(target_arch = "aarch64") {
            Arch::Aarch64
        } else {
            panic!("Unsupported architecture");
        };

        Self { os, arch }
    }

    pub fn os_string(&self) -> &'static str {
        match self.os {
            Os::Linux => "linux",
            Os::MacOs => "macos",
            Os::Windows => "windows",
        }
    }

    pub fn arch_string(&self) -> &'static str {
        match self.arch {
            Arch::X86_64 => "x86_64",
            Arch::Aarch64 => "aarch64",
        }
    }
}

/// Download a file from a URL with progress tracking
pub async fn download_file(url: &str, dest: &Path) -> LspResult<()> {
    // Security: Only allow HTTPS
    if !url.starts_with("https://") {
        return Err(LspError::DownloadFailed(
            "Only HTTPS URLs are allowed".to_string(),
        ));
    }

    // Security: Whitelist allowed hosts
    let allowed_hosts = ["github.com", "releases.rust-lang.org"];
    let host = url
        .strip_prefix("https://")
        .and_then(|s| s.split('/').next())
        .ok_or_else(|| LspError::DownloadFailed("Invalid URL".to_string()))?;

    if !allowed_hosts.contains(&host) {
        return Err(LspError::DownloadFailed(format!(
            "Host {} not in whitelist",
            host
        )));
    }

    info!("Downloading {} to {:?}", url, dest);

    let client = Client::builder()
        .timeout(Duration::from_secs(300))
        .build()?;

    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        return Err(LspError::DownloadFailed(format!(
            "HTTP {}: {}",
            response.status(),
            response.status().canonical_reason().unwrap_or("Unknown")
        )));
    }

    let bytes = response.bytes().await?;

    // Security: Size limit (200MB)
    const MAX_SIZE: usize = 200 * 1024 * 1024;
    let byte_len = bytes.len();
    if byte_len > MAX_SIZE {
        return Err(LspError::DownloadFailed(format!(
            "File too large: {} bytes (max: {})",
            byte_len, MAX_SIZE
        )));
    }

    // Create parent directory if it doesn't exist
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    tokio::fs::write(dest, bytes).await?;
    debug!("Downloaded {} bytes to {:?}", byte_len, dest);
    Ok(())
}

/// Verify file checksum
pub async fn verify_checksum(file_path: &Path, expected_checksum: &str) -> LspResult<()> {
    // Check env var bypass
    if env::var("TYPEMILL_SKIP_CHECKSUM_VERIFICATION")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
    {
        warn!("⚠️  CHECKSUM VERIFICATION DISABLED (TYPEMILL_SKIP_CHECKSUM_VERIFICATION=1)");
        warn!("   This is insecure and should only be used for development");
        return Ok(());
    }

    // Warn on placeholders
    if expected_checksum.starts_with("placeholder") {
        warn!("⚠️  Using placeholder checksum for {}", file_path.display());
        warn!("   Update with real SHA256 checksum for production use");
        return Ok(());
    }

    debug!("Verifying checksum for {:?}", file_path);
    let actual = sha256(file_path).await?;

    if actual != expected_checksum {
        return Err(LspError::ChecksumMismatch {
            expected: expected_checksum.to_string(),
            actual,
        });
    }

    debug!("Checksum verified: {}", actual);
    Ok(())
}

/// Calculate SHA256 checksum of a file
pub async fn sha256(path: &Path) -> LspResult<String> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let bytes = fs::read(&path)?;
        let hash = Sha256::digest(&bytes);
        Ok(format!("{:x}", hash))
    })
    .await
    .map_err(|e| LspError::InstallationFailed(format!("Task failed: {}", e)))?
}

/// Decompress a gzip file
pub async fn decompress_gzip(src: &Path, dest: &Path) -> LspResult<()> {
    let src = src.to_path_buf();
    let dest = dest.to_path_buf();

    tokio::task::spawn_blocking(move || {
        use flate2::read::GzDecoder;
        use std::io::copy;

        debug!("Decompressing {:?} to {:?}", src, dest);

        let file = fs::File::open(&src)?;
        let mut decoder = GzDecoder::new(file);

        // Create parent directory
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut out_file = fs::File::create(&dest)?;
        copy(&mut decoder, &mut out_file)?;

        debug!("Decompressed to {:?}", dest);
        Ok(())
    })
    .await
    .map_err(|e| LspError::InstallationFailed(format!("Task failed: {}", e)))?
}

/// Make a file executable (Unix only)
#[cfg(unix)]
pub async fn make_executable(path: &Path) -> LspResult<()> {
    use std::os::unix::fs::PermissionsExt;

    debug!("Making {:?} executable", path);
    let mut perms = tokio::fs::metadata(path).await?.permissions();
    perms.set_mode(0o755);
    tokio::fs::set_permissions(path, perms).await?;
    Ok(())
}

#[cfg(not(unix))]
pub async fn make_executable(_path: &Path) -> LspResult<()> {
    // No-op on non-Unix platforms
    Ok(())
}

/// Get the default cache directory for LSP binaries
pub fn get_cache_dir() -> PathBuf {
    // Use ~/.mill/lsp for cache
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());

    PathBuf::from(home).join(".mill").join("lsp")
}

/// Check if a binary exists in PATH
pub fn check_binary_in_path(name: &str) -> Option<PathBuf> {
    which::which(name).ok()
}

/// Install an npm package globally
pub async fn install_npm_package(package_name: &str, binary_name: &str) -> LspResult<PathBuf> {
    info!("Installing npm package: {}", package_name);

    // Check if npm is available
    if which::which("npm").is_err() {
        return Err(LspError::RuntimeNotFound(
            "npm (Node.js package manager)".to_string(),
        ));
    }

    // Run npm install -g
    let status = tokio::process::Command::new("npm")
        .args(["install", "-g", package_name])
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
pub async fn install_pip_package(package_name: &str, binary_name: &str) -> LspResult<PathBuf> {
    info!("Installing pip package: {}", package_name);

    // Try pipx first (handles PEP 668 environments), then fall back to pip
    if which::which("pipx").is_ok() {
        debug!("Using pipx for installation (PEP 668 compliant)");
        let status = tokio::process::Command::new("pipx")
            .args(["install", package_name])
            .status()
            .await
            .map_err(|e| LspError::DownloadFailed(format!("Failed to run pipx: {}", e)))?;

        if !status.success() {
            return Err(LspError::DownloadFailed(format!(
                "pipx install failed with exit code: {:?}",
                status.code()
            )));
        }
    } else {
        // Fall back to pip with --user flag
        let pip_cmd = if which::which("pip3").is_ok() {
            "pip3"
        } else if which::which("pip").is_ok() {
            "pip"
        } else {
            return Err(LspError::RuntimeNotFound(
                "pip, pip3, or pipx (Python package manager)".to_string(),
            ));
        };

        debug!("Using {} with --user flag", pip_cmd);

        // Try --user first, then with --break-system-packages if that fails (PEP 668)
        let mut status = tokio::process::Command::new(pip_cmd)
            .args(["install", "--user", package_name])
            .status()
            .await
            .map_err(|e| LspError::DownloadFailed(format!("Failed to run {}: {}", pip_cmd, e)))?;

        if !status.success() {
            warn!("pip install --user failed, trying with --break-system-packages");
            status = tokio::process::Command::new(pip_cmd)
                .args(["install", "--user", "--break-system-packages", package_name])
                .status()
                .await
                .map_err(|e| {
                    LspError::DownloadFailed(format!("Failed to run {}: {}", pip_cmd, e))
                })?;

            if !status.success() {
                return Err(LspError::DownloadFailed(
                    "pip install failed even with --break-system-packages. \
                     Consider installing pipx: apt install pipx OR pip install --user pipx"
                        .to_string(),
                ));
            }
        }
    }

    debug!("Python package installation completed");

    // Find the installed binary
    let binary_path = which::which(binary_name).map_err(|_| {
        LspError::DownloadFailed(format!(
            "Binary '{}' not found after installation. \
             Ensure Python bin directory is in PATH:\n\
             - For pip --user: ~/.local/bin (Linux/Mac) or %APPDATA%\\Python\\Scripts (Windows)\n\
             - For pipx: ~/.local/bin\n\
             Add to PATH and try again.",
            binary_name
        ))
    })?;

    info!("✅ Installed {} to {:?}", package_name, binary_path);
    Ok(binary_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = Platform::current();
        // Just verify it doesn't panic
        assert!(!platform.os_string().is_empty());
        assert!(!platform.arch_string().is_empty());
    }

    #[test]
    fn test_cache_dir() {
        let cache_dir = get_cache_dir();
        assert!(cache_dir.to_string_lossy().contains(".mill"));
    }

    #[test]
    fn test_checksum_bypass() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            // Test that env var bypass works
            env::set_var("TYPEMILL_SKIP_CHECKSUM_VERIFICATION", "1");
            let result = verify_checksum(Path::new("/nonexistent"), "placeholder").await;
            env::remove_var("TYPEMILL_SKIP_CHECKSUM_VERIFICATION");

            // Should succeed due to bypass
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_placeholder_checksum() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            // Test that placeholder checksums pass with warning
            let result = verify_checksum(Path::new("/nonexistent"), "placeholder_abc123").await;
            // Should succeed with warning
            assert!(result.is_ok());
        });
    }
}
