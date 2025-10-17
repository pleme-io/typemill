//! Markdown Language Plugin
//!
//! Provides support for detecting and updating file references in Markdown documents.
//! This enables `rename.plan` to track markdown link references when files are moved.

use async_trait::async_trait;
use cb_plugin_api::{
    ImportSupport, LanguageMetadata, LanguagePlugin, ManifestData, ParsedSource,
    PluginCapabilities, PluginError, PluginResult, Symbol, SymbolKind, SourceLocation,
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
    pub const CAPABILITIES: PluginCapabilities = PluginCapabilities {
        imports: true,  // We support "imports" (file references)
        workspace: false,
    };

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

    fn import_support(&self) -> Option<&dyn ImportSupport> {
        Some(&self.import_support)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
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
                1 => SymbolKind::Module,    // # Top level
                2 => SymbolKind::Class,     // ## Section
                3 => SymbolKind::Function,  // ### Subsection
                _ => SymbolKind::Other,     // #### and below
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
}
