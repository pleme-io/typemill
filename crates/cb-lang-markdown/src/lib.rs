//! Markdown Language Plugin
//!
//! Provides support for detecting and updating file references in Markdown documents.
//! This enables `rename.plan` to track markdown link references when files are moved.

use async_trait::async_trait;
use cb_plugin_api::{
    import_support::{
        ImportAdvancedSupport, ImportMoveSupport, ImportMutationSupport, ImportParser,
        ImportRenameSupport,
    },
    LanguageMetadata, LanguagePlugin, ManifestData, ParsedSource, PluginCapabilities,
    PluginError, PluginResult, SourceLocation, Symbol, SymbolKind,
};
use cb_plugin_api::codebuddy_plugin;
use regex::Regex;
use std::path::Path;
use tracing::debug;

mod import_support_impl;

use import_support_impl::MarkdownImportSupport;

// Self-register the plugin with the Codebuddy system.
codebuddy_plugin! {
    name: "markdown",
    extensions: ["md", "markdown"],
    manifest: "package.json",
    capabilities: MarkdownPlugin::CAPABILITIES,
    factory: MarkdownPlugin::arc,
    lsp: None
}

/// Markdown language plugin
///
/// Detects and updates file references in markdown links:
/// - `[text](path.md)` - Standard markdown links
/// - `[text](path.md#anchor)` - Links with anchors
/// - `![alt](image.png)` - Image references
///
/// Does NOT process:
/// - Code blocks (triple backticks)
/// - Inline code (single backticks)
/// - HTML `<a href="">` tags (use markdown syntax instead)
pub struct MarkdownPlugin {
    metadata: LanguageMetadata,
    import_support: MarkdownImportSupport,
}

impl MarkdownPlugin {
    /// The capabilities of this plugin.
    pub const CAPABILITIES: PluginCapabilities = PluginCapabilities::none()
        .with_imports(); // We support "imports" (file references)

    pub fn new() -> Self {
        Self {
            metadata: LanguageMetadata {
                name: "Markdown",
                extensions: &["md", "markdown"],
                manifest_filename: "package.json", // No specific manifest for markdown
                source_dir: "docs",
                entry_point: "README.md",
                module_separator: "/",
            },
            import_support: MarkdownImportSupport::new(),
        }
    }

    /// Create a boxed instance for plugin registry
    pub fn arc() -> Box<dyn LanguagePlugin> {
        Box::new(Self::new())
    }
}

impl Default for MarkdownPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguagePlugin for MarkdownPlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &self.metadata
    }

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        // Parse markdown to extract headers as symbols
        let symbols = extract_headers(source);

        Ok(ParsedSource {
            data: serde_json::json!({
                "language": "markdown",
                "headers": symbols.len(),
            }),
            symbols,
        })
    }

    async fn analyze_manifest(&self, _path: &Path) -> PluginResult<ManifestData> {
        // Markdown files don't have a manifest
        Err(PluginError::not_supported(
            "Markdown does not have a manifest file",
        ))
    }

    fn capabilities(&self) -> PluginCapabilities {
        Self::CAPABILITIES
    }

    fn import_parser(&self) -> Option<&dyn ImportParser> {
        Some(&self.import_support)
    }

    fn import_rename_support(&self) -> Option<&dyn ImportRenameSupport> {
        Some(&self.import_support)
    }

    fn import_move_support(&self) -> Option<&dyn ImportMoveSupport> {
        Some(&self.import_support)
    }

    fn import_mutation_support(&self) -> Option<&dyn ImportMutationSupport> {
        Some(&self.import_support)
    }

    fn import_advanced_support(&self) -> Option<&dyn ImportAdvancedSupport> {
        Some(&self.import_support)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn rewrite_file_references(
        &self,
        content: &str,
        old_path: &std::path::Path,
        new_path: &std::path::Path,
        current_file: &std::path::Path,
        project_root: &std::path::Path,
        _rename_info: Option<&serde_json::Value>,
    ) -> Option<(String, usize)> {
        tracing::info!(
            "MarkdownPlugin::rewrite_file_references CALLED - old_path={}, new_path={}, current_file={}",
            old_path.display(),
            new_path.display(),
            current_file.display()
        );

        // For markdown links, we need to compute file-relative paths
        // because markdown links should be relative to the file they're in
        // (this makes them work correctly when viewing files locally)

        let current_dir = current_file.parent()?;

        // Compute file-relative paths from current file's directory
        let old_relative = pathdiff::diff_paths(old_path, current_dir)?;
        let new_relative = pathdiff::diff_paths(new_path, current_dir)?;

        // Convert to string with forward slashes for markdown
        let old_relative_str = old_relative.to_string_lossy().replace('\\', "/");
        let new_relative_str = new_relative.to_string_lossy().replace('\\', "/");

        // ALSO compute project-relative paths for matching
        let old_project_relative = old_path
            .strip_prefix(project_root)
            .unwrap_or(old_path)
            .to_string_lossy()
            .replace('\\', "/");

        debug!(
            old_path = ?old_path,
            new_path = ?new_path,
            old_relative = %old_relative_str,
            new_relative = %new_relative_str,
            old_project_relative = %old_project_relative,
            "Rewriting markdown links with file-relative paths"
        );

        // First pass: rewrite project-relative paths to file-relative paths
        let (mut result, mut count) = ImportRenameSupport::rewrite_imports_for_rename(
            &self.import_support,
            content,
            &old_project_relative,
            &new_relative_str,
        );

        // Second pass: rewrite any existing file-relative paths
        let (result2, count2) = ImportRenameSupport::rewrite_imports_for_rename(
            &self.import_support,
            &result,
            &old_relative_str,
            &new_relative_str,
        );

        if count2 > 0 {
            result = result2;
            count += count2;
        }

        debug!(total_changes = count, "Completed markdown link rewriting");

        Some((result, count))
    }
}

/// Extract markdown headers as symbols
fn extract_headers(content: &str) -> Vec<Symbol> {
    let header_regex = Regex::new(r"^(#{1,6})\s+(.+)$").unwrap();
    let mut symbols = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        if let Some(captures) = header_regex.captures(line) {
            let level = captures.get(1).unwrap().as_str().len();
            let title = captures.get(2).unwrap().as_str().trim().to_string();

            // Map header level to symbol kind
            let kind = match level {
                1 => SymbolKind::Module,   // # Top level
                2 => SymbolKind::Class,    // ## Section
                3 => SymbolKind::Function, // ### Subsection
                _ => SymbolKind::Other,    // #### and below
            };

            symbols.push(Symbol {
                name: title,
                kind,
                location: SourceLocation {
                    line: line_num + 1,
                    column: 0,
                },
                documentation: None,
            });
        }
    }

    debug!(headers = symbols.len(), "Extracted markdown headers");
    symbols
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_markdown_headers() {
        let plugin = MarkdownPlugin::new();
        let source = r#"# Main Title
Some text here.

## Section 1
More text.

### Subsection 1.1
Details.

## Section 2
"#;

        let parsed = plugin.parse(source).await.unwrap();
        assert_eq!(parsed.symbols.len(), 4);
        assert_eq!(parsed.symbols[0].name, "Main Title");
        assert_eq!(parsed.symbols[1].name, "Section 1");
        assert_eq!(parsed.symbols[2].name, "Subsection 1.1");
        assert_eq!(parsed.symbols[3].name, "Section 2");
    }

    #[test]
    fn test_metadata() {
        let plugin = MarkdownPlugin::new();
        let metadata = plugin.metadata();

        assert_eq!(metadata.name, "Markdown");
        assert!(metadata.extensions.contains(&"md"));
        assert!(metadata.extensions.contains(&"markdown"));
    }

    #[test]
    fn test_capabilities() {
        let plugin = MarkdownPlugin::new();
        let caps = plugin.capabilities();

        assert!(caps.imports); // Markdown supports file references
        assert!(!caps.workspace); // No workspace operations
    }

    #[test]
    fn test_rewrite_file_references_override_same_directory() {
        use std::path::PathBuf;

        let plugin = MarkdownPlugin::new();

        // Test case: Both files in same directory
        // - project_root/docs/api.md → docs/api-reference.md
        // - project_root/docs/examples.md contains link

        let project_root = PathBuf::from("/tmp/test_project");
        let old_path = project_root.join("docs/api.md");
        let new_path = project_root.join("docs/api-reference.md");
        let current_file = project_root.join("docs/examples.md");

        let content = r#"Check [API](docs/api.md) for details.
"#;

        let result = plugin.rewrite_file_references(
            content,
            &old_path,
            &new_path,
            &current_file,
            &project_root,
            None,
        );

        assert!(result.is_some(), "rewrite_file_references should return Some");

        let (updated_content, count) = result.unwrap();

        // Should convert project-relative to file-relative
        // "docs/api.md" → "api-reference.md" (file-relative)
        assert_eq!(count, 1, "Should update 1 reference");
        assert!(
            updated_content.contains("(api-reference.md)"),
            "Link should be converted to file-relative path. Actual content:\n{}",
            updated_content
        );
    }

    #[test]
    fn test_rewrite_file_references_override_cross_directory() {
        use std::path::PathBuf;

        let plugin = MarkdownPlugin::new();

        // Test case: File moving to different directory
        // - project_root/docs/development/contributing.md → CONTRIBUTING.md
        // - project_root/docs/index.md contains link

        let project_root = PathBuf::from("/tmp/test_project");
        let old_path = project_root.join("docs/development/contributing.md");
        let new_path = project_root.join("CONTRIBUTING.md");
        let current_file = project_root.join("docs/index.md");

        let content = r#"Check [contributing guide](docs/development/contributing.md).
"#;

        let result = plugin.rewrite_file_references(
            content,
            &old_path,
            &new_path,
            &current_file,
            &project_root,
            None,
        );

        assert!(result.is_some(), "rewrite_file_references should return Some");

        let (updated_content, count) = result.unwrap();

        // Should convert project-relative to file-relative
        // "docs/development/contributing.md" → "../CONTRIBUTING.md"
        assert_eq!(count, 1, "Should update 1 reference");
        assert!(
            updated_content.contains("(../CONTRIBUTING.md)"),
            "Link should be converted to file-relative path. Actual content:\n{}",
            updated_content
        );
    }
}
