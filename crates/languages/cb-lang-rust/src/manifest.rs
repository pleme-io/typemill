//! Cargo.toml manifest file handling
//!
//! This module provides functionality for parsing and manipulating Cargo.toml
//! manifest files, extracting dependency information, and updating dependencies.

use cb_plugin_api::{Dependency, DependencySource, ManifestData, PluginError, PluginResult};
use std::path::Path;
use toml_edit::{value, DocumentMut, Item};

/// Parse a Cargo.toml file and extract manifest information
pub fn parse_cargo_toml(content: &str) -> PluginResult<ManifestData> {
    let doc = content
        .parse::<DocumentMut>()
        .map_err(|e| PluginError::manifest(format!("Failed to parse Cargo.toml: {}", e)))?;

    // Extract package information
    let package_table = doc
        .get("package")
        .and_then(|i| i.as_table())
        .ok_or_else(|| PluginError::manifest("Missing [package] section in Cargo.toml"))?;

    let name = package_table
        .get("name")
        .and_then(|i| i.as_str())
        .ok_or_else(|| PluginError::manifest("Missing 'name' field in [package]"))?
        .to_string();

    let version = package_table
        .get("version")
        .and_then(|i| i.as_str())
        .ok_or_else(|| PluginError::manifest("Missing 'version' field in [package]"))?
        .to_string();

    // Extract dependencies
    let dependencies = extract_dependencies(&doc, "dependencies");
    let dev_dependencies = extract_dependencies(&doc, "dev-dependencies");

    Ok(ManifestData {
        name,
        version,
        dependencies,
        dev_dependencies,
        raw_data: serde_json::to_value(doc.to_string())
            .map_err(|e| PluginError::internal(format!("Failed to serialize manifest: {}", e)))?,
    })
}

/// Extract dependencies from a specific table in the TOML document
fn extract_dependencies(doc: &DocumentMut, table_name: &str) -> Vec<Dependency> {
    let mut deps = Vec::new();

    if let Some(deps_table) = doc.get(table_name).and_then(|i| i.as_table()) {
        for (name, value) in deps_table.iter() {
            let source = match value {
                Item::Value(val) if val.is_str() => {
                    // Simple version string: dep = "1.0"
                    DependencySource::Version(val.as_str().unwrap_or("").to_string())
                }
                Item::Value(val) if val.is_inline_table() => {
                    // Inline table: dep = { version = "1.0", features = [...] }
                    if let Some(table) = val.as_inline_table() {
                        parse_dependency_source(table)
                    } else {
                        continue;
                    }
                }
                _ => continue,
            };

            deps.push(Dependency {
                name: name.to_string(),
                source,
            });
        }
    }

    deps
}

/// Parse dependency source from an inline table
fn parse_dependency_source(table: &toml_edit::InlineTable) -> DependencySource {
    // Check for path dependency
    if let Some(path) = table.get("path").and_then(|v| v.as_str()) {
        return DependencySource::Path(path.to_string());
    }

    // Check for git dependency
    if let Some(git_url) = table.get("git").and_then(|v| v.as_str()) {
        let rev = table.get("rev").and_then(|v| v.as_str()).map(|s| s.to_string());
        return DependencySource::Git {
            url: git_url.to_string(),
            rev,
        };
    }

    // Default to version
    if let Some(version) = table.get("version").and_then(|v| v.as_str()) {
        DependencySource::Version(version.to_string())
    } else {
        DependencySource::Version("*".to_string())
    }
}

/// Update a Cargo.toml file by renaming a dependency
pub fn rename_dependency(
    content: &str,
    old_name: &str,
    new_name: &str,
    new_path: Option<&str>,
) -> PluginResult<String> {
    let mut doc = content
        .parse::<DocumentMut>()
        .map_err(|e| PluginError::manifest(format!("Failed to parse Cargo.toml: {}", e)))?;

    // Helper function to preserve metadata when renaming
    fn rename_in_table(
        deps: &mut dyn toml_edit::TableLike,
        old_name: &str,
        new_name: &str,
        new_path: Option<&str>,
    ) {
        if let Some(old_dep) = deps.remove(old_name) {
            let new_dep = match old_dep {
                Item::Value(ref val) if val.is_inline_table() => {
                    if let Some(table) = val.as_inline_table() {
                        let mut new_table = toml_edit::InlineTable::new();

                        // Copy all existing key-value pairs
                        for (key, value) in table.iter() {
                            new_table.insert(key, value.clone());
                        }

                        // Update or add the path field
                        if let Some(path) = new_path {
                            new_table.insert("path", path.into());
                        }

                        value(new_table)
                    } else {
                        old_dep
                    }
                }
                Item::Value(ref val) if val.is_str() => {
                    if let Some(path) = new_path {
                        // Convert to inline table with path
                        let mut new_table = toml_edit::InlineTable::new();
                        new_table.insert("path", path.into());
                        value(new_table)
                    } else {
                        old_dep
                    }
                }
                _ => {
                    if let Some(path) = new_path {
                        let mut new_table = toml_edit::InlineTable::new();
                        new_table.insert("path", path.into());
                        value(new_table)
                    } else {
                        old_dep
                    }
                }
            };

            deps.insert(new_name, new_dep);
        }
    }

    // Update in [dependencies]
    if let Some(deps) = doc.get_mut("dependencies").and_then(Item::as_table_like_mut) {
        rename_in_table(deps, old_name, new_name, new_path);
    }

    // Update in [dev-dependencies]
    if let Some(dev_deps) = doc
        .get_mut("dev-dependencies")
        .and_then(Item::as_table_like_mut)
    {
        rename_in_table(dev_deps, old_name, new_name, new_path);
    }

    // Update in [build-dependencies]
    if let Some(build_deps) = doc
        .get_mut("build-dependencies")
        .and_then(Item::as_table_like_mut)
    {
        rename_in_table(build_deps, old_name, new_name, new_path);
    }

    Ok(doc.to_string())
}

/// Load and parse a Cargo.toml file from a path
pub async fn load_cargo_toml(path: &Path) -> PluginResult<ManifestData> {
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| PluginError::manifest(format!("Failed to read Cargo.toml: {}", e)))?;

    parse_cargo_toml(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_cargo_toml() {
        let content = r#"
[package]
name = "test-crate"
version = "0.1.0"

[dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }
my-local-dep = { path = "../my-dep" }
"#;

        let manifest = parse_cargo_toml(content).unwrap();
        assert_eq!(manifest.name, "test-crate");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.dependencies.len(), 3);

        // Check serde (version)
        assert!(manifest.dependencies.iter().any(|d| {
            d.name == "serde" && matches!(&d.source, DependencySource::Version(v) if v == "1.0")
        }));

        // Check my-local-dep (path)
        assert!(manifest.dependencies.iter().any(|d| {
            d.name == "my-local-dep"
                && matches!(&d.source, DependencySource::Path(p) if p == "../my-dep")
        }));
    }

    #[test]
    fn test_rename_dependency() {
        let cargo_toml = r#"
[package]
name = "test-crate"
version = "0.1.0"

[dependencies]
cb-mcp-proxy = { path = "../cb-mcp-proxy" }
other-dep = "1.0"
"#;

        let result = rename_dependency(cargo_toml, "cb-mcp-proxy", "cb-plugins", Some("../cb-plugins"))
            .unwrap();

        assert!(result.contains("cb-plugins"));
        assert!(!result.contains("cb-mcp-proxy"));
        assert!(result.contains("../cb-plugins"));
    }

    #[test]
    fn test_rename_dependency_preserves_metadata() {
        let cargo_toml = r#"
[package]
name = "test-crate"
version = "0.1.0"

[dependencies]
my-dep = { path = "../my-dep", version = "0.1", optional = true, features = ["feat1", "feat2"], default-features = false }
"#;

        let result = rename_dependency(cargo_toml, "my-dep", "renamed-dep", Some("../renamed-dep"))
            .unwrap();

        // Verify the dependency was renamed
        assert!(result.contains("renamed-dep"));
        assert!(!result.contains("my-dep ="));

        // Verify all metadata was preserved
        assert!(result.contains("../renamed-dep"));
        assert!(result.contains("optional = true"));
        assert!(result.contains("features = [\"feat1\", \"feat2\"]"));
        assert!(result.contains("default-features = false"));
        assert!(result.contains("version = \"0.1\""));
    }

    #[test]
    fn test_parse_git_dependency() {
        let content = r#"
[package]
name = "test-crate"
version = "0.1.0"

[dependencies]
my-git-dep = { git = "https://github.com/user/repo", rev = "abc123" }
"#;

        let manifest = parse_cargo_toml(content).unwrap();
        assert_eq!(manifest.dependencies.len(), 1);

        let dep = &manifest.dependencies[0];
        assert_eq!(dep.name, "my-git-dep");
        assert!(matches!(
            &dep.source,
            DependencySource::Git { url, rev }
            if url == "https://github.com/user/repo" && rev.as_deref() == Some("abc123")
        ));
    }
}
