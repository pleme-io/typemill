//! Service for managing import updates across the codebase

use cb_plugin_api::PluginRegistry;
use codebuddy_ast::{find_project_files, update_imports_for_rename, ImportPathResolver};
use codebuddy_foundation::protocol::DependencyUpdate;
use codebuddy_foundation::protocol::{ApiError as ServerError, ApiResult as ServerResult};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tracing::{debug, info};

/// Service for managing import path updates
pub struct ImportService {
    /// Project root directory
    project_root: PathBuf,
    /// Language plugin registry for multi-language support
    plugin_registry: Arc<PluginRegistry>,
}

impl ImportService {
    /// Create a new import service with a custom plugin registry
    ///
    /// # Arguments
    ///
    /// * `project_root` - Root directory of the project
    /// * `plugin_registry` - Registry of language plugins to use
    pub fn new(project_root: impl AsRef<Path>, plugin_registry: Arc<PluginRegistry>) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
            plugin_registry,
        }
    }

    /// Update imports after a file rename
    ///
    /// Returns an EditPlan that should be applied via FileService.apply_edit_plan()
    pub async fn update_imports_for_rename(
        &self,
        old_path: &Path,
        new_path: &Path,
        rename_info: Option<&serde_json::Value>,
        dry_run: bool,
        scan_scope: Option<cb_plugin_api::ScanScope>,
    ) -> ServerResult<codebuddy_foundation::protocol::EditPlan> {
        info!(
            old_path = ?old_path,
            new_path = ?new_path,
            dry_run = dry_run,
            "Updating imports for rename"
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

        // Find and update imports using adapters
        debug!(
            old_abs = ?old_abs,
            new_abs = ?new_abs,
            project_root = ?self.project_root,
            dry_run = dry_run,
            has_rename_info = rename_info.is_some(),
            "Calling update_imports_for_rename"
        );
        let edit_plan = update_imports_for_rename(
            &old_abs,
            &new_abs,
            &self.project_root,
            self.plugin_registry.all(),
            rename_info,
            dry_run,
            scan_scope,
        )
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to update imports: {}", e)))?;

        debug!(
            edits_count = edit_plan.edits.len(),
            "update_imports_for_rename created EditPlan"
        );

        info!(
            edits = edit_plan.edits.len(),
            dry_run = dry_run,
            "Import update EditPlan created"
        );

        Ok(edit_plan)
    }

    /// Find all files that would be affected by a rename
    pub async fn find_affected_files(&self, file_path: &Path) -> ServerResult<Vec<PathBuf>> {
        let resolver = ImportPathResolver::new(&self.project_root);

        // Get all project files using adapters
        let project_files = find_project_files(&self.project_root, self.plugin_registry.all())
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to find project files: {}", e)))?;

        // Find files importing the target (pass plugins for plugin-aware detection)
        let affected = resolver
            .find_affected_files(file_path, &project_files, self.plugin_registry.all())
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to find affected files: {}", e)))?;

        Ok(affected)
    }

    /// Check if a file imports another file
    pub async fn check_import_dependency(
        &self,
        source_file: &Path,
        target_file: &Path,
    ) -> ServerResult<bool> {
        let content = tokio::fs::read_to_string(source_file)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to read file: {}", e)))?;

        let target_stem = target_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        // Simple check for import references
        Ok(content.contains(target_stem)
            && (content.contains("import") || content.contains("require")))
    }

    /// Update an import reference in a file by delegating to the appropriate language plugin.
    pub async fn update_import_reference(
        &self,
        file_path: &Path,
        update: &DependencyUpdate,
    ) -> ServerResult<bool> {
        let extension = match file_path.extension().and_then(|s| s.to_str()) {
            Some(ext) => ext,
            None => return Ok(false), // No extension, cannot determine language
        };

        let plugin = match self.plugin_registry.find_by_extension(extension) {
            Some(p) => p,
            None => {
                debug!("No plugin found for extension: {}", extension);
                return Ok(false);
            }
        };

        let import_advanced = match plugin.import_advanced_support() {
            Some(is) => is,
            None => {
                debug!(
                    "Plugin for {} does not support advanced import operations",
                    extension
                );
                return Ok(false);
            }
        };

        let content = match fs::read_to_string(file_path).await {
            Ok(c) => c,
            Err(_) => return Ok(false), // File not found
        };

        let original_content = content.clone();
        let updated_content = cb_plugin_api::ImportAdvancedSupport::update_import_reference(
            import_advanced,
            file_path,
            &content,
            update,
        )
        .map_err(|e| ServerError::Internal(format!("Failed to update import reference: {}", e)))?;

        if original_content == updated_content {
            return Ok(false); // No changes were made
        }

        fs::write(file_path, updated_content).await.map_err(|e| {
            ServerError::Internal(format!(
                "Failed to write updated content to {}: {}",
                file_path.display(),
                e
            ))
        })?;

        info!(
            file_path = %file_path.display(),
            old_ref = %update.old_reference,
            new_ref = %update.new_reference,
            "Successfully updated import reference via plugin"
        );

        Ok(true)
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
        let registry = Arc::new(PluginRegistry::new());
        let service = ImportService::new(temp_dir.path(), registry);

        assert_eq!(service.project_root, temp_dir.path());
    }
}
