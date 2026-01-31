//! Consolidation post-processing for TypeScript/JavaScript package consolidation operations
//!
//! **LANGUAGE-SPECIFIC**: This module contains TypeScript-specific logic for npm package consolidation.
//! It handles the post-processing tasks that must occur after moving files during a consolidation
//! operation.
//!
//! This module handles the post-processing tasks that must occur after moving
//! files during a consolidation operation:
//! 1. Flatten nested src/ directory structure
//! 2. Merge dependencies from source package.json
//! 3. Update imports across workspace
//! 4. Clean up workspace package.json

use mill_foundation::protocol::ConsolidationMetadata;
use mill_plugin_api::{PluginApiError, PluginResult};
use serde_json::{json, Value};
use std::path::Path;
use tokio::fs;
use tracing::{debug, info, warn};

/// Execute consolidation post-processing after directory move
///
/// This handles TypeScript-specific consolidation tasks:
/// 1. Fix directory structure (flatten nested src/)
/// 2. Merge package.json dependencies
/// 3. Update imports across workspace
/// 4. Clean up workspace package.json
pub async fn execute_consolidation_post_processing(
    metadata: &ConsolidationMetadata,
    project_root: &Path,
) -> PluginResult<()> {
    info!(
        source_package = %metadata.source_crate_name,
        target_package = %metadata.target_crate_name,
        target_module = %metadata.target_module_name,
        "Executing TypeScript consolidation post-processing"
    );

    // Task 1: Fix nested src/ structure
    flatten_nested_src_directory(&metadata.target_module_path).await?;

    // Task 2: Merge dependencies from source package.json
    let source_package_json = Path::new(&metadata.source_crate_path).join("package.json");
    let target_package_json = Path::new(&metadata.target_crate_path).join("package.json");

    if source_package_json.exists() && target_package_json.exists() {
        merge_package_json_dependencies(&source_package_json, &target_package_json).await?;
    }

    // Task 3: Update imports across workspace
    update_imports_for_consolidation(
        &metadata.source_crate_name,
        &metadata.target_crate_name,
        &metadata.target_module_name,
        project_root,
    )
    .await?;

    // Task 4: Clean up workspace package.json (remove source from workspaces)
    cleanup_workspace_package_json(&metadata.source_crate_path, project_root).await?;

    // Task 5: Remove leftover package.json from module directory
    let module_package_json = Path::new(&metadata.target_module_path).join("package.json");
    if module_package_json.exists() {
        fs::remove_file(&module_package_json).await.map_err(|e| {
            PluginApiError::internal(format!("Failed to remove package.json: {}", e))
        })?;
        info!("Removed leftover package.json from module directory");
    }

    info!("TypeScript consolidation post-processing complete");
    Ok(())
}

/// Flatten nested src/ directory: module/src/* → module/*
async fn flatten_nested_src_directory(module_path: &str) -> PluginResult<()> {
    let module_dir = Path::new(module_path);
    let nested_src = module_dir.join("src");

    if !nested_src.exists() {
        info!(
            module_path = %module_path,
            "No nested src/ directory, skipping flatten"
        );
        return Ok(());
    }

    info!(
        nested_src = %nested_src.display(),
        "Flattening nested src/ directory"
    );

    // Move all files from module/src/* to module/*
    let mut entries = fs::read_dir(&nested_src)
        .await
        .map_err(|e| PluginApiError::internal(format!("Failed to read nested src/: {}", e)))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| PluginApiError::internal(format!("Failed to iterate src/ entries: {}", e)))?
    {
        let file_name = entry.file_name();
        let source = entry.path();
        let target = module_dir.join(&file_name);

        fs::rename(&source, &target).await.map_err(|e| {
            PluginApiError::internal(format!(
                "Failed to move {} to {}: {}",
                source.display(),
                target.display(),
                e
            ))
        })?;

        info!(
            file = %file_name.to_string_lossy(),
            "Moved file from nested src/"
        );
    }

    // Remove empty src/ directory
    fs::remove_dir(&nested_src)
        .await
        .map_err(|e| PluginApiError::internal(format!("Failed to remove empty src/: {}", e)))?;

    Ok(())
}

/// Merge dependencies from source package.json to target package.json
async fn merge_package_json_dependencies(
    source_path: &Path,
    target_path: &Path,
) -> PluginResult<()> {
    info!(
        source = %source_path.display(),
        target = %target_path.display(),
        "Merging package.json dependencies"
    );

    let source_content = fs::read_to_string(source_path).await.map_err(|e| {
        PluginApiError::internal(format!("Failed to read source package.json: {}", e))
    })?;

    let target_content = fs::read_to_string(target_path).await.map_err(|e| {
        PluginApiError::internal(format!("Failed to read target package.json: {}", e))
    })?;

    let source: Value = serde_json::from_str(&source_content).map_err(|e| {
        PluginApiError::internal(format!("Failed to parse source package.json: {}", e))
    })?;

    let mut target: Value = serde_json::from_str(&target_content).map_err(|e| {
        PluginApiError::internal(format!("Failed to parse target package.json: {}", e))
    })?;

    let mut modified = false;

    // Merge dependencies
    if let Some(source_deps) = source.get("dependencies").and_then(|d| d.as_object()) {
        // Ensure dependencies object exists
        if target.get("dependencies").is_none() {
            target["dependencies"] = json!({});
        }

        if let Some(target_deps) = target["dependencies"].as_object_mut() {
            for (name, version) in source_deps {
                if !target_deps.contains_key(name) {
                    target_deps.insert(name.clone(), version.clone());
                    modified = true;
                    debug!(dependency = %name, "Added dependency from source");
                }
            }
        }
    }

    // Merge devDependencies
    if let Some(source_deps) = source.get("devDependencies").and_then(|d| d.as_object()) {
        // Ensure devDependencies object exists
        if target.get("devDependencies").is_none() {
            target["devDependencies"] = json!({});
        }

        if let Some(target_deps) = target["devDependencies"].as_object_mut() {
            for (name, version) in source_deps {
                if !target_deps.contains_key(name) {
                    target_deps.insert(name.clone(), version.clone());
                    modified = true;
                    debug!(dependency = %name, "Added devDependency from source");
                }
            }
        }
    }

    // Merge peerDependencies
    if let Some(source_deps) = source.get("peerDependencies").and_then(|d| d.as_object()) {
        // Ensure peerDependencies object exists
        if target.get("peerDependencies").is_none() {
            target["peerDependencies"] = json!({});
        }

        if let Some(target_deps) = target["peerDependencies"].as_object_mut() {
            for (name, version) in source_deps {
                if !target_deps.contains_key(name) {
                    target_deps.insert(name.clone(), version.clone());
                    modified = true;
                    debug!(dependency = %name, "Added peerDependency from source");
                }
            }
        }
    }

    if modified {
        let updated_content = serde_json::to_string_pretty(&target).map_err(|e| {
            PluginApiError::internal(format!("Failed to serialize package.json: {}", e))
        })?;

        fs::write(target_path, updated_content).await.map_err(|e| {
            PluginApiError::internal(format!("Failed to write target package.json: {}", e))
        })?;

        info!("Dependencies merged successfully");
    } else {
        info!("No new dependencies to merge");
    }

    Ok(())
}

/// Update imports across workspace for consolidation
///
/// When consolidating packages, all imports need to be updated:
/// - `import { x } from 'old-package'` → `import { x } from 'new-package/module'`
/// - `require('old-package')` → `require('new-package/module')`
async fn update_imports_for_consolidation(
    source_package_name: &str,
    target_package_name: &str,
    target_module_name: &str,
    project_root: &Path,
) -> PluginResult<()> {
    info!(
        source_package = %source_package_name,
        target_package = %target_package_name,
        target_module = %target_module_name,
        "Updating imports across workspace for consolidation"
    );

    let mut files_updated = 0;
    let mut total_replacements = 0;

    update_imports_in_workspace_directory(
        project_root,
        source_package_name,
        target_package_name,
        target_module_name,
        &mut files_updated,
        &mut total_replacements,
    )
    .await?;

    info!(
        files_updated = files_updated,
        replacements = total_replacements,
        "Updated imports across workspace for consolidation"
    );

    Ok(())
}

/// Recursively update imports in workspace directory
async fn update_imports_in_workspace_directory(
    dir: &Path,
    source_package: &str,
    target_package: &str,
    target_module: &str,
    files_updated: &mut usize,
    total_replacements: &mut usize,
) -> PluginResult<()> {
    // Skip common directories
    let dir_name = dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if matches!(
        dir_name,
        "node_modules" | ".git" | "dist" | "build" | ".next" | "coverage"
    ) {
        return Ok(());
    }

    let entries_result = fs::read_dir(dir).await;
    if entries_result.is_err() {
        return Ok(());
    }

    let mut entries = entries_result.unwrap();

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| PluginApiError::internal(format!("Failed to iterate directory: {}", e)))?
    {
        let path = entry.path();

        if path.is_dir() {
            Box::pin(update_imports_in_workspace_directory(
                &path,
                source_package,
                target_package,
                target_module,
                files_updated,
                total_replacements,
            ))
            .await?;
        } else {
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if matches!(ext, "ts" | "tsx" | "js" | "jsx" | "mts" | "mjs" | "cts" | "cjs") {
                update_imports_in_single_file(
                    &path,
                    source_package,
                    target_package,
                    target_module,
                    files_updated,
                    total_replacements,
                )
                .await?;
            }
        }
    }

    Ok(())
}

/// Update imports in a single TypeScript/JavaScript file
async fn update_imports_in_single_file(
    file_path: &Path,
    source_package: &str,
    target_package: &str,
    target_module: &str,
    files_updated: &mut usize,
    total_replacements: &mut usize,
) -> PluginResult<()> {
    let content_result = fs::read_to_string(file_path).await;
    if content_result.is_err() {
        return Ok(());
    }

    let content = content_result.unwrap();

    // Skip if file doesn't contain the source package
    if !content.contains(source_package) {
        return Ok(());
    }

    let mut new_content = content.clone();
    let mut replacements = 0;

    // Pattern 1: ES6 imports with quotes
    // import { x } from 'old-package' → import { x } from 'new-package/module'
    let patterns = [
        (
            format!("from '{}'", source_package),
            format!("from '{}/{}'", target_package, target_module),
        ),
        (
            format!("from \"{}\"", source_package),
            format!("from \"{}/{}\"", target_package, target_module),
        ),
        // Pattern 2: require statements
        (
            format!("require('{}')", source_package),
            format!("require('{}/{}')", target_package, target_module),
        ),
        (
            format!("require(\"{}\")", source_package),
            format!("require(\"{}/{}\")", target_package, target_module),
        ),
        // Pattern 3: Dynamic imports
        (
            format!("import('{}')", source_package),
            format!("import('{}/{}')", target_package, target_module),
        ),
        (
            format!("import(\"{}\")", source_package),
            format!("import(\"{}/{}\")", target_package, target_module),
        ),
    ];

    for (from, to) in &patterns {
        if new_content.contains(from) {
            let count = new_content.matches(from).count();
            new_content = new_content.replace(from, to);
            replacements += count;
        }
    }

    if replacements > 0 {
        fs::write(file_path, new_content).await.map_err(|e| {
            PluginApiError::internal(format!("Failed to write {}: {}", file_path.display(), e))
        })?;

        *files_updated += 1;
        *total_replacements += replacements;

        info!(
            file = %file_path.display(),
            replacements = replacements,
            "Updated imports for consolidation"
        );
    }

    Ok(())
}

/// Clean up workspace package.json after consolidation
///
/// Removes source package from workspaces array
async fn cleanup_workspace_package_json(
    source_package_path: &str,
    project_root: &Path,
) -> PluginResult<()> {
    let workspace_json = project_root.join("package.json");

    if !workspace_json.exists() {
        warn!("Workspace package.json not found, skipping cleanup");
        return Ok(());
    }

    // Calculate relative path from project root
    let source_path = Path::new(source_package_path);
    let source_relative = source_path
        .strip_prefix(project_root)
        .unwrap_or(source_path)
        .to_string_lossy()
        .to_string();

    let content = fs::read_to_string(&workspace_json).await.map_err(|e| {
        PluginApiError::internal(format!("Failed to read workspace package.json: {}", e))
    })?;

    let mut parsed: Value = serde_json::from_str(&content).map_err(|e| {
        PluginApiError::internal(format!("Failed to parse workspace package.json: {}", e))
    })?;

    let mut modified = false;

    // Handle array format: "workspaces": ["packages/*"]
    if let Some(Value::Array(workspaces)) = parsed.get_mut("workspaces") {
        let before_len = workspaces.len();
        workspaces.retain(|v| {
            if let Some(s) = v.as_str() {
                // Don't retain if it matches the source path
                s != source_relative && s != source_package_path
            } else {
                true
            }
        });

        if workspaces.len() < before_len {
            modified = true;
            info!(
                source_path = %source_relative,
                "Removed from workspace members"
            );
        }
    }

    // Handle object format (Yarn v1): "workspaces": { "packages": [...] }
    if let Some(Value::Object(workspaces)) = parsed.get_mut("workspaces") {
        if let Some(Value::Array(packages)) = workspaces.get_mut("packages") {
            let before_len = packages.len();
            packages.retain(|v| {
                if let Some(s) = v.as_str() {
                    s != source_relative && s != source_package_path
                } else {
                    true
                }
            });

            if packages.len() < before_len {
                modified = true;
                info!(
                    source_path = %source_relative,
                    "Removed from workspace packages"
                );
            }
        }
    }

    if modified {
        let updated = serde_json::to_string_pretty(&parsed).map_err(|e| {
            PluginApiError::internal(format!("Failed to serialize package.json: {}", e))
        })?;

        fs::write(&workspace_json, updated).await.map_err(|e| {
            PluginApiError::internal(format!("Failed to write workspace package.json: {}", e))
        })?;

        info!("Workspace package.json cleanup complete");
    } else {
        info!("No workspace package.json cleanup needed");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_flatten_nested_src_directory() {
        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("src");
        fs::create_dir(&src_dir).await.unwrap();

        let file1 = src_dir.join("index.ts");
        fs::write(&file1, "export const x = 1;").await.unwrap();

        flatten_nested_src_directory(dir.path().to_str().unwrap())
            .await
            .unwrap();

        assert!(!src_dir.exists(), "src/ should be removed");
        assert!(
            dir.path().join("index.ts").exists(),
            "index.ts should be moved up"
        );
    }

    #[tokio::test]
    async fn test_merge_package_json_dependencies() {
        let dir = tempdir().unwrap();

        let source_json = dir.path().join("source.json");
        fs::write(
            &source_json,
            r#"{
            "name": "source",
            "dependencies": {
                "lodash": "^4.0.0",
                "axios": "^1.0.0"
            },
            "devDependencies": {
                "typescript": "^5.0.0"
            }
        }"#,
        )
        .await
        .unwrap();

        let target_json = dir.path().join("target.json");
        fs::write(
            &target_json,
            r#"{
            "name": "target",
            "dependencies": {
                "lodash": "^4.17.0"
            }
        }"#,
        )
        .await
        .unwrap();

        merge_package_json_dependencies(&source_json, &target_json)
            .await
            .unwrap();

        let result = fs::read_to_string(&target_json).await.unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        // lodash should keep target version
        assert_eq!(
            parsed["dependencies"]["lodash"].as_str(),
            Some("^4.17.0")
        );
        // axios should be added
        assert_eq!(parsed["dependencies"]["axios"].as_str(), Some("^1.0.0"));
        // typescript should be added to devDependencies
        assert_eq!(
            parsed["devDependencies"]["typescript"].as_str(),
            Some("^5.0.0")
        );
    }

    #[tokio::test]
    async fn test_update_imports_in_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.ts");

        fs::write(
            &file_path,
            r#"
import { foo } from 'old-package';
import { bar } from "old-package";
const x = require('old-package');
"#,
        )
        .await
        .unwrap();

        let mut files = 0;
        let mut replacements = 0;

        update_imports_in_single_file(
            &file_path,
            "old-package",
            "new-package",
            "module",
            &mut files,
            &mut replacements,
        )
        .await
        .unwrap();

        assert_eq!(files, 1);
        assert_eq!(replacements, 3);

        let content = fs::read_to_string(&file_path).await.unwrap();
        assert!(content.contains("from 'new-package/module'"));
        assert!(content.contains("from \"new-package/module\""));
        assert!(content.contains("require('new-package/module')"));
    }
}
