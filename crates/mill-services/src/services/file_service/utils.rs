use super::FileService;
use codebuddy_foundation::protocol::ApiResult as ServerResult;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, error, info, warn};

impl FileService {
    /// Run post-operation validation if configured
    /// Returns validation results to be included in the operation response
    pub(super) async fn run_validation(&self) -> Option<Value> {
        use std::process::Command;

        if !self.validation_config.enabled {
            return None;
        }

        info!(
            command = %self.validation_config.command,
            "Running post-operation validation"
        );

        // Run validation command in the project root
        let output = match Command::new("sh")
            .arg("-c")
            .arg(&self.validation_config.command)
            .current_dir(&self.project_root)
            .output()
        {
            Ok(output) => output,
            Err(e) => {
                error!(error = %e, "Failed to execute validation command");
                return Some(json!({
                    "validation_status": "error",
                    "validation_error": format!("Failed to execute command: {}", e)
                }));
            }
        };

        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if success {
            info!("Validation passed");
            Some(json!({
                "validation_status": "passed",
                "validation_command": self.validation_config.command
            }))
        } else {
            warn!(
                stderr = %stderr,
                "Validation failed"
            );

            // For Report action, just include the errors in the response
            match self.validation_config.on_failure {
                codebuddy_config::config::ValidationFailureAction::Report => Some(json!({
                    "validation_status": "failed",
                    "validation_command": self.validation_config.command,
                    "validation_errors": stderr,
                    "validation_stdout": stdout,
                    "suggestion": format!(
                        "Validation failed. Run '{}' to see details. Consider reviewing changes before committing.",
                        self.validation_config.command
                    )
                })),
                codebuddy_config::config::ValidationFailureAction::Rollback => {
                    warn!(
                        stderr = %stderr,
                        "Validation failed. Executing automatic rollback via 'git reset --hard HEAD'"
                    );

                    let rollback_output = Command::new("git")
                        .args(["reset", "--hard", "HEAD"])
                        .current_dir(&self.project_root)
                        .output();

                    let (rollback_status, rollback_error) = match rollback_output {
                        Ok(out) if out.status.success() => {
                            info!("Rollback completed successfully");
                            ("rollback_succeeded", None)
                        }
                        Ok(out) => {
                            let error_msg = String::from_utf8_lossy(&out.stderr).to_string();
                            error!(error = %error_msg, "Rollback command failed");
                            ("rollback_failed", Some(error_msg))
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to execute rollback command");
                            ("rollback_failed", Some(e.to_string()))
                        }
                    };

                    Some(json!({
                        "validation_status": "failed",
                        "validation_action": rollback_status,
                        "validation_command": self.validation_config.command,
                        "validation_errors": stderr,
                        "rollback_error": rollback_error,
                        "suggestion": if rollback_status == "rollback_succeeded" {
                            "Validation failed and changes were automatically rolled back using git."
                        } else {
                            "Validation failed and automatic rollback failed. Please manually revert changes."
                        }
                    }))
                }
                codebuddy_config::config::ValidationFailureAction::Interactive => Some(json!({
                    "validation_status": "failed",
                    "validation_action": "interactive_prompt",
                    "validation_command": self.validation_config.command,
                    "validation_errors": stderr,
                    "validation_stdout": stdout,
                    "rollback_available": true,
                    "suggestion": "Validation failed. Please review the errors and decide whether to keep or revert the changes. Run 'git reset --hard HEAD' to rollback."
                })),
            }
        }
    }

    /// Convert a path to absolute path within the project
    pub fn to_absolute_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.project_root.join(path)
        }
    }

    /// Adjust a relative path based on depth change
    #[allow(dead_code)]
    pub(super) fn adjust_relative_path(
        &self,
        path: &str,
        old_depth: usize,
        new_depth: usize,
    ) -> String {
        let depth_diff = new_depth as i32 - old_depth as i32;

        if depth_diff > 0 {
            // Moved deeper, add more "../"
            let additional_uplevels = "../".repeat(depth_diff as usize);
            format!("{}{}", additional_uplevels, path)
        } else if depth_diff < 0 {
            // Moved shallower, remove "../"
            let uplevels_to_remove = (-depth_diff) as usize;
            let mut remaining = path;
            for _ in 0..uplevels_to_remove {
                remaining = remaining.strip_prefix("../").unwrap_or(remaining);
            }
            remaining.to_string()
        } else {
            path.to_string()
        }
    }

    /// Update documentation file references after directory rename
    pub(super) async fn update_documentation_references(
        &self,
        old_dir_path: &Path,
        new_dir_path: &Path,
        dry_run: bool,
    ) -> ServerResult<DocumentationUpdateReport> {
        let old_rel = old_dir_path
            .strip_prefix(&self.project_root)
            .unwrap_or(old_dir_path);
        let new_rel = new_dir_path
            .strip_prefix(&self.project_root)
            .unwrap_or(new_dir_path);

        let old_path_str = old_rel.to_string_lossy();
        let new_path_str = new_rel.to_string_lossy();

        // Documentation file patterns
        let doc_patterns = ["*.md", "*.txt", "README*", "CHANGELOG*", "CONTRIBUTING*"];

        let mut updated_files = Vec::new();
        let mut failed_files = Vec::new();
        let mut total_references = 0;

        // Walk project root for documentation files
        let walker = ignore::WalkBuilder::new(&self.project_root)
            .hidden(false)
            .git_ignore(true)
            .build();

        for entry in walker.flatten() {
            let path = entry.path();

            // Check if matches doc pattern
            if !path.is_file() {
                continue;
            }

            let matches_pattern = doc_patterns.iter().any(|pattern| {
                if pattern.starts_with('*') {
                    path.extension()
                        .and_then(|e| e.to_str())
                        .map(|e| pattern.ends_with(e))
                        .unwrap_or(false)
                } else {
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with(pattern.trim_end_matches('*')))
                        .unwrap_or(false)
                }
            });

            if !matches_pattern {
                continue;
            }

            // Read file content
            match fs::read_to_string(&path).await {
                Ok(content) => {
                    // Count and replace references
                    let count = content.matches(old_path_str.as_ref()).count();
                    if count == 0 {
                        continue;
                    }

                    total_references += count;

                    if dry_run {
                        info!(
                            file = %path.display(),
                            references = count,
                            "[DRY RUN] Would update documentation references"
                        );
                        updated_files.push(path.to_string_lossy().to_string());
                    } else {
                        let new_content =
                            content.replace(old_path_str.as_ref(), new_path_str.as_ref());

                        match fs::write(&path, new_content).await {
                            Ok(_) => {
                                info!(
                                    file = %path.display(),
                                    references = count,
                                    old = %old_path_str,
                                    new = %new_path_str,
                                    "Updated documentation references"
                                );
                                updated_files.push(path.to_string_lossy().to_string());
                            }
                            Err(e) => {
                                warn!(
                                    file = %path.display(),
                                    error = %e,
                                    "Failed to update documentation file"
                                );
                                failed_files.push(format!("{}: {}", path.display(), e));
                            }
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
                    // Skip binary files silently
                    debug!(file = %path.display(), "Skipping binary file");
                }
                Err(e) => {
                    warn!(
                        file = %path.display(),
                        error = %e,
                        "Failed to read documentation file"
                    );
                    failed_files.push(format!("{}: {}", path.display(), e));
                }
            }
        }

        Ok(DocumentationUpdateReport {
            files_updated: updated_files.len(),
            references_updated: total_references,
            updated_files,
            failed_files,
        })
    }
}

/// Result of documentation reference updates
#[derive(Debug, Clone, serde::Serialize)]
pub struct DocumentationUpdateReport {
    /// Number of documentation files updated
    pub files_updated: usize,
    /// Number of path references updated
    pub references_updated: usize,
    /// Paths of updated documentation files
    pub updated_files: Vec<String>,
    /// Files that failed to update
    pub failed_files: Vec<String>,
}
