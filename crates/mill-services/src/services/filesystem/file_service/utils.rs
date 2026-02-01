use super::FileService;
use mill_foundation::errors::MillError as ServerError;
use mill_foundation::validation::ValidationFailureAction;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, error, info, warn};

type ServerResult<T> = Result<T, ServerError>;

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

        // SECURITY: Validate the command before execution
        // For now, we implement a simple allowlist of safe prefixes/commands
        // This prevents completely arbitrary code execution from a malicious config
        // TODO: Move this policy to a robust configuration file or security policy
        let safe_prefixes = [
            "cargo check",
            "cargo test",
            "cargo build",
            "cargo clippy",
            "cargo fmt",
            "npm test",
            "npm run build",
            "npm run lint",
            "yarn test",
            "yarn build",
            "yarn lint",
            "pnpm test",
            "pnpm build",
            "pnpm lint",
            "pytest",
            "python -m pytest",
            "black",
            "ruff",
            "mypy",
            "go test",
            "go vet",
            "go fmt",
            "dotnet test",
            "dotnet build",
            "make test",
            "make check",
        ];

        let is_safe = safe_prefixes
            .iter()
            .any(|prefix| self.validation_config.command.trim().starts_with(prefix));

        if !is_safe {
            error!(
                command = %self.validation_config.command,
                "Validation command blocked by security policy. Command must start with a known safe prefix (e.g., 'cargo', 'npm', 'go', 'make')."
            );
            return Some(json!({
                "validation_status": "error",
                "validation_error": format!("Security Error: Command '{}' is not in the allowed list.", self.validation_config.command)
            }));
        }

        // Run validation command in the project root
        // SECURITY: Parse command string to avoid shell injection
        let (program, args) = match parse_command_line(&self.validation_config.command) {
            Some(res) => res,
            None => {
                return Some(json!({
                   "validation_status": "error",
                   "validation_error": "Empty validation command"
               }));
            }
        };

        let output = match Command::new(&program)
            .args(&args)
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
                ValidationFailureAction::Report => Some(json!({
                    "validation_status": "failed",
                    "validation_command": self.validation_config.command,
                    "validation_errors": stderr,
                    "validation_stdout": stdout,
                    "suggestion": format!(
                        "Validation failed. Run '{}' to see details. Consider reviewing changes before committing.",
                        self.validation_config.command
                    )
                })),
                ValidationFailureAction::Rollback => {
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
                ValidationFailureAction::Interactive => Some(json!({
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
    ///
    /// # ⚠️ DEPRECATED
    /// This method does NOT validate path containment. Use `to_absolute_path_checked`
    /// for all security-sensitive operations. This method will be removed in a future version.
    #[deprecated(since = "0.4.0", note = "Use to_absolute_path_checked for security")]
    pub fn to_absolute_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.project_root.join(path)
        }
    }

    /// Convert path to absolute and verify it's within project root
    ///
    /// This performs canonicalization and containment checking to prevent
    /// directory traversal attacks. Supports both existing and non-existent paths
    /// (for file creation operations).
    ///
    /// # Errors
    /// Returns error if path escapes project root or cannot be validated
    pub fn to_absolute_path_checked(&self, path: &Path) -> ServerResult<PathBuf> {
        // Convert to absolute
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.project_root.join(path)
        };

        // Try to canonicalize the full path if it exists
        let canonical = if abs_path.exists() {
            abs_path.canonicalize().map_err(|e| {
                ServerError::invalid_request(format!(
                    "Path canonicalization failed for {:?}: {}",
                    abs_path, e
                ))
            })?
        } else {
            // Path doesn't exist - find first existing ancestor and build from there
            let mut current = abs_path.clone();
            let mut components_to_add = Vec::new();

            // Walk up until we find an existing directory
            while !current.exists() {
                if let Some(filename) = current.file_name() {
                    components_to_add.push(filename.to_os_string());
                    if let Some(parent) = current.parent() {
                        current = parent.to_path_buf();
                    } else {
                        // Reached root without finding existing path
                        return Err(ServerError::invalid_request(format!(
                            "Cannot validate path: no existing ancestor found for {:?}",
                            abs_path
                        )));
                    }
                } else {
                    return Err(ServerError::invalid_request(format!(
                        "Invalid path: no filename component in {:?}",
                        current
                    )));
                }
            }

            // Canonicalize the existing ancestor
            let mut canonical = current.canonicalize().map_err(|e| {
                ServerError::invalid_request(format!(
                    "Path canonicalization failed for {:?}: {}",
                    current, e
                ))
            })?;

            // Add back the non-existing components
            for component in components_to_add.iter().rev() {
                canonical = canonical.join(component);
            }

            canonical
        };

        // Verify containment within project root using cached canonical root
        if !canonical.starts_with(&self.canonical_project_root) {
            return Err(ServerError::auth(format!(
                "Path traversal detected: {:?} escapes project root {:?}",
                path, self.project_root
            )));
        }

        Ok(canonical)
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

/// Parse command line string into program and arguments
/// Handles basic quoting (single and double quotes) and backslash escaping
fn parse_command_line(input: &str) -> Option<(String, Vec<String>)> {
    parse_command_line_internal(input, cfg!(windows))
}

fn parse_command_line_internal(input: &str, is_windows: bool) -> Option<(String, Vec<String>)> {
    let mut args = Vec::new();
    let mut current_arg = String::new();
    let mut in_quote = None;
    let mut escaped = false;
    let mut arg_started = false;

    for c in input.chars() {
        if escaped {
            current_arg.push(c);
            escaped = false;
            arg_started = true;
            continue;
        }

        match c {
            '\\' => {
                if is_windows {
                    current_arg.push(c);
                    arg_started = true;
                } else {
                    escaped = true;
                    arg_started = true;
                }
            }
            '"' if in_quote == Some('"') => {
                in_quote = None;
                arg_started = true;
            }
            '"' if in_quote.is_none() => {
                in_quote = Some('"');
                arg_started = true;
            }
            '\'' if in_quote == Some('\'') => {
                in_quote = None;
                arg_started = true;
            }
            '\'' if in_quote.is_none() => {
                in_quote = Some('\'');
                arg_started = true;
            }
            ' ' | '\t' | '\n' | '\r' if in_quote.is_none() => {
                if arg_started {
                    args.push(current_arg);
                    current_arg = String::new();
                    arg_started = false;
                }
            }
            _ => {
                current_arg.push(c);
                arg_started = true;
            }
        }
    }

    if arg_started {
        args.push(current_arg);
    }

    if args.is_empty() {
        return None;
    }

    let program = args.remove(0);
    Some((program, args))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_line_unix() {
        let (prog, args) = parse_command_line_internal(r#"cargo check "foo bar""#, false).unwrap();
        assert_eq!(prog, "cargo");
        assert_eq!(args, vec!["check", "foo bar"]);

        // Escaped quote
        let (prog, args) = parse_command_line_internal(r#"echo "foo \"bar\"""#, false).unwrap();
        assert_eq!(prog, "echo");
        assert_eq!(args, vec!["foo \"bar\""]);

        // Escaped backslash
        let (prog, args) = parse_command_line_internal(r#"echo foo\\bar"#, false).unwrap();
        assert_eq!(prog, "echo");
        assert_eq!(args, vec![r#"foo\bar"#]);
    }

    #[test]
    fn test_parse_command_line_windows() {
        // Windows path with backslashes
        let (prog, args) = parse_command_line_internal(r#"cargo check C:\Path\To\File"#, true).unwrap();
        assert_eq!(prog, "cargo");
        assert_eq!(args, vec!["check", r#"C:\Path\To\File"#]);

        // Quoted string on Windows (backslash kept literal)
        let (prog, args) = parse_command_line_internal(r#"echo "C:\Program Files""#, true).unwrap();
        assert_eq!(prog, "echo");
        assert_eq!(args, vec![r#"C:\Program Files"#]);
    }

    #[test]
    fn test_parse_command_line_empty_args() {
        // Empty quoted string
        let (prog, args) = parse_command_line_internal(r#"git commit -m """#, false).unwrap();
        assert_eq!(prog, "git");
        assert_eq!(args, vec!["commit", "-m", ""]);

        // Empty quoted string in middle
        let (prog, args) = parse_command_line_internal(r#"echo "" foo"#, false).unwrap();
        assert_eq!(prog, "echo");
        assert_eq!(args, vec!["", "foo"]);

        // Whitespace handling
        let (prog, args) = parse_command_line_internal(r#"echo   foo"#, false).unwrap();
        assert_eq!(prog, "echo");
        assert_eq!(args, vec!["foo"]);
    }
}
