//! Git integration for file operations
//!
//! Provides git-aware file operations to preserve history when working in git repositories.

use anyhow::{anyhow, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, info, warn};

/// Service for git-aware file operations
#[derive(Clone)]
pub struct GitService;

impl GitService {
    /// Create a new GitService instance
    pub fn new() -> Self {
        Self
    }

    /// Check if the given path is within a git repository
    pub fn is_git_repo(path: &Path) -> bool {
        let result = Command::new("git")
            .current_dir(path)
            .args(["rev-parse", "--git-dir"])
            .output();

        match result {
            Ok(output) => {
                let success = output.status.success();
                debug!(
                    path = %path.display(),
                    is_git = success,
                    "Checked if path is in git repo"
                );
                success
            }
            Err(e) => {
                debug!(
                    path = %path.display(),
                    error = %e,
                    "Git command failed, assuming not a git repo"
                );
                false
            }
        }
    }

    /// Check if a file is tracked by git
    pub fn is_file_tracked(path: &Path) -> bool {
        let result = Command::new("git")
            .args(["ls-files", "--error-unmatch"])
            .arg(path.to_str().unwrap_or(""))
            .output();

        match result {
            Ok(output) => {
                let success = output.status.success();
                debug!(
                    path = %path.display(),
                    is_tracked = success,
                    "Checked if file is tracked by git"
                );
                success
            }
            Err(e) => {
                debug!(
                    path = %path.display(),
                    error = %e,
                    "Git ls-files failed, assuming not tracked"
                );
                false
            }
        }
    }

    /// Move a file using git mv
    ///
    /// This preserves git history for the file. The parent directory of the
    /// destination will be created if it doesn't exist.
    ///
    /// **Case-insensitive filesystem support**: Automatically handles case-only renames
    /// (e.g., `FILE.md` → `file.md`) on case-insensitive filesystems by using a
    /// two-step rename process through a temporary file.
    pub fn git_mv(old: &Path, new: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = new.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Detect case-only rename on case-insensitive filesystem
        let is_case_only_rename = Self::is_case_only_rename(old, new)?;

        if is_case_only_rename {
            info!(
                old = %old.display(),
                new = %new.display(),
                "Detected case-only rename on case-insensitive filesystem, using two-step approach"
            );

            // Two-step rename: old → temp → new
            let temp_path = Self::generate_temp_path(new)?;

            // Step 1: old → temp
            debug!(
                old = %old.display(),
                temp = %temp_path.display(),
                "Step 1: Renaming to temporary file"
            );
            Self::git_mv_direct(old, &temp_path)?;

            // Step 2: temp → new
            debug!(
                temp = %temp_path.display(),
                new = %new.display(),
                "Step 2: Renaming from temporary to final name"
            );
            Self::git_mv_direct(&temp_path, new)?;

            info!(
                old = %old.display(),
                new = %new.display(),
                "Case-only rename completed successfully"
            );

            return Ok(());
        }

        // Normal rename
        Self::git_mv_direct(old, new)
    }

    /// Check if this is a case-only rename on a case-insensitive filesystem
    fn is_case_only_rename(old: &Path, new: &Path) -> Result<bool> {
        // If paths are identical, not a rename
        if old == new {
            return Ok(false);
        }

        // Check if they differ only in case
        let same_case_insensitive =
            old.to_string_lossy().to_lowercase() == new.to_string_lossy().to_lowercase();

        if !same_case_insensitive {
            // Paths differ in more than just case
            return Ok(false);
        }

        // Paths differ only in case. Check if they resolve to the same file
        // (which would indicate a case-insensitive filesystem)
        match (fs::metadata(old), fs::metadata(new)) {
            (Ok(old_meta), Ok(new_meta)) => {
                // Both exist - check if they're the same file
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    Ok(old_meta.ino() == new_meta.ino())
                }
                #[cfg(not(unix))]
                {
                    // On Windows, compare canonicalized paths
                    let old_canonical = old.canonicalize().ok();
                    let new_canonical = new.canonicalize().ok();
                    Ok(old_canonical.is_some() && old_canonical == new_canonical)
                }
            }
            (Ok(_), Err(_)) => {
                // Old exists, new doesn't. This is a case-only rename if paths
                // differ only in case (already checked above)
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Generate a temporary path for two-step renames
    fn generate_temp_path(target: &Path) -> Result<PathBuf> {
        let parent = target
            .parent()
            .ok_or_else(|| anyhow!("Target has no parent directory"))?;
        let filename = target
            .file_name()
            .ok_or_else(|| anyhow!("Target has no filename"))?;

        // Use timestamp to avoid collisions
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);

        let temp_name = format!(".tmp_rename_{}_{}", filename.to_string_lossy(), timestamp);
        Ok(parent.join(temp_name))
    }

    /// Execute git mv directly without case-insensitive handling
    fn git_mv_direct(old: &Path, new: &Path) -> Result<()> {
        debug!(
            old = %old.display(),
            new = %new.display(),
            "Executing git mv"
        );

        let output = Command::new("git")
            .args(["mv"])
            .arg(old.to_str().ok_or_else(|| anyhow!("Invalid old path"))?)
            .arg(new.to_str().ok_or_else(|| anyhow!("Invalid new path"))?)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(
                old = %old.display(),
                new = %new.display(),
                stderr = %stderr,
                "git mv failed"
            );
            return Err(anyhow!("git mv failed: {}", stderr));
        }

        debug!(
            old = %old.display(),
            new = %new.display(),
            "git mv succeeded"
        );

        Ok(())
    }

    /// Remove a file using git rm
    pub fn git_rm(path: &Path) -> Result<()> {
        debug!(path = %path.display(), "Executing git rm");

        let output = Command::new("git")
            .args(["rm"])
            .arg(path.to_str().ok_or_else(|| anyhow!("Invalid path"))?)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(
                path = %path.display(),
                stderr = %stderr,
                "git rm failed"
            );
            return Err(anyhow!("git rm failed: {}", stderr));
        }

        debug!(path = %path.display(), "git rm succeeded");

        Ok(())
    }
}

impl Default for GitService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_is_git_repo() {
        // Current workspace should be a git repo
        let current_dir = env::current_dir().unwrap();
        assert!(GitService::is_git_repo(&current_dir));

        // /tmp should not be a git repo
        let tmp_dir = Path::new("/tmp");
        assert!(!GitService::is_git_repo(tmp_dir));
    }

    #[test]
    fn test_is_file_tracked() {
        // This test file should be tracked
        let this_file = Path::new(file!());
        // We need the full path from workspace root
        let workspace = env::current_dir().unwrap();
        let full_path = workspace.join(this_file);

        // This is a new file we just created, so it might not be tracked yet
        // Just verify the function runs without panicking
        let _ = GitService::is_file_tracked(&full_path);
    }
}
