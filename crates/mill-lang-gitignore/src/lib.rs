//! .gitignore Language Plugin
//!
//! Provides support for detecting and updating file path patterns in .gitignore files.
//! This enables `rename.plan` to update ignore patterns when files/directories are moved.

use async_trait::async_trait;
use mill_plugin_api::mill_plugin;
use mill_plugin_api::{
    import_support::ImportRenameSupport, LanguageMetadata, LanguagePlugin, ManifestData,
    ParsedSource, PluginCapabilities, PluginError, PluginResult,
};
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
    factory: GitignoreLanguagePlugin::arc,
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
            },
            import_support: GitignoreImportSupport::new(),
        }
    }

    pub fn arc() -> std::sync::Arc<dyn LanguagePlugin> {
        std::sync::Arc::new(Self::new())
    }
}

impl Default for GitignoreLanguagePlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguagePlugin for GitignoreLanguagePlugin {
    fn get_metadata(&self) -> &LanguageMetadata {
        &self.metadata
    }

    fn parse(&self, _content: &str, _file_path: &Path) -> PluginResult<ParsedSource> {
        // .gitignore doesn't need full AST parsing, but we need to implement this
        Ok(ParsedSource {
            imports: vec![],
            symbols: vec![],
            errors: vec![],
        })
    }

    fn capabilities(&self) -> PluginCapabilities {
        Self::CAPABILITIES
    }

    fn handles_extension(&self, extension: &str) -> bool {
        // .gitignore has no extension, but we'll match on filename instead
        extension.is_empty()
    }

    fn handles_file(&self, file_path: &Path) -> bool {
        // Match based on filename
        file_path
            .file_name()
            .and_then(|f| f.to_str())
            .map(|name| name == ".gitignore")
            .unwrap_or(false)
    }

    async fn get_manifest(&self, _workspace_root: &Path) -> PluginResult<Option<ManifestData>> {
        // .gitignore is not a package manifest
        Ok(None)
    }
}

#[async_trait]
impl ImportRenameSupport for GitignoreLanguagePlugin {
    async fn rewrite_import_paths(
        &self,
        content: &str,
        _file_path: &Path,
        old_path: &Path,
        new_path: &Path,
        _rename_info: Option<&serde_json::Value>,
    ) -> PluginResult<(String, usize)> {
        debug!(
            old_path = %old_path.display(),
            new_path = %new_path.display(),
            "Rewriting paths in .gitignore file"
        );

        self.import_support
            .rewrite_gitignore_patterns(content, old_path, new_path)
    }

    async fn detect_imports(
        &self,
        _content: &str,
        _file_path: &Path,
        _search_path: &Path,
    ) -> PluginResult<Vec<String>> {
        // For .gitignore, we don't detect imports in the traditional sense
        Ok(vec![])
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
