//! Go go.mod handler

use super::{UpdateDependenciesArgs, DependencyUpdateResult};
use anyhow::Result;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Handle go.mod updates
pub async fn handle_go_mod_update(args: UpdateDependenciesArgs) -> Result<DependencyUpdateResult> {
    let is_dry_run = args.dry_run.unwrap_or(false);
    let mut changes = Vec::new();

    if !is_dry_run {
        match update_go_mod_file(&args, &mut changes).await {
            Ok(_) => {
                return Ok(DependencyUpdateResult {
                    success: true,
                    file_type: "go.mod".to_string(),
                    file_path: args.file_path,
                    changes_made: changes,
                    dry_run: false,
                    error: None,
                });
            }
            Err(e) => {
                return Ok(DependencyUpdateResult {
                    success: false,
                    file_type: "go.mod".to_string(),
                    file_path: args.file_path,
                    changes_made: vec![],
                    dry_run: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // Dry run
    collect_go_mod_changes(&args, &mut changes).await;

    Ok(DependencyUpdateResult {
        success: true,
        file_type: "go.mod".to_string(),
        file_path: args.file_path,
        changes_made: changes,
        dry_run: true,
        error: None,
    })
}

/// Update go.mod file using go commands
async fn update_go_mod_file(args: &UpdateDependenciesArgs, changes: &mut Vec<String>) -> Result<()> {
    let path = Path::new(&args.file_path);
    let dir = path.parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine directory for go.mod"))?;

    // Add dependencies using `go get`
    if let Some(deps) = &args.add_dependencies {
        for (name, version) in deps {
            let version_str = version.as_str()
                .ok_or_else(|| anyhow::anyhow!("Version must be a string"))?;

            let module_spec = if version_str.is_empty() {
                name.clone()
            } else {
                format!("{}@{}", name, version_str)
            };

            let output = Command::new("go")
                .arg("get")
                .arg(&module_spec)
                .current_dir(dir)
                .output()?;

            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("Failed to add dependency {}: {}", module_spec, error));
            }

            changes.push(format!("Add dependency: {}", module_spec));
        }
    }

    // Remove dependencies - Go doesn't have a direct remove command, so we'll manually edit
    if let Some(remove_deps) = &args.remove_dependencies {
        let content = fs::read_to_string(path)?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let mut in_require_block = false;
        let mut modified = false;

        lines.retain(|line| {
            let trimmed = line.trim();

            // Track if we're in a require block
            if trimmed.starts_with("require (") {
                in_require_block = true;
                return true;
            }
            if in_require_block && trimmed == ")" {
                in_require_block = false;
                return true;
            }

            // Check if this line contains a dependency to remove
            for dep_name in remove_deps {
                if (in_require_block || trimmed.starts_with("require ")) && trimmed.contains(dep_name) {
                    modified = true;
                    changes.push(format!("Remove dependency: {}", dep_name));
                    return false;
                }
            }

            true
        });

        if modified {
            fs::write(path, lines.join("\n"))?;

            // Run go mod tidy to clean up
            let output = Command::new("go")
                .arg("mod")
                .arg("tidy")
                .current_dir(dir)
                .output()?;

            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                tracing::warn!("go mod tidy failed: {}", error);
            }
        }
    }

    // Update Go version (this is trickier - we'll update the go.mod file directly)
    if let Some(version) = &args.update_version {
        let content = fs::read_to_string(path)?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        for line in &mut lines {
            if line.trim().starts_with("go ") {
                *line = format!("go {}", version);
                changes.push(format!("Update Go version to: {}", version));
                break;
            }
        }

        fs::write(path, lines.join("\n"))?;
    }

    Ok(())
}

/// Collect changes for dry run
async fn collect_go_mod_changes(args: &UpdateDependenciesArgs, changes: &mut Vec<String>) {
    if let Some(deps) = &args.add_dependencies {
        for (name, version) in deps {
            let module_spec = if let Some(version_str) = version.as_str() {
                if version_str.is_empty() {
                    name.clone()
                } else {
                    format!("{}@{}", name, version_str)
                }
            } else {
                name.clone()
            };
            changes.push(format!("Would add dependency: {}", module_spec));
        }
    }

    if let Some(remove_deps) = &args.remove_dependencies {
        for dep_name in remove_deps {
            changes.push(format!("Would remove dependency: {}", dep_name));
        }
    }

    if let Some(version) = &args.update_version {
        changes.push(format!("Would update Go version to: {}", version));
    }
}