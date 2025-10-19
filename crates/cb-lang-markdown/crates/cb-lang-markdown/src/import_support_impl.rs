//! Import support implementation for Markdown
//!
//! Treats markdown file links as "imports" for the purpose of file rename tracking.

use cb_plugin_api::{ImportSupport, PluginResult};
use codebuddy_foundation::protocol::DependencyUpdate;
use regex::{Captures, Regex};
use std::path::Path;
use tracing::debug;

/// Import support for markdown files
///
/// Detects and updates file references in markdown links
pub struct MarkdownImportSupport {
    /// Regex for inline markdown links: [text](path) or ![alt](path)
    inline_link_regex: Regex,
    /// Regex for reference-style link definitions: [ref]: path
    ref_definition_regex: Regex,
    /// Regex for autolinks: <path>
    autolink_regex: Regex,
}

impl MarkdownImportSupport {
    pub fn new() -> Self {
        // Matches inline links: [text](path) or ![alt](path)
        // Fixed regex: [^] means "not ]", not "not \"
        let inline_link_regex = Regex::new(r"!?\[([^]]+)\]\(([^)]+)\)").unwrap();

        // Matches reference-style link definitions: [ref]: path
        // Must be at start of line (after optional whitespace)
        let ref_definition_regex = Regex::new(r"(?m)^\s*\[([^]]+)\]:\s*(\S+)").unwrap();

        // Matches autolinks: <path>
        // Excludes mailto: and other URL schemes
        let autolink_regex = Regex::new(r"<([^>]+)>").unwrap();

        Self {
            inline_link_regex,
            ref_definition_regex,
            autolink_regex,
        }
    }

    /// Check if a path looks like a file reference (not a URL)
    fn is_file_reference(path: &str) -> bool {
        // Not a URL (http://, https://, mailto:, etc.)
        !path.starts_with("http://")
            && !path.starts_with("https://")
            && !path.starts_with("mailto:")
            && !path.starts_with("ftp://")
            && !path.starts_with('#') // Not just an anchor
    }

    /// Extract the path without anchor
    fn path_without_anchor(path: &str) -> &str {
        path.split('#').next().unwrap_or(path)
    }

    /// Normalize path for comparison (resolve relative paths)
    fn normalize_path(path: &str) -> String {
        // Remove leading ./
        let trimmed = path.trim_start_matches("./");

        // For comparison purposes, we keep ../ segments intact
        // The normalization is mainly about removing leading ./ for consistency
        // Full path resolution (../../foo -> /bar/foo) requires context of the current file
        trimmed.to_string()
    }

    /// Extract anchor from path if present
    fn extract_anchor(path: &str) -> String {
        if path.contains('#') {
            path.split('#')
                .nth(1)
                .map(|a| format!("#{}", a))
                .unwrap_or_default()
        } else {
            String::new()
        }
    }

    /// Build markdown inline link with preserved anchor and image syntax
    fn build_link(full_match: &str, link_text: &str, new_path: &str, original_path: &str) -> String {
        let anchor = Self::extract_anchor(original_path);
        let prefix = if full_match.starts_with('!') { "!" } else { "" };
        format!("{}[{}]({}{})", prefix, link_text, new_path, anchor)
    }

    /// Build markdown reference-style link definition with preserved anchor and whitespace
    fn build_ref_definition(full_match: &str, ref_label: &str, new_path: &str, original_path: &str) -> String {
        let anchor = Self::extract_anchor(original_path);
        let leading_ws = full_match
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect::<String>();
        format!("{}[{}]: {}{}", leading_ws, ref_label, new_path, anchor)
    }

    /// Build markdown autolink with preserved anchor
    fn build_autolink(new_path: &str, original_path: &str) -> String {
        let anchor = Self::extract_anchor(original_path);
        format!("<{}{}>", new_path, anchor)
    }
}

impl Default for MarkdownImportSupport {
    fn default() -> Self {
        Self::new()
    }
}

impl ImportSupport for MarkdownImportSupport {
    fn parse_imports(&self, content: &str) -> Vec<String> {
        let mut imports = Vec::new();

        // 1. Parse inline links: [text](path) or ![alt](path)
        for captures in self.inline_link_regex.captures_iter(content) {
            if let Some(path_match) = captures.get(2) {
                let path = path_match.as_str();

                if Self::is_file_reference(path) {
                    let clean_path = Self::path_without_anchor(path);
                    if !clean_path.is_empty() {
                        imports.push(Self::normalize_path(clean_path));
                    }
                }
            }
        }

        // 2. Parse reference-style link definitions: [ref]: path
        for captures in self.ref_definition_regex.captures_iter(content) {
            if let Some(path_match) = captures.get(2) {
                let path = path_match.as_str();

                if Self::is_file_reference(path) {
                    let clean_path = Self::path_without_anchor(path);
                    if !clean_path.is_empty() {
                        imports.push(Self::normalize_path(clean_path));
                    }
                }
            }
        }

        // 3. Parse autolinks: <path>
        for captures in self.autolink_regex.captures_iter(content) {
            if let Some(path_match) = captures.get(1) {
                let path = path_match.as_str();

                if Self::is_file_reference(path) {
                    let clean_path = Self::path_without_anchor(path);
                    if !clean_path.is_empty() {
                        imports.push(Self::normalize_path(clean_path));
                    }
                }
            }
        }

        debug!(
            imports = imports.len(),
            "Parsed markdown file references (inline + reference-style + autolinks)"
        );
        imports
    }

    fn rewrite_imports_for_rename(
        &self,
        content: &str,
        old_name: &str,
        new_name: &str,
    ) -> (String, usize) {
        let mut count = 0;

        // Rewrite inline links
        let mut result = self.inline_link_regex.replace_all(content, |caps: &Captures| {
            let full_match = caps.get(0).unwrap().as_str();
            let link_text = caps.get(1).unwrap().as_str();
            let path = caps.get(2).unwrap().as_str();

            if Self::is_file_reference(path) {
                let clean_path = Self::path_without_anchor(path);

                if clean_path == old_name || clean_path == format!("./{}", old_name) {
                    count += 1;
                    return Self::build_link(full_match, link_text, new_name, path);
                }
            }

            full_match.to_string()
        }).to_string();

        // Rewrite reference-style link definitions
        result = self.ref_definition_regex.replace_all(&result, |caps: &Captures| {
            let full_match = caps.get(0).unwrap().as_str();
            let ref_label = caps.get(1).unwrap().as_str();
            let path = caps.get(2).unwrap().as_str();

            if Self::is_file_reference(path) {
                let clean_path = Self::path_without_anchor(path);

                if clean_path == old_name || clean_path == format!("./{}", old_name) {
                    count += 1;
                    return Self::build_ref_definition(full_match, ref_label, new_name, path);
                }
            }

            full_match.to_string()
        }).to_string();

        // Rewrite autolinks
        result = self.autolink_regex.replace_all(&result, |caps: &Captures| {
            let full_match = caps.get(0).unwrap().as_str();
            let path = caps.get(1).unwrap().as_str();

            if Self::is_file_reference(path) {
                let clean_path = Self::path_without_anchor(path);

                if clean_path == old_name || clean_path == format!("./{}", old_name) {
                    count += 1;
                    return Self::build_autolink(new_name, path);
                }
            }

            full_match.to_string()
        }).to_string();

        debug!(changes = count, "Rewrote markdown links for rename (inline + reference-style + autolinks)");
        (result, count)
    }

    fn rewrite_imports_for_move(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
    ) -> (String, usize) {
        let mut count = 0;

        let old_str = old_path.to_string_lossy();
        let new_str = new_path.to_string_lossy();

        // Rewrite inline links
        let mut result = self.inline_link_regex.replace_all(content, |caps: &Captures| {
            let full_match = caps.get(0).unwrap().as_str();
            let link_text = caps.get(1).unwrap().as_str();
            let path = caps.get(2).unwrap().as_str();

            if Self::is_file_reference(path) {
                let clean_path = Self::path_without_anchor(path);
                let normalized = Self::normalize_path(clean_path);

                if normalized == Self::normalize_path(&old_str)
                    || clean_path.ends_with(old_path.to_str().unwrap_or(""))
                {
                    count += 1;
                    return Self::build_link(full_match, link_text, &new_str, path);
                }
            }

            full_match.to_string()
        }).to_string();

        // Rewrite reference-style link definitions
        result = self.ref_definition_regex.replace_all(&result, |caps: &Captures| {
            let full_match = caps.get(0).unwrap().as_str();
            let ref_label = caps.get(1).unwrap().as_str();
            let path = caps.get(2).unwrap().as_str();

            if Self::is_file_reference(path) {
                let clean_path = Self::path_without_anchor(path);
                let normalized = Self::normalize_path(clean_path);

                if normalized == Self::normalize_path(&old_str)
                    || clean_path.ends_with(old_path.to_str().unwrap_or(""))
                {
                    count += 1;
                    return Self::build_ref_definition(full_match, ref_label, &new_str, path);
                }
            }

            full_match.to_string()
        }).to_string();

        // Rewrite autolinks
        result = self.autolink_regex.replace_all(&result, |caps: &Captures| {
            let full_match = caps.get(0).unwrap().as_str();
            let path = caps.get(1).unwrap().as_str();

            if Self::is_file_reference(path) {
                let clean_path = Self::path_without_anchor(path);
                let normalized = Self::normalize_path(clean_path);

                if normalized == Self::normalize_path(&old_str)
                    || clean_path.ends_with(old_path.to_str().unwrap_or(""))
                {
                    count += 1;
                    return Self::build_autolink(&new_str, path);
                }
            }

            full_match.to_string()
        }).to_string();

        debug!(changes = count, old_path = ?old_path, new_path = ?new_path, "Rewrote markdown links for move (inline + reference-style + autolinks)");
        (result, count)
    }

    fn contains_import(&self, content: &str, module: &str) -> bool {
        let imports = self.parse_imports(content);
        imports.iter().any(|imp| imp == module || imp.ends_with(module))
    }

    fn add_import(&self, content: &str, module: &str) -> String {
        // For markdown, "adding an import" means adding a link at the end
        // This is rarely used, but we provide a basic implementation
        format!("{}\n\n[{}]({})", content.trim_end(), module, module)
    }

    fn remove_import(&self, content: &str, module: &str) -> String {
        // Remove inline links
        let mut result = self.inline_link_regex.replace_all(content, |caps: &Captures| {
            let full_match = caps.get(0).unwrap().as_str();
            let path = caps.get(2).unwrap().as_str();

            if Self::is_file_reference(path) {
                let clean_path = Self::path_without_anchor(path);
                let normalized = Self::normalize_path(clean_path);

                if normalized == module || clean_path.ends_with(module) {
                    return String::new(); // Remove the link
                }
            }

            full_match.to_string()
        }).to_string();

        // Remove reference-style link definitions
        result = self.ref_definition_regex.replace_all(&result, |caps: &Captures| {
            let full_match = caps.get(0).unwrap().as_str();
            let path = caps.get(2).unwrap().as_str();

            if Self::is_file_reference(path) {
                let clean_path = Self::path_without_anchor(path);
                let normalized = Self::normalize_path(clean_path);

                if normalized == module || clean_path.ends_with(module) {
                    return String::new(); // Remove the definition
                }
            }

            full_match.to_string()
        }).to_string();

        // Remove autolinks
        result = self.autolink_regex.replace_all(&result, |caps: &Captures| {
            let full_match = caps.get(0).unwrap().as_str();
            let path = caps.get(1).unwrap().as_str();

            if Self::is_file_reference(path) {
                let clean_path = Self::path_without_anchor(path);
                let normalized = Self::normalize_path(clean_path);

                if normalized == module || clean_path.ends_with(module) {
                    return String::new(); // Remove the autolink
                }
            }

            full_match.to_string()
        }).to_string();

        result
    }

    fn update_import_reference(
        &self,
        file_path: &Path,
        content: &str,
        update: &DependencyUpdate,
    ) -> PluginResult<String> {
        debug!(
            file = ?file_path,
            old_ref = %update.old_reference,
            new_ref = %update.new_reference,
            "Updating markdown import reference"
        );

        // Use rewrite_imports_for_rename which handles the link syntax
        let (updated_content, changes) = self.rewrite_imports_for_rename(
            content,
            &update.old_reference,
            &update.new_reference,
        );

        if changes > 0 {
            debug!(changes, "Updated markdown file references");
        }

        Ok(updated_content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_imports() {
        let support = MarkdownImportSupport::new();
        let content = r#"
# Documentation

See [Architecture](docs/architecture/ARCHITECTURE.md) for details.
Also check [API Reference](docs/api/API_REFERENCE.md#overview).
Visit [our website](https://example.com) for more info.
        "#;

        let imports = support.parse_imports(content);
        assert_eq!(imports.len(), 2);
        assert!(imports.contains(&"docs/architecture/ARCHITECTURE.md".to_string()));
        assert!(imports.contains(&"docs/api/API_REFERENCE.md".to_string()));
        // URL should not be included
        assert!(!imports.iter().any(|i| i.contains("example.com")));
    }

    #[test]
    fn test_rewrite_imports_for_rename() {
        let support = MarkdownImportSupport::new();
        let content = r#"
See [Architecture](docs/architecture/ARCHITECTURE.md) for details.
Also [here](docs/architecture/ARCHITECTURE.md#overview).
        "#;

        let (updated, count) = support.rewrite_imports_for_rename(
            content,
            "docs/architecture/ARCHITECTURE.md",
            "docs/architecture/overview.md",
        );

        assert_eq!(count, 2);
        assert!(updated.contains("docs/architecture/overview.md"));
        assert!(updated.contains("docs/architecture/overview.md#overview"));
        assert!(!updated.contains("ARCHITECTURE.md"));
    }

    #[test]
    fn test_rewrite_imports_preserves_images() {
        let support = MarkdownImportSupport::new();
        let content = "![Diagram](docs/img/old.png)";

        let (updated, count) = support.rewrite_imports_for_rename(
            content,
            "docs/img/old.png",
            "docs/img/new.png",
        );

        assert_eq!(count, 1);
        assert!(updated.contains("![Diagram](docs/img/new.png)"));
    }

    #[test]
    fn test_contains_import() {
        let support = MarkdownImportSupport::new();
        let content = "See [Architecture](docs/ARCHITECTURE.md)";

        assert!(support.contains_import(content, "docs/ARCHITECTURE.md"));
        assert!(!support.contains_import(content, "OTHER.md"));
    }

    #[test]
    fn test_remove_import() {
        let support = MarkdownImportSupport::new();
        let content = "See [Architecture](docs/ARCHITECTURE.md) and [API](docs/API.md)";

        let updated = support.remove_import(content, "docs/ARCHITECTURE.md");

        assert!(!updated.contains("ARCHITECTURE.md"));
        assert!(updated.contains("API.md"));
    }
}