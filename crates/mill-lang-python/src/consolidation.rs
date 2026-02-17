//! Consolidation post-processing for Python package consolidation operations
//!
//! **LANGUAGE-SPECIFIC**: This module contains Python-specific logic for package consolidation.
//! It handles the post-processing tasks that must occur after moving files during a consolidation
//! operation.
//!
//! This module handles the post-processing tasks that must occur after moving
//! files during a consolidation operation:
//! 1. Flatten nested src/ directory structure
//! 2. Ensure __init__.py exists for Python packages
//! 3. Merge dependencies from source pyproject.toml
//! 4. Update imports across workspace
//! 5. Clean up workspace pyproject.toml

use mill_foundation::protocol::ConsolidationMetadata;
use mill_plugin_api::{PluginApiError, PluginResult};
use std::path::Path;
use tokio::fs;
use toml_edit::{value, Array, DocumentMut, Item, Table};
use tracing::{debug, info, warn};

/// Execute consolidation post-processing after directory move
///
/// This handles Python-specific consolidation tasks:
/// 1. Fix directory structure (flatten nested src/)
/// 2. Ensure __init__.py exists
/// 3. Merge pyproject.toml dependencies
/// 4. Update imports across workspace
/// 5. Clean up workspace pyproject.toml
pub async fn execute_consolidation_post_processing(
    metadata: &ConsolidationMetadata,
    project_root: &Path,
) -> PluginResult<()> {
    info!(
        source_package = %metadata.source_crate_name,
        target_package = %metadata.target_crate_name,
        target_module = %metadata.target_module_name,
        "Executing Python consolidation post-processing"
    );

    // Task 1: Fix nested src/ structure
    flatten_nested_src_directory(&metadata.target_module_path).await?;

    // Task 2: Ensure __init__.py exists
    ensure_init_py(&metadata.target_module_path).await?;

    // Task 3: Merge dependencies from source pyproject.toml
    let source_pyproject = Path::new(&metadata.source_crate_path).join("pyproject.toml");
    let target_pyproject = Path::new(&metadata.target_crate_path).join("pyproject.toml");

    if source_pyproject.exists() && target_pyproject.exists() {
        merge_pyproject_dependencies(&source_pyproject, &target_pyproject).await?;
    }

    // Task 4: Update imports across workspace
    update_imports_for_consolidation(
        &metadata.source_crate_name,
        &metadata.target_crate_name,
        &metadata.target_module_name,
        project_root,
    )
    .await?;

    // Task 5: Clean up workspace pyproject.toml (remove source from workspaces)
    cleanup_workspace_pyproject(&metadata.source_crate_path, project_root).await?;

    // Task 6: Remove leftover pyproject.toml from module directory
    let module_pyproject = Path::new(&metadata.target_module_path).join("pyproject.toml");
    if module_pyproject.exists() {
        fs::remove_file(&module_pyproject).await.map_err(|e| {
            PluginApiError::internal(format!("Failed to remove pyproject.toml: {}", e))
        })?;
        info!("Removed leftover pyproject.toml from module directory");
    }

    info!("Python consolidation post-processing complete");
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

/// Ensure __init__.py exists in the module directory
async fn ensure_init_py(module_path: &str) -> PluginResult<()> {
    let init_py = Path::new(module_path).join("__init__.py");

    if init_py.exists() {
        debug!(
            module_path = %module_path,
            "__init__.py already exists"
        );
        return Ok(());
    }

    fs::write(&init_py, "# Auto-generated by consolidation\n")
        .await
        .map_err(|e| PluginApiError::internal(format!("Failed to create __init__.py: {}", e)))?;

    info!(
        init_py = %init_py.display(),
        "Created __init__.py for module"
    );

    Ok(())
}

/// Merge dependencies from source pyproject.toml to target pyproject.toml
async fn merge_pyproject_dependencies(source_path: &Path, target_path: &Path) -> PluginResult<()> {
    info!(
        source = %source_path.display(),
        target = %target_path.display(),
        "Merging pyproject.toml dependencies"
    );

    let source_content = fs::read_to_string(source_path).await.map_err(|e| {
        PluginApiError::internal(format!("Failed to read source pyproject.toml: {}", e))
    })?;

    let target_content = fs::read_to_string(target_path).await.map_err(|e| {
        PluginApiError::internal(format!("Failed to read target pyproject.toml: {}", e))
    })?;

    let source: DocumentMut = source_content.parse().map_err(|e| {
        PluginApiError::internal(format!("Failed to parse source pyproject.toml: {}", e))
    })?;

    let mut target: DocumentMut = target_content.parse().map_err(|e| {
        PluginApiError::internal(format!("Failed to parse target pyproject.toml: {}", e))
    })?;

    let mut modified = false;

    // Merge [project.dependencies] (PEP 621)
    if let Some(source_deps) = source
        .get("project")
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_array())
    {
        modified |= merge_dependencies_array(&mut target, source_deps, "project", "dependencies")?;
    }

    // Merge [tool.poetry.dependencies] (Poetry)
    if let Some(source_deps) = source
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_table())
    {
        modified |= merge_poetry_dependencies(&mut target, source_deps)?;
    }

    // Merge [project.optional-dependencies] (PEP 621)
    if let Some(source_optional) = source
        .get("project")
        .and_then(|p| p.get("optional-dependencies"))
        .and_then(|o| o.as_table())
    {
        for (group, deps) in source_optional.iter() {
            if let Some(deps_array) = deps.as_array() {
                modified |= merge_optional_dependencies(&mut target, group, deps_array)?;
            }
        }
    }

    if modified {
        fs::write(target_path, target.to_string())
            .await
            .map_err(|e| {
                PluginApiError::internal(format!("Failed to write target pyproject.toml: {}", e))
            })?;

        info!("Dependencies merged successfully");
    } else {
        info!("No new dependencies to merge");
    }

    Ok(())
}

/// Merge an array of dependencies into the target document
fn merge_dependencies_array(
    target: &mut DocumentMut,
    source_deps: &toml_edit::Array,
    section: &str,
    key: &str,
) -> PluginResult<bool> {
    // Ensure section exists
    if !target.contains_key(section) {
        target[section] = Item::Table(Table::new());
    }

    let section_table = target[section]
        .as_table_mut()
        .ok_or_else(|| PluginApiError::internal(format!("[{}] is not a table", section)))?;

    // Get or create dependencies array
    let deps_array = if section_table.contains_key(key) {
        section_table[key].as_array_mut().ok_or_else(|| {
            PluginApiError::internal(format!("[{}.{}] is not an array", section, key))
        })?
    } else {
        section_table[key] = value(Array::new());
        section_table[key].as_array_mut().unwrap()
    };

    let mut modified = false;

    for dep in source_deps.iter() {
        if let Some(dep_str) = dep.as_str() {
            // Extract package name from dependency string (e.g., "requests>=2.0" -> "requests")
            let pkg_name = extract_package_name(dep_str);

            // Check if dependency already exists
            let already_exists = deps_array.iter().any(|d| {
                d.as_str()
                    .map(|s| extract_package_name(s) == pkg_name)
                    .unwrap_or(false)
            });

            if !already_exists {
                deps_array.push(dep_str);
                modified = true;
                debug!(dependency = %dep_str, "Added dependency from source");
            }
        }
    }

    Ok(modified)
}

/// Merge Poetry dependencies (table format)
fn merge_poetry_dependencies(
    target: &mut DocumentMut,
    source_deps: &toml_edit::Table,
) -> PluginResult<bool> {
    // Ensure [tool.poetry.dependencies] exists
    if !target.contains_key("tool") {
        target["tool"] = Item::Table(Table::new());
    }

    let tool = target["tool"]
        .as_table_mut()
        .ok_or_else(|| PluginApiError::internal("[tool] is not a table"))?;

    if !tool.contains_key("poetry") {
        tool["poetry"] = Item::Table(Table::new());
    }

    let poetry = tool["poetry"]
        .as_table_mut()
        .ok_or_else(|| PluginApiError::internal("[tool.poetry] is not a table"))?;

    if !poetry.contains_key("dependencies") {
        poetry["dependencies"] = Item::Table(Table::new());
    }

    let deps = poetry["dependencies"]
        .as_table_mut()
        .ok_or_else(|| PluginApiError::internal("[tool.poetry.dependencies] is not a table"))?;

    let mut modified = false;

    for (name, version) in source_deps.iter() {
        // Skip python version constraint
        if name == "python" {
            continue;
        }

        if !deps.contains_key(name) {
            deps.insert(name, version.clone());
            modified = true;
            debug!(dependency = %name, "Added Poetry dependency from source");
        }
    }

    Ok(modified)
}

/// Merge optional dependencies
fn merge_optional_dependencies(
    target: &mut DocumentMut,
    group: &str,
    source_deps: &toml_edit::Array,
) -> PluginResult<bool> {
    // Ensure [project.optional-dependencies] exists
    if !target.contains_key("project") {
        target["project"] = Item::Table(Table::new());
    }

    let project = target["project"]
        .as_table_mut()
        .ok_or_else(|| PluginApiError::internal("[project] is not a table"))?;

    if !project.contains_key("optional-dependencies") {
        project["optional-dependencies"] = Item::Table(Table::new());
    }

    let optional = project["optional-dependencies"]
        .as_table_mut()
        .ok_or_else(|| {
            PluginApiError::internal("[project.optional-dependencies] is not a table")
        })?;

    // Get or create the group array
    let group_array = if optional.contains_key(group) {
        optional[group].as_array_mut().ok_or_else(|| {
            PluginApiError::internal(format!(
                "[project.optional-dependencies.{}] is not an array",
                group
            ))
        })?
    } else {
        optional[group] = value(Array::new());
        optional[group].as_array_mut().unwrap()
    };

    let mut modified = false;

    for dep in source_deps.iter() {
        if let Some(dep_str) = dep.as_str() {
            let pkg_name = extract_package_name(dep_str);

            let already_exists = group_array.iter().any(|d| {
                d.as_str()
                    .map(|s| extract_package_name(s) == pkg_name)
                    .unwrap_or(false)
            });

            if !already_exists {
                group_array.push(dep_str);
                modified = true;
                debug!(group = %group, dependency = %dep_str, "Added optional dependency from source");
            }
        }
    }

    Ok(modified)
}

/// Extract package name from a PEP 508 dependency string
fn extract_package_name(dep: &str) -> &str {
    dep.split(['>', '<', '=', '!', '[', ';', ' '])
        .next()
        .unwrap_or(dep)
        .trim()
}

/// Update imports across workspace for consolidation
///
/// When consolidating packages, all imports need to be updated:
/// - `from old_package import x` → `from new_package.module import x`
/// - `import old_package` → `import new_package.module`
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

    // Convert package names to Python module format (hyphens to underscores)
    let source_module = source_package_name.replace('-', "_");
    let target_module_full = format!(
        "{}_{}",
        target_package_name.replace('-', "_"),
        target_module_name
    );

    let mut files_updated = 0;
    let mut total_replacements = 0;

    update_imports_in_workspace_directory(
        project_root,
        &source_module,
        &target_package_name.replace('-', "_"),
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

    let _ = target_module_full; // suppress warning
    Ok(())
}

/// Recursively update imports in workspace directory
async fn update_imports_in_workspace_directory(
    dir: &Path,
    source_module: &str,
    target_package: &str,
    target_module: &str,
    files_updated: &mut usize,
    total_replacements: &mut usize,
) -> PluginResult<()> {
    // Skip common directories
    let dir_name = dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if matches!(
        dir_name,
        ".git"
            | ".venv"
            | "venv"
            | "__pycache__"
            | ".mypy_cache"
            | ".pytest_cache"
            | "dist"
            | "build"
            | "node_modules"
            | ".tox"
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
                source_module,
                target_package,
                target_module,
                files_updated,
                total_replacements,
            ))
            .await?;
        } else {
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if ext == "py" {
                update_imports_in_single_file(
                    &path,
                    source_module,
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

/// Update imports in a single Python file
async fn update_imports_in_single_file(
    file_path: &Path,
    source_module: &str,
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

    // Skip if file doesn't contain the source module
    if !content.contains(source_module) {
        return Ok(());
    }

    let mut new_content = content.clone();
    let mut replacements = 0;

    // Pattern 1: from source_module import x → from target_package.target_module import x
    let from_pattern = format!("from {} import", source_module);
    let from_replacement = format!("from {}.{} import", target_package, target_module);
    if new_content.contains(&from_pattern) {
        let count = new_content.matches(&from_pattern).count();
        new_content = new_content.replace(&from_pattern, &from_replacement);
        replacements += count;
    }

    // Pattern 2: import source_module → import target_package.target_module as source_module
    // This is trickier - we need to handle cases like:
    // - import source_module
    // - import source_module as alias
    // For simplicity, we'll handle the common case
    let import_pattern = format!("import {}", source_module);
    if new_content.contains(&import_pattern) {
        // Find lines that match and update them
        let lines: Vec<&str> = new_content.lines().collect();
        let mut new_lines = Vec::new();

        for line in &lines {
            let trimmed = line.trim();
            if trimmed == format!("import {}", source_module) {
                // import source_module → import target_package.target_module as source_module
                let indent = line.len() - line.trim_start().len();
                let spaces: String = line.chars().take(indent).collect();
                new_lines.push(format!(
                    "{}import {}.{} as {}",
                    spaces, target_package, target_module, source_module
                ));
                replacements += 1;
            } else if trimmed.starts_with(&format!("import {} ", source_module)) {
                // import source_module as alias → import target_package.target_module as alias
                let new_line = line.replace(
                    &format!("import {}", source_module),
                    &format!("import {}.{}", target_package, target_module),
                );
                new_lines.push(new_line);
                replacements += 1;
            } else {
                new_lines.push((*line).to_string());
            }
        }

        new_content = new_lines.join("\n");
        // Preserve trailing newline
        if content.ends_with('\n') && !new_content.ends_with('\n') {
            new_content.push('\n');
        }
    }

    // Pattern 3: source_module.x → target_package.target_module.x (qualified usage)
    // This is complex to do safely, so we'll skip it for now
    // The import changes should cover most use cases

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

/// Clean up workspace pyproject.toml after consolidation
///
/// Removes source package from PDM workspace members
async fn cleanup_workspace_pyproject(
    source_package_path: &str,
    project_root: &Path,
) -> PluginResult<()> {
    let workspace_toml = project_root.join("pyproject.toml");

    if !workspace_toml.exists() {
        warn!("Workspace pyproject.toml not found, skipping cleanup");
        return Ok(());
    }

    // Calculate relative path from project root
    let source_path = Path::new(source_package_path);
    let source_relative = source_path
        .strip_prefix(project_root)
        .unwrap_or(source_path)
        .to_string_lossy()
        .to_string();

    let content = fs::read_to_string(&workspace_toml).await.map_err(|e| {
        PluginApiError::internal(format!("Failed to read workspace pyproject.toml: {}", e))
    })?;

    let mut doc: DocumentMut = content.parse().map_err(|e| {
        PluginApiError::internal(format!("Failed to parse workspace pyproject.toml: {}", e))
    })?;

    let mut modified = false;

    // Handle PDM workspace: [tool.pdm.workspace.members]
    if let Some(members) = doc
        .get_mut("tool")
        .and_then(|t| t.get_mut("pdm"))
        .and_then(|p| p.get_mut("workspace"))
        .and_then(|w| w.get_mut("members"))
        .and_then(|m| m.as_array_mut())
    {
        let before_len = members.len();
        members.retain(|v| {
            if let Some(s) = v.as_str() {
                s != source_relative && s != source_package_path
            } else {
                true
            }
        });

        if members.len() < before_len {
            modified = true;
            info!(
                source_path = %source_relative,
                "Removed from PDM workspace members"
            );
        }
    }

    // Handle Poetry workspace: [tool.poetry.packages]
    if let Some(packages) = doc
        .get_mut("tool")
        .and_then(|t| t.get_mut("poetry"))
        .and_then(|p| p.get_mut("packages"))
        .and_then(|p| p.as_array_of_tables_mut())
    {
        let before_len = packages.len();
        packages.retain(|pkg| {
            if let Some(from) = pkg.get("from").and_then(|f| f.as_str()) {
                from != source_relative && from != source_package_path
            } else {
                true
            }
        });

        if packages.len() < before_len {
            modified = true;
            info!(
                source_path = %source_relative,
                "Removed from Poetry packages"
            );
        }
    }

    if modified {
        fs::write(&workspace_toml, doc.to_string())
            .await
            .map_err(|e| {
                PluginApiError::internal(format!("Failed to write workspace pyproject.toml: {}", e))
            })?;

        info!("Workspace pyproject.toml cleanup complete");
    } else {
        info!("No workspace pyproject.toml cleanup needed");
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

        let file1 = src_dir.join("main.py");
        fs::write(&file1, "def main(): pass").await.unwrap();

        flatten_nested_src_directory(dir.path().to_str().unwrap())
            .await
            .unwrap();

        assert!(!src_dir.exists(), "src/ should be removed");
        assert!(
            dir.path().join("main.py").exists(),
            "main.py should be moved up"
        );
    }

    #[tokio::test]
    async fn test_ensure_init_py() {
        let dir = tempdir().unwrap();

        ensure_init_py(dir.path().to_str().unwrap()).await.unwrap();

        assert!(
            dir.path().join("__init__.py").exists(),
            "__init__.py should be created"
        );
    }

    #[tokio::test]
    async fn test_extract_package_name() {
        assert_eq!(extract_package_name("requests>=2.0"), "requests");
        assert_eq!(extract_package_name("numpy==1.21.0"), "numpy");
        assert_eq!(extract_package_name("pandas[sql]>=1.0"), "pandas");
        assert_eq!(
            extract_package_name("pytest ; python_version >= '3.8'"),
            "pytest"
        );
        assert_eq!(extract_package_name("simple_package"), "simple_package");
    }

    #[tokio::test]
    async fn test_update_imports_in_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.py");

        fs::write(
            &file_path,
            r#"
from old_package import foo
import old_package
from old_package import bar, baz
"#,
        )
        .await
        .unwrap();

        let mut files = 0;
        let mut replacements = 0;

        update_imports_in_single_file(
            &file_path,
            "old_package",
            "new_package",
            "module",
            &mut files,
            &mut replacements,
        )
        .await
        .unwrap();

        assert_eq!(files, 1);
        assert!(replacements >= 2); // At least the from imports

        let content = fs::read_to_string(&file_path).await.unwrap();
        assert!(content.contains("from new_package.module import foo"));
        assert!(content.contains("from new_package.module import bar, baz"));
    }
}
