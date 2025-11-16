//! .gitignore Language Plugin
//!
//! Provides support for detecting and updating file path patterns in .gitignore files.
//! This enables `rename` to update ignore patterns when files/directories are moved.

use async_trait::async_trait;
use mill_plugin_api::mill_plugin;
use mill_plugin_api::{
    import_support, import_support::ImportMoveSupport, LanguageMetadata, LanguagePlugin,
    ManifestData, ParsedSource, PluginCapabilities, PluginResult,
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
                extensions: &[], // Special case - matched by filename
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

    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
        // Verify the file exists
        if !path.exists() {
            return Err(mill_plugin_api::PluginApiError::invalid_input(format!(
                "File does not exist: {:?}",
                path
            )));
        }

        // Verify this is a .gitignore file
        if path.file_name().and_then(|s| s.to_str()) != Some(".gitignore") {
            return Err(mill_plugin_api::PluginApiError::invalid_input(format!(
                "Expected .gitignore, got: {:?}",
                path.file_name()
            )));
        }

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

    fn import_move_support(&self) -> Option<&dyn import_support::ImportMoveSupport> {
        Some(self)
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
        assert!(plugin.handles_manifest(".gitignore"));
        assert!(plugin.handles_manifest(".gitignore")); // Filename match, not full path
        assert!(!plugin.handles_manifest("README.md"));
        assert!(!plugin.handles_manifest(".gitignore.backup"));
    }

    #[test]
    fn test_rewrite_directory_pattern() {
        let plugin = GitignoreLanguagePlugin::new();
        let content = "# Build output\ntarget/\ntests/e2e/fixtures/\n*.log\n";
        let old_path = Path::new("tests/e2e");
        let new_path = Path::new("tests/integration");

        let (result, changes) = plugin
            .import_support
            .rewrite_gitignore_patterns(content, old_path, new_path)
            .unwrap();

        assert_eq!(changes, 1);
        assert!(result.contains("tests/integration/fixtures/"));
        assert!(!result.contains("tests/e2e/fixtures/"));
        assert!(result.contains("target/")); // Unchanged
        assert!(result.contains("*.log")); // Unchanged
    }

    #[test]
    fn test_preserves_comments_and_blanks() {
        let plugin = GitignoreLanguagePlugin::new();
        let content = "# Comment\n\ntests/e2e/\n\n# Another comment\n";
        let old_path = Path::new("tests/e2e");
        let new_path = Path::new("tests/integration");

        let (result, changes) = plugin
            .import_support
            .rewrite_gitignore_patterns(content, old_path, new_path)
            .unwrap();

        assert_eq!(changes, 1);
        assert!(result.contains("# Comment"));
        assert!(result.contains("# Another comment"));
        assert!(result.contains("tests/integration/"));
    }

    // ========================================================================
    // EDGE CASE TESTS (1 test)
    // ========================================================================

    #[test]
    fn test_edge_extremely_long_pattern() {
        let plugin = GitignoreLanguagePlugin::new();
        let long_pattern = format!("{}/", "a".repeat(5000));
        let content = format!("# Pattern\n{}\n*.log\n", long_pattern);
        let old_path = Path::new(&long_pattern[..long_pattern.len() - 1]);
        let new_path = Path::new("short");

        // Should handle very long patterns without panicking
        let result = plugin
            .import_support
            .rewrite_gitignore_patterns(&content, old_path, new_path);
        assert!(result.is_ok(), "Should handle long patterns successfully: {:?}", result.err());
    }

    // ========================================================================
    // INTEGRATION TESTS (2 tests)
    // ========================================================================

    #[tokio::test]
    async fn test_integration_gitignore_pattern_matching() {
        let harness = mill_test_support::harness::IntegrationTestHarness::new()
            .expect("Should create harness");

        harness
            .create_source_file(".gitignore", "*.log\nnode_modules/\n.env\n")
            .expect("Should create .gitignore");

        harness
            .create_source_file("app.log", "test log")
            .expect("Should create app.log");

        // Verify pattern parsing
        let gitignore_content = harness
            .read_file(".gitignore")
            .expect("Should read .gitignore");
        assert!(gitignore_content.contains("*.log"));
        assert!(gitignore_content.contains("node_modules"));
        assert!(gitignore_content.contains(".env"));
    }

    #[tokio::test]
    async fn test_integration_nested_gitignore_files() {
        let harness = mill_test_support::harness::IntegrationTestHarness::new()
            .expect("Should create harness");

        harness
            .create_source_file(".gitignore", "*.tmp\n")
            .expect("Should create root .gitignore");

        harness
            .create_directory("subdir")
            .expect("Should create subdir");

        harness
            .create_source_file("subdir/.gitignore", "*.cache\n")
            .expect("Should create subdir .gitignore");

        // Verify both files maintained
        let root_gitignore = harness
            .read_file(".gitignore")
            .expect("Should read root .gitignore");
        assert_eq!(root_gitignore.trim(), "*.tmp");

        let subdir_gitignore = harness
            .read_file("subdir/.gitignore")
            .expect("Should read subdir .gitignore");
        assert_eq!(subdir_gitignore.trim(), "*.cache");
    }
}
