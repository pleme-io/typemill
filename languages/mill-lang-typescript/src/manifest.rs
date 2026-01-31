//! package.json manifest file handling
//!
//! This module provides functionality for parsing and manipulating package.json
//! manifest files, extracting dependency information, and updating dependencies.
//!
//! # package.json Format
//!
//! A package.json file contains:
//! - Package metadata: name, version, description, author
//! - Dependencies: dependencies, devDependencies, peerDependencies, optionalDependencies
//! - Scripts: npm/yarn script commands
//! - Configuration: type (module/commonjs), main, types, exports
//! - Workspace: workspace configuration for monorepos
//!
//! # Example
//!
//! ```json
//! {
//!   "name": "my-package",
//!   "version": "1.0.0",
//!   "dependencies": {
//!     "react": "^18.0.0",
//!     "lodash": "~4.17.0"
//!   },
//!   "devDependencies": {
//!     "typescript": "^5.0.0"
//!   }
//! }
//! ```

use mill_lang_common::read_manifest;
use mill_plugin_api::{Dependency, DependencySource, ManifestData, PluginApiError, PluginResult};
use serde_json::{Map, Value};
use std::path::Path;
use tracing::{debug, warn};

/// Parse a package.json file and extract manifest information
pub fn parse_package_json(content: &str) -> PluginResult<ManifestData> {
    debug!("Parsing package.json content");

    let json: Value = serde_json::from_str(content)
        .map_err(|e| PluginApiError::manifest(format!("Failed to parse package.json: {}", e)))?;

    let obj = json
        .as_object()
        .ok_or_else(|| PluginApiError::manifest("package.json root must be an object"))?;

    // Extract package information
    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| PluginApiError::manifest("Missing 'name' field in package.json"))?
        .to_string();

    let version = obj
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0")
        .to_string();

    // Extract dependencies
    let dependencies = extract_dependencies(obj, "dependencies");
    let dev_dependencies = extract_dependencies(obj, "devDependencies");

    debug!(
        package = %name,
        version = %version,
        dependencies_count = dependencies.len(),
        dev_dependencies_count = dev_dependencies.len(),
        "Parsed package.json successfully"
    );

    Ok(ManifestData {
        name,
        version,
        dependencies,
        dev_dependencies,
        raw_data: json,
    })
}

/// Extract dependencies from a specific field in the package.json
fn extract_dependencies(obj: &Map<String, Value>, field_name: &str) -> Vec<Dependency> {
    let mut deps = Vec::new();

    if let Some(deps_obj) = obj.get(field_name).and_then(|v| v.as_object()) {
        for (name, value) in deps_obj.iter() {
            let source = parse_dependency_source(value);
            deps.push(Dependency {
                name: name.clone(),
                source,
            });
        }
    }

    deps
}

/// Parse dependency source from package.json value
fn parse_dependency_source(value: &Value) -> DependencySource {
    let value_str = match value.as_str() {
        Some(s) => s,
        None => return DependencySource::Version("*".to_string()),
    };

    // Git dependency: "git://..." or "git+https://..." or "github:user/repo"
    if value_str.starts_with("git://")
        || value_str.starts_with("git+")
        || value_str.contains("github.com")
        || value_str.starts_with("github:")
        || value_str.starts_with("bitbucket:")
        || value_str.starts_with("gitlab:")
    {
        // Extract revision if specified with #
        let parts: Vec<&str> = value_str.split('#').collect();
        let url = parts[0].to_string();
        let rev = parts.get(1).map(|s| s.to_string());

        return DependencySource::Git { url, rev };
    }

    // Local path dependency: "file:..." or relative/absolute paths
    if value_str.starts_with("file:")
        || value_str.starts_with("./")
        || value_str.starts_with("../")
        || value_str.starts_with('/')
    {
        let path = value_str.strip_prefix("file:").unwrap_or(value_str);
        return DependencySource::Path(path.to_string());
    }

    // Workspace dependency: "workspace:*" or "workspace:^1.0.0"
    if value_str.starts_with("workspace:") {
        let version = value_str.strip_prefix("workspace:").unwrap_or("*");
        return DependencySource::Version(format!("workspace:{}", version));
    }

    // URL dependency: "http://..." or "https://..."
    if value_str.starts_with("http://") || value_str.starts_with("https://") {
        return DependencySource::Git {
            url: value_str.to_string(),
            rev: None,
        };
    }

    // Default: version specifier (^1.0.0, ~1.0.0, >=1.0.0, 1.0.0, etc.)
    DependencySource::Version(value_str.to_string())
}

/// Update a dependency version in package.json content
pub fn update_dependency(content: &str, dep_name: &str, new_version: &str) -> PluginResult<String> {
    debug!(
        dependency = %dep_name,
        version = %new_version,
        "Updating dependency in package.json"
    );

    let mut json: Value = serde_json::from_str(content)
        .map_err(|e| PluginApiError::manifest(format!("Failed to parse package.json: {}", e)))?;

    let obj = json
        .as_object_mut()
        .ok_or_else(|| PluginApiError::manifest("package.json root must be an object"))?;

    let mut found = false;

    // Update in dependencies
    if let Some(deps) = obj.get_mut("dependencies").and_then(|v| v.as_object_mut()) {
        if deps.contains_key(dep_name) {
            deps.insert(dep_name.to_string(), Value::String(new_version.to_string()));
            found = true;
        }
    }

    // Update in devDependencies
    if let Some(dev_deps) = obj
        .get_mut("devDependencies")
        .and_then(|v| v.as_object_mut())
    {
        if dev_deps.contains_key(dep_name) {
            dev_deps.insert(dep_name.to_string(), Value::String(new_version.to_string()));
            found = true;
        }
    }

    // Update in peerDependencies
    if let Some(peer_deps) = obj
        .get_mut("peerDependencies")
        .and_then(|v| v.as_object_mut())
    {
        if peer_deps.contains_key(dep_name) {
            peer_deps.insert(dep_name.to_string(), Value::String(new_version.to_string()));
            found = true;
        }
    }

    // Update in optionalDependencies
    if let Some(opt_deps) = obj
        .get_mut("optionalDependencies")
        .and_then(|v| v.as_object_mut())
    {
        if opt_deps.contains_key(dep_name) {
            opt_deps.insert(dep_name.to_string(), Value::String(new_version.to_string()));
            found = true;
        }
    }

    if !found {
        warn!(dependency = %dep_name, "Dependency not found in package.json");
        return Err(PluginApiError::manifest(format!(
            "Dependency {} not found in package.json",
            dep_name
        )));
    }

    // Serialize with pretty formatting (2 spaces, like npm)
    serde_json::to_string_pretty(&json)
        .map(|s| s + "\n") // Add trailing newline like npm
        .map_err(|e| PluginApiError::manifest(format!("Failed to serialize package.json: {}", e)))
}

/// Generate a new package.json file
pub fn generate_manifest(package_name: &str, dependencies: &[String]) -> String {
    let mut manifest = serde_json::json!({
        "name": package_name,
        "version": "0.1.0",
        "private": true,
        "type": "module"
    });

    if !dependencies.is_empty() {
        let deps: Map<String, Value> = dependencies
            .iter()
            .map(|dep| (dep.clone(), Value::String("*".to_string())))
            .collect();

        if let Some(obj) = manifest.as_object_mut() {
            obj.insert("dependencies".to_string(), Value::Object(deps));
        }
    }

    serde_json::to_string_pretty(&manifest).unwrap() + "\n"
}

/// Load and parse a package.json file from a path
pub async fn load_package_json(path: &Path) -> PluginResult<ManifestData> {
    let content = read_manifest(path).await?;
    parse_package_json(&content)
}

// ============================================================================
// Dependency Merging for Consolidation
// ============================================================================

/// Version comparison result for semver-like versions
#[derive(Debug, PartialEq, Eq)]
pub enum VersionComparison {
    /// First version is higher
    FirstHigher,
    /// Second version is higher
    SecondHigher,
    /// Versions are equal or incomparable
    Equal,
}

/// Helper to strip version range prefixes
fn strip_version_prefix(v: &str) -> &str {
    v.trim_start_matches('^')
        .trim_start_matches('~')
        .trim_start_matches(">=")
        .trim_start_matches('>')
        .trim_start_matches("<=")
        .trim_start_matches('<')
}

/// Compare two npm version strings and determine which is higher
///
/// Handles common npm version formats:
/// - Exact versions: "1.0.0", "2.3.4"
/// - Caret ranges: "^1.0.0"
/// - Tilde ranges: "~1.0.0"
/// - Comparison ranges: ">=1.0.0", ">1.0.0", "<=1.0.0", "<1.0.0"
/// - Star/latest: "*", "latest"
///
/// Returns which version constraint is more permissive/higher
pub fn compare_versions(v1: &str, v2: &str) -> VersionComparison {
    // Normalize versions by stripping range prefixes
    let v1_clean = strip_version_prefix(v1);
    let v2_clean = strip_version_prefix(v2);

    // Handle special cases
    if v1 == "*" || v1 == "latest" {
        return VersionComparison::FirstHigher;
    }
    if v2 == "*" || v2 == "latest" {
        return VersionComparison::SecondHigher;
    }

    // Parse as semver-like (major.minor.patch)
    let parse_semver = |v: &str| -> Option<(u32, u32, u32)> {
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() < 3 {
            return None;
        }
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        // Handle patch with pre-release suffix (e.g., "0-beta.1")
        let patch_str = parts[2].split('-').next()?;
        let patch = patch_str.parse().ok()?;
        Some((major, minor, patch))
    };

    match (parse_semver(v1_clean), parse_semver(v2_clean)) {
        (Some((m1, n1, p1)), Some((m2, n2, p2))) => {
            if (m1, n1, p1) > (m2, n2, p2) {
                VersionComparison::FirstHigher
            } else if (m1, n1, p1) < (m2, n2, p2) {
                VersionComparison::SecondHigher
            } else {
                VersionComparison::Equal
            }
        }
        _ => VersionComparison::Equal,
    }
}

/// Merge result for dependency consolidation
#[derive(Debug)]
pub struct MergeDependenciesResult {
    /// Number of dependencies merged (added to target)
    pub merged_count: usize,
    /// Number of conflicts (dependency exists in both, kept target version)
    pub conflict_count: usize,
    /// Number of upgrades (dependency exists in both, used higher source version)
    pub upgrade_count: usize,
    /// Conflict details: (package_name, target_version, source_version)
    pub conflicts: Vec<(String, String, String)>,
}

/// Merge dependencies from source package.json into base package.json
///
/// This function merges all dependency sections:
/// - dependencies
/// - devDependencies
/// - peerDependencies
/// - optionalDependencies
///
/// When both base and source have the same dependency:
/// - If versions are compatible (same major), keep the higher version
/// - If versions conflict, keep base version and record in conflicts
///
/// # Arguments
/// * `base_content` - The target package.json content (will receive merged deps)
/// * `source_content` - The source package.json content (deps to merge from)
///
/// # Returns
/// Updated package.json content as a JSON string with trailing newline
pub fn merge_package_json_dependencies(
    base_content: &str,
    source_content: &str,
) -> PluginResult<String> {
    debug!("Merging package.json dependencies");

    let mut base_json: Value = serde_json::from_str(base_content)
        .map_err(|e| PluginApiError::manifest(format!("Failed to parse base package.json: {}", e)))?;

    let source_json: Value = serde_json::from_str(source_content)
        .map_err(|e| PluginApiError::manifest(format!("Failed to parse source package.json: {}", e)))?;

    let base_obj = base_json
        .as_object_mut()
        .ok_or_else(|| PluginApiError::manifest("Base package.json root must be an object"))?;

    let source_obj = source_json
        .as_object()
        .ok_or_else(|| PluginApiError::manifest("Source package.json root must be an object"))?;

    // Get target package name to skip self-dependencies (clone to avoid borrow)
    let target_package_name = base_obj
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let mut result = MergeDependenciesResult {
        merged_count: 0,
        conflict_count: 0,
        upgrade_count: 0,
        conflicts: Vec::new(),
    };

    // Merge each dependency section
    for section in [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ] {
        if let Some(source_deps) = source_obj.get(section).and_then(|d| d.as_object()) {
            // Ensure target has this section
            if !base_obj.contains_key(section) {
                base_obj.insert(section.to_string(), Value::Object(Map::new()));
            }

            if let Some(base_deps) = base_obj.get_mut(section).and_then(|d| d.as_object_mut()) {
                for (dep_name, dep_value) in source_deps {
                    // Skip self-dependency (would create circular dependency)
                    if dep_name == &target_package_name {
                        warn!(
                            dependency = %dep_name,
                            section = %section,
                            "Skipping self-dependency during consolidation merge"
                        );
                        continue;
                    }

                    if let Some(existing_value) = base_deps.get(dep_name) {
                        // Dependency exists in both - compare versions
                        // Clone the strings to avoid borrow conflicts
                        let existing_version = existing_value.as_str().unwrap_or("").to_string();
                        let source_version = dep_value.as_str().unwrap_or("").to_string();

                        match compare_versions(&source_version, &existing_version) {
                            VersionComparison::FirstHigher => {
                                // Source version is higher - upgrade
                                base_deps.insert(dep_name.clone(), dep_value.clone());
                                result.upgrade_count += 1;
                                debug!(
                                    dependency = %dep_name,
                                    old_version = %existing_version,
                                    new_version = %source_version,
                                    "Upgraded dependency to higher version"
                                );
                            }
                            _ => {
                                // Keep existing version, record conflict
                                result.conflict_count += 1;
                                debug!(
                                    dependency = %dep_name,
                                    section = %section,
                                    target_version = %existing_version,
                                    source_version = %source_version,
                                    "Dependency conflict, keeping target version"
                                );
                                result.conflicts.push((
                                    dep_name.clone(),
                                    existing_version,
                                    source_version,
                                ));
                            }
                        }
                    } else {
                        // New dependency - add it
                        base_deps.insert(dep_name.clone(), dep_value.clone());
                        result.merged_count += 1;
                    }
                }
            }
        }
    }

    debug!(
        merged = result.merged_count,
        conflicts = result.conflict_count,
        upgrades = result.upgrade_count,
        "Completed package.json dependency merge"
    );

    // Serialize with pretty formatting (2 spaces, like npm)
    serde_json::to_string_pretty(&base_json)
        .map(|s| s + "\n") // Add trailing newline like npm
        .map_err(|e| PluginApiError::manifest(format!("Failed to serialize package.json: {}", e)))
}

/// Merge dependencies and return detailed result
///
/// Same as `merge_package_json_dependencies` but returns the merge statistics
/// for logging/reporting purposes.
pub fn merge_package_json_dependencies_with_result(
    base_content: &str,
    source_content: &str,
) -> PluginResult<(String, MergeDependenciesResult)> {
    debug!("Merging package.json dependencies with detailed result");

    let mut base_json: Value = serde_json::from_str(base_content)
        .map_err(|e| PluginApiError::manifest(format!("Failed to parse base package.json: {}", e)))?;

    let source_json: Value = serde_json::from_str(source_content)
        .map_err(|e| PluginApiError::manifest(format!("Failed to parse source package.json: {}", e)))?;

    let base_obj = base_json
        .as_object_mut()
        .ok_or_else(|| PluginApiError::manifest("Base package.json root must be an object"))?;

    let source_obj = source_json
        .as_object()
        .ok_or_else(|| PluginApiError::manifest("Source package.json root must be an object"))?;

    let target_package_name = base_obj
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let mut result = MergeDependenciesResult {
        merged_count: 0,
        conflict_count: 0,
        upgrade_count: 0,
        conflicts: Vec::new(),
    };

    for section in [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ] {
        if let Some(source_deps) = source_obj.get(section).and_then(|d| d.as_object()) {
            if !base_obj.contains_key(section) {
                base_obj.insert(section.to_string(), Value::Object(Map::new()));
            }

            if let Some(base_deps) = base_obj.get_mut(section).and_then(|d| d.as_object_mut()) {
                for (dep_name, dep_value) in source_deps {
                    if dep_name == &target_package_name {
                        continue;
                    }

                    if let Some(existing_value) = base_deps.get(dep_name) {
                        let existing_version = existing_value.as_str().unwrap_or("");
                        let source_version = dep_value.as_str().unwrap_or("");

                        match compare_versions(source_version, existing_version) {
                            VersionComparison::FirstHigher => {
                                base_deps.insert(dep_name.clone(), dep_value.clone());
                                result.upgrade_count += 1;
                            }
                            _ => {
                                result.conflict_count += 1;
                                result.conflicts.push((
                                    dep_name.clone(),
                                    existing_version.to_string(),
                                    source_version.to_string(),
                                ));
                            }
                        }
                    } else {
                        base_deps.insert(dep_name.clone(), dep_value.clone());
                        result.merged_count += 1;
                    }
                }
            }
        }
    }

    let merged_content = serde_json::to_string_pretty(&base_json)
        .map(|s| s + "\n")
        .map_err(|e| PluginApiError::manifest(format!("Failed to serialize package.json: {}", e)))?;

    Ok((merged_content, result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_package_json() {
        let content = r#"{
  "name": "my-package",
  "version": "1.0.0",
  "dependencies": {
    "react": "^18.0.0",
    "lodash": "~4.17.0"
  },
  "devDependencies": {
    "typescript": "^5.0.0"
  }
}"#;

        let manifest = parse_package_json(content).unwrap();
        assert_eq!(manifest.name, "my-package");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.dependencies.len(), 2);
        assert_eq!(manifest.dev_dependencies.len(), 1);

        assert!(manifest.dependencies.iter().any(|d| {
            d.name == "react" && matches!(&d.source, DependencySource::Version(v) if v == "^18.0.0")
        }));

        assert!(manifest.dev_dependencies.iter().any(|d| {
            d.name == "typescript"
                && matches!(&d.source, DependencySource::Version(v) if v == "^5.0.0")
        }));
    }

    #[test]
    fn test_parse_git_dependency() {
        let content = r#"{
  "name": "test-package",
  "version": "1.0.0",
  "dependencies": {
    "my-lib": "git+https://github.com/user/repo.git#v1.0.0",
    "other-lib": "github:user/other"
  }
}"#;

        let manifest = parse_package_json(content).unwrap();
        assert_eq!(manifest.dependencies.len(), 2);

        let git_dep = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "my-lib")
            .unwrap();
        assert!(matches!(
            &git_dep.source,
            DependencySource::Git { url, rev }
            if url == "git+https://github.com/user/repo.git" && rev.as_deref() == Some("v1.0.0")
        ));

        let github_dep = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "other-lib")
            .unwrap();
        assert!(matches!(&github_dep.source, DependencySource::Git { .. }));
    }

    #[test]
    fn test_parse_local_path_dependency() {
        let content = r#"{
  "name": "test-package",
  "version": "1.0.0",
  "dependencies": {
    "local-lib": "file:../local-lib",
    "relative-lib": "./libs/my-lib"
  }
}"#;

        let manifest = parse_package_json(content).unwrap();
        assert_eq!(manifest.dependencies.len(), 2);

        let local_dep = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "local-lib")
            .unwrap();
        assert!(matches!(&local_dep.source, DependencySource::Path(p) if p == "../local-lib"));

        let relative_dep = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "relative-lib")
            .unwrap();
        assert!(matches!(&relative_dep.source, DependencySource::Path(p) if p == "./libs/my-lib"));
    }

    #[test]
    fn test_parse_workspace_dependency() {
        let content = r#"{
  "name": "test-package",
  "version": "1.0.0",
  "dependencies": {
    "workspace-lib": "workspace:*",
    "workspace-lib2": "workspace:^1.0.0"
  }
}"#;

        let manifest = parse_package_json(content).unwrap();
        assert_eq!(manifest.dependencies.len(), 2);

        let ws_dep = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "workspace-lib")
            .unwrap();
        assert!(matches!(&ws_dep.source, DependencySource::Version(v) if v == "workspace:*"));
    }

    #[test]
    fn test_update_dependency() {
        let content = r#"{
  "name": "test-package",
  "version": "1.0.0",
  "dependencies": {
    "react": "^17.0.0",
    "lodash": "^4.0.0"
  },
  "devDependencies": {
    "typescript": "^4.0.0"
  }
}"#;

        let result = update_dependency(content, "react", "^18.0.0").unwrap();
        assert!(result.contains("\"react\": \"^18.0.0\""));
        assert!(!result.contains("^17.0.0"));

        // Test dev dependency
        let result2 = update_dependency(content, "typescript", "^5.0.0").unwrap();
        assert!(result2.contains("\"typescript\": \"^5.0.0\""));
    }

    #[test]
    fn test_update_nonexistent_dependency() {
        let content = r#"{
  "name": "test-package",
  "version": "1.0.0",
  "dependencies": {
    "react": "^17.0.0"
  }
}"#;

        let result = update_dependency(content, "nonexistent", "^1.0.0");
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_manifest() {
        let result = generate_manifest("my-new-package", &["react".to_string(), "vue".to_string()]);

        assert!(result.contains("\"name\": \"my-new-package\""));
        assert!(result.contains("\"version\": \"0.1.0\""));
        assert!(result.contains("\"react\""));
        assert!(result.contains("\"vue\""));
        assert!(result.contains("\"type\": \"module\""));
    }

    #[test]
    fn test_generate_manifest_without_dependencies() {
        let result = generate_manifest("empty-package", &[]);

        assert!(result.contains("\"name\": \"empty-package\""));
        assert!(!result.contains("dependencies"));
    }

    #[test]
    fn test_parse_version_ranges() {
        let content = r#"{
  "name": "test-package",
  "version": "1.0.0",
  "dependencies": {
    "caret": "^1.0.0",
    "tilde": "~1.0.0",
    "exact": "1.0.0",
    "gte": ">=1.0.0",
    "range": "1.0.0 - 2.0.0",
    "latest": "latest"
  }
}"#;

        let manifest = parse_package_json(content).unwrap();
        assert_eq!(manifest.dependencies.len(), 6);

        // All should parse as version dependencies
        for dep in &manifest.dependencies {
            assert!(matches!(&dep.source, DependencySource::Version(_)));
        }
    }
}
