//! Import/rename support implementation for TOML files

use cb_plugin_api::{ImportRenameSupport, PluginResult};
use std::path::Path;
use toml_edit::{DocumentMut, Item, Value};

pub struct TomlImportSupport;

impl TomlImportSupport {
    pub fn new() -> Self {
        Self
    }

    /// Rewrite paths in TOML file
    pub fn rewrite_toml_paths(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
    ) -> PluginResult<(String, usize)> {
        let mut doc: DocumentMut = content.parse().map_err(|e| {
            cb_plugin_api::PluginError::parse(format!("Failed to parse TOML: {}", e))
        })?;
        let mut changes = 0;

        let old_path_str = old_path.to_string_lossy();
        let new_path_str = new_path.to_string_lossy();

        // Update root items
        for (_key, item) in doc.iter_mut() {
            Self::update_toml_item(item, &old_path_str, &new_path_str, &mut changes);
        }

        Ok((doc.to_string(), changes))
    }

    /// Recursively update paths in TOML values
    fn update_toml_item(item: &mut Item, old_path: &str, new_path: &str, changes: &mut usize) {
        match item {
            Item::Value(Value::String(s)) => {
                let formatted = s.value();
                if Self::is_path_like(formatted) {
                    // Skip if already updated (idempotency check for nested renames)
                    let is_nested_rename = new_path.starts_with(&format!("{}/", old_path));
                    if is_nested_rename && formatted.contains(new_path) {
                        return;
                    }

                    // Match at start of path, not anywhere
                    if formatted == old_path || formatted.starts_with(&format!("{}/", old_path)) {
                        let new_value = formatted.replacen(old_path, new_path, 1);
                        *s = toml_edit::Formatted::new(new_value);
                        *changes += 1;
                    }
                }
            }
            Item::Table(table) => {
                for (_key, value) in table.iter_mut() {
                    Self::update_toml_item(value, old_path, new_path, changes);
                }
            }
            Item::ArrayOfTables(array) => {
                for table in array.iter_mut() {
                    for (_key, value) in table.iter_mut() {
                        Self::update_toml_item(value, old_path, new_path, changes);
                    }
                }
            }
            Item::Value(Value::Array(arr)) => {
                for value in arr.iter_mut() {
                    if let Value::String(s) = value {
                        let formatted = s.value();
                        if Self::is_path_like(formatted) {
                            // Skip if already updated (idempotency check for nested renames)
                            let is_nested_rename = new_path.starts_with(&format!("{}/", old_path));
                            if is_nested_rename && formatted.contains(new_path) {
                                continue;
                            }

                            // Match at start of path, not anywhere
                            if formatted == old_path
                                || formatted.starts_with(&format!("{}/", old_path))
                            {
                                let new_value = formatted.replacen(old_path, new_path, 1);
                                *s = toml_edit::Formatted::new(new_value);
                                *changes += 1;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn is_path_like(s: &str) -> bool {
        s.contains('/')
            || s.contains('\\')
            || s.ends_with(".rs")
            || s.ends_with(".toml")
            || s.ends_with(".md")
            || s.ends_with(".yml")
            || s.ends_with(".yaml")
    }
}

impl ImportRenameSupport for TomlImportSupport {
    fn rewrite_imports_for_rename(
        &self,
        content: &str,
        old_name: &str,
        new_name: &str,
    ) -> (String, usize) {
        // For TOML, old_name and new_name are path patterns
        // We use the internal method which handles TOML structure properly
        match self.rewrite_toml_paths(content, Path::new(old_name), Path::new(new_name)) {
            Ok((new_content, count)) => (new_content, count),
            Err(_) => (content.to_string(), 0),
        }
    }
}
