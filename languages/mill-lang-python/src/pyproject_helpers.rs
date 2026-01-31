//! Python pyproject.toml helpers for consolidation operations
//!
//! This module provides utilities for merging Python package dependencies during
//! consolidation operations. It supports multiple Python packaging formats:
//!
//! - **PEP 621** (`[project.dependencies]`) - Standard Python packaging format
//! - **Poetry** (`[tool.poetry.dependencies]`) - Poetry dependency management
//!
//! # Consolidation Workflow
//!
//! When consolidating Python packages, dependencies from the source package need
//! to be merged into the target package. This module handles:
//!
//! 1. Parsing dependencies from source pyproject.toml
//! 2. Merging into target pyproject.toml
//! 3. Handling version conflicts (keeps existing version with warning)
//! 4. Supporting both regular and optional/dev dependencies

use mill_plugin_api::{PluginApiError, PluginResult};
use std::path::Path;
use tokio::fs;
use toml_edit::{Array, DocumentMut, Item, Table, Value};
use tracing::{debug, info, warn};

/// Result of a dependency merge operation
#[derive(Debug, Default)]
pub struct MergeResult {
    /// Number of dependencies successfully merged
    pub merged_count: usize,
    /// Number of dependencies skipped due to conflicts
    pub conflict_count: usize,
    /// Dependencies that were skipped (name, reason)
    pub skipped: Vec<(String, String)>,
}

/// Merge dependencies from source pyproject.toml into target pyproject.toml
///
/// This function handles both PEP 621 and Poetry formats, auto-detecting the format
/// of both source and target files.
///
/// # Arguments
/// * `source_toml_path` - Path to the source package's pyproject.toml
/// * `target_toml_path` - Path to the target package's pyproject.toml
///
/// # Returns
/// * `Ok(())` on success
/// * `Err` if parsing or writing fails
///
/// # Example
/// ```ignore
/// // Before merge:
/// // source: [project.dependencies] = ["requests>=2.0"]
/// // target: [project.dependencies] = ["click>=8.0"]
/// //
/// // After merge:
/// // target: [project.dependencies] = ["click>=8.0", "requests>=2.0"]
/// ```
pub async fn merge_pyproject_dependencies(
    source_toml_path: &Path,
    target_toml_path: &Path,
) -> PluginResult<MergeResult> {
    info!(
        source = %source_toml_path.display(),
        target = %target_toml_path.display(),
        "Merging pyproject.toml dependencies (consolidation)"
    );

    // Read both TOML files
    let source_content = fs::read_to_string(source_toml_path).await.map_err(|e| {
        PluginApiError::internal(format!("Failed to read source pyproject.toml: {}", e))
    })?;

    let target_content = fs::read_to_string(target_toml_path).await.map_err(|e| {
        PluginApiError::internal(format!("Failed to read target pyproject.toml: {}", e))
    })?;

    // Parse both documents
    let source_doc = source_content.parse::<DocumentMut>().map_err(|e| {
        PluginApiError::internal(format!("Failed to parse source pyproject.toml: {}", e))
    })?;

    let mut target_doc = target_content.parse::<DocumentMut>().map_err(|e| {
        PluginApiError::internal(format!("Failed to parse target pyproject.toml: {}", e))
    })?;

    let mut result = MergeResult::default();

    // Extract target package name for self-dependency detection
    let target_package_name = get_package_name(&target_doc);

    // Detect source format and extract dependencies
    let source_format = detect_pyproject_format(&source_doc);
    let target_format = detect_pyproject_format(&target_doc);

    debug!(
        source_format = ?source_format,
        target_format = ?target_format,
        "Detected pyproject.toml formats"
    );

    // Merge based on detected formats
    match (source_format, target_format) {
        (PyProjectFormat::Pep621, PyProjectFormat::Pep621) => {
            merge_pep621_dependencies(
                &source_doc,
                &mut target_doc,
                target_package_name.as_deref(),
                &mut result,
            )?;
        }
        (PyProjectFormat::Poetry, PyProjectFormat::Poetry) => {
            merge_poetry_dependencies(
                &source_doc,
                &mut target_doc,
                target_package_name.as_deref(),
                &mut result,
            )?;
        }
        (PyProjectFormat::Pep621, PyProjectFormat::Poetry) => {
            // Convert PEP 621 deps to Poetry format
            warn!("Source uses PEP 621, target uses Poetry - cross-format merge");
            merge_pep621_to_poetry(
                &source_doc,
                &mut target_doc,
                target_package_name.as_deref(),
                &mut result,
            )?;
        }
        (PyProjectFormat::Poetry, PyProjectFormat::Pep621) => {
            // Convert Poetry deps to PEP 621 format
            warn!("Source uses Poetry, target uses PEP 621 - cross-format merge");
            merge_poetry_to_pep621(
                &source_doc,
                &mut target_doc,
                target_package_name.as_deref(),
                &mut result,
            )?;
        }
        (PyProjectFormat::Unknown, _) => {
            warn!("Source pyproject.toml has unknown format, skipping merge");
        }
        (_, PyProjectFormat::Unknown) => {
            warn!("Target pyproject.toml has unknown format, skipping merge");
        }
    }

    // Write merged content back to target
    fs::write(target_toml_path, target_doc.to_string())
        .await
        .map_err(|e| {
            PluginApiError::internal(format!("Failed to write merged pyproject.toml: {}", e))
        })?;

    info!(
        merged = result.merged_count,
        conflicts = result.conflict_count,
        "Completed pyproject.toml dependency merge"
    );

    Ok(result)
}

/// Merge dependencies from source content into target content (for testing)
///
/// Returns the merged content as a string.
pub fn merge_pyproject_dependencies_content(
    source_content: &str,
    target_content: &str,
) -> PluginResult<(String, MergeResult)> {
    let source_doc = source_content.parse::<DocumentMut>().map_err(|e| {
        PluginApiError::internal(format!("Failed to parse source pyproject.toml: {}", e))
    })?;

    let mut target_doc = target_content.parse::<DocumentMut>().map_err(|e| {
        PluginApiError::internal(format!("Failed to parse target pyproject.toml: {}", e))
    })?;

    let mut result = MergeResult::default();
    let target_package_name = get_package_name(&target_doc);

    let source_format = detect_pyproject_format(&source_doc);
    let target_format = detect_pyproject_format(&target_doc);

    match (source_format, target_format) {
        (PyProjectFormat::Pep621, PyProjectFormat::Pep621) => {
            merge_pep621_dependencies(
                &source_doc,
                &mut target_doc,
                target_package_name.as_deref(),
                &mut result,
            )?;
        }
        (PyProjectFormat::Poetry, PyProjectFormat::Poetry) => {
            merge_poetry_dependencies(
                &source_doc,
                &mut target_doc,
                target_package_name.as_deref(),
                &mut result,
            )?;
        }
        (PyProjectFormat::Pep621, PyProjectFormat::Poetry) => {
            merge_pep621_to_poetry(
                &source_doc,
                &mut target_doc,
                target_package_name.as_deref(),
                &mut result,
            )?;
        }
        (PyProjectFormat::Poetry, PyProjectFormat::Pep621) => {
            merge_poetry_to_pep621(
                &source_doc,
                &mut target_doc,
                target_package_name.as_deref(),
                &mut result,
            )?;
        }
        _ => {
            warn!("Unknown pyproject.toml format, skipping merge");
        }
    }

    Ok((target_doc.to_string(), result))
}

// ============================================================================
// Format Detection
// ============================================================================

/// Supported pyproject.toml formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PyProjectFormat {
    /// PEP 621 standard format ([project] section)
    Pep621,
    /// Poetry format ([tool.poetry] section)
    Poetry,
    /// Unknown or unsupported format
    Unknown,
}

/// Detect the format of a pyproject.toml document
fn detect_pyproject_format(doc: &DocumentMut) -> PyProjectFormat {
    // Check for Poetry format first (more specific)
    if doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("dependencies"))
        .is_some()
    {
        return PyProjectFormat::Poetry;
    }

    // Check for PEP 621 format
    if doc
        .get("project")
        .and_then(|p| p.get("dependencies"))
        .is_some()
    {
        return PyProjectFormat::Pep621;
    }

    // If project section exists without dependencies, still consider it PEP 621
    if doc.get("project").is_some() {
        return PyProjectFormat::Pep621;
    }

    // If tool.poetry exists without dependencies, still consider it Poetry
    if doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .is_some()
    {
        return PyProjectFormat::Poetry;
    }

    PyProjectFormat::Unknown
}

/// Extract package name from pyproject.toml
fn get_package_name(doc: &DocumentMut) -> Option<String> {
    // Try PEP 621 format first
    if let Some(name) = doc
        .get("project")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
    {
        return Some(name.to_string());
    }

    // Try Poetry format
    if let Some(name) = doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
    {
        return Some(name.to_string());
    }

    None
}

// ============================================================================
// PEP 621 Format Handling
// ============================================================================

/// Merge PEP 621 format dependencies
fn merge_pep621_dependencies(
    source_doc: &DocumentMut,
    target_doc: &mut DocumentMut,
    target_package_name: Option<&str>,
    result: &mut MergeResult,
) -> PluginResult<()> {
    // Merge regular dependencies
    if let Some(source_deps) = source_doc
        .get("project")
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_array())
    {
        ensure_pep621_dependencies(target_doc);

        if let Some(target_deps) = target_doc
            .get_mut("project")
            .and_then(|p| p.get_mut("dependencies"))
            .and_then(|d| d.as_array_mut())
        {
            merge_pep621_dep_array(source_deps, target_deps, target_package_name, result);
        }
    }

    // Merge optional dependencies (extras)
    if let Some(source_optional) = source_doc
        .get("project")
        .and_then(|p| p.get("optional-dependencies"))
        .and_then(|d| d.as_table())
    {
        ensure_pep621_optional_dependencies(target_doc);

        if let Some(target_optional) = target_doc
            .get_mut("project")
            .and_then(|p| p.get_mut("optional-dependencies"))
            .and_then(|d| d.as_table_mut())
        {
            for (extra_name, deps) in source_optional.iter() {
                if let Some(deps_array) = deps.as_array() {
                    // Create target extra if it doesn't exist
                    if !target_optional.contains_key(extra_name) {
                        target_optional.insert(extra_name, Item::Value(Value::Array(Array::new())));
                    }

                    if let Some(target_extra) = target_optional
                        .get_mut(extra_name)
                        .and_then(|d| d.as_array_mut())
                    {
                        merge_pep621_dep_array(deps_array, target_extra, target_package_name, result);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Ensure [project.dependencies] exists in target
fn ensure_pep621_dependencies(doc: &mut DocumentMut) {
    if doc.get("project").is_none() {
        doc["project"] = Item::Table(Table::new());
    }

    if let Some(project) = doc.get_mut("project").and_then(|p| p.as_table_mut()) {
        if !project.contains_key("dependencies") {
            project.insert("dependencies", Item::Value(Value::Array(Array::new())));
        }
    }
}

/// Ensure [project.optional-dependencies] exists in target
fn ensure_pep621_optional_dependencies(doc: &mut DocumentMut) {
    if doc.get("project").is_none() {
        doc["project"] = Item::Table(Table::new());
    }

    if let Some(project) = doc.get_mut("project").and_then(|p| p.as_table_mut()) {
        if !project.contains_key("optional-dependencies") {
            project.insert("optional-dependencies", Item::Table(Table::new()));
        }
    }
}

/// Merge PEP 621 dependency arrays
fn merge_pep621_dep_array(
    source: &toml_edit::Array,
    target: &mut toml_edit::Array,
    target_package_name: Option<&str>,
    result: &mut MergeResult,
) {
    for source_dep in source.iter() {
        if let Some(dep_str) = source_dep.as_str() {
            let dep_name = extract_package_name_from_pep621_dep(dep_str);

            // Skip self-dependency
            if Some(dep_name.as_str()) == target_package_name {
                warn!(
                    dependency = %dep_name,
                    "Skipping self-dependency during consolidation merge"
                );
                result.skipped.push((dep_name.clone(), "self-dependency".to_string()));
                continue;
            }

            // Check if dependency already exists
            let exists = target.iter().any(|t| {
                t.as_str()
                    .map(|s| extract_package_name_from_pep621_dep(s) == dep_name)
                    .unwrap_or(false)
            });

            if exists {
                result.conflict_count += 1;
                debug!(
                    dependency = %dep_name,
                    "Dependency already exists in target, keeping existing version"
                );
                result.skipped.push((dep_name.clone(), "already exists".to_string()));
            } else {
                target.push(dep_str);
                result.merged_count += 1;
                debug!(dependency = %dep_str, "Added dependency to target");
            }
        }
    }
}

/// Extract package name from PEP 621 dependency string
///
/// Examples:
/// - "requests>=2.0.0" -> "requests"
/// - "numpy[full]>=1.20" -> "numpy"
/// - "mypackage @ https://..." -> "mypackage"
fn extract_package_name_from_pep621_dep(dep: &str) -> String {
    let dep = dep.trim();

    // Remove extras [...]
    let dep = if let Some(bracket_pos) = dep.find('[') {
        &dep[..bracket_pos]
    } else {
        dep
    };

    // Handle version specifiers
    for sep in [">=", "<=", "==", "!=", "~=", ">", "<", "@", ";"] {
        if let Some(pos) = dep.find(sep) {
            return dep[..pos].trim().to_string();
        }
    }

    dep.trim().to_string()
}

// ============================================================================
// Poetry Format Handling
// ============================================================================

/// Merge Poetry format dependencies
fn merge_poetry_dependencies(
    source_doc: &DocumentMut,
    target_doc: &mut DocumentMut,
    target_package_name: Option<&str>,
    result: &mut MergeResult,
) -> PluginResult<()> {
    // Merge regular dependencies
    if let Some(source_deps) = source_doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_table())
    {
        ensure_poetry_dependencies(target_doc);

        if let Some(target_deps) = target_doc
            .get_mut("tool")
            .and_then(|t| t.get_mut("poetry"))
            .and_then(|p| p.get_mut("dependencies"))
            .and_then(|d| d.as_table_mut())
        {
            merge_poetry_dep_table(source_deps, target_deps, target_package_name, result);
        }
    }

    // Merge dev-dependencies
    if let Some(source_dev) = source_doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("dev-dependencies"))
        .and_then(|d| d.as_table())
    {
        ensure_poetry_dev_dependencies(target_doc);

        if let Some(target_dev) = target_doc
            .get_mut("tool")
            .and_then(|t| t.get_mut("poetry"))
            .and_then(|p| p.get_mut("dev-dependencies"))
            .and_then(|d| d.as_table_mut())
        {
            merge_poetry_dep_table(source_dev, target_dev, target_package_name, result);
        }
    }

    // Merge extras (Poetry group dependencies)
    if let Some(source_group) = source_doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("group"))
        .and_then(|g| g.as_table())
    {
        ensure_poetry_group(target_doc);

        if let Some(target_group) = target_doc
            .get_mut("tool")
            .and_then(|t| t.get_mut("poetry"))
            .and_then(|p| p.get_mut("group"))
            .and_then(|g| g.as_table_mut())
        {
            for (group_name, group_deps) in source_group.iter() {
                if let Some(deps) = group_deps.get("dependencies").and_then(|d| d.as_table()) {
                    // Create target group if it doesn't exist
                    if !target_group.contains_key(group_name) {
                        let mut new_group = Table::new();
                        new_group.insert("dependencies", Item::Table(Table::new()));
                        target_group.insert(group_name, Item::Table(new_group));
                    }

                    if let Some(target_deps) = target_group
                        .get_mut(group_name)
                        .and_then(|g| g.get_mut("dependencies"))
                        .and_then(|d| d.as_table_mut())
                    {
                        merge_poetry_dep_table(deps, target_deps, target_package_name, result);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Ensure [tool.poetry.dependencies] exists in target
fn ensure_poetry_dependencies(doc: &mut DocumentMut) {
    ensure_poetry_section(doc);

    if let Some(poetry) = doc
        .get_mut("tool")
        .and_then(|t| t.get_mut("poetry"))
        .and_then(|p| p.as_table_mut())
    {
        if !poetry.contains_key("dependencies") {
            poetry.insert("dependencies", Item::Table(Table::new()));
        }
    }
}

/// Ensure [tool.poetry.dev-dependencies] exists in target
fn ensure_poetry_dev_dependencies(doc: &mut DocumentMut) {
    ensure_poetry_section(doc);

    if let Some(poetry) = doc
        .get_mut("tool")
        .and_then(|t| t.get_mut("poetry"))
        .and_then(|p| p.as_table_mut())
    {
        if !poetry.contains_key("dev-dependencies") {
            poetry.insert("dev-dependencies", Item::Table(Table::new()));
        }
    }
}

/// Ensure [tool.poetry.group] exists in target
fn ensure_poetry_group(doc: &mut DocumentMut) {
    ensure_poetry_section(doc);

    if let Some(poetry) = doc
        .get_mut("tool")
        .and_then(|t| t.get_mut("poetry"))
        .and_then(|p| p.as_table_mut())
    {
        if !poetry.contains_key("group") {
            poetry.insert("group", Item::Table(Table::new()));
        }
    }
}

/// Ensure [tool.poetry] section exists
fn ensure_poetry_section(doc: &mut DocumentMut) {
    if doc.get("tool").is_none() {
        doc["tool"] = Item::Table(Table::new());
    }

    if let Some(tool) = doc.get_mut("tool").and_then(|t| t.as_table_mut()) {
        if !tool.contains_key("poetry") {
            tool.insert("poetry", Item::Table(Table::new()));
        }
    }
}

/// Merge Poetry dependency tables
fn merge_poetry_dep_table(
    source: &toml_edit::Table,
    target: &mut toml_edit::Table,
    target_package_name: Option<&str>,
    result: &mut MergeResult,
) {
    for (dep_name, dep_value) in source.iter() {
        // Skip python version constraint
        if dep_name == "python" {
            continue;
        }

        // Skip self-dependency
        if Some(dep_name) == target_package_name {
            warn!(
                dependency = %dep_name,
                "Skipping self-dependency during consolidation merge"
            );
            result.skipped.push((dep_name.to_string(), "self-dependency".to_string()));
            continue;
        }

        // Check if dependency already exists
        if target.contains_key(dep_name) {
            result.conflict_count += 1;
            debug!(
                dependency = %dep_name,
                "Dependency already exists in target, keeping existing version"
            );
            result.skipped.push((dep_name.to_string(), "already exists".to_string()));
        } else {
            target.insert(dep_name, dep_value.clone());
            result.merged_count += 1;
            debug!(dependency = %dep_name, "Added dependency to target");
        }
    }
}

// ============================================================================
// Cross-Format Conversion
// ============================================================================

/// Merge PEP 621 dependencies into Poetry format target
fn merge_pep621_to_poetry(
    source_doc: &DocumentMut,
    target_doc: &mut DocumentMut,
    target_package_name: Option<&str>,
    result: &mut MergeResult,
) -> PluginResult<()> {
    // Convert and merge regular dependencies
    if let Some(source_deps) = source_doc
        .get("project")
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_array())
    {
        ensure_poetry_dependencies(target_doc);

        if let Some(target_deps) = target_doc
            .get_mut("tool")
            .and_then(|t| t.get_mut("poetry"))
            .and_then(|p| p.get_mut("dependencies"))
            .and_then(|d| d.as_table_mut())
        {
            for source_dep in source_deps.iter() {
                if let Some(dep_str) = source_dep.as_str() {
                    let (name, version) = parse_pep621_to_poetry_dep(dep_str);

                    // Skip python version constraint
                    if name == "python" {
                        continue;
                    }

                    // Skip self-dependency
                    if Some(name.as_str()) == target_package_name {
                        warn!(
                            dependency = %name,
                            "Skipping self-dependency during consolidation merge"
                        );
                        result.skipped.push((name.clone(), "self-dependency".to_string()));
                        continue;
                    }

                    if target_deps.contains_key(&name) {
                        result.conflict_count += 1;
                        result.skipped.push((name.clone(), "already exists".to_string()));
                    } else {
                        target_deps.insert(&name, toml_edit::value(version));
                        result.merged_count += 1;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Merge Poetry dependencies into PEP 621 format target
fn merge_poetry_to_pep621(
    source_doc: &DocumentMut,
    target_doc: &mut DocumentMut,
    target_package_name: Option<&str>,
    result: &mut MergeResult,
) -> PluginResult<()> {
    // Convert and merge regular dependencies
    if let Some(source_deps) = source_doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_table())
    {
        ensure_pep621_dependencies(target_doc);

        if let Some(target_deps) = target_doc
            .get_mut("project")
            .and_then(|p| p.get_mut("dependencies"))
            .and_then(|d| d.as_array_mut())
        {
            for (dep_name, dep_value) in source_deps.iter() {
                // Skip python version constraint
                if dep_name == "python" {
                    continue;
                }

                // Skip self-dependency
                if Some(dep_name) == target_package_name {
                    warn!(
                        dependency = %dep_name,
                        "Skipping self-dependency during consolidation merge"
                    );
                    result.skipped.push((dep_name.to_string(), "self-dependency".to_string()));
                    continue;
                }

                // Check if already exists
                let exists = target_deps.iter().any(|t| {
                    t.as_str()
                        .map(|s| extract_package_name_from_pep621_dep(s) == dep_name)
                        .unwrap_or(false)
                });

                if exists {
                    result.conflict_count += 1;
                    result.skipped.push((dep_name.to_string(), "already exists".to_string()));
                } else {
                    let pep621_dep = convert_poetry_to_pep621_dep(dep_name, dep_value);
                    target_deps.push(&pep621_dep);
                    result.merged_count += 1;
                }
            }
        }
    }

    Ok(())
}

/// Parse PEP 621 dependency string to Poetry format (name, version)
fn parse_pep621_to_poetry_dep(dep: &str) -> (String, String) {
    let dep = dep.trim();

    // Handle extras - preserve them in a more complex structure
    // For now, we just extract name and version
    let name = extract_package_name_from_pep621_dep(dep);

    // Extract version constraint
    for sep in [">=", "<=", "==", "!=", "~="] {
        if let Some(pos) = dep.find(sep) {
            let version = dep[pos..].trim();
            // Convert >= to ^, == to exact version for Poetry
            let poetry_version = if sep == ">=" {
                format!("^{}", &version[2..])
            } else if sep == "==" {
                version[2..].to_string()
            } else {
                version.to_string()
            };
            return (name, poetry_version);
        }
    }

    (name, "*".to_string())
}

/// Convert Poetry dependency value to PEP 621 format string
fn convert_poetry_to_pep621_dep(name: &str, value: &Item) -> String {
    // Simple string version
    if let Some(version) = value.as_str() {
        if version == "*" {
            return name.to_string();
        }
        // Convert Poetry ^ to PEP 621 >=
        if version.starts_with('^') {
            return format!("{}>={}",name, &version[1..]);
        }
        return format!("{}=={}", name, version);
    }

    // Table format with version key
    if let Some(table) = value.as_table() {
        if let Some(version) = table.get("version").and_then(|v| v.as_str()) {
            if version == "*" {
                return name.to_string();
            }
            if version.starts_with('^') {
                return format!("{}>={}",name, &version[1..]);
            }
            return format!("{}=={}", name, version);
        }
    }

    // Inline table format
    if let Some(inline) = value.as_inline_table() {
        if let Some(version) = inline.get("version").and_then(|v| v.as_str()) {
            if version == "*" {
                return name.to_string();
            }
            if version.starts_with('^') {
                return format!("{}>={}",name, &version[1..]);
            }
            return format!("{}=={}", name, version);
        }
    }

    // Default: just the name
    name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_package_name_from_pep621_dep() {
        assert_eq!(
            extract_package_name_from_pep621_dep("requests>=2.0.0"),
            "requests"
        );
        assert_eq!(
            extract_package_name_from_pep621_dep("numpy[full]>=1.20"),
            "numpy"
        );
        assert_eq!(
            extract_package_name_from_pep621_dep("mypackage"),
            "mypackage"
        );
        assert_eq!(
            extract_package_name_from_pep621_dep("django==3.2"),
            "django"
        );
    }

    #[test]
    fn test_detect_pep621_format() {
        let content = r#"
[project]
name = "mypackage"
dependencies = ["requests>=2.0"]
"#;
        let doc: DocumentMut = content.parse().unwrap();
        assert_eq!(detect_pyproject_format(&doc), PyProjectFormat::Pep621);
    }

    #[test]
    fn test_detect_poetry_format() {
        let content = r#"
[tool.poetry]
name = "mypackage"

[tool.poetry.dependencies]
python = "^3.8"
requests = "^2.0"
"#;
        let doc: DocumentMut = content.parse().unwrap();
        assert_eq!(detect_pyproject_format(&doc), PyProjectFormat::Poetry);
    }

    #[test]
    fn test_merge_pep621_dependencies() {
        let source = r#"
[project]
name = "source-pkg"
dependencies = ["requests>=2.0", "click>=8.0"]
"#;
        let target = r#"
[project]
name = "target-pkg"
dependencies = ["numpy>=1.0"]
"#;
        let (result, merge_result) = merge_pyproject_dependencies_content(source, target).unwrap();

        assert!(result.contains("numpy"));
        assert!(result.contains("requests"));
        assert!(result.contains("click"));
        assert_eq!(merge_result.merged_count, 2);
        assert_eq!(merge_result.conflict_count, 0);
    }

    #[test]
    fn test_merge_pep621_with_conflict() {
        let source = r#"
[project]
name = "source-pkg"
dependencies = ["requests>=2.0", "numpy>=2.0"]
"#;
        let target = r#"
[project]
name = "target-pkg"
dependencies = ["numpy>=1.0"]
"#;
        let (result, merge_result) = merge_pyproject_dependencies_content(source, target).unwrap();

        // Should keep existing numpy version
        assert!(result.contains("numpy>=1.0"));
        assert!(result.contains("requests>=2.0"));
        assert_eq!(merge_result.merged_count, 1); // Only requests
        assert_eq!(merge_result.conflict_count, 1); // numpy conflict
    }

    #[test]
    fn test_merge_pep621_skip_self_dependency() {
        let source = r#"
[project]
name = "source-pkg"
dependencies = ["target-pkg>=1.0", "requests>=2.0"]
"#;
        let target = r#"
[project]
name = "target-pkg"
dependencies = ["numpy>=1.0"]
"#;
        let (result, merge_result) = merge_pyproject_dependencies_content(source, target).unwrap();

        // Should NOT contain target-pkg (self-dependency)
        assert!(!result.contains("target-pkg>=1.0"));
        assert!(result.contains("requests>=2.0"));
        assert_eq!(merge_result.merged_count, 1); // Only requests
        assert!(merge_result.skipped.iter().any(|(name, reason)|
            name == "target-pkg" && reason == "self-dependency"
        ));
    }

    #[test]
    fn test_merge_poetry_dependencies() {
        let source = r#"
[tool.poetry]
name = "source-pkg"

[tool.poetry.dependencies]
python = "^3.8"
requests = "^2.0"
click = { version = "^8.0", optional = true }
"#;
        let target = r#"
[tool.poetry]
name = "target-pkg"

[tool.poetry.dependencies]
python = "^3.9"
numpy = "^1.0"
"#;
        let (result, merge_result) = merge_pyproject_dependencies_content(source, target).unwrap();

        // Should contain all dependencies
        assert!(result.contains("numpy"));
        assert!(result.contains("requests"));
        assert!(result.contains("click"));
        // Python should remain unchanged (from target)
        assert!(result.contains("python = \"^3.9\""));
        assert_eq!(merge_result.merged_count, 2); // requests and click
        assert_eq!(merge_result.conflict_count, 0);
    }

    #[test]
    fn test_merge_pep621_optional_dependencies() {
        let source = r#"
[project]
name = "source-pkg"
dependencies = []

[project.optional-dependencies]
dev = ["pytest>=7.0", "black>=22.0"]
"#;
        let target = r#"
[project]
name = "target-pkg"
dependencies = ["numpy>=1.0"]

[project.optional-dependencies]
dev = ["mypy>=0.9"]
"#;
        let (result, merge_result) = merge_pyproject_dependencies_content(source, target).unwrap();

        // Should contain all optional dependencies
        assert!(result.contains("mypy"));
        assert!(result.contains("pytest"));
        assert!(result.contains("black"));
        assert_eq!(merge_result.merged_count, 2); // pytest and black
    }

    #[test]
    fn test_parse_pep621_to_poetry_dep() {
        let (name, version) = parse_pep621_to_poetry_dep("requests>=2.0.0");
        assert_eq!(name, "requests");
        assert_eq!(version, "^2.0.0");

        let (name, version) = parse_pep621_to_poetry_dep("django==3.2");
        assert_eq!(name, "django");
        assert_eq!(version, "3.2");

        let (name, version) = parse_pep621_to_poetry_dep("flask");
        assert_eq!(name, "flask");
        assert_eq!(version, "*");
    }
}
