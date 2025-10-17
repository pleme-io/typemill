//! YAML Language Plugin
//!
//! Provides support for detecting and updating file references in YAML config files.
//! This enables `rename.plan` to track path references when files are moved.

use async_trait::async_trait;
use cb_plugin_api::{
    import_support::{ImportRenameSupport},
    LanguageMetadata, LanguagePlugin, ManifestData, ParsedSource, PluginCapabilities,
    PluginError, PluginResult,
};
use cb_plugin_registry::codebuddy_plugin;
use std::path::Path;
use tracing::debug;

mod import_support_impl;

use import_support_impl::YamlImportSupport;

// Self-register the plugin with the Codebuddy system.
codebuddy_plugin! {
    name: "yaml",
    extensions: ["yaml", "yml"],
    manifest: "package.json",
    capabilities: YamlLanguagePlugin::CAPABILITIES,
    factory: YamlLanguagePlugin::arc,
    lsp: None
}

/// YAML language plugin
///
/// Detects and updates file references in YAML configuration files:
/// - Path values in any YAML field
/// - Paths in sequences
/// - Paths in nested mappings
///
/// Does NOT process:
/// - Non-path string values (URLs, names, etc.)
pub struct YamlLanguagePlugin {
    metadata: LanguageMetadata,
    import_support: YamlImportSupport,
}

impl YamlLanguagePlugin {
    /// The capabilities of this plugin.
    pub const CAPABILITIES: PluginCapabilities = PluginCapabilities {
        imports: true, // We support file references
        workspace: false,
    };

    pub fn new() -> Self {
        Self {
            metadata: LanguageMetadata {
                name: "yaml",
                extensions: &["yaml", "yml"],
                manifest_filename: "package.json",
                source_dir: ".",
                entry_point: "main.yml",
                module_separator: "/",
            },
            import_support: YamlImportSupport::new(),
        }
    }

    /// Create a boxed instance for plugin registry
    pub fn arc() -> Box<dyn LanguagePlugin> {
        Box::new(Self::new())
    }
}

impl Default for YamlLanguagePlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguagePlugin for YamlLanguagePlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &self.metadata
    }

    async fn parse(&self, _source: &str) -> PluginResult<ParsedSource> {
        // YAML files don't have symbols we care about extracting
        Ok(ParsedSource {
            data: serde_json::json!({
                "language": "yaml",
            }),
            symbols: vec![],
        })
    }

    async fn analyze_manifest(&self, _path: &Path) -> PluginResult<ManifestData> {
        // YAML files don't have manifest data
        Err(PluginError::not_supported(
            "YAML plugin does not analyze manifest data",
        ))
    }

    fn capabilities(&self) -> PluginCapabilities {
        Self::CAPABILITIES
    }

    fn import_rename_support(&self) -> Option<&dyn ImportRenameSupport> {
        Some(&self.import_support)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn rewrite_file_references(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
        _current_file: &Path,
        _project_root: &Path,
        _rename_info: Option<&serde_json::Value>,
    ) -> Option<(String, usize)> {
        match self.import_support.rewrite_yaml_paths(content, old_path, new_path) {
            Ok((new_content, count)) => {
                if count > 0 {
                    debug!(
                        changes = count,
                        old_path = %old_path.display(),
                        new_path = %new_path.display(),
                        "Updated paths in YAML file"
                    );
                }
                Some((new_content, count))
            }
            Err(e) => {
                tracing::error!(
                    error = ?e,
                    "Failed to rewrite YAML paths"
                );
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_yaml_plugin_basic() {
        let plugin = YamlLanguagePlugin::new();
        let plugin_trait: &dyn LanguagePlugin = &plugin;

        assert_eq!(plugin_trait.metadata().name, "yaml");
        assert_eq!(plugin_trait.metadata().extensions, &["yaml", "yml"]);
        assert!(plugin_trait.handles_extension("yaml"));
        assert!(plugin_trait.handles_extension("yml"));
        assert!(!plugin_trait.handles_extension("rs"));
    }

    #[test]
    fn test_updates_yaml_paths() {
        let content = r#"
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test --manifest-path integration-tests/Cargo.toml
"#;

        let plugin = YamlLanguagePlugin::new();
        let result = plugin.rewrite_file_references(
            content,
            Path::new("integration-tests"),
            Path::new("tests"),
            Path::new("."),
            Path::new("."),
            None,
        );

        assert!(result.is_some());
        let (new_content, count) = result.unwrap();
        assert_eq!(count, 1);
        assert!(new_content.contains("tests/Cargo.toml"));
    }

    #[test]
    fn test_preserves_non_path_strings() {
        let content = r#"
name: CI
on: push
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test --manifest-path integration-tests/Cargo.toml
"#;

        let plugin = YamlLanguagePlugin::new();
        let result = plugin.rewrite_file_references(
            content,
            Path::new("integration-tests"),
            Path::new("tests"),
            Path::new("."),
            Path::new("."),
            None,
        );

        assert!(result.is_some());
        let (new_content, count) = result.unwrap();
        assert_eq!(count, 1);
        assert!(new_content.contains("name: CI"));
        assert!(new_content.contains("on: push"));
        assert!(new_content.contains("tests/Cargo.toml"));
    }

    #[test]
    fn test_yaml_formatting_preservation() {
        // CRITICAL TEST: Verify that all formatting is preserved
        let content = r#"# Important config file
jobs:
  test:  # Run all tests
    runs-on: ubuntu-latest

    steps:
      - run: cargo test --manifest-path integration-tests/Cargo.toml
"#;

        let plugin = YamlLanguagePlugin::new();
        let result = plugin.rewrite_file_references(
            content,
            Path::new("integration-tests"),
            Path::new("tests"),
            Path::new("."),
            Path::new("."),
            None,
        );

        assert!(result.is_some());
        let (new_content, count) = result.unwrap();

        // Verify the path was updated
        assert_eq!(count, 1);
        assert!(new_content.contains("tests/Cargo.toml"));

        // CRITICAL: Verify formatting preserved
        assert!(
            new_content.contains("# Important config file"),
            "Top comment lost! Got:\n{}",
            new_content
        );
        assert!(
            new_content.contains("  test:  # Run all tests"),
            "Inline comment lost! Got:\n{}",
            new_content
        );

        // Check blank line preservation (empty line after ubuntu-latest)
        let lines: Vec<&str> = new_content.lines().collect();
        let content_lines: Vec<&str> = content.lines().collect();
        assert_eq!(
            lines.len(),
            content_lines.len(),
            "Line count changed! Expected {} lines, got {}. Original:\n{}\nResult:\n{}",
            content_lines.len(),
            lines.len(),
            content,
            new_content
        );

        // Verify blank line exists at expected position
        let has_blank_line = lines.iter().any(|line| line.trim().is_empty());
        assert!(has_blank_line, "Blank line lost! Got:\n{}", new_content);

        // Verify trailing newline
        assert!(
            new_content.ends_with('\n'),
            "Trailing newline lost!"
        );
    }

    #[test]
    fn test_preserves_indentation_exactly() {
        let content = "    deep:\n      nested:  value with integration-tests/path\n";

        let plugin = YamlLanguagePlugin::new();
        let result = plugin.rewrite_file_references(
            content,
            Path::new("integration-tests"),
            Path::new("tests"),
            Path::new("."),
            Path::new("."),
            None,
        );

        assert!(result.is_some());
        let (new_content, count) = result.unwrap();
        assert_eq!(count, 1);

        // Exact indentation must be preserved
        assert!(new_content.starts_with("    deep:"));
        assert!(new_content.contains("      nested:"));
    }

    #[test]
    fn test_preserves_multiple_comments() {
        let content = r#"# Header comment
# Another header
jobs:
  # Job section
  test:
    runs-on: ubuntu-latest  # inline comment
    steps:
      - run: cargo test --manifest-path integration-tests/Cargo.toml  # test step
# Footer comment
"#;

        let plugin = YamlLanguagePlugin::new();
        let result = plugin.rewrite_file_references(
            content,
            Path::new("integration-tests"),
            Path::new("tests"),
            Path::new("."),
            Path::new("."),
            None,
        );

        assert!(result.is_some());
        let (new_content, count) = result.unwrap();
        assert_eq!(count, 1);

        // All comments must be preserved
        assert!(new_content.contains("# Header comment"));
        assert!(new_content.contains("# Another header"));
        assert!(new_content.contains("  # Job section"));
        assert!(new_content.contains("# inline comment"));
        assert!(new_content.contains("# test step"));
        assert!(new_content.contains("# Footer comment"));
    }

    #[test]
    fn test_handles_list_items() {
        let content = r#"paths:
  - integration-tests/src
  - integration-tests/tests
  - other/path
"#;

        let plugin = YamlLanguagePlugin::new();
        let result = plugin.rewrite_file_references(
            content,
            Path::new("integration-tests"),
            Path::new("tests"),
            Path::new("."),
            Path::new("."),
            None,
        );

        assert!(result.is_some());
        let (new_content, count) = result.unwrap();
        assert_eq!(count, 2);
        assert!(new_content.contains("- tests/src"));
        assert!(new_content.contains("- tests/tests"));
        assert!(new_content.contains("- other/path"));
    }

    #[tokio::test]
    async fn test_capabilities() {
        let plugin = YamlLanguagePlugin::new();
        let caps = plugin.capabilities();

        assert!(caps.imports);
        assert!(!caps.workspace);
    }
}
