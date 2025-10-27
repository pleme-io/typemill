//! LSP cache management (~/.mill/lsp/)

use crate::error::{LspError, Result};
use std::path::PathBuf;
use tracing::{debug, info};

/// Get the LSP cache directory (~/.mill/lsp/)
pub fn cache_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| {
        LspError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine home directory",
        ))
    })?;

    Ok(home.join(".mill").join("lsp"))
}

/// Get the path for a specific LSP binary
pub fn lsp_binary_path(lsp_name: &str) -> Result<PathBuf> {
    // Security: Prevent directory traversal
    if lsp_name.contains("..") || lsp_name.contains('/') || lsp_name.contains('\\') {
        return Err(LspError::InvalidLspName(lsp_name.to_string()));
    }

    let dir = cache_dir()?;
    Ok(dir.join(lsp_name))
}

/// Check if an LSP is cached
pub fn is_cached(lsp_name: &str) -> bool {
    lsp_binary_path(lsp_name)
        .map(|path| path.exists())
        .unwrap_or(false)
}

/// Initialize cache directory
pub async fn init_cache() -> Result<()> {
    let dir = cache_dir()?;

    if !dir.exists() {
        info!("Creating LSP cache directory: {}", dir.display());
        tokio::fs::create_dir_all(&dir).await?;
    }

    Ok(())
}

/// Clean up old or unused LSP binaries
pub async fn cleanup_cache() -> Result<Vec<PathBuf>> {
    let dir = cache_dir()?;
    let removed = Vec::new();

    if !dir.exists() {
        return Ok(removed);
    }

    // For now, just return empty list
    // Future: implement version tracking and cleanup
    debug!("Cache cleanup not yet implemented");

    Ok(removed)
}

/// Get cache statistics
pub async fn cache_stats() -> Result<CacheStats> {
    let dir = cache_dir()?;

    if !dir.exists() {
        return Ok(CacheStats {
            total_size: 0,
            lsp_count: 0,
        });
    }

    let mut total_size = 0u64;
    let mut lsp_count = 0;

    let mut entries = tokio::fs::read_dir(&dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_file() {
            if let Ok(metadata) = entry.metadata().await {
                total_size += metadata.len();
                lsp_count += 1;
            }
        }
    }

    Ok(CacheStats {
        total_size,
        lsp_count,
    })
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_size: u64,
    pub lsp_count: usize,
}

impl CacheStats {
    pub fn total_size_mb(&self) -> f64 {
        self.total_size as f64 / (1024.0 * 1024.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_dir() {
        let dir = cache_dir().unwrap();
        assert!(dir.ends_with(".mill/lsp"));
    }

    #[test]
    fn test_lsp_binary_path() {
        let path = lsp_binary_path("rust-analyzer").unwrap();
        assert!(path.ends_with("rust-analyzer"));
    }

    #[test]
    fn test_path_traversal_prevention() {
        assert!(lsp_binary_path("../etc/passwd").is_err());
        assert!(lsp_binary_path("subdir/file").is_err());
        assert!(lsp_binary_path("normal-name").is_ok());
    }
}
