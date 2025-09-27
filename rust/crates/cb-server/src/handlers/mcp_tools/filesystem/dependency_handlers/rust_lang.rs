//! Rust Cargo.toml handler

use super::{UpdateDependenciesArgs, DependencyUpdateResult};
use anyhow::Result;
use std::fs;
use std::path::Path;

/// Handle Cargo.toml updates
pub async fn handle_cargo_toml_update(args: UpdateDependenciesArgs) -> Result<DependencyUpdateResult> {
    let is_dry_run = args.dry_run.unwrap_or(false);
    let mut changes = Vec::new();

    if !is_dry_run {
        match update_cargo_toml_file(&args, &mut changes).await {
            Ok(_) => {
                return Ok(DependencyUpdateResult {
                    success: true,
                    file_type: "Cargo.toml".to_string(),
                    file_path: args.file_path,
                    changes_made: changes,
                    dry_run: false,
                    error: None,
                });
            }
            Err(e) => {
                return Ok(DependencyUpdateResult {
                    success: false,
                    file_type: "Cargo.toml".to_string(),
                    file_path: args.file_path,
                    changes_made: vec![],
                    dry_run: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // Dry run
    collect_cargo_changes(&args, &mut changes).await;

    Ok(DependencyUpdateResult {
        success: true,
        file_type: "Cargo.toml".to_string(),
        file_path: args.file_path,
        changes_made: changes,
        dry_run: true,
        error: None,
    })
}

/// Actually update the Cargo.toml file
async fn update_cargo_toml_file(args: &UpdateDependenciesArgs, changes: &mut Vec<String>) -> Result<()> {
    let path = Path::new(&args.file_path);
    let content = fs::read_to_string(path)?;

    // Parse TOML
    let mut doc: toml_edit::Document = content.parse()?;

    // Add dependencies
    if let Some(deps) = &args.add_dependencies {
        let dependencies = doc["dependencies"]
            .or_insert(toml_edit::table())
            .as_table_like_mut()
            .ok_or_else(|| anyhow::anyhow!("dependencies section is not a table"))?;

        for (name, version) in deps {
            let version_str = version.as_str()
                .ok_or_else(|| anyhow::anyhow!("Version must be a string"))?;
            dependencies.insert(name, toml_edit::value(version_str));
            changes.push(format!("Add dependency: {} = \"{}\"", name, version_str));
        }
    }

    // Add dev dependencies
    if let Some(dev_deps) = &args.add_dev_dependencies {
        let dev_dependencies = doc["dev-dependencies"]
            .or_insert(toml_edit::table())
            .as_table_like_mut()
            .ok_or_else(|| anyhow::anyhow!("dev-dependencies section is not a table"))?;

        for (name, version) in dev_deps {
            let version_str = version.as_str()
                .ok_or_else(|| anyhow::anyhow!("Version must be a string"))?;
            dev_dependencies.insert(name, toml_edit::value(version_str));
            changes.push(format!("Add dev dependency: {} = \"{}\"", name, version_str));
        }
    }

    // Remove dependencies
    if let Some(remove_deps) = &args.remove_dependencies {
        for dep_name in remove_deps {
            // Remove from [dependencies]
            if let Some(deps_table) = doc["dependencies"].as_table_like_mut() {
                if deps_table.remove(dep_name).is_some() {
                    changes.push(format!("Remove dependency: {}", dep_name));
                }
            }
            // Remove from [dev-dependencies]
            if let Some(dev_deps_table) = doc["dev-dependencies"].as_table_like_mut() {
                if dev_deps_table.remove(dep_name).is_some() {
                    changes.push(format!("Remove dev dependency: {}", dep_name));
                }
            }
        }
    }

    // Update version
    if let Some(version) = &args.update_version {
        if let Some(package) = doc["package"].as_table_like_mut() {
            package.insert("version", toml_edit::value(version));
            changes.push(format!("Update version to: {}", version));
        }
    }

    // Write back to file
    fs::write(path, doc.to_string())?;

    Ok(())
}

/// Collect changes for dry run
async fn collect_cargo_changes(args: &UpdateDependenciesArgs, changes: &mut Vec<String>) {
    if let Some(deps) = &args.add_dependencies {
        for (name, version) in deps {
            changes.push(format!("Would add dependency: {} = \"{}\"", name, version));
        }
    }

    if let Some(dev_deps) = &args.add_dev_dependencies {
        for (name, version) in dev_deps {
            changes.push(format!("Would add dev dependency: {} = \"{}\"", name, version));
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