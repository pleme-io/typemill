use crate::error::{AstError, AstResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::debug;

/// Cached information about a file's imports
#[derive(Debug, Clone)]
pub struct FileImportInfo {
    /// The files that this file imports
    pub imports: Vec<PathBuf>,
    /// Last modified time when this cache entry was created
    pub last_modified: std::time::SystemTime,
}

/// Resolves and updates import paths when files are moved or renamed
pub struct ImportPathResolver {
    /// Project root directory
    project_root: PathBuf,
    /// Cache of file import information for performance
    /// Maps file path -> (imports, last_modified_time)
    pub(crate) import_cache: Arc<Mutex<HashMap<PathBuf, FileImportInfo>>>,
}

impl ImportPathResolver {
    /// Create a new import path resolver
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
            import_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get the project root directory
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// Create a new resolver with a shared cache (for performance)
    pub fn with_cache(
        project_root: impl AsRef<Path>,
        cache: Arc<Mutex<HashMap<PathBuf, FileImportInfo>>>,
    ) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
            import_cache: cache,
        }
    }

    /// Clear the import cache
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.import_cache.lock() {
            cache.clear();
            debug!("Cleared import cache");
        }
    }

    /// Get cache statistics for monitoring
    pub fn cache_stats(&self) -> (usize, usize) {
        if let Ok(cache) = self.import_cache.lock() {
            let total = cache.len();
            let valid = cache
                .iter()
                .filter(|(path, info)| {
                    if let Ok(metadata) = std::fs::metadata(path) {
                        if let Ok(modified) = metadata.modified() {
                            return modified == info.last_modified;
                        }
                    }
                    false
                })
                .count();
            (total, valid)
        } else {
            (0, 0)
        }
    }

    /// Calculate the new import path after a file rename
    pub fn calculate_new_import_path(
        &self,
        importing_file: &Path,
        old_target_path: &Path,
        new_target_path: &Path,
        original_import: &str,
    ) -> AstResult<String> {
        // Handle different import styles
        if original_import.starts_with("./") || original_import.starts_with("../") {
            // Relative import - calculate new relative path
            self.calculate_relative_import(importing_file, new_target_path)
        } else if original_import.starts_with("@/") || original_import.starts_with("~/") {
            // Alias import - update the path after the alias
            self.update_alias_import(original_import, old_target_path, new_target_path)
        } else {
            // Absolute or package import - might not need updating
            Ok(original_import.to_string())
        }
    }

    /// Calculate relative import path between two files
    pub(crate) fn calculate_relative_import(
        &self,
        from_file: &Path,
        to_file: &Path,
    ) -> AstResult<String> {
        let from_dir = from_file
            .parent()
            .ok_or_else(|| AstError::parse("Invalid source file path"))?;

        let relative = pathdiff::diff_paths(to_file, from_dir)
            .ok_or_else(|| AstError::parse("Cannot calculate relative path"))?;

        // Remove extension for TypeScript/JavaScript imports
        let mut relative_str = relative.to_string_lossy().to_string();
        if let Some(ext) = to_file.extension() {
            let ext_str = ext.to_str().unwrap_or("");
            if matches!(ext_str, "ts" | "tsx" | "js" | "jsx") {
                relative_str = relative_str
                    .trim_end_matches(&format!(".{}", ext_str))
                    .to_string();
            }
        }

        // Ensure relative imports start with ./ or ../
        if !relative_str.starts_with("../") && !relative_str.starts_with("./") {
            relative_str = format!("./{}", relative_str);
        }

        // Convert backslashes to forward slashes for cross-platform compatibility
        Ok(relative_str.replace('\\', "/"))
    }

    /// Update alias-based imports
    fn update_alias_import(
        &self,
        original_import: &str,
        old_path: &Path,
        new_path: &Path,
    ) -> AstResult<String> {
        // Extract the alias prefix (e.g., "@/", "~/")
        let alias_end = original_import.find('/').unwrap_or(original_import.len());
        let alias = &original_import[..alias_end];

        // Get the path after the alias
        let path_after_alias = if alias_end < original_import.len() {
            &original_import[alias_end + 1..]
        } else {
            ""
        };

        // Check if the old path matches this import
        if old_path.to_string_lossy().contains(path_after_alias) {
            // Replace the old path component with the new one
            let new_path_str = new_path.to_string_lossy();
            let new_path_component =
                new_path_str.trim_start_matches(&self.project_root.to_string_lossy().to_string());
            Ok(format!("{}{}", alias, new_path_component))
        } else {
            Ok(original_import.to_string())
        }
    }
}
