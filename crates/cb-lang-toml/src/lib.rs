//! TOML Language Plugin
//!
//! Provides support for detecting and updating file references in TOML config files.
//! This enables `rename.plan` to track path references when files are moved.

use async_trait::async_trait;
use cb_plugin_api::{
    import_support::{ImportRenameSupport},
    LanguageMetadata, LanguagePlugin, ManifestData, ParsedSource, PluginCapabilities,
    PluginError, PluginResult,
};
use cb_plugin_api::codebuddy_plugin;
use std::path::Path;
use tracing::debug;

mod import_support_impl;

use import_support_impl::TomlImportSupport;

// Self-register the plugin with the Codebuddy system.
codebuddy_plugin! {
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
    pub const CAPABILITIES: PluginCapabilities = PluginCapabilities::none()
        .with_imports(); // We support file references

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
        Err(PluginError::not_supported(
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
        _rename_info: Option<&serde_json::Value>,
    ) -> Option<(String, usize)> {
        match self.import_support.rewrite_toml_paths(content, old_path, new_path) {
            Ok((new_content, count)) => {
                if count > 0 {
                    debug!(
                        changes = count,
                        old_path = %old_path.display(),
                        new_path = %new_path.display(),
                        "Updated paths in TOML file"
                    );
                }
                Some((new_content, count))
            }
            Err(e) => {
                tracing::error!(
                    error = ?e,
                    "Failed to rewrite TOML paths"
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
}
