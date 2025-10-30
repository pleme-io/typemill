//! Import/rename support implementation for TOML files

use mill_plugin_api::{ImportRenameSupport, PluginResult};
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
        update_exact_matches: bool,
    ) -> PluginResult<(String, usize)> {
        let mut doc: DocumentMut = content.parse().map_err(|e| {
            mill_plugin_api::PluginError::parse(format!("Failed to parse TOML: {}", e))
        })?;
        let mut changes = 0;

        let old_path_str = old_path.to_string_lossy();
        let new_path_str = new_path.to_string_lossy();

        // Update root items
        for (_key, item) in doc.iter_mut() {
            Self::update_toml_item(
                item,
                &old_path_str,
                &new_path_str,
                update_exact_matches,
                &mut changes,
            );
        }

        Ok((doc.to_string(), changes))
    }

    /// Recursively update paths in TOML values
    fn update_toml_item(
        item: &mut Item,
        old_path: &str,
        new_path: &str,
        update_exact_matches: bool,
        changes: &mut usize,
    ) {
        match item {
            Item::Value(Value::String(s)) => {
                let formatted = s.value();
                if Self::should_update_string(formatted, old_path, update_exact_matches) {
                    // Skip if already updated (idempotency check for nested renames)
                    let is_nested_rename = new_path.starts_with(&format!("{}/", old_path));
                    if is_nested_rename && formatted.contains(new_path) {
                        return;
                    }

                    // Check for cargo command-line flags first
                    if update_exact_matches {
                        if let Some(new_value) =
                            Self::update_cargo_flags(formatted, old_path, new_path)
                        {
                            *s = toml_edit::Formatted::new(new_value);
                            *changes += 1;
                            return;
                        }
                    }

                    // Check for basename match (exact identifier matching)
                    if update_exact_matches {
                        if let (Some(old_basename), Some(new_basename)) = (
                            std::path::Path::new(old_path).file_name(),
                            std::path::Path::new(new_path).file_name(),
                        ) {
                            let old_basename_str = old_basename.to_string_lossy();
                            let new_basename_str = new_basename.to_string_lossy();

                            if formatted == &*old_basename_str {
                                *s = toml_edit::Formatted::new(new_basename_str.to_string());
                                *changes += 1;
                                return;
                            }
                        }
                    }

                    // Path-based replacement (full path or path prefix)
                    if formatted == old_path || formatted.starts_with(&format!("{}/", old_path)) {
                        let new_value = formatted.replacen(old_path, new_path, 1);
                        *s = toml_edit::Formatted::new(new_value);
                        *changes += 1;
                    }
                }
            }
            Item::Table(table) => {
                for (_key, value) in table.iter_mut() {
                    Self::update_toml_item(
                        value,
                        old_path,
                        new_path,
                        update_exact_matches,
                        changes,
                    );
                }
            }
            Item::ArrayOfTables(array) => {
                for table in array.iter_mut() {
                    for (_key, value) in table.iter_mut() {
                        Self::update_toml_item(
                            value,
                            old_path,
                            new_path,
                            update_exact_matches,
                            changes,
                        );
                    }
                }
            }
            Item::Value(Value::Array(arr)) => {
                for value in arr.iter_mut() {
                    if let Value::String(s) = value {
                        let formatted = s.value();
                        if Self::should_update_string(formatted, old_path, update_exact_matches) {
                            // Skip if already updated (idempotency check for nested renames)
                            let is_nested_rename = new_path.starts_with(&format!("{}/", old_path));
                            if is_nested_rename && formatted.contains(new_path) {
                                continue;
                            }

                            // Check for cargo command-line flags first
                            if update_exact_matches {
                                if let Some(new_value) =
                                    Self::update_cargo_flags(formatted, old_path, new_path)
                                {
                                    *s = toml_edit::Formatted::new(new_value);
                                    *changes += 1;
                                    continue;
                                }
                            }

                            // Check for basename match (exact identifier matching)
                            if update_exact_matches {
                                if let (Some(old_basename), Some(new_basename)) = (
                                    std::path::Path::new(old_path).file_name(),
                                    std::path::Path::new(new_path).file_name(),
                                ) {
                                    let old_basename_str = old_basename.to_string_lossy();
                                    let new_basename_str = new_basename.to_string_lossy();

                                    if formatted == &*old_basename_str {
                                        *s =
                                            toml_edit::Formatted::new(new_basename_str.to_string());
                                        *changes += 1;
                                        continue;
                                    }
                                }
                            }

                            // Path-based replacement (full path or path prefix)
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

    /// Check if a string should be updated based on matching strategy
    fn should_update_string(s: &str, old_path: &str, update_exact_matches: bool) -> bool {
        if Self::is_path_like(s) {
            // Always update path-like strings
            return true;
        }

        if update_exact_matches {
            // Check for exact match (identifier matching)
            // Match either the full path or just the basename
            // This handles cases like "cb-test-support" matching "/workspace/crates/cb-test-support"
            if s == old_path {
                return true;
            }

            // Also match against the directory/file name (basename)
            if let Some(basename) = std::path::Path::new(old_path).file_name() {
                let basename_str = basename.to_string_lossy();
                if s == basename_str {
                    return true;
                }
            }
        }

        false
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

    /// Update cargo command-line flags containing crate names
    ///
    /// Handles patterns like:
    /// - "-p cb-lang-rust" → "-p mill-lang-rust"
    /// - "--package cb-lang-rust" → "--package mill-lang-rust"
    /// - "check -p cb-lang-rust -p other" → "check -p mill-lang-rust -p other"
    fn update_cargo_flags(s: &str, old_path: &str, new_path: &str) -> Option<String> {
        // Extract crate name from path (last component)
        let old_crate_name = std::path::Path::new(old_path)
            .file_name()?
            .to_string_lossy();
        let new_crate_name = std::path::Path::new(new_path)
            .file_name()?
            .to_string_lossy();

        // Check if string contains cargo package flags
        let has_package_flag = s.contains("-p ") || s.contains("--package ");

        if !has_package_flag {
            return None;
        }

        // Replace crate names after -p or --package flags
        let mut result = s.to_string();
        let mut changed = false;

        // Handle -p flag
        for pattern in [
            format!("-p {}", old_crate_name),
            format!("--package {}", old_crate_name),
        ] {
            if result.contains(&pattern) {
                let replacement = pattern.replace(&*old_crate_name, &new_crate_name);
                result = result.replace(&pattern, &replacement);
                changed = true;
            }
        }

        if changed {
            Some(result)
        } else {
            None
        }
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
        // Default to false for exact matches (legacy/generic path)
        match self.rewrite_toml_paths(content, Path::new(old_name), Path::new(new_name), false) {
            Ok((new_content, count)) => (new_content, count),
            Err(_) => (content.to_string(), 0),
        }
    }
}
