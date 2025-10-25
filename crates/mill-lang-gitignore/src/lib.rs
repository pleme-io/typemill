//! .gitignore Language Plugin
//!
//! Provides support for detecting and updating file path patterns in .gitignore files.
//! This enables `rename` to update ignore patterns when files/directories are moved.

use async_trait::async_trait;
use mill_plugin_api::mill_plugin;
use mill_plugin_api::{
    import_support::ImportMoveSupport, LanguageMetadata, LanguagePlugin, ManifestData,
    ParsedSource, PluginCapabilities, PluginResult,
};
use serde_json::json;
use std::path::Path;
use tracing::debug;

mod import_support_impl;

use import_support_impl::GitignoreImportSupport;

// Self-register the plugin with the TypeMill system.
mill_plugin! {
    name: "gitignore",
    extensions: [],  // .gitignore has no extension
    manifest: ".gitignore",
    capabilities: GitignoreLanguagePlugin::CAPABILITIES,
    factory: GitignoreLanguagePlugin::boxed,
    lsp: None
}

/// .gitignore language plugin
///
/// Detects and updates file path patterns in .gitignore files:
/// - Directory patterns (e.g., `tests/e2e/`)
/// - File patterns (e.g., `tests/e2e/*.tmp`)
/// - Preserves comments (lines starting with #)
/// - Preserves generic glob patterns (e.g., `*.log`, `target/`)
///
/// Does NOT process:
/// - Comment lines (starting with #)
/// - Blank lines
/// - Generic patterns without specific paths
pub struct GitignoreLanguagePlugin {
    metadata: LanguageMetadata,
    import_support: GitignoreImportSupport,
}

impl GitignoreLanguagePlugin {
    /// The capabilities of this plugin.
    pub const CAPABILITIES: PluginCapabilities = PluginCapabilities::none().with_imports();

    pub fn new() -> Self {
        Self {
            metadata: LanguageMetadata {
                name: "gitignore",
                extensions: &[],  // Special case - matched by filename
                manifest_filename: ".gitignore",
                source_dir: ".",
                entry_point: ".gitignore",
                module_separator: "/",
            },
            import_support: GitignoreImportSupport::new(),
        }
    }

    pub fn boxed() -> Box<dyn LanguagePlugin> {
        Box::new(Self::new())
    }
}

impl Default for GitignoreLanguagePlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguagePlugin for GitignoreLanguagePlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &self.metadata
    }

    async fn parse(&self, _source: &str) -> PluginResult<ParsedSource> {
        // .gitignore doesn't need full AST parsing, but we need to implement this
        Ok(ParsedSource {
            data: json!({}),
            symbols: vec![],
        })
    }

    async fn analyze_manifest(&self, _path: &Path) -> PluginResult<ManifestData> {
        // .gitignore is not a package manifest - return minimal data
        Ok(ManifestData {
            name: ".gitignore".to_string(),
            version: "0.0.0".to_string(),
            dependencies: vec![],
            dev_dependencies: vec![],
            raw_data: json!({}),
        })
    }

    fn capabilities(&self) -> PluginCapabilities {
        Self::CAPABILITIES
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ImportMoveSupport for GitignoreLanguagePlugin {
    fn rewrite_imports_for_move(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
    ) -> (String, usize) {
        debug!(
            old_path = %old_path.display(),
            new_path = %new_path.display(),
            "Rewriting paths in .gitignore file"
        );

        self.import_support
            .rewrite_gitignore_patterns(content, old_path, new_path)
            .unwrap_or_else(|_| (content.to_string(), 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handles_gitignore() {
        let plugin = GitignoreLanguagePlugin::new();
        assert!(plugin.handles_file(Path::new(".gitignore")));
        assert!(plugin.handles_file(Path::new("/project/.gitignore")));
        assert!(!plugin.handles_file(Path::new("README.md")));
        assert!(!plugin.handles_file(Path::new(".gitignore.backup")));
    }

    #[tokio::test]
    async fn test_rewrite_directory_pattern() {
        let plugin = GitignoreLanguagePlugin::new();
        let content = "# Build output\ntarget/\ntests/e2e/fixtures/\n*.log\n";
        let old_path = Path::new("tests/e2e");
        let new_path = Path::new("tests/integration");

        let (result, changes) = plugin
            .rewrite_import_paths(content, Path::new(".gitignore"), old_path, new_path, None)
            .await
            .unwrap();

        assert_eq!(changes, 1);
        assert!(result.contains("tests/integration/fixtures/"));
        assert!(!result.contains("tests/e2e/fixtures/"));
        assert!(result.contains("target/")); // Unchanged
        assert!(result.contains("*.log")); // Unchanged
    }

    #[tokio::test]
    async fn test_preserves_comments_and_blanks() {
        let plugin = GitignoreLanguagePlugin::new();
        let content = "# Comment\n\ntests/e2e/\n\n# Another comment\n";
        let old_path = Path::new("tests/e2e");
        let new_path = Path::new("tests/integration");

        let (result, changes) = plugin
            .rewrite_import_paths(content, Path::new(".gitignore"), old_path, new_path, None)
            .await
            .unwrap();

        assert_eq!(changes, 1);
        assert!(result.contains("# Comment"));
        assert!(result.contains("# Another comment"));
        assert!(result.contains("tests/integration/"));
    }
}
