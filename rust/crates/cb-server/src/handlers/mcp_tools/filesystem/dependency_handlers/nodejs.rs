//! Node.js package.json handler

use super::{UpdateDependenciesArgs, DependencyUpdateResult};
use anyhow::Result;
use serde_json::{json, Value, Map};
use std::fs;
use std::path::Path;

/// Handle package.json updates
pub async fn handle_package_json_update(args: UpdateDependenciesArgs) -> Result<DependencyUpdateResult> {
    let is_dry_run = args.dry_run.unwrap_or(false);
    let mut changes = Vec::new();

    // If not dry run, we'll actually modify the file
    if !is_dry_run {
        match update_package_json_file(&args, &mut changes).await {
            Ok(_) => {
                return Ok(DependencyUpdateResult {
                    success: true,
                    file_type: "package.json".to_string(),
                    file_path: args.file_path,
                    changes_made: changes,
                    dry_run: false,
                    error: None,
                });
            }
            Err(e) => {
                return Ok(DependencyUpdateResult {
                    success: false,
                    file_type: "package.json".to_string(),
                    file_path: args.file_path,
                    changes_made: vec![],
                    dry_run: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // For dry run, just collect what would be changed
    collect_package_json_changes(&args, &mut changes).await;

    Ok(DependencyUpdateResult {
        success: true,
        file_type: "package.json".to_string(),
        file_path: args.file_path,
        changes_made: changes,
        dry_run: true,
        error: None,
    })
}

/// Actually update the package.json file
async fn update_package_json_file(args: &UpdateDependenciesArgs, changes: &mut Vec<String>) -> Result<()> {
    let path = Path::new(&args.file_path);

    // Read existing package.json
    let content = fs::read_to_string(path)?;
    let mut package_json: Value = serde_json::from_str(&content)?;

    // Ensure package_json is an object
    let obj = package_json.as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("package.json is not a valid JSON object"))?;

    // Add dependencies
    if let Some(deps) = &args.add_dependencies {
        let dependencies = obj.entry("dependencies").or_insert_with(|| json!({}));
        if let Some(dep_obj) = dependencies.as_object_mut() {
            for (name, version) in deps {
                dep_obj.insert(name.clone(), version.clone());
                changes.push(format!("Add dependency: {} @ {}", name, version));
            }
        }
    }

    // Add dev dependencies
    if let Some(dev_deps) = &args.add_dev_dependencies {
        let dev_dependencies = obj.entry("devDependencies").or_insert_with(|| json!({}));
        if let Some(dev_dep_obj) = dev_dependencies.as_object_mut() {
            for (name, version) in dev_deps {
                dev_dep_obj.insert(name.clone(), version.clone());
                changes.push(format!("Add dev dependency: {} @ {}", name, version));
            }
        }
    }

    // Remove dependencies
    if let Some(remove_deps) = &args.remove_dependencies {
        for dep_name in remove_deps {
            // Remove from dependencies
            if let Some(dependencies) = obj.get_mut("dependencies").and_then(|v| v.as_object_mut()) {
                if dependencies.remove(dep_name).is_some() {
                    changes.push(format!("Remove dependency: {}", dep_name));
                }
            }
            // Remove from devDependencies
            if let Some(dev_dependencies) = obj.get_mut("devDependencies").and_then(|v| v.as_object_mut()) {
                if dev_dependencies.remove(dep_name).is_some() {
                    changes.push(format!("Remove dev dependency: {}", dep_name));
                }
            }
        }
    }

    // Add scripts
    if let Some(scripts) = &args.add_scripts {
        let script_obj = obj.entry("scripts").or_insert_with(|| json!({}));
        if let Some(script_map) = script_obj.as_object_mut() {
            for (name, command) in scripts {
                script_map.insert(name.clone(), command.clone());
                changes.push(format!("Add script: {} = {}", name, command));
            }
        }
    }

    // Remove scripts
    if let Some(remove_scripts) = &args.remove_scripts {
        if let Some(scripts) = obj.get_mut("scripts").and_then(|v| v.as_object_mut()) {
            for script_name in remove_scripts {
                if scripts.remove(script_name).is_some() {
                    changes.push(format!("Remove script: {}", script_name));
                }
            }
        }
    }

    // Update version
    if let Some(version) = &args.update_version {
        obj.insert("version".to_string(), json!(version));
        changes.push(format!("Update version to: {}", version));
    }

    // Update workspace config
    if let Some(workspace_config) = &args.workspace_config {
        if let Some(workspaces) = &workspace_config.workspaces {
            obj.insert("workspaces".to_string(), json!(workspaces));
            changes.push(format!("Update workspaces: {:?}", workspaces));
        }
    }

    // Write back to file with pretty formatting
    let formatted = serde_json::to_string_pretty(&package_json)?;
    fs::write(path, formatted)?;

    Ok(())
}

/// Collect what changes would be made (for dry run)
async fn collect_package_json_changes(args: &UpdateDependenciesArgs, changes: &mut Vec<String>) {
    if let Some(deps) = &args.add_dependencies {
        for (name, version) in deps {
            changes.push(format!("Would add dependency: {} @ {}", name, version));
        }
    }

    if let Some(dev_deps) = &args.add_dev_dependencies {
        for (name, version) in dev_deps {
            changes.push(format!("Would add dev dependency: {} @ {}", name, version));
        }
    }

    if let Some(remove_deps) = &args.remove_dependencies {
        for dep_name in remove_deps {
            changes.push(format!("Would remove dependency: {}", dep_name));
        }
    }

    if let Some(scripts) = &args.add_scripts {
        for (name, command) in scripts {
            changes.push(format!("Would add script: {} = {}", name, command));
        }
    }

    if let Some(remove_scripts) = &args.remove_scripts {
        for script_name in remove_scripts {
            changes.push(format!("Would remove script: {}", script_name));
        }
    }

    if let Some(version) = &args.update_version {
        changes.push(format!("Would update version to: {}", version));
    }

    if let Some(workspace_config) = &args.workspace_config {
        if let Some(workspaces) = &workspace_config.workspaces {
            changes.push(format!("Would update workspaces: {:?}", workspaces));
        }
    }
}