//! Java language plugin for Codebuddy
//!
//! Provides AST parsing, symbol extraction, and manifest analysis for Java.

mod parser;
mod manifest;

use cb_plugin_api::{
    LanguagePlugin, LanguageMetadata, LanguageCapabilities, ManifestData, ParsedSource, PluginResult,
};
use async_trait::async_trait;
use std::path::Path;

/// Java language plugin
pub struct JavaPlugin {
    metadata: LanguageMetadata,
}

impl JavaPlugin {
    pub fn new() -> Self {
        Self {
            metadata: LanguageMetadata::JAVA,
        }
    }
}

impl Default for JavaPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguagePlugin for JavaPlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &self.metadata
    }

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        parser::parse_source(source)
    }

    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
        manifest::analyze_manifest(path).await
    }

    fn capabilities(&self) -> LanguageCapabilities {
        LanguageCapabilities {
            imports: false,  // Java doesn't have import support yet
            workspace: false, // Java doesn't have workspace support yet
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let plugin = JavaPlugin::new();
        assert_eq!(plugin.metadata().name, "Java");
    }

    #[test]
    fn test_file_extensions() {
        let plugin = JavaPlugin::new();
        let extensions = plugin.metadata().extensions;
        assert!(!extensions.is_empty());
        assert!(extensions.contains(&"java"));
    }

    #[test]
    fn test_java_capabilities() {
        let plugin = JavaPlugin::new();
        let caps = plugin.capabilities();

        assert!(!caps.imports, "Java plugin should not support imports yet");
        assert!(!caps.workspace, "Java plugin should not support workspace yet");
    }

    #[test]
    fn test_java_metadata() {
        let plugin = JavaPlugin::new();

        assert_eq!(plugin.metadata().manifest_filename, "pom.xml");
        assert_eq!(plugin.metadata().entry_point, "");
        assert_eq!(plugin.metadata().module_separator, ".");
        assert_eq!(plugin.metadata().source_dir, "src/main/java");
    }
}