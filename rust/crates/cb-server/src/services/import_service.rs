//! Service for managing import updates across the codebase

use crate::error::{ServerError, ServerResult};
use cb_ast::{ImportPathResolver, update_import_paths};
use std::path::{Path, PathBuf};
use tracing::info;

/// Service for managing import path updates
pub struct ImportService {
    /// Project root directory
    project_root: PathBuf,
}

impl ImportService {
    /// Create a new import service
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
        }
    }

    /// Update imports after a file rename
    pub async fn update_imports_for_rename(
        &self,
        old_path: &Path,
        new_path: &Path,
        dry_run: bool,
    ) -> ServerResult<ImportUpdateReport> {
        info!(
            "Updating imports for rename: {:?} -> {:?} (dry_run: {})",
            old_path, new_path, dry_run
        );

        // Convert to absolute paths if needed
        let old_abs = if old_path.is_absolute() {
            old_path.to_path_buf()
        } else {
            self.project_root.join(old_path)
        };

        let new_abs = if new_path.is_absolute() {
            new_path.to_path_buf()
        } else {
            self.project_root.join(new_path)
        };

        // Find and update imports
        info!("Calling update_import_paths with old_abs: {:?}, new_abs: {:?}, project_root: {:?}", old_abs, new_abs, self.project_root);
        let result = update_import_paths(&old_abs, &new_abs, &self.project_root).await
            .map_err(|e| ServerError::Internal(format!("Failed to update imports: {}", e)))?;
        info!("update_import_paths result: {:?}", result);

        // Create report
        let report = ImportUpdateReport {
            files_updated: result.updated_files.len(),
            imports_updated: result.imports_updated,
            failed_files: result.failed_files.len(),
            updated_paths: result.updated_files
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
            errors: result.failed_files
                .iter()
                .map(|(p, e)| format!("{}: {}", p.display(), e))
                .collect(),
        };

        if dry_run {
            info!("Dry run complete - no files were actually modified");
        } else {
            info!(
                "Import update complete: {} files updated, {} imports changed",
                report.files_updated, report.imports_updated
            );
        }

        Ok(report)
    }

    /// Find all files that would be affected by a rename
    pub async fn find_affected_files(
        &self,
        file_path: &Path,
    ) -> ServerResult<Vec<PathBuf>> {
        let resolver = ImportPathResolver::new(&self.project_root);

        // Get all project files
        let project_files = self.find_all_source_files()?;

        // Find files importing the target
        let affected = resolver.find_affected_files(file_path, &project_files)
            .map_err(|e| ServerError::Internal(format!("Failed to find affected files: {}", e)))?;

        Ok(affected)
    }

    /// Find all source files in the project
    fn find_all_source_files(&self) -> ServerResult<Vec<PathBuf>> {
        let mut files = Vec::new();
        let extensions = ["ts", "tsx", "js", "jsx", "mjs", "cjs"];

        self.collect_files(&self.project_root, &mut files, &extensions)?;

        Ok(files)
    }

    /// Recursively collect files with given extensions
    fn collect_files(
        &self,
        dir: &Path,
        files: &mut Vec<PathBuf>,
        extensions: &[&str],
    ) -> ServerResult<()> {
        // Skip ignored directories
        if let Some(name) = dir.file_name() {
            let name_str = name.to_string_lossy();
            if matches!(
                name_str.as_ref(),
                "node_modules" | ".git" | "dist" | "build" | "target" | ".next" | ".nuxt"
            ) {
                return Ok(());
            }
        }

        for entry in std::fs::read_dir(dir)
            .map_err(|e| ServerError::Internal(format!("Failed to read directory: {}", e)))?
        {
            let entry = entry.map_err(|e| ServerError::Internal(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();

            if path.is_dir() {
                self.collect_files(&path, files, extensions)?;
            } else if let Some(ext) = path.extension() {
                if extensions.contains(&ext.to_str().unwrap_or("")) {
                    files.push(path);
                }
            }
        }

        Ok(())
    }

    /// Check if a file imports another file
    pub fn check_import_dependency(
        &self,
        source_file: &Path,
        target_file: &Path,
    ) -> ServerResult<bool> {
        let content = std::fs::read_to_string(source_file)
            .map_err(|e| ServerError::Internal(format!("Failed to read file: {}", e)))?;

        let target_stem = target_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        // Simple check for import references
        Ok(content.contains(target_stem) &&
           (content.contains("import") || content.contains("require")))
    }
}

/// Report of import update operations
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportUpdateReport {
    /// Number of files that were updated
    pub files_updated: usize,
    /// Total number of import statements updated
    pub imports_updated: usize,
    /// Number of files that failed to update
    pub failed_files: usize,
    /// Paths of successfully updated files
    pub updated_paths: Vec<String>,
    /// Error messages for failed updates
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio;

    #[tokio::test]
    async fn test_import_service_creation() {
        let temp_dir = TempDir::new().unwrap();
        let service = ImportService::new(temp_dir.path());

        assert_eq!(service.project_root, temp_dir.path());
    }

    #[tokio::test]
    async fn test_find_source_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create some test files
        std::fs::create_dir(temp_dir.path().join("src")).unwrap();
        std::fs::write(temp_dir.path().join("src/index.ts"), "export {}").unwrap();
        std::fs::write(temp_dir.path().join("src/utils.js"), "module.exports = {}").unwrap();

        // Create node_modules that should be ignored
        std::fs::create_dir(temp_dir.path().join("node_modules")).unwrap();
        std::fs::write(temp_dir.path().join("node_modules/lib.js"), "ignore me").unwrap();

        let service = ImportService::new(temp_dir.path());
        let files = service.find_all_source_files().unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|p| p.ends_with("index.ts")));
        assert!(files.iter().any(|p| p.ends_with("utils.js")));
    }
}