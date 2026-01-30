//! Cargo.toml manifest implementation
//!
//! Implements `ManifestOps` trait for Rust/Cargo projects.

use super::{DependencyInfo, ManifestOps};
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use toml_edit::{DocumentMut, Item, Value as TomlValue};
use tracing::debug;

/// Cargo.toml manifest wrapper
pub struct CargoManifest {
    doc: DocumentMut,
}

impl ManifestOps for CargoManifest {
    fn parse(content: &str) -> ServerResult<Self> {
        let doc = content
            .parse::<DocumentMut>()
            .map_err(|e| ServerError::parse(format!("Failed to parse Cargo.toml: {}", e)))?;
        Ok(Self { doc })
    }

    fn sections() -> &'static [&'static str] {
        &["dependencies", "dev-dependencies", "build-dependencies"]
    }

    fn default_section() -> &'static str {
        "dependencies"
    }

    fn find_dependency(&self, name: &str) -> Option<(&'static str, DependencyInfo)> {
        // Check [dependencies]
        if let Some(deps) = self.doc.get("dependencies").and_then(|d| d.as_table()) {
            if let Some(dep) = deps.get(name) {
                let info = extract_cargo_dep_info(name, dep);
                return Some(("dependencies", info));
            }
        }

        // Check [dev-dependencies]
        if let Some(deps) = self.doc.get("dev-dependencies").and_then(|d| d.as_table()) {
            if let Some(dep) = deps.get(name) {
                let info = extract_cargo_dep_info(name, dep);
                return Some(("dev-dependencies", info));
            }
        }

        // Check [build-dependencies]
        if let Some(deps) = self
            .doc
            .get("build-dependencies")
            .and_then(|d| d.as_table())
        {
            if let Some(dep) = deps.get(name) {
                let info = extract_cargo_dep_info(name, dep);
                return Some(("build-dependencies", info));
            }
        }

        None
    }

    fn has_dependency(&self, section: &str, name: &str) -> bool {
        self.doc
            .get(section)
            .and_then(|s| s.as_table())
            .and_then(|t| t.get(name))
            .is_some()
    }

    fn add_dependency(
        &mut self,
        section: &str,
        name: &str,
        info: &DependencyInfo,
    ) -> ServerResult<()> {
        // Ensure section exists
        if !self.doc.contains_key(section) {
            self.doc[section] = Item::Table(toml_edit::Table::new());
        }

        let table = self.doc[section]
            .as_table_mut()
            .ok_or_else(|| ServerError::parse(format!("[{}] is not a table", section)))?;

        // Build the dependency value
        let dep_value = build_cargo_dep_value(info);
        table[name] = dep_value;

        debug!(dependency = %name, section = %section, "Added dependency to Cargo.toml");
        Ok(())
    }

    fn serialize(&self) -> String {
        self.doc.to_string()
    }
}

/// Extract dependency info from a TOML item
fn extract_cargo_dep_info(dep_name: &str, dep_item: &Item) -> DependencyInfo {
    let mut version = String::new();
    let mut features = None;
    let mut optional = None;

    match dep_item {
        Item::Value(TomlValue::String(v)) => {
            // Simple version string: "1.0"
            version = v.value().to_string();
        }
        Item::Value(TomlValue::InlineTable(table)) => {
            // Inline table: { version = "1.0", features = ["full"] }
            version = extract_version_from_table(table);
            features = extract_features_from_table(table);
            optional = table.get("optional").and_then(|v| v.as_bool());
        }
        Item::Table(table) => {
            // Regular table (multi-line)
            if let Some(v) = table.get("version").and_then(|v| v.as_str()) {
                version = v.to_string();
            } else if table.get("workspace").and_then(|v| v.as_bool()) == Some(true) {
                version = "workspace".to_string();
            } else if let Some(path) = table.get("path").and_then(|v| v.as_str()) {
                version = format!("path = \"{}\"", path);
            } else if let Some(git) = table.get("git").and_then(|v| v.as_str()) {
                version = format!("git = \"{}\"", git);
            }

            if let Some(f) = table.get("features").and_then(|v| v.as_array()) {
                let feat_list: Vec<String> = f
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                if !feat_list.is_empty() {
                    features = Some(feat_list);
                }
            }

            optional = table.get("optional").and_then(|v| v.as_bool());
        }
        _ => {
            version = "unknown".to_string();
        }
    }

    DependencyInfo {
        name: dep_name.to_string(),
        version,
        features,
        optional,
        already_exists: None,
    }
}

/// Extract version from inline table, handling various formats
fn extract_version_from_table(table: &toml_edit::InlineTable) -> String {
    if let Some(v) = table.get("version").and_then(|v| v.as_str()) {
        v.to_string()
    } else if table.get("workspace").and_then(|v| v.as_bool()) == Some(true) {
        "workspace".to_string()
    } else if let Some(path) = table.get("path").and_then(|v| v.as_str()) {
        format!("path = \"{}\"", path)
    } else if let Some(git) = table.get("git").and_then(|v| v.as_str()) {
        format!("git = \"{}\"", git)
    } else {
        "unknown".to_string()
    }
}

/// Extract features array from inline table
fn extract_features_from_table(table: &toml_edit::InlineTable) -> Option<Vec<String>> {
    table.get("features").and_then(|v| v.as_array()).map(|f| {
        f.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    })
}

/// Build a TOML value for a dependency
fn build_cargo_dep_value(info: &DependencyInfo) -> Item {
    // If no features and not optional, and it's a simple version (not path/git/workspace),
    // use simple version string
    if info.features.is_none()
        && info.optional.is_none()
        && !info.version.contains('=')
        && info.version != "workspace"
    {
        return Item::Value(TomlValue::String(toml_edit::Formatted::new(
            info.version.clone(),
        )));
    }

    // Otherwise build inline table
    let mut table = toml_edit::InlineTable::new();

    // Handle special version formats
    if info.version.starts_with("path = ") {
        let path = info
            .version
            .strip_prefix("path = \"")
            .unwrap_or(&info.version);
        let path = path.strip_suffix('"').unwrap_or(path);
        table.insert(
            "path",
            TomlValue::String(toml_edit::Formatted::new(path.to_string())),
        );
    } else if info.version.starts_with("git = ") {
        let git = info
            .version
            .strip_prefix("git = \"")
            .unwrap_or(&info.version);
        let git = git.strip_suffix('"').unwrap_or(git);
        table.insert(
            "git",
            TomlValue::String(toml_edit::Formatted::new(git.to_string())),
        );
    } else if info.version == "workspace" {
        table.insert(
            "workspace",
            TomlValue::Boolean(toml_edit::Formatted::new(true)),
        );
    } else {
        table.insert(
            "version",
            TomlValue::String(toml_edit::Formatted::new(info.version.clone())),
        );
    }

    // Add features if present
    if let Some(features) = &info.features {
        let mut arr = toml_edit::Array::new();
        for f in features {
            arr.push(f.as_str());
        }
        table.insert("features", TomlValue::Array(arr));
    }

    // Add optional flag if present
    if let Some(opt) = info.optional {
        table.insert(
            "optional",
            TomlValue::Boolean(toml_edit::Formatted::new(opt)),
        );
    }

    Item::Value(TomlValue::InlineTable(table))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_dependency_simple() {
        let content = r#"
[dependencies]
serde = "1.0"
"#;
        let manifest = CargoManifest::parse(content).unwrap();
        let result = manifest.find_dependency("serde");
        assert!(result.is_some());
        let (section, info) = result.unwrap();
        assert_eq!(section, "dependencies");
        assert_eq!(info.name, "serde");
        assert_eq!(info.version, "1.0");
    }

    #[test]
    fn test_find_dependency_with_features() {
        let content = r#"
[dependencies]
tokio = { version = "1.0", features = ["full", "rt"] }
"#;
        let manifest = CargoManifest::parse(content).unwrap();
        let result = manifest.find_dependency("tokio");
        assert!(result.is_some());
        let (_, info) = result.unwrap();
        assert_eq!(info.version, "1.0");
        assert_eq!(
            info.features,
            Some(vec!["full".to_string(), "rt".to_string()])
        );
    }

    #[test]
    fn test_find_dependency_dev() {
        let content = r#"
[dev-dependencies]
tempfile = "3.0"
"#;
        let manifest = CargoManifest::parse(content).unwrap();
        let result = manifest.find_dependency("tempfile");
        assert!(result.is_some());
        let (section, _) = result.unwrap();
        assert_eq!(section, "dev-dependencies");
    }

    #[test]
    fn test_find_dependency_workspace() {
        let content = r#"
[dependencies]
my-crate = { workspace = true }
"#;
        let manifest = CargoManifest::parse(content).unwrap();
        let result = manifest.find_dependency("my-crate");
        assert!(result.is_some());
        let (_, info) = result.unwrap();
        assert_eq!(info.version, "workspace");
    }

    #[test]
    fn test_add_dependency() {
        let content = r#"
[dependencies]
existing = "1.0"
"#;
        let mut manifest = CargoManifest::parse(content).unwrap();
        let info = DependencyInfo {
            name: "new-dep".to_string(),
            version: "2.0".to_string(),
            features: None,
            optional: None,
            already_exists: None,
        };
        manifest
            .add_dependency("dependencies", "new-dep", &info)
            .unwrap();
        let serialized = manifest.serialize();
        assert!(serialized.contains("new-dep"));
        assert!(serialized.contains("2.0"));
    }

    #[test]
    fn test_has_dependency() {
        let content = r#"
[dependencies]
serde = "1.0"
"#;
        let manifest = CargoManifest::parse(content).unwrap();
        assert!(manifest.has_dependency("dependencies", "serde"));
        assert!(!manifest.has_dependency("dependencies", "tokio"));
        assert!(!manifest.has_dependency("dev-dependencies", "serde"));
    }

    #[test]
    fn test_sections() {
        assert_eq!(
            CargoManifest::sections(),
            &["dependencies", "dev-dependencies", "build-dependencies"]
        );
    }

    #[test]
    fn test_default_section() {
        assert_eq!(CargoManifest::default_section(), "dependencies");
    }
}
