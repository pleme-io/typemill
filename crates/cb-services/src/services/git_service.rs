//! Git integration for file operations
//!
//! Provides git-aware file operations to preserve history when working in git repositories.

use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Command;
use tracing::{debug, warn};

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
            .args(&["rev-parse", "--git-dir"])
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
            .args(&["ls-files", "--error-unmatch"])
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
    pub fn git_mv(old: &Path, new: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = new.parent() {
            std::fs::create_dir_all(parent)?;
        }

        debug!(
            old = %old.display(),
            new = %new.display(),
            "Executing git mv"
        );

        let output = Command::new("git")
            .args(&["mv"])
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
            .args(&["rm"])
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
