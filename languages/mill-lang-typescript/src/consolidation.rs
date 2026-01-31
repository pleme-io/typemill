//! Consolidation post-processing for TypeScript/npm package consolidation operations
//!
//! **LANGUAGE-SPECIFIC**: This module contains TypeScript/npm-specific logic for package
//! consolidation. It handles the post-processing tasks that must occur after moving files
//! during a consolidation operation.
//!
//! This module handles:
//! 1. Flattening nested src/ directory structure (if applicable)
//! 2. Renaming index.ts to mod.ts for directory modules (optional)
//! 3. Adding module export to target package's index.ts
//! 4. Merging package.json dependencies
//! 5. Updating imports across workspace
//! 6. Cleaning up workspace configuration
//!
//! # TypeScript vs Rust Consolidation
//!
//! While Rust has a strict module system with lib.rs/mod.rs conventions, TypeScript
//! is more flexible:
//! - Entry points can be index.ts, index.js, or configured in package.json "main"
//! - ES6 modules use import/export with relative paths or package names
//! - Workspace configuration varies between npm, yarn, and pnpm

use crate::manifest::{merge_package_json_dependencies, parse_package_json};
use mill_plugin_api::{PluginApiError, PluginResult};
use std::path::Path;
use tokio::fs;
use tracing::{debug, info, warn};

/// Consolidation metadata for TypeScript/npm packages
#[derive(Debug, Clone)]
pub struct TypeScriptConsolidationMetadata {
    /// Whether this is a consolidation operation
    pub is_consolidation: bool,
    /// Name of the source package being consolidated
    pub source_package_name: String,
    /// Absolute path to source package root
    pub source_package_path: String,
    /// Name of the target package receiving the code
    pub target_package_name: String,
    /// Absolute path to target package root
    pub target_package_path: String,
    /// Module name in target package (directory name)
    pub target_module_name: String,
    /// Absolute path to target module directory
    pub target_module_path: String,
}

impl From<&mill_foundation::protocol::ConsolidationMetadata> for TypeScriptConsolidationMetadata {
    fn from(metadata: &mill_foundation::protocol::ConsolidationMetadata) -> Self {
        Self {
            is_consolidation: metadata.is_consolidation,
            source_package_name: metadata.source_crate_name.clone(),
            source_package_path: metadata.source_crate_path.clone(),
            target_package_name: metadata.target_crate_name.clone(),
            target_package_path: metadata.target_crate_path.clone(),
            target_module_name: metadata.target_module_name.clone(),
            target_module_path: metadata.target_module_path.clone(),
        }
    }
}

/// Execute consolidation post-processing after directory move for TypeScript/npm packages
///
/// This handles TypeScript-specific consolidation tasks:
/// 1. Fix directory structure (flatten nested src/ if applicable)
/// 2. Merge package.json dependencies
/// 3. Add module export to target index.ts
/// 4. Update imports across workspace
/// 5. Clean up workspace configuration
/// 6. Remove source package from target's dependencies
pub async fn execute_consolidation_post_processing(
    metadata: &TypeScriptConsolidationMetadata,
    project_root: &Path,
) -> PluginResult<()> {
    info!(
        source_package = %metadata.source_package_name,
        target_package = %metadata.target_package_name,
        target_module = %metadata.target_module_name,
        "Executing TypeScript consolidation post-processing"
    );

    // Task 1: Flatten nested src/ directory structure
    flatten_nested_src_directory(&metadata.target_module_path).await?;

    // Task 2: Merge dependencies from source package.json
    let source_package_json = Path::new(&metadata.source_package_path).join("package.json");
    let target_package_json = Path::new(&metadata.target_package_path).join("package.json");

    if source_package_json.exists() && target_package_json.exists() {
        merge_package_json_deps(&source_package_json, &target_package_json).await?;
    }

    // Task 3: Add module export to target index.ts
    add_module_export_to_target_index(
        &metadata.target_package_path,
        &metadata.target_module_name,
    )
    .await?;

    // Task 4: Update imports across workspace for consolidation
    update_imports_for_consolidation(
        &metadata.source_package_name,
        &metadata.target_package_name,
        &metadata.target_module_name,
        project_root,
    )
    .await?;

    // Task 5: Clean up workspace configuration
    cleanup_workspace_config(
        &metadata.source_package_path,
        &metadata.source_package_name,
        project_root,
    )
    .await?;

    // Task 6: Remove source package dependency from target's package.json
    remove_source_dependency_from_target(&metadata.source_package_name, &metadata.target_package_path)
        .await?;

    info!("TypeScript consolidation post-processing complete");
    Ok(())
}

/// Flatten nested src/ directory structure if present
///
/// When moving a package into another package's src/ directory, we may end up with
/// target/src/module/src/* which should be flattened to target/src/module/*
async fn flatten_nested_src_directory(module_path: &str) -> PluginResult<()> {
    let module_dir = Path::new(module_path);
    let nested_src = module_dir.join("src");

    if !nested_src.exists() {
        debug!(
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

        debug!(
            file = %file_name.to_string_lossy(),
            "Moved file from nested src/"
        );
    }

    // Remove empty src/ directory
    fs::remove_dir(&nested_src)
        .await
        .map_err(|e| PluginApiError::internal(format!("Failed to remove empty src/: {}", e)))?;

    // Remove package.json if it exists (should be merged already)
    let package_json = module_dir.join("package.json");
    if package_json.exists() {
        fs::remove_file(&package_json)
            .await
            .map_err(|e| PluginApiError::internal(format!("Failed to remove package.json: {}", e)))?;
        info!("Removed leftover package.json from module directory");
    }

    Ok(())
}

/// Merge dependencies from source package.json into target package.json
async fn merge_package_json_deps(
    source_package_json: &Path,
    target_package_json: &Path,
) -> PluginResult<()> {
    info!(
        source = %source_package_json.display(),
        target = %target_package_json.display(),
        "Merging package.json dependencies (consolidation)"
    );

    // Read both package.json files
    let source_content = fs::read_to_string(source_package_json).await.map_err(|e| {
        PluginApiError::internal(format!("Failed to read source package.json: {}", e))
    })?;

    let target_content = fs::read_to_string(target_package_json).await.map_err(|e| {
        PluginApiError::internal(format!("Failed to read target package.json: {}", e))
    })?;

    // Merge dependencies using the manifest module function
    let merged_content = merge_package_json_dependencies(&target_content, &source_content)?;

    // Write merged content back to target
    fs::write(target_package_json, &merged_content)
        .await
        .map_err(|e| {
            PluginApiError::internal(format!("Failed to write merged package.json: {}", e))
        })?;

    info!("Completed package.json dependency merge");
    Ok(())
}

/// Add module export to target package's index.ts
///
/// After consolidation, add an export statement to expose the consolidated module:
/// `export * from './module-name';` or `export { ... } from './module-name';`
async fn add_module_export_to_target_index(
    target_package_path: &str,
    module_name: &str,
) -> PluginResult<()> {
    // Try common entry point locations
    let entry_points = ["src/index.ts", "src/index.js", "index.ts", "index.js"];
    let target_path = Path::new(target_package_path);

    for entry_point in entry_points {
        let index_path = target_path.join(entry_point);
        if index_path.exists() {
            let content = fs::read_to_string(&index_path)
                .await
                .map_err(|e| PluginApiError::internal(format!("Failed to read {}: {}", entry_point, e)))?;

            // Check if export already exists
            let export_statement = format!("export * from './{}';", module_name);
            let export_statement_alt = format!("export * from \"./{}\";", module_name);

            if content.contains(&export_statement) || content.contains(&export_statement_alt) {
                debug!(
                    module = %module_name,
                    entry_point = %entry_point,
                    "Module export already exists, skipping"
                );
                return Ok(());
            }

            // Find insertion point (after last export/import statement, or at end)
            let lines: Vec<&str> = content.lines().collect();
            let mut insertion_line = lines.len();

            // Find the last import/export line to insert after it
            for (i, line) in lines.iter().enumerate().rev() {
                let trimmed = line.trim();
                if trimmed.starts_with("export ") || trimmed.starts_with("import ") {
                    insertion_line = i + 1;
                    break;
                }
            }

            // Insert the export statement
            let mut new_lines = lines.clone();
            new_lines.insert(insertion_line, &export_statement);
            let new_content = new_lines.join("\n");

            // Preserve trailing newline if original had one
            let final_content = if content.ends_with('\n') {
                format!("{}\n", new_content)
            } else {
                new_content
            };

            fs::write(&index_path, final_content)
                .await
                .map_err(|e| PluginApiError::internal(format!("Failed to write {}: {}", entry_point, e)))?;

            info!(
                entry_point = %entry_point,
                module = %module_name,
                "Added module export to target index"
            );
            return Ok(());
        }
    }

    warn!(
        target_package_path = %target_package_path,
        "No entry point found (index.ts/index.js), skipping module export"
    );
    Ok(())
}

/// Update imports across workspace for consolidation
///
/// When consolidating packages, all imports need to be updated:
/// - `import { foo } from 'old-package';` -> `import { foo } from 'new-package/module';`
/// - `import foo from 'old-package';` -> `import foo from 'new-package/module';`
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

    update_imports_in_directory(
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

/// Recursively update imports in a directory
async fn update_imports_in_directory(
    dir: &Path,
    source_package_name: &str,
    target_package_name: &str,
    target_module_name: &str,
    files_updated: &mut usize,
    total_replacements: &mut usize,
) -> PluginResult<()> {
    // Skip common non-source directories
    let dir_name = dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if matches!(
        dir_name,
        "node_modules" | ".git" | "dist" | "build" | "coverage" | ".next" | ".nuxt"
    ) {
        return Ok(());
    }

    let entries_result = fs::read_dir(dir).await;
    if entries_result.is_err() {
        return Ok(()); // Skip directories we can't read
    }

    let mut entries = entries_result.unwrap();

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| PluginApiError::internal(format!("Failed to iterate directory: {}", e)))?
    {
        let path = entry.path();

        if path.is_dir() {
            Box::pin(update_imports_in_directory(
                &path,
                source_package_name,
                target_package_name,
                target_module_name,
                files_updated,
                total_replacements,
            ))
            .await?;
        } else {
            let ext = path.extension().and_then(|s| s.to_str());
            if matches!(ext, Some("ts") | Some("tsx") | Some("js") | Some("jsx") | Some("mjs") | Some("cjs")) {
                update_imports_in_file(
                    &path,
                    source_package_name,
                    target_package_name,
                    target_module_name,
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
async fn update_imports_in_file(
    file_path: &Path,
    source_package_name: &str,
    target_package_name: &str,
    target_module_name: &str,
    files_updated: &mut usize,
    total_replacements: &mut usize,
) -> PluginResult<()> {
    let content_result = fs::read_to_string(file_path).await;
    if content_result.is_err() {
        return Ok(()); // Skip files we can't read
    }

    let content = content_result.unwrap();

    // Skip if file doesn't contain the source package name
    if !content.contains(source_package_name) {
        return Ok(());
    }

    let mut new_content = content.clone();
    let mut replacement_count = 0;

    // Build the new import path
    let new_import_path = format!("{}/{}", target_package_name, target_module_name);

    // Pattern 1: from 'source-package' -> from 'target-package/module'
    // Pattern 2: from "source-package" -> from "target-package/module"
    for quote in ['\'', '"'] {
        let old_pattern = format!("from {}{}{}", quote, source_package_name, quote);
        let new_pattern = format!("from {}{}{}", quote, new_import_path, quote);

        let count = new_content.matches(&old_pattern).count();
        if count > 0 {
            new_content = new_content.replace(&old_pattern, &new_pattern);
            replacement_count += count;
        }
    }

    // Pattern 3: require('source-package') -> require('target-package/module')
    // Pattern 4: require("source-package") -> require("target-package/module")
    for quote in ['\'', '"'] {
        let old_pattern = format!("require({}{}{})", quote, source_package_name, quote);
        let new_pattern = format!("require({}{}{})", quote, new_import_path, quote);

        let count = new_content.matches(&old_pattern).count();
        if count > 0 {
            new_content = new_content.replace(&old_pattern, &new_pattern);
            replacement_count += count;
        }
    }

    // Pattern 5: import('source-package') -> import('target-package/module') (dynamic imports)
    for quote in ['\'', '"'] {
        let old_pattern = format!("import({}{}{})", quote, source_package_name, quote);
        let new_pattern = format!("import({}{}{})", quote, new_import_path, quote);

        let count = new_content.matches(&old_pattern).count();
        if count > 0 {
            new_content = new_content.replace(&old_pattern, &new_pattern);
            replacement_count += count;
        }
    }

    if replacement_count > 0 {
        fs::write(file_path, new_content).await.map_err(|e| {
            PluginApiError::internal(format!("Failed to write {}: {}", file_path.display(), e))
        })?;

        *files_updated += 1;
        *total_replacements += replacement_count;

        info!(
            file = %file_path.display(),
            replacements = replacement_count,
            "Updated imports for consolidation"
        );
    }

    Ok(())
}

/// Clean up workspace configuration after consolidation
///
/// Removes the source package from workspace configuration:
/// - For npm/yarn: package.json "workspaces" array
/// - For pnpm: pnpm-workspace.yaml "packages" list
async fn cleanup_workspace_config(
    source_package_path: &str,
    source_package_name: &str,
    project_root: &Path,
) -> PluginResult<()> {
    // Try to find and update workspace root package.json
    let root_package_json = project_root.join("package.json");
    if root_package_json.exists() {
        if let Ok(content) = fs::read_to_string(&root_package_json).await {
            if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                let mut modified = false;

                // Calculate relative path from project root
                let source_path = Path::new(source_package_path);
                let relative_path = source_path
                    .strip_prefix(project_root)
                    .unwrap_or(source_path)
                    .to_string_lossy()
                    .to_string();

                // Remove from workspaces array
                if let Some(workspaces) = json.get_mut("workspaces") {
                    match workspaces {
                        serde_json::Value::Array(arr) => {
                            let before_len = arr.len();
                            arr.retain(|v| {
                                v.as_str() != Some(&relative_path) &&
                                v.as_str() != Some(source_package_name)
                            });
                            if arr.len() < before_len {
                                modified = true;
                                info!(
                                    source_package = %source_package_name,
                                    "Removed from workspace members"
                                );
                            }
                        }
                        serde_json::Value::Object(obj) => {
                            if let Some(serde_json::Value::Array(packages)) = obj.get_mut("packages") {
                                let before_len = packages.len();
                                packages.retain(|v| {
                                    v.as_str() != Some(&relative_path) &&
                                    v.as_str() != Some(source_package_name)
                                });
                                if packages.len() < before_len {
                                    modified = true;
                                    info!(
                                        source_package = %source_package_name,
                                        "Removed from workspace packages"
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }

                if modified {
                    let updated_content = serde_json::to_string_pretty(&json)
                        .map_err(|e| PluginApiError::internal(format!("Failed to serialize package.json: {}", e)))?;

                    fs::write(&root_package_json, format!("{}\n", updated_content))
                        .await
                        .map_err(|e| PluginApiError::internal(format!("Failed to write package.json: {}", e)))?;
                }
            }
        }
    }

    // Try to update pnpm-workspace.yaml if it exists
    let pnpm_workspace = project_root.join("pnpm-workspace.yaml");
    if pnpm_workspace.exists() {
        if let Ok(content) = fs::read_to_string(&pnpm_workspace).await {
            let source_path = Path::new(source_package_path);
            let relative_path = source_path
                .strip_prefix(project_root)
                .unwrap_or(source_path)
                .to_string_lossy()
                .to_string();

            // Simple line-based removal for pnpm workspace
            let new_content: String = content
                .lines()
                .filter(|line| {
                    let trimmed = line.trim().trim_start_matches('-').trim();
                    let unquoted = trimmed.trim_matches('\'').trim_matches('"');
                    unquoted != relative_path && unquoted != source_package_name
                })
                .collect::<Vec<_>>()
                .join("\n");

            if new_content != content {
                fs::write(&pnpm_workspace, format!("{}\n", new_content))
                    .await
                    .map_err(|e| PluginApiError::internal(format!("Failed to write pnpm-workspace.yaml: {}", e)))?;

                info!(
                    source_package = %source_package_name,
                    "Removed from pnpm workspace"
                );
            }
        }
    }

    Ok(())
}

/// Remove source package dependency from target package's package.json
///
/// After consolidation, the target package should no longer depend on the source package
/// since the source code is now part of the target package.
async fn remove_source_dependency_from_target(
    source_package_name: &str,
    target_package_path: &str,
) -> PluginResult<()> {
    let target_package_json = Path::new(target_package_path).join("package.json");

    if !target_package_json.exists() {
        warn!(
            target_path = %target_package_path,
            "Target package.json not found, skipping dependency removal"
        );
        return Ok(());
    }

    let content = fs::read_to_string(&target_package_json).await.map_err(|e| {
        PluginApiError::internal(format!("Failed to read target package.json: {}", e))
    })?;

    let mut json: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        PluginApiError::internal(format!("Failed to parse target package.json: {}", e))
    })?;

    let mut modified = false;

    // Remove from all dependency sections
    for section in ["dependencies", "devDependencies", "peerDependencies", "optionalDependencies"] {
        if let Some(deps) = json.get_mut(section).and_then(|d| d.as_object_mut()) {
            if deps.remove(source_package_name).is_some() {
                modified = true;
                info!(
                    source_package = %source_package_name,
                    section = %section,
                    "Removed source package dependency from target"
                );
            }
        }
    }

    if modified {
        let updated_content = serde_json::to_string_pretty(&json)
            .map_err(|e| PluginApiError::internal(format!("Failed to serialize package.json: {}", e)))?;

        fs::write(&target_package_json, format!("{}\n", updated_content))
            .await
            .map_err(|e| PluginApiError::internal(format!("Failed to write package.json: {}", e)))?;

        info!("Target package.json dependency cleanup complete");
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
        let module_dir = dir.path().join("module");
        fs::create_dir(&module_dir).await.unwrap();

        let src_dir = module_dir.join("src");
        fs::create_dir(&src_dir).await.unwrap();

        let file1 = src_dir.join("file1.ts");
        fs::write(&file1, "export const x = 1;").await.unwrap();

        flatten_nested_src_directory(module_dir.to_str().unwrap())
            .await
            .unwrap();

        assert!(!src_dir.exists(), "src/ should be removed");
        assert!(
            module_dir.join("file1.ts").exists(),
            "file1.ts should be moved up"
        );
    }

    #[tokio::test]
    async fn test_add_module_export_to_target_index() {
        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("src");
        fs::create_dir(&src_dir).await.unwrap();

        let index_path = src_dir.join("index.ts");
        fs::write(&index_path, "export * from './existing';\n").await.unwrap();

        add_module_export_to_target_index(dir.path().to_str().unwrap(), "new_module")
            .await
            .unwrap();

        let content = fs::read_to_string(&index_path).await.unwrap();
        assert!(
            content.contains("export * from './new_module';"),
            "Should contain new module export"
        );
        assert!(
            content.contains("export * from './existing';"),
            "Should preserve existing export"
        );
    }

    #[tokio::test]
    async fn test_add_module_export_skips_if_exists() {
        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("src");
        fs::create_dir(&src_dir).await.unwrap();

        let index_path = src_dir.join("index.ts");
        let original_content = "export * from './existing';\nexport * from './my_module';\n";
        fs::write(&index_path, original_content).await.unwrap();

        add_module_export_to_target_index(dir.path().to_str().unwrap(), "my_module")
            .await
            .unwrap();

        let content = fs::read_to_string(&index_path).await.unwrap();
        // Should not duplicate the export
        assert_eq!(
            content.matches("export * from './my_module';").count(),
            1,
            "Should not duplicate existing export"
        );
    }

    #[tokio::test]
    async fn test_remove_source_dependency_from_target() {
        let dir = tempdir().unwrap();
        let package_json = dir.path().join("package.json");

        fs::write(
            &package_json,
            r#"{
  "name": "target-package",
  "dependencies": {
    "source-package": "^1.0.0",
    "other-dep": "^2.0.0"
  },
  "devDependencies": {
    "source-package": "^1.0.0"
  }
}"#,
        )
        .await
        .unwrap();

        remove_source_dependency_from_target("source-package", dir.path().to_str().unwrap())
            .await
            .unwrap();

        let content = fs::read_to_string(&package_json).await.unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Source package should be removed from both sections
        assert!(
            !json["dependencies"]
                .as_object()
                .unwrap()
                .contains_key("source-package"),
            "Should remove source-package from dependencies"
        );
        assert!(
            !json["devDependencies"]
                .as_object()
                .unwrap()
                .contains_key("source-package"),
            "Should remove source-package from devDependencies"
        );
        // Other deps should be preserved
        assert!(
            json["dependencies"]
                .as_object()
                .unwrap()
                .contains_key("other-dep"),
            "Should preserve other-dep"
        );
    }

    #[tokio::test]
    async fn test_update_imports_in_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.ts");

        fs::write(
            &file_path,
            r#"import { foo } from 'old-package';
import bar from "old-package";
const x = require('old-package');
const y = await import('old-package');
import { other } from 'unrelated-package';
"#,
        )
        .await
        .unwrap();

        let mut files_updated = 0;
        let mut total_replacements = 0;

        update_imports_in_file(
            &file_path,
            "old-package",
            "new-package",
            "module",
            &mut files_updated,
            &mut total_replacements,
        )
        .await
        .unwrap();

        assert_eq!(files_updated, 1, "Should update 1 file");
        assert_eq!(total_replacements, 4, "Should make 4 replacements");

        let content = fs::read_to_string(&file_path).await.unwrap();
        assert!(content.contains("from 'new-package/module'"));
        assert!(content.contains("from \"new-package/module\""));
        assert!(content.contains("require('new-package/module')"));
        assert!(content.contains("import('new-package/module')"));
        assert!(content.contains("from 'unrelated-package'")); // Should not change
    }

    #[tokio::test]
    async fn test_cleanup_workspace_config() {
        let dir = tempdir().unwrap();
        let packages_dir = dir.path().join("packages");
        fs::create_dir(&packages_dir).await.unwrap();
        let source_dir = packages_dir.join("source-package");
        fs::create_dir(&source_dir).await.unwrap();

        // Create root package.json with workspaces
        let root_package_json = dir.path().join("package.json");
        fs::write(
            &root_package_json,
            r#"{
  "name": "workspace-root",
  "workspaces": [
    "packages/source-package",
    "packages/target-package"
  ]
}"#,
        )
        .await
        .unwrap();

        cleanup_workspace_config(
            source_dir.to_str().unwrap(),
            "source-package",
            dir.path(),
        )
        .await
        .unwrap();

        let content = fs::read_to_string(&root_package_json).await.unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        let workspaces = json["workspaces"].as_array().unwrap();
        assert_eq!(workspaces.len(), 1, "Should have 1 workspace member");
        assert_eq!(
            workspaces[0].as_str().unwrap(),
            "packages/target-package",
            "Should keep target-package"
        );
    }
}
