//! Import/rename support implementation for YAML files

use cb_plugin_api::{ImportRenameSupport, PluginResult};
use serde_yaml::Value;
use std::path::Path;

pub struct YamlImportSupport;

impl YamlImportSupport {
    pub fn new() -> Self {
        Self
    }

    /// Rewrite paths in YAML file
    pub fn rewrite_yaml_paths(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
    ) -> PluginResult<(String, usize)> {
        let mut value: Value = serde_yaml::from_str(content).map_err(|e| {
            cb_plugin_api::PluginError::parse(format!("Failed to parse YAML: {}", e))
        })?;
        let mut changes = 0;

        let old_path_str = old_path.to_string_lossy();
        let new_path_str = new_path.to_string_lossy();

        Self::update_yaml_value(&mut value, &old_path_str, &new_path_str, &mut changes);

        let new_content = serde_yaml::to_string(&value).map_err(|e| {
            cb_plugin_api::PluginError::internal(format!("Failed to serialize YAML: {}", e))
        })?;

        Ok((new_content, changes))
    }

    fn update_yaml_value(
        value: &mut Value,
        old_path: &str,
        new_path: &str,
        changes: &mut usize,
    ) {
        match value {
            Value::String(s) => {
                if s.contains(old_path) && Self::is_path_like(s) {
                    *s = s.replace(old_path, new_path);
                    *changes += 1;
                }
            }
            Value::Sequence(seq) => {
                for item in seq.iter_mut() {
                    Self::update_yaml_value(item, old_path, new_path, changes);
                }
            }
            Value::Mapping(map) => {
                for (_k, v) in map.iter_mut() {
                    Self::update_yaml_value(v, old_path, new_path, changes);
                }
            }
            _ => {}
        }
    }

    fn is_path_like(s: &str) -> bool {
        s.contains('/') || s.contains('\\') || s.ends_with(".rs") || s.ends_with(".toml") ||
        s.ends_with(".yml") || s.ends_with(".yaml") || s.ends_with(".md")
    }
}

impl ImportRenameSupport for YamlImportSupport {
    fn rewrite_imports_for_rename(
        &self,
        content: &str,
        old_name: &str,
        new_name: &str,
    ) -> (String, usize) {
        // For YAML, old_name and new_name are path patterns
        // We use the internal method which handles YAML structure properly
        match self.rewrite_yaml_paths(content, Path::new(old_name), Path::new(new_name)) {
            Ok((new_content, count)) => (new_content, count),
            Err(_) => (content.to_string(), 0),
        }
    }
}
