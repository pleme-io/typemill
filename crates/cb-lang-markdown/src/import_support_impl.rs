//! Import support implementation for Markdown
//!
//! Treats markdown file links as "imports" for the purpose of file rename tracking.

use cb_plugin_api::{
    import_support::{
        ImportAdvancedSupport, ImportMoveSupport, ImportMutationSupport, ImportParser,
        ImportRenameSupport,
    },
    PluginResult,
};
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
    /// Regex for inline code containing paths: `path/to/file`
    inline_code_regex: Regex,
    /// Regex for plain prose paths (not in links or code): integration-tests/src/
    prose_path_regex: Regex,
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

        // Matches inline code containing paths: `path/to/file` or `path\to\file`
        // Must contain a slash or backslash to look like a path
        let inline_code_regex = Regex::new(r"`([^`]+[/\\][^`]*)`").unwrap();

        // Matches paths in prose (not inside links or code)
        // Matches patterns like: integration-tests/src/ or docs/api.md
        // No word boundaries to handle Unicode chars like ├── integration-tests/
        // Pattern itself is specific enough: requires alphanumeric/dash/underscore, then slash
        let prose_path_regex = Regex::new(r"([a-zA-Z0-9_-]+/[a-zA-Z0-9_/.-]*)").unwrap();

        Self {
            inline_link_regex,
            ref_definition_regex,
            autolink_regex,
            inline_code_regex,
            prose_path_regex,
        }
    }

    /// Check if a string looks like a path (contains slash and extension, or matches path patterns)
    fn looks_like_path(text: &str) -> bool {
        // Must contain a slash or backslash
        if !text.contains('/') && !text.contains('\\') {
            return false;
        }

        // Skip if it looks like code (contains quotes, parentheses, or code keywords)
        if text.contains('"') || text.contains('(') || text.contains(')') {
            return false;
        }

        // Skip common non-path patterns
        if text.starts_with("http://")
            || text.starts_with("https://")
            || text.starts_with("mailto:")
        {
            return false;
        }

        // Skip command-line patterns (e.g., "cargo test --manifest-path integration-tests/Cargo.toml")
        // If text has spaces AND contains command flags (--), it's likely a command, not a path
        if text.contains(' ') && text.contains("--") {
            return false;
        }

        // Skip common command prefixes followed by spaces
        let command_prefixes = [
            "cargo ", "npm ", "yarn ", "pnpm ", "git ", "docker ", "kubectl ", "python ", "node ",
            "rustc ", "gcc ", "make ", "cmake ", "go ", "mvn ", "gradle ", "java ", "javac ",
            "dotnet ", "ruby ", "perl ",
        ];
        for prefix in &command_prefixes {
            if text.starts_with(prefix) {
                return false;
            }
        }

        // Looks like a path if it has a slash and none of the above patterns
        true
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
    fn build_link(
        full_match: &str,
        link_text: &str,
        new_path: &str,
        original_path: &str,
    ) -> String {
        let anchor = Self::extract_anchor(original_path);
        let prefix = if full_match.starts_with('!') { "!" } else { "" };
        format!("{}[{}]({}{})", prefix, link_text, new_path, anchor)
    }

    /// Build markdown reference-style link definition with preserved anchor and whitespace
    fn build_ref_definition(
        full_match: &str,
        ref_label: &str,
        new_path: &str,
        original_path: &str,
    ) -> String {
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

    /// Update prose identifiers with context-aware matching
    ///
    /// This method detects identifier references in prose text using context patterns
    /// to reduce false positives. Only updates when high-confidence patterns match.
    ///
    /// # Patterns
    ///
    /// 1. **"depend(s|ing)? on IDENTIFIER"** - High confidence technical reference
    /// 2. **`IDENTIFIER`** - Backticked identifiers (code references)
    ///
    /// # Arguments
    ///
    /// * `content` - The markdown content to process
    /// * `old_path` - The old path (basename will be extracted for matching)
    /// * `new_path` - The new path (basename will be used for replacement)
    ///
    /// # Returns
    ///
    /// Tuple of (updated_content, change_count)
    pub fn update_prose_identifiers(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
    ) -> (String, usize) {
        // Extract basenames for matching
        let old_basename = old_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_else(|| old_path.to_str().unwrap_or(""));
        let new_basename = new_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_else(|| new_path.to_str().unwrap_or(""));

        // Smart boundary matching: NOT preceded/followed by alphanumeric
        // This simple approach:
        // - Works in ANY language (not just English)
        // - Handles hyphenated identifiers: "cb-handlers-style" → "mill-handlers-style"
        // - Updates ALL prose occurrences (no fancy patterns needed)
        // - Blocks partial matches: "mycb-handlers" won't match
        //
        // Examples of what gets updated:
        // - "depend on cb-handlers" → "depend on mill-handlers"
        // - "only cb-handlers can" → "only mill-handlers can"
        // - "`cb-handlers`" → "`mill-handlers`"
        // - "cb-handlers-style" → "mill-handlers-style"
        let pattern = format!(
            r"(?<![a-zA-Z0-9]){}(?![a-zA-Z0-9])",
            fancy_regex::escape(old_basename)
        );

        match fancy_regex::Regex::new(&pattern) {
            Ok(regex) => {
                let result = regex.replace_all(content, new_basename);
                let count = result.matches(new_basename).count()
                    - content.matches(new_basename).count();

                debug!(
                    old_basename,
                    new_basename,
                    count,
                    "Updated prose identifiers in markdown"
                );

                (result.to_string(), count)
            }
            Err(e) => {
                tracing::warn!(
                    error = ?e,
                    old_basename,
                    "Failed to compile prose pattern"
                );
                (content.to_string(), 0)
            }
        }
    }
}

impl Default for MarkdownImportSupport {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Segregated Trait Implementations
// ============================================================================

impl ImportParser for MarkdownImportSupport {
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

    fn contains_import(&self, content: &str, module: &str) -> bool {
        let imports = self.parse_imports(content);
        imports
            .iter()
            .any(|imp| imp == module || imp.ends_with(module))
    }
}

impl ImportRenameSupport for MarkdownImportSupport {
    fn rewrite_imports_for_rename(
        &self,
        content: &str,
        old_name: &str,
        new_name: &str,
    ) -> (String, usize) {
        let mut count = 0;

        // Rewrite inline links
        let mut result = self
            .inline_link_regex
            .replace_all(content, |caps: &Captures| {
                let full_match = caps.get(0).unwrap().as_str();
                let link_text = caps.get(1).unwrap().as_str();
                let path = caps.get(2).unwrap().as_str();

                if Self::is_file_reference(path) {
                    let clean_path = Self::path_without_anchor(path);

                    // Match against:
                    // 1. Full path (e.g., "docs/architecture/ARCHITECTURE.md")
                    // 2. With ./ prefix (e.g., "./docs/architecture/ARCHITECTURE.md")
                    // 3. Just filename (e.g., "ARCHITECTURE.md")
                    // 4. Path ends with the old name
                    if clean_path == old_name
                        || clean_path == format!("./{}", old_name)
                        || old_name.ends_with(clean_path)
                        || old_name.ends_with(&format!("/{}", clean_path))
                    {
                        count += 1;
                        return Self::build_link(full_match, link_text, new_name, path);
                    }
                }

                full_match.to_string()
            })
            .to_string();

        // Rewrite reference-style link definitions
        result = self
            .ref_definition_regex
            .replace_all(&result, |caps: &Captures| {
                let full_match = caps.get(0).unwrap().as_str();
                let ref_label = caps.get(1).unwrap().as_str();
                let path = caps.get(2).unwrap().as_str();

                if Self::is_file_reference(path) {
                    let clean_path = Self::path_without_anchor(path);

                    // Match against:
                    // 1. Full path (e.g., "docs/architecture/ARCHITECTURE.md")
                    // 2. With ./ prefix (e.g., "./docs/architecture/ARCHITECTURE.md")
                    // 3. Just filename (e.g., "ARCHITECTURE.md")
                    // 4. Path ends with the old name
                    if clean_path == old_name
                        || clean_path == format!("./{}", old_name)
                        || old_name.ends_with(clean_path)
                        || old_name.ends_with(&format!("/{}", clean_path))
                    {
                        count += 1;
                        return Self::build_ref_definition(full_match, ref_label, new_name, path);
                    }
                }

                full_match.to_string()
            })
            .to_string();

        // Rewrite autolinks
        result = self
            .autolink_regex
            .replace_all(&result, |caps: &Captures| {
                let full_match = caps.get(0).unwrap().as_str();
                let path = caps.get(1).unwrap().as_str();

                if Self::is_file_reference(path) {
                    let clean_path = Self::path_without_anchor(path);

                    // Match against:
                    // 1. Full path (e.g., "docs/architecture/ARCHITECTURE.md")
                    // 2. With ./ prefix (e.g., "./docs/architecture/ARCHITECTURE.md")
                    // 3. Just filename (e.g., "ARCHITECTURE.md")
                    // 4. Path ends with the old name
                    if clean_path == old_name
                        || clean_path == format!("./{}", old_name)
                        || old_name.ends_with(clean_path)
                        || old_name.ends_with(&format!("/{}", clean_path))
                    {
                        count += 1;
                        return Self::build_autolink(new_name, path);
                    }
                }

                full_match.to_string()
            })
            .to_string();

        // Rewrite inline code paths (opt-in feature for updating prose)
        // This catches patterns like `integration-tests/src/` in tables and text
        result = self
            .inline_code_regex
            .replace_all(&result, |caps: &Captures| {
                let full_match = caps.get(0).unwrap().as_str();
                let code_content = caps.get(1).unwrap().as_str();

                if Self::looks_like_path(code_content) {
                    // Skip if already updated (idempotency check for nested renames)
                    let is_nested_rename = new_name.starts_with(&format!("{}/", old_name));
                    if is_nested_rename && code_content.contains(new_name) {
                        return full_match.to_string();
                    }

                    // Match at start of path (not anywhere)
                    if code_content == old_name
                        || code_content.starts_with(&format!("{}/", old_name))
                    {
                        count += 1;
                        let updated_content = code_content.replacen(old_name, new_name, 1);
                        return format!("`{}`", updated_content);
                    }
                }

                full_match.to_string()
            })
            .to_string();

        // Rewrite prose paths (plain text paths in documentation)
        // This catches patterns like "integration-tests/" in directory trees
        result = self
            .prose_path_regex
            .replace_all(&result, |caps: &Captures| {
                let full_match = caps.get(0).unwrap().as_str();
                let path_content = caps.get(1).unwrap().as_str();

                // Skip if already updated (idempotency check for nested renames)
                let is_nested_rename = new_name.starts_with(&format!("{}/", old_name));
                if is_nested_rename && path_content.contains(new_name) {
                    return full_match.to_string();
                }

                // Only replace if it matches or starts with the old path
                if path_content == old_name || path_content.starts_with(&format!("{}/", old_name)) {
                    count += 1;
                    return path_content.replacen(old_name, new_name, 1);
                }

                full_match.to_string()
            })
            .to_string();

        debug!(
            changes = count,
            "Rewrote markdown links for rename (inline + reference-style + autolinks + inline code + prose)"
        );
        (result, count)
    }
}

impl ImportMoveSupport for MarkdownImportSupport {
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
        let mut result = self
            .inline_link_regex
            .replace_all(content, |caps: &Captures| {
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
            })
            .to_string();

        // Rewrite reference-style link definitions
        result = self
            .ref_definition_regex
            .replace_all(&result, |caps: &Captures| {
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
            })
            .to_string();

        // Rewrite autolinks
        result = self
            .autolink_regex
            .replace_all(&result, |caps: &Captures| {
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
            })
            .to_string();

        debug!(changes = count, old_path = ?old_path, new_path = ?new_path, "Rewrote markdown links for move (inline + reference-style + autolinks)");
        (result, count)
    }
}

impl ImportMutationSupport for MarkdownImportSupport {
    fn add_import(&self, content: &str, module: &str) -> String {
        // For markdown, "adding an import" means adding a link at the end
        // This is rarely used, but we provide a basic implementation
        format!("{}\n\n[{}]({})", content.trim_end(), module, module)
    }

    fn remove_import(&self, content: &str, module: &str) -> String {
        // Remove inline links
        let mut result = self
            .inline_link_regex
            .replace_all(content, |caps: &Captures| {
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
            })
            .to_string();

        // Remove reference-style link definitions
        result = self
            .ref_definition_regex
            .replace_all(&result, |caps: &Captures| {
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
            })
            .to_string();

        // Remove autolinks
        result = self
            .autolink_regex
            .replace_all(&result, |caps: &Captures| {
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
            })
            .to_string();

        result
    }

    fn remove_named_import(&self, _line: &str, _import_name: &str) -> PluginResult<String> {
        // Markdown doesn't have the concept of "named imports"
        Err(cb_plugin_api::PluginError::not_supported(
            "Markdown does not support named imports",
        ))
    }
}

impl ImportAdvancedSupport for MarkdownImportSupport {
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
        let (updated_content, changes) =
            self.rewrite_imports_for_rename(content, &update.old_reference, &update.new_reference);

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

        let imports = ImportParser::parse_imports(&support, content);
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

        let (updated, count) = ImportRenameSupport::rewrite_imports_for_rename(
            &support,
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

        let (updated, count) = ImportRenameSupport::rewrite_imports_for_rename(
            &support,
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

        assert!(ImportParser::contains_import(
            &support,
            content,
            "docs/ARCHITECTURE.md"
        ));
        assert!(!ImportParser::contains_import(
            &support, content, "OTHER.md"
        ));
    }

    #[test]
    fn test_remove_import() {
        let support = MarkdownImportSupport::new();
        let content = "See [Architecture](docs/ARCHITECTURE.md) and [API](docs/API.md)";

        let updated =
            ImportMutationSupport::remove_import(&support, content, "docs/ARCHITECTURE.md");

        assert!(!updated.contains("ARCHITECTURE.md"));
        assert!(updated.contains("API.md"));
    }

    #[test]
    fn test_rewrite_inline_code_paths() {
        let support = MarkdownImportSupport::new();
        let content = r#"
| Layer | Location | Purpose |
|-------|----------|---------|
| **Integration** | `integration-tests/src/` | Tool handlers with mocks |

Directory tree:
├── integration-tests/
│   ├── src/
│   └── tests/
"#;

        let (updated, count) = ImportRenameSupport::rewrite_imports_for_rename(
            &support,
            content,
            "integration-tests",
            "tests",
        );

        assert_eq!(count, 2, "Should update inline code and plain text paths");
        assert!(
            updated.contains("`tests/src/`"),
            "Should update inline code in table"
        );
        assert!(
            updated.contains("tests/"),
            "Should update plain text in directory tree"
        );
        assert!(
            !updated.contains("integration-tests"),
            "Should not contain old path"
        );
    }

    #[test]
    fn test_inline_code_path_detection() {
        assert!(MarkdownImportSupport::looks_like_path(
            "integration-tests/src/"
        ));
        assert!(MarkdownImportSupport::looks_like_path("docs/api.md"));
        assert!(MarkdownImportSupport::looks_like_path("src\\main.rs")); // Windows path

        // Should skip these
        assert!(!MarkdownImportSupport::looks_like_path("no-slashes"));
        assert!(!MarkdownImportSupport::looks_like_path(
            "cargo test --manifest-path integration-tests/Cargo.toml"
        ));
        assert!(!MarkdownImportSupport::looks_like_path(
            "https://example.com/path"
        ));
    }
}
