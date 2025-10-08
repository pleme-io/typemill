//! File system operation utilities
//!
//! Provides standardized async file operations with consistent error handling
//! for language plugin implementations.

use cb_plugin_api::{PluginError, PluginResult};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::debug;

/// Read a manifest file with standardized error handling
///
/// # Example
///
/// ```rust,ignore
/// use cb_lang_common::io::read_manifest;
///
/// let content = read_manifest(Path::new("Cargo.toml")).await?;
/// ```
pub async fn read_manifest(path: &Path) -> PluginResult<String> {
    debug!(path = %path.display(), "Reading manifest file");

    fs::read_to_string(path).await.map_err(|e| {
        PluginError::manifest(format!("Failed to read manifest {}: {}", path.display(), e))
    })
}

/// Read a source file with standardized error handling
pub async fn read_source(path: &Path) -> PluginResult<String> {
    debug!(path = %path.display(), "Reading source file");

    fs::read_to_string(path).await.map_err(|e| {
        PluginError::internal(format!("Failed to read file {}: {}", path.display(), e))
    })
}

/// Find all source files recursively with extension filtering
///
/// # Arguments
///
/// * `dir` - Root directory to search
/// * `extensions` - Array of file extensions to match (without leading dot)
///
/// # Example
///
/// ```rust,ignore
/// let rust_files = find_source_files(root, &["rs"]).await?;
/// let web_files = find_source_files(root, &["ts", "tsx", "js", "jsx"]).await?;
/// ```
pub async fn find_source_files(dir: &Path, extensions: &[&str]) -> PluginResult<Vec<PathBuf>> {
    debug!(
        dir = %dir.display(),
        extensions = ?extensions,
        "Finding source files"
    );

    let mut result = Vec::new();
    let mut queue = vec![dir.to_path_buf()];

    while let Some(current_dir) = queue.pop() {
        let mut entries = fs::read_dir(&current_dir).await.map_err(|e| {
            PluginError::internal(format!(
                "Failed to read directory {}: {}",
                current_dir.display(),
                e
            ))
        })?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| PluginError::internal(format!("Failed to read entry: {}", e)))?
        {
            let path = entry.path();
            let metadata = entry
                .metadata()
                .await
                .map_err(|e| PluginError::internal(format!("Failed to get metadata: {}", e)))?;

            if metadata.is_dir() {
                queue.push(path);
            } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if extensions.contains(&ext) {
                    result.push(path);
                }
            }
        }
    }

    debug!(files_found = result.len(), "Source file search complete");
    Ok(result)
}

/// Check if a file exists
pub async fn file_exists(path: &Path) -> bool {
    fs::metadata(path).await.is_ok()
}

/// Check if a directory exists
pub async fn dir_exists(path: &Path) -> bool {
    fs::metadata(path)
        .await
        .map(|m| m.is_dir())
        .unwrap_or(false)
}

/// Create a directory and all parent directories if they don't exist
pub async fn ensure_dir(path: &Path) -> PluginResult<()> {
    if !dir_exists(path).await {
        fs::create_dir_all(path).await.map_err(|e| {
            PluginError::internal(format!(
                "Failed to create directory {}: {}",
                path.display(),
                e
            ))
        })?;
    }
    Ok(())
}

/// Normalize a module path relative to project root
///
/// Converts absolute or relative paths to normalized form for import statements.
///
/// # Example
///
/// ```rust,ignore
/// let normalized = normalize_module_path(
///     Path::new("/project/src/utils/helpers.rs"),
///     Path::new("/project")
/// );
/// assert_eq!(normalized, "src/utils/helpers.rs");
/// ```
pub fn normalize_module_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

/// Get the relative path from one file to another
///
/// Used for calculating relative import paths.
pub fn relative_path(from: &Path, to: &Path) -> PathBuf {
    pathdiff::diff_paths(to, from.parent().unwrap_or(from)).unwrap_or_else(|| to.to_path_buf())
}

/// Convert a file path to a module path using given separator
///
/// Automatically strips any file extension, making this work with any language.
///
/// # Example
///
/// ```rust,ignore
/// let module = file_path_to_module(
///     Path::new("src/utils/helpers.rs"),
///     "::"
/// );
/// assert_eq!(module, "src::utils::helpers");
/// ```
pub fn file_path_to_module(path: &Path, separator: &str) -> String {
    // Strip extension if present by reconstructing path from parent + stem
    let without_ext = if let Some(stem) = path.file_stem() {
        if let Some(parent) = path.parent() {
            let parent_str = parent.to_string_lossy();
            if parent_str.is_empty() || parent_str == "." {
                stem.to_string_lossy().to_string()
            } else {
                format!("{}/{}", parent_str, stem.to_string_lossy())
            }
        } else {
            stem.to_string_lossy().to_string()
        }
    } else {
        path.to_string_lossy().to_string()
    };

    // Replace path separators with module separator
    without_ext.replace(['/', '\\'], separator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_read_manifest_success() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("Cargo.toml");

        let mut file = File::create(&manifest_path).await.unwrap();
        file.write_all(b"[package]\nname = \"test\"").await.unwrap();
        file.flush().await.unwrap();
        drop(file); // Ensure file is closed

        let content = read_manifest(&manifest_path).await.unwrap();
        assert!(content.contains("name = \"test\""));
    }

    #[tokio::test]
    async fn test_read_manifest_not_found() {
        let result = read_manifest(Path::new("/nonexistent/Cargo.toml")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_source_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        File::create(temp_dir.path().join("main.rs")).await.unwrap();
        File::create(temp_dir.path().join("lib.rs")).await.unwrap();
        File::create(temp_dir.path().join("readme.md"))
            .await
            .unwrap();

        let files = find_source_files(temp_dir.path(), &["rs"]).await.unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|p| p.extension().unwrap() == "rs"));
    }

    #[tokio::test]
    async fn test_file_exists() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        assert!(!file_exists(&file_path).await);

        File::create(&file_path).await.unwrap();
        assert!(file_exists(&file_path).await);
    }

    #[tokio::test]
    async fn test_ensure_dir() {
        let temp_dir = TempDir::new().unwrap();
        let nested_dir = temp_dir.path().join("a/b/c");

        ensure_dir(&nested_dir).await.unwrap();
        assert!(dir_exists(&nested_dir).await);
    }

    #[test]
    fn test_file_path_to_module() {
        assert_eq!(
            file_path_to_module(Path::new("src/utils/helpers.rs"), "::"),
            "src::utils::helpers"
        );

        assert_eq!(
            file_path_to_module(Path::new("src/main.py"), "."),
            "src.main"
        );
    }

    #[test]
    fn test_normalize_module_path() {
        let normalized =
            normalize_module_path(Path::new("/project/src/main.rs"), Path::new("/project"));
        assert_eq!(normalized, "src/main.rs");
    }
}
