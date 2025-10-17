//! Import/rename support implementation for YAML files
//!
//! CRITICAL: This implementation uses line-by-line string replacement to preserve:
//! - Comments (both # at line start and inline)
//! - Indentation (exact whitespace)
//! - Blank lines
//! - Key ordering
//! - Trailing newlines
//!
//! We do NOT use serde_yaml parsing as it destroys all formatting.

use cb_plugin_api::{ImportRenameSupport, PluginResult};
use std::path::Path;

pub struct YamlImportSupport;

impl YamlImportSupport {
    pub fn new() -> Self {
        Self
    }

    /// Rewrite paths in YAML file while preserving ALL formatting
    ///
    /// This function processes YAML line-by-line using string replacement instead of
    /// parsing/serializing to preserve comments, indentation, and blank lines.
    pub fn rewrite_yaml_paths(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
    ) -> PluginResult<(String, usize)> {
        let old_path_str = old_path.to_string_lossy();
        let new_path_str = new_path.to_string_lossy();

        let mut changes = 0;
        let mut result_lines = Vec::new();

        // Process line by line to preserve formatting
        for line in content.lines() {
            let mut line_modified = line.to_string();

            // Skip comment-only lines (preserve them unchanged)
            let trimmed = line.trim();
            if trimmed.starts_with('#') || trimmed.is_empty() {
                result_lines.push(line_modified);
                continue;
            }

            // Only update values (after ':'), not keys
            if let Some(colon_pos) = line.find(':') {
                let key_part = &line[..colon_pos];
                let value_part = &line[colon_pos + 1..];

                // Skip if already updated (idempotency check for nested renames)
                let is_nested_rename = new_path_str.as_ref().starts_with(&format!("{}/", old_path_str));
                if is_nested_rename && value_part.contains(new_path_str.as_ref()) {
                    // Already updated, skip
                } else if value_part.contains(old_path_str.as_ref()) && Self::is_path_like(value_part.trim()) {
                    let new_value = value_part.replacen(old_path_str.as_ref(), new_path_str.as_ref(), 1);
                    line_modified = format!("{}:{}", key_part, new_value);
                    changes += 1;
                }
            } else if line.contains(old_path_str.as_ref()) && Self::is_path_like(line.trim()) {
                // Handle lines without colon (e.g., list items like "- some/path")
                // Skip if already updated (idempotency check for nested renames)
                let is_nested_rename = new_path_str.as_ref().starts_with(&format!("{}/", old_path_str));
                if !(is_nested_rename && line.contains(new_path_str.as_ref())) {
                    line_modified = line.replacen(old_path_str.as_ref(), new_path_str.as_ref(), 1);
                    changes += 1;
                }
            }

            result_lines.push(line_modified);
        }

        let mut modified = result_lines.join("\n");

        // Preserve trailing newline if original had one
        if content.ends_with('\n') && !modified.ends_with('\n') {
            modified.push('\n');
        }

        if changes > 0 {
            tracing::info!(
                changes = changes,
                old_path = %old_path.display(),
                new_path = %new_path.display(),
                "Updated paths in YAML file (formatting preserved)"
            );
        }

        Ok((modified, changes))
    }

    fn is_path_like(s: &str) -> bool {
        s.contains('/') || s.contains('\\') ||
        s.ends_with(".rs") || s.ends_with(".toml") ||
        s.ends_with(".yml") || s.ends_with(".yaml") ||
        s.ends_with(".md") || s.ends_with(".json") ||
        s.ends_with(".js") || s.ends_with(".ts")
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
