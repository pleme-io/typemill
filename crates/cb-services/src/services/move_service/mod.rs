//! Move Service - Unified planning logic for file and directory moves/renames
//!
//! This service provides the single source of truth for all move and rename operations.
//! It handles:
//! - File moves/renames with import updates
//! - Directory moves/renames with import updates
//! - Cargo package detection and manifest updates
//! - Workspace member updates
//! - Dependent crate path updates
//!
//! # Architecture
//!
//! The MoveService is used by both:
//! - Move handlers (MCP protocol) - convert EditPlan → MovePlan
//! - Rename operations (internal) - use EditPlan directly
//!
//! This eliminates duplication and ensures consistent behavior.

mod planner;

use crate::services::reference_updater::ReferenceUpdater;
use cb_plugin_api::{PluginRegistry, ScanScope};
use codebuddy_foundation::protocol::{
    ApiError as ServerError, ApiResult as ServerResult, EditPlan,
};
use std::path::{Path, PathBuf};
use tracing::info;

/// Unified move/rename planning service
pub struct MoveService<'a> {
    /// Reference updater for import analysis
    reference_updater: &'a ReferenceUpdater,
    /// Language plugin registry
    plugin_registry: &'a PluginRegistry,
    /// Project root directory
    project_root: &'a Path,
}

impl<'a> MoveService<'a> {
    /// Create a new MoveService
    pub fn new(
        reference_updater: &'a ReferenceUpdater,
        plugin_registry: &'a PluginRegistry,
        project_root: &'a Path,
    ) -> Self {
        Self {
            reference_updater,
            plugin_registry,
            project_root,
        }
    }

    /// Plan a file move/rename with import updates
    ///
    /// Returns an EditPlan containing all necessary changes:
    /// - Import/use statement updates
    /// - Module declaration updates
    /// - Qualified path updates
    pub async fn plan_file_move(
        &self,
        old_path: &Path,
        new_path: &Path,
        scan_scope: Option<ScanScope>,
    ) -> ServerResult<EditPlan> {
        info!(
            old_path = %old_path.display(),
            new_path = %new_path.display(),
            "Planning file move"
        );

        let old_abs = self.to_absolute_path(old_path);
        let new_abs = self.to_absolute_path(new_path);

        // Validate source file exists
        if !old_abs.exists() {
            return Err(ServerError::NotFound(format!(
                "Source file does not exist: {}",
                old_abs.display()
            )));
        }

        // Plan import/reference updates
        planner::plan_file_move(
            &old_abs,
            &new_abs,
            self.reference_updater,
            self.plugin_registry,
            scan_scope,
            None, // No RenameScope - use default behavior
        )
        .await
    }

    /// Plan a directory move/rename with import updates
    ///
    /// Returns an EditPlan containing all necessary changes:
    /// - Import/use statement updates for all files in directory
    /// - Module declaration updates
    /// - Cargo.toml manifest updates (if Cargo package)
    /// - Workspace member updates (if Cargo package)
    /// - Dependent crate path updates (if Cargo package)
    pub async fn plan_directory_move(
        &self,
        old_path: &Path,
        new_path: &Path,
        scan_scope: Option<ScanScope>,
    ) -> ServerResult<EditPlan> {
        info!(
            old_path = %old_path.display(),
            new_path = %new_path.display(),
            "Planning directory move"
        );

        let old_abs = self.to_absolute_path(old_path);
        let new_abs = self.to_absolute_path(new_path);

        // Validate source directory exists
        if !old_abs.exists() {
            return Err(ServerError::NotFound(format!(
                "Source directory does not exist: {}",
                old_abs.display()
            )));
        }

        if !old_abs.is_dir() {
            return Err(ServerError::InvalidRequest(format!(
                "Path is not a directory: {}",
                old_abs.display()
            )));
        }

        // Plan directory move with Cargo package support
        planner::plan_directory_move(
            &old_abs,
            &new_abs,
            self.reference_updater,
            self.plugin_registry,
            self.project_root,
            scan_scope,
            None, // No RenameScope - use default behavior
        )
        .await
    }

    /// Plan a file move/rename with RenameScope filtering
    ///
    /// Wrapper that accepts RenameScope for file filtering
    pub async fn plan_file_move_with_scope(
        &self,
        old_path: &Path,
        new_path: &Path,
        rename_scope: Option<&codebuddy_foundation::core::rename_scope::RenameScope>,
    ) -> ServerResult<EditPlan> {
        info!(
            old_path = %old_path.display(),
            new_path = %new_path.display(),
            "Planning file move with scope"
        );

        let old_abs = self.to_absolute_path(old_path);
        let new_abs = self.to_absolute_path(new_path);

        // Validate source file exists
        if !old_abs.exists() {
            return Err(ServerError::NotFound(format!(
                "Source file does not exist: {}",
                old_abs.display()
            )));
        }

        // Choose ScanScope based on RenameScope settings
        // Use ScanScope::All if string literals should be updated
        let scan_scope = if let Some(scope) = rename_scope {
            if scope.update_string_literals || scope.update_comments {
                Some(ScanScope::All)
            } else {
                Some(ScanScope::AllUseStatements)
            }
        } else {
            // Default: comprehensive scanning
            Some(ScanScope::All)
        };

        // Call planner directly to pass RenameScope
        let mut edit_plan = planner::plan_file_move(
            &old_abs,
            &new_abs,
            self.reference_updater,
            self.plugin_registry,
            scan_scope,
            rename_scope,
        )
        .await?;

        // Apply RenameScope filtering to edits as additional safety measure
        // Note: With the updated find_project_files(), files should already be filtered correctly,
        // but we keep this for belt-and-suspenders safety
        if let Some(scope) = rename_scope {
            edit_plan.edits.retain(|edit| {
                if let Some(ref file_path) = edit.file_path {
                    scope.should_include_file(Path::new(file_path))
                } else {
                    true // Keep edits without file paths
                }
            });
        }

        Ok(edit_plan)
    }

    /// Plan a directory move/rename with RenameScope filtering
    ///
    /// Wrapper that accepts RenameScope for file filtering
    pub async fn plan_directory_move_with_scope(
        &self,
        old_path: &Path,
        new_path: &Path,
        rename_scope: Option<&codebuddy_foundation::core::rename_scope::RenameScope>,
    ) -> ServerResult<EditPlan> {
        info!(
            old_path = %old_path.display(),
            new_path = %new_path.display(),
            "Planning directory move with scope"
        );

        let old_abs = self.to_absolute_path(old_path);
        let new_abs = self.to_absolute_path(new_path);

        // Validate source directory exists
        if !old_abs.exists() {
            return Err(ServerError::NotFound(format!(
                "Source directory does not exist: {}",
                old_abs.display()
            )));
        }

        if !old_abs.is_dir() {
            return Err(ServerError::InvalidRequest(format!(
                "Path is not a directory: {}",
                old_abs.display()
            )));
        }

        // Choose ScanScope based on RenameScope settings
        // Use ScanScope::All if string literals should be updated
        let scan_scope = if let Some(scope) = rename_scope {
            if scope.update_string_literals || scope.update_comments {
                Some(ScanScope::All)
            } else {
                Some(ScanScope::AllUseStatements)
            }
        } else {
            // Default: comprehensive scanning
            Some(ScanScope::All)
        };

        // Call planner directly to pass RenameScope
        let mut edit_plan = planner::plan_directory_move(
            &old_abs,
            &new_abs,
            self.reference_updater,
            self.plugin_registry,
            self.project_root,
            scan_scope,
            rename_scope,
        )
        .await?;

        // Apply RenameScope filtering to edits as additional safety measure
        // Note: With the updated find_project_files(), files should already be filtered correctly,
        // but we keep this for belt-and-suspenders safety
        if let Some(scope) = rename_scope {
            edit_plan.edits.retain(|edit| {
                if let Some(ref file_path) = edit.file_path {
                    scope.should_include_file(Path::new(file_path))
                } else {
                    true // Keep edits without file paths
                }
            });
        }

        Ok(edit_plan)
    }

    /// Convert relative path to absolute path
    fn to_absolute_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.project_root.join(path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_absolute_path() {
        let project_root = PathBuf::from("/project");
        let plugin_registry = PluginRegistry::new();
        let reference_updater = ReferenceUpdater::new(&project_root);

        let service = MoveService::new(&reference_updater, &plugin_registry, &project_root);

        // Relative path
        let rel = service.to_absolute_path(Path::new("src/main.rs"));
        assert_eq!(rel, PathBuf::from("/project/src/main.rs"));

        // Absolute path
        let abs = service.to_absolute_path(Path::new("/abs/path.rs"));
        assert_eq!(abs, PathBuf::from("/abs/path.rs"));
    }
}
