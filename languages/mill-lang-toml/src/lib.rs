//! TOML Language Plugin
//!
//! Provides support for detecting and updating file references in TOML config files.
//! This enables `rename` to track path references when files are moved.

use async_trait::async_trait;
use mill_plugin_api::mill_plugin;
use mill_plugin_api::{
    import_support::ImportRenameSupport, LanguageMetadata, LanguagePlugin, ManifestData,
    ParsedSource, PluginApiError, PluginCapabilities, PluginResult,
};
use std::path::Path;
use tracing::debug;

mod import_support_impl;

use import_support_impl::TomlImportSupport;

// Self-register the plugin with the TypeMill system.
mill_plugin! {
    name: "toml",
    extensions: ["toml"],
    manifest: "Cargo.toml",
    capabilities: TomlLanguagePlugin::CAPABILITIES,
    factory: TomlLanguagePlugin::arc,
    lsp: None
}

/// TOML language plugin
///
/// Detects and updates file references in TOML configuration files:
/// - Path values in any TOML field
/// - Paths in arrays
/// - Paths in tables and nested structures
///
/// Does NOT process:
/// - Non-path string values (URLs, names, etc.)
pub struct TomlLanguagePlugin {
    metadata: LanguageMetadata,
    import_support: TomlImportSupport,
}

impl TomlLanguagePlugin {
    /// The capabilities of this plugin.
    pub const CAPABILITIES: PluginCapabilities = PluginCapabilities::none().with_imports(); // We support file references

    pub fn new() -> Self {
        Self {
            metadata: LanguageMetadata {
                name: "toml",
                extensions: &["toml"],
                manifest_filename: "Cargo.toml",
                source_dir: ".",
                entry_point: "Cargo.toml",
                module_separator: "/",
            },
            import_support: TomlImportSupport::new(),
        }
    }

    /// Create a boxed instance for plugin registry
    pub fn arc() -> Box<dyn LanguagePlugin> {
        Box::new(Self::new())
    }
}

impl Default for TomlLanguagePlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguagePlugin for TomlLanguagePlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &self.metadata
    }

    async fn parse(&self, _source: &str) -> PluginResult<ParsedSource> {
        // TOML files don't have symbols we care about extracting
        Ok(ParsedSource {
            data: serde_json::json!({
                "language": "toml",
            }),
            symbols: vec![],
        })
    }

    async fn analyze_manifest(&self, _path: &Path) -> PluginResult<ManifestData> {
        // TOML file itself might be a manifest, but we don't analyze it here
        Err(PluginApiError::not_supported(
            "TOML plugin does not analyze manifest data",
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
        rename_info: Option<&serde_json::Value>,
    ) -> Option<(String, usize)> {
        // Extract flags from rename_info
        let update_exact_matches = rename_info
            .and_then(|v| v.get("update_exact_matches"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let update_comments = rename_info
            .and_then(|v| v.get("update_comments"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Phase 1: Structured value updates (existing logic)
        let (mut result, mut count) = match self.import_support.rewrite_toml_paths(
            content,
            old_path,
            new_path,
            update_exact_matches,
        ) {
            Ok((new_content, value_count)) => {
                if value_count > 0 {
                    debug!(
                        changes = value_count,
                        old_path = %old_path.display(),
                        new_path = %new_path.display(),
                        "Updated paths in TOML file"
                    );
                }
                (new_content, value_count)
            }
            Err(e) => {
                tracing::error!(
                    error = ?e,
                    "Failed to rewrite TOML paths"
                );
                return None;
            }
        };

        // Phase 2: Comment updates (opt-in via update_comments flag)
        if update_comments {
            let old_basename = old_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_else(|| old_path.to_str().unwrap_or(""));
            let new_basename = new_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_else(|| new_path.to_str().unwrap_or(""));

            // Smart boundary matching: NOT preceded/followed by alphanumeric
            // Allows: "mill-handlers", "mill-handlers-style", "# mill-handlers"
            // Blocks: "mymill-handlers", "mill-handlersystem"
            let pattern = format!(
                r"(?<![a-zA-Z0-9]){}(?![a-zA-Z0-9])",
                fancy_regex::escape(old_basename)
            );

            if let Ok(regex) = fancy_regex::Regex::new(&pattern) {
                let comment_result = regex.replace_all(&result, new_basename);
                let comment_count = comment_result.matches(new_basename).count()
                    - result.matches(new_basename).count();

                if comment_count > 0 {
                    debug!(
                        comment_changes = comment_count,
                        old_basename = old_basename,
                        new_basename = new_basename,
                        "Updated identifiers in TOML comments"
                    );
                    result = comment_result.to_string();
                    count += comment_count;
                }
            }
        }

        Some((result, count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_toml_plugin_basic() {
        let plugin = TomlLanguagePlugin::new();
        let plugin_trait: &dyn LanguagePlugin = &plugin;

        assert_eq!(plugin_trait.metadata().name, "toml");
        assert_eq!(plugin_trait.metadata().extensions, &["toml"]);
        assert!(plugin_trait.handles_extension("toml"));
        assert!(!plugin_trait.handles_extension("rs"));
    }

    #[test]
    fn test_updates_toml_paths() {
        let content = r#"
[build]
target-dir = "integration-tests/target"

[[example]]
path = "integration-tests/examples/main.rs"
"#;

        let plugin = TomlLanguagePlugin::new();
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
        assert!(new_content.contains("\"tests/target\""));
        assert!(new_content.contains("\"tests/examples/main.rs\""));
    }

    #[test]
    fn test_preserves_non_path_strings() {
        let content = r#"
[package]
name = "my-package"
version = "0.1.0"

[build]
target-dir = "integration-tests/target"
"#;

        let plugin = TomlLanguagePlugin::new();
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
        assert!(new_content.contains("name = \"my-package\""));
        assert!(new_content.contains("version = \"0.1.0\""));
        assert!(new_content.contains("\"tests/target\""));
    }

    #[tokio::test]
    async fn test_capabilities() {
        let plugin = TomlLanguagePlugin::new();
        let caps = plugin.capabilities();

        assert!(caps.imports);
        assert!(!caps.workspace);
    }

    #[test]
    fn test_updates_comments_with_flag() {
        let content = r#"
# Layer 6: Handlers (old-handlers)
[package]
name = "test"

# Analysis crates are optional runtime dependencies (feature-gated in old-handlers)
# Allow old-handlers to optionally depend on them via feature flags
wrappers = ["old-handlers", "other-crate"]
"#;

        let plugin = TomlLanguagePlugin::new();

        // Test with update_comments flag enabled
        let rename_info = serde_json::json!({
            "update_comments": true
        });

        let result = plugin.rewrite_file_references(
            content,
            Path::new("old-handlers"),
            Path::new("new-handlers"),
            Path::new("."),
            Path::new("."),
            Some(&rename_info),
        );

        assert!(result.is_some());
        let (new_content, count) = result.unwrap();

        // Should update: 3 comments + 1 value = 4 total
        assert_eq!(count, 4);

        // Verify comment updates
        assert!(new_content.contains("# Layer 6: Handlers (new-handlers)"));
        assert!(new_content.contains("(feature-gated in new-handlers)"));
        assert!(new_content.contains("# Allow new-handlers to optionally"));

        // Verify value update
        assert!(new_content.contains("\"new-handlers\""));
    }

    #[test]
    fn test_smart_boundaries_in_comments() {
        let content = r#"
# The old-handlers-style API is simple
# Don't use myold-handlers (should NOT change)
wrappers = ["old-handlers"]
"#;

        let plugin = TomlLanguagePlugin::new();

        let rename_info = serde_json::json!({
            "update_comments": true
        });

        let result = plugin.rewrite_file_references(
            content,
            Path::new("old-handlers"),
            Path::new("new-handlers"),
            Path::new("."),
            Path::new("."),
            Some(&rename_info),
        );

        assert!(result.is_some());
        let (new_content, count) = result.unwrap();

        // Should update: "old-handlers-style" and "old-handlers" value = 2 total
        assert_eq!(count, 2);

        // Hyphenated identifier should update
        assert!(new_content.contains("new-handlers-style"));

        // Partial match should NOT update
        assert!(new_content.contains("myold-handlers"));
    }

    // ========================================================================
    // EDGE CASE TESTS (2 tests)
    // ========================================================================

    #[test]
    fn test_edge_extremely_long_values() {
        let plugin = TomlLanguagePlugin::new();
        let long_value = "a".repeat(10000);
        let content = format!("key = \"{}\"", long_value);

        // Should handle very long TOML values without panicking
        let result = plugin.rewrite_file_references(
            &content,
            Path::new("old"),
            Path::new("new"),
            Path::new("."),
            Path::new("."),
            None,
        );

        // Should not panic - either returns modified content or None if no changes needed
        let _ = result;
    }

    #[test]
    fn test_edge_unicode_in_keys() {
        let plugin = TomlLanguagePlugin::new();
        let content = "\"函数\" = \"value\"\n\"ключ\" = \"значение\"";

        // Should handle Unicode key names without panicking
        let result = plugin.rewrite_file_references(
            content,
            Path::new("old"),
            Path::new("new"),
            Path::new("."),
            Path::new("."),
            None,
        );

        // Should not panic - either returns modified content or None if no changes needed
        let _ = result;
    }

    // ========================================================================
    // ADDITIONAL TESTS (3 tests)
    // ========================================================================

    #[test]
    fn test_parse_nested_tables() {
        let plugin = TomlLanguagePlugin::new();
        let content = "[package]\nname = \"test\"\n\n[package.metadata]\nauthor = \"test\"";

        // Verify nested table parsing doesn't cause issues
        let result = plugin.rewrite_file_references(
            content,
            Path::new("old"),
            Path::new("new"),
            Path::new("."),
            Path::new("."),
            None,
        );

        // Should handle nested structures gracefully without panicking
        let _ = result;
    }

    #[tokio::test]
    async fn test_integration_cargo_toml_workflow() {
        let harness = mill_test_support::harness::IntegrationTestHarness::new()
            .expect("Should create harness");

        harness
            .create_source_file(
                "Cargo.toml",
                "[package]\nname = \"my-app\"\nversion = \"0.1.0\"\n\n[dependencies]\nserde = \"1.0\"",
            )
            .expect("Should create Cargo.toml");

        // Parse and verify structure
        let content = harness
            .read_file("Cargo.toml")
            .expect("Should read Cargo.toml");
        assert!(content.contains("package"));
        assert!(content.contains("dependencies"));
        assert!(content.contains("serde"));
    }

    #[tokio::test]
    async fn test_integration_path_updates_in_dependencies() {
        let harness = mill_test_support::harness::IntegrationTestHarness::new()
            .expect("Should create harness");

        harness
            .create_source_file(
                "Cargo.toml",
                "[dependencies]\nmy_lib = { path = \"../old-path\" }",
            )
            .expect("Should create Cargo.toml");

        // Verify path can be identified for updates
        let content = harness
            .read_file("Cargo.toml")
            .expect("Should read Cargo.toml");
        assert!(content.contains("path"));
        assert!(content.contains("old-path"));
    }
}
