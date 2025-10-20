//! Reference detection trait for language plugins
//!
//! Provides language-specific detection of affected files when renaming/moving code.

use async_trait::async_trait;
use std::path::{Path, PathBuf};

/// Language-specific reference detection for file moves/renames
///
/// This trait enables plugins to provide custom logic for detecting which files
/// are affected by a rename or move operation. For example, Rust needs to detect
/// cross-crate imports that reference the old crate name.
#[async_trait]
pub trait ReferenceDetector: Send + Sync {
    /// Find files affected by a rename or move operation
    ///
    /// This method analyzes the project to find all files that reference the old
    /// path and would need updates when the file/directory is moved or renamed.
    ///
    /// # Arguments
    ///
    /// * `old_path` - The current path being renamed/moved
    /// * `new_path` - The target path after the rename/move
    /// * `project_root` - Root directory of the project
    /// * `project_files` - List of all files in the project (for scanning)
    ///
    /// # Returns
    ///
    /// List of file paths that contain references needing updates
    ///
    /// # Default Implementation
    ///
    /// Returns an empty vector. Language plugins should override to provide
    /// language-specific reference detection.
    ///
    /// # Example
    ///
    /// For Rust, this would detect files with `use old_crate::module` imports
    /// when renaming a crate directory.
    async fn find_affected_files(
        &self,
        _old_path: &Path,
        _new_path: &Path,
        _project_root: &Path,
        _project_files: &[PathBuf],
    ) -> Vec<PathBuf> {
        // Default: no language-specific detection
        Vec::new()
    }
}
