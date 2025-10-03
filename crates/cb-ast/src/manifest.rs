//! Language-agnostic dependency manifest manipulation
//!
//! This module provides a unified interface for updating dependencies across
//! different package managers and languages (Cargo, npm, etc.)

use crate::error::{AstError, AstResult};
use std::path::Path;
use toml_edit::{value, DocumentMut, Item};

/// A trait for interacting with language-specific dependency manifest files.
pub trait Manifest: Send + Sync {
    /// Renames a dependency and/or updates its path.
    ///
    /// # Arguments
    ///
    /// * `old_name` - The current name of the dependency
    /// * `new_name` - The new name for the dependency
    /// * `new_path` - Optional new path for the dependency (for local/path dependencies)
    fn rename_dependency(
        &mut self,
        old_name: &str,
        new_name: &str,
        new_path: Option<&str>,
    ) -> AstResult<()>;

    /// Serializes the manifest content back to a string.
    fn to_string(&self) -> AstResult<String>;
}

/// Rust Cargo.toml manifest handler
pub struct CargoManifest(DocumentMut);

impl Manifest for CargoManifest {
    fn rename_dependency(
        &mut self,
        old_name: &str,
        new_name: &str,
        new_path: Option<&str>,
    ) -> AstResult<()> {
        // Helper function to preserve metadata when renaming
        fn rename_in_table(
            deps: &mut dyn toml_edit::TableLike,
            old_name: &str,
            new_name: &str,
            new_path: Option<&str>,
        ) {
            if let Some(old_dep) = deps.remove(old_name) {
                // Preserve all existing metadata
                let new_dep = match old_dep {
                    Item::Value(ref val) if val.is_inline_table() => {
                        // It's an inline table, preserve all fields and update path
                        if let Some(table) = val.as_inline_table() {
                            // Manually copy fields instead of clone to avoid compiler issues
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
                        // It's a version string
                        if let Some(path) = new_path {
                            // Convert to inline table with path
                            let mut new_table = toml_edit::InlineTable::new();
                            new_table.insert("path", path.into());
                            value(new_table)
                        } else {
                            // Keep the version string
                            old_dep
                        }
                    }
                    _ => {
                        // For other types, preserve as-is or create new table if path provided
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
        if let Some(deps) = self
            .0
            .get_mut("dependencies")
            .and_then(Item::as_table_like_mut)
        {
            rename_in_table(deps, old_name, new_name, new_path);
        }

        // Update in [dev-dependencies]
        if let Some(dev_deps) = self
            .0
            .get_mut("dev-dependencies")
            .and_then(Item::as_table_like_mut)
        {
            rename_in_table(dev_deps, old_name, new_name, new_path);
        }

        // Update in [build-dependencies]
        if let Some(build_deps) = self
            .0
            .get_mut("build-dependencies")
            .and_then(Item::as_table_like_mut)
        {
            rename_in_table(build_deps, old_name, new_name, new_path);
        }

        Ok(())
    }

    fn to_string(&self) -> AstResult<String> {
        Ok(self.0.to_string())
    }
}

/// npm/pnpm package.json manifest handler
pub struct NpmManifest(serde_json::Value);

impl Manifest for NpmManifest {
    fn rename_dependency(
        &mut self,
        old_name: &str,
        new_name: &str,
        new_path: Option<&str>,
    ) -> AstResult<()> {
        if let Some(obj) = self.0.as_object_mut() {
            // Update in "dependencies"
            if let Some(deps) = obj.get_mut("dependencies").and_then(|v| v.as_object_mut()) {
                if let Some(old_value) = deps.remove(old_name) {
                    let new_value = if let Some(path) = new_path {
                        serde_json::Value::String(format!("file:{}", path))
                    } else {
                        old_value
                    };
                    deps.insert(new_name.to_string(), new_value);
                }
            }

            // Update in "devDependencies"
            if let Some(dev_deps) = obj
                .get_mut("devDependencies")
                .and_then(|v| v.as_object_mut())
            {
                if let Some(old_value) = dev_deps.remove(old_name) {
                    let new_value = if let Some(path) = new_path {
                        serde_json::Value::String(format!("file:{}", path))
                    } else {
                        old_value
                    };
                    dev_deps.insert(new_name.to_string(), new_value);
                }
            }
        }

        Ok(())
    }

    fn to_string(&self) -> AstResult<String> {
        serde_json::to_string_pretty(&self.0).map_err(|e| AstError::Analysis {
            message: format!("Failed to serialize package.json: {}", e),
        })
    }
}

/// Factory function to load the appropriate manifest handler based on file name
pub fn load_manifest(path: &Path, content: &str) -> AstResult<Box<dyn Manifest>> {
    match path.file_name().and_then(|s| s.to_str()) {
        Some("Cargo.toml") => {
            let doc = content
                .parse::<DocumentMut>()
                .map_err(|e| AstError::Analysis {
                    message: format!("Failed to parse Cargo.toml: {}", e),
                })?;
            Ok(Box::new(CargoManifest(doc)))
        }
        Some("package.json") => {
            let json = serde_json::from_str(content).map_err(|e| AstError::Analysis {
                message: format!("Failed to parse package.json: {}", e),
            })?;
            Ok(Box::new(NpmManifest(json)))
        }
        _ => Err(AstError::Analysis {
            message: format!(
                "Unsupported manifest type: {:?}",
                path.file_name().unwrap_or_default()
            ),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cargo_manifest_rename_dependency() {
        let cargo_toml = r#"
[package]
name = "test-crate"

[dependencies]
cb-mcp-proxy = { path = "../cb-mcp-proxy" }
other-dep = "1.0"
"#;

        let doc = cargo_toml.parse::<DocumentMut>().unwrap();
        let mut manifest = CargoManifest(doc);

        manifest
            .rename_dependency("cb-mcp-proxy", "cb-plugins", Some("../cb-plugins"))
            .unwrap();

        let result = manifest.to_string().unwrap();
        assert!(result.contains("cb-plugins"));
        assert!(!result.contains("cb-mcp-proxy"));
        assert!(result.contains("../cb-plugins"));
    }

    #[test]
    fn test_cargo_manifest_preserves_metadata() {
        let cargo_toml = r#"
[package]
name = "test-crate"

[dependencies]
my-dep = { path = "../my-dep", version = "0.1", optional = true, features = ["feat1", "feat2"], default-features = false }
"#;

        let doc = cargo_toml.parse::<DocumentMut>().unwrap();
        let mut manifest = CargoManifest(doc);

        manifest
            .rename_dependency("my-dep", "renamed-dep", Some("../renamed-dep"))
            .unwrap();

        let result = manifest.to_string().unwrap();

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
    fn test_npm_manifest_rename_dependency() {
        let package_json = r#"{
  "name": "test-package",
  "dependencies": {
    "@old/package": "file:../old-path",
    "other-dep": "^1.0.0"
  }
}"#;

        let json = serde_json::from_str(package_json).unwrap();
        let mut manifest = NpmManifest(json);

        manifest
            .rename_dependency("@old/package", "@new/package", Some("../new-path"))
            .unwrap();

        let result = manifest.to_string().unwrap();
        assert!(result.contains("@new/package"));
        assert!(!result.contains("@old/package"));
        assert!(result.contains("file:../new-path"));
    }

    #[test]
    fn test_load_manifest_cargo() {
        let path = Path::new("Cargo.toml");
        let content = r#"
[package]
name = "test"

[dependencies]
"#;

        let manifest = load_manifest(path, content).unwrap();
        assert!(manifest.to_string().unwrap().contains("[package]"));
    }

    #[test]
    fn test_load_manifest_npm() {
        let path = Path::new("package.json");
        let content = r#"{"name": "test", "dependencies": {}}"#;

        let manifest = load_manifest(path, content).unwrap();
        assert!(manifest.to_string().unwrap().contains("name"));
    }
}
