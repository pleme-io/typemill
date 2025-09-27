//! Python requirements.txt and pyproject.toml handler

use super::{UpdateDependenciesArgs, DependencyUpdateResult};
use anyhow::Result;
use std::fs;
use std::path::Path;

/// Handle requirements.txt updates
pub async fn handle_requirements_update(args: UpdateDependenciesArgs) -> Result<DependencyUpdateResult> {
    let is_dry_run = args.dry_run.unwrap_or(false);
    let mut changes = Vec::new();

    if !is_dry_run {
        match update_requirements_file(&args, &mut changes).await {
            Ok(_) => {
                return Ok(DependencyUpdateResult {
                    success: true,
                    file_type: "requirements.txt".to_string(),
                    file_path: args.file_path,
                    changes_made: changes,
                    dry_run: false,
                    error: None,
                });
            }
            Err(e) => {
                return Ok(DependencyUpdateResult {
                    success: false,
                    file_type: "requirements.txt".to_string(),
                    file_path: args.file_path,
                    changes_made: vec![],
                    dry_run: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // Dry run
    collect_requirements_changes(&args, &mut changes).await;

    Ok(DependencyUpdateResult {
        success: true,
        file_type: "requirements.txt".to_string(),
        file_path: args.file_path,
        changes_made: changes,
        dry_run: true,
        error: None,
    })
}

/// Handle pyproject.toml updates
pub async fn handle_pyproject_update(args: UpdateDependenciesArgs) -> Result<DependencyUpdateResult> {
    let is_dry_run = args.dry_run.unwrap_or(false);
    let mut changes = Vec::new();

    if !is_dry_run {
        match update_pyproject_file(&args, &mut changes).await {
            Ok(_) => {
                return Ok(DependencyUpdateResult {
                    success: true,
                    file_type: "pyproject.toml".to_string(),
                    file_path: args.file_path,
                    changes_made: changes,
                    dry_run: false,
                    error: None,
                });
            }
            Err(e) => {
                return Ok(DependencyUpdateResult {
                    success: false,
                    file_type: "pyproject.toml".to_string(),
                    file_path: args.file_path,
                    changes_made: vec![],
                    dry_run: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // Dry run
    collect_pyproject_changes(&args, &mut changes).await;

    Ok(DependencyUpdateResult {
        success: true,
        file_type: "pyproject.toml".to_string(),
        file_path: args.file_path,
        changes_made: changes,
        dry_run: true,
        error: None,
    })
}

/// Update requirements.txt file
async fn update_requirements_file(args: &UpdateDependenciesArgs, changes: &mut Vec<String>) -> Result<()> {
    let path = Path::new(&args.file_path);
    let content = if path.exists() {
        fs::read_to_string(path)?
    } else {
        String::new()
    };

    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    // Add dependencies
    if let Some(deps) = &args.add_dependencies {
        for (name, version) in deps {
            let version_str = version.as_str()
                .ok_or_else(|| anyhow::anyhow!("Version must be a string"))?;
            let requirement = if version_str.is_empty() {
                name.clone()
            } else {
                format!("{}=={}", name, version_str)
            };
            lines.push(requirement.clone());
            changes.push(format!("Add dependency: {}", requirement));
        }
    }

    // Remove dependencies
    if let Some(remove_deps) = &args.remove_dependencies {
        let original_len = lines.len();
        lines.retain(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return true;
            }

            // Extract package name from requirement line
            let package_name = trimmed
                .split(|c: char| c == '=' || c == '>' || c == '<' || c == '!' || c == '~' || c == ' ')
                .next()
                .unwrap_or("")
                .trim();

            !remove_deps.contains(&package_name.to_string())
        });

        let removed_count = original_len - lines.len();
        if removed_count > 0 {
            for dep_name in remove_deps {
                changes.push(format!("Remove dependency: {}", dep_name));
            }
        }
    }

    // Write back to file
    fs::write(path, lines.join("\n") + "\n")?;

    Ok(())
}

/// Update pyproject.toml file
async fn update_pyproject_file(args: &UpdateDependenciesArgs, changes: &mut Vec<String>) -> Result<()> {
    let path = Path::new(&args.file_path);
    let content = fs::read_to_string(path)?;

    // Parse TOML
    let mut doc: toml_edit::Document = content.parse()?;

    // Add dependencies
    if let Some(deps) = &args.add_dependencies {
        let dependencies = doc["project"]["dependencies"]
            .or_insert(toml_edit::array())
            .as_array_mut()
            .ok_or_else(|| anyhow::anyhow!("dependencies must be an array"))?;

        for (name, version) in deps {
            let version_str = version.as_str()
                .ok_or_else(|| anyhow::anyhow!("Version must be a string"))?;
            let requirement = if version_str.is_empty() {
                name.clone()
            } else {
                format!("{}=={}", name, version_str)
            };
            dependencies.push(requirement.clone());
            changes.push(format!("Add dependency: {}", requirement));
        }
    }

    // Add dev dependencies (optional-dependencies.dev)
    if let Some(dev_deps) = &args.add_dev_dependencies {
        let dev_dependencies = doc["project"]["optional-dependencies"]["dev"]
            .or_insert(toml_edit::array())
            .as_array_mut()
            .ok_or_else(|| anyhow::anyhow!("dev dependencies must be an array"))?;

        for (name, version) in dev_deps {
            let version_str = version.as_str()
                .ok_or_else(|| anyhow::anyhow!("Version must be a string"))?;
            let requirement = if version_str.is_empty() {
                name.clone()
            } else {
                format!("{}=={}", name, version_str)
            };
            dev_dependencies.push(requirement.clone());
            changes.push(format!("Add dev dependency: {}", requirement));
        }
    }

    // Update version
    if let Some(version) = &args.update_version {
        if let Some(project) = doc["project"].as_table_like_mut() {
            project.insert("version", toml_edit::value(version));
            changes.push(format!("Update version to: {}", version));
        }
    }

    // Write back to file
    fs::write(path, doc.to_string())?;

    Ok(())
}

/// Collect changes for requirements.txt dry run
async fn collect_requirements_changes(args: &UpdateDependenciesArgs, changes: &mut Vec<String>) {
    if let Some(deps) = &args.add_dependencies {
        for (name, version) in deps {
            let requirement = if let Some(version_str) = version.as_str() {
                if version_str.is_empty() {
                    name.clone()
                } else {
                    format!("{}=={}", name, version_str)
                }
            } else {
                name.clone()
            };
            changes.push(format!("Would add dependency: {}", requirement));
        }
    }

    if let Some(remove_deps) = &args.remove_dependencies {
        for dep_name in remove_deps {
            changes.push(format!("Would remove dependency: {}", dep_name));
        }
    }
}

/// Collect changes for pyproject.toml dry run
async fn collect_pyproject_changes(args: &UpdateDependenciesArgs, changes: &mut Vec<String>) {
    if let Some(deps) = &args.add_dependencies {
        for (name, version) in deps {
            let requirement = if let Some(version_str) = version.as_str() {
                if version_str.is_empty() {
                    name.clone()
                } else {
                    format!("{}=={}", name, version_str)
                }
            } else {
                name.clone()
            };
            changes.push(format!("Would add dependency: {}", requirement));
        }
    }

    if let Some(dev_deps) = &args.add_dev_dependencies {
        for (name, version) in dev_deps {
            let requirement = if let Some(version_str) = version.as_str() {
                if version_str.is_empty() {
                    name.clone()
                } else {
                    format!("{}=={}", name, version_str)
                }
            } else {
                name.clone()
            };
            changes.push(format!("Would add dev dependency: {}", requirement));
        }
    }

    if let Some(remove_deps) = &args.remove_dependencies {
        for dep_name in remove_deps {
            changes.push(format!("Would remove dependency: {}", dep_name));
        }
    }

    if let Some(version) = &args.update_version {
        changes.push(format!("Would update version to: {}", version));
    }
}