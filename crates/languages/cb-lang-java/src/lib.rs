//! Java language plugin for Codebuddy
//!
//! Provides AST parsing, symbol extraction, and manifest analysis for Java.

mod parser;
mod manifest;

use cb_plugin_api::{
    LanguageIntelligencePlugin, ManifestData, ParsedSource, PluginResult,
};
use async_trait::async_trait;
use std::path::Path;

/// Java language plugin
pub struct JavaPlugin;

impl JavaPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JavaPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguageIntelligencePlugin for JavaPlugin {
    fn name(&self) -> &'static str {
        "Java"
    }

    fn file_extensions(&self) -> Vec<&'static str> {
        vec!["java"]
    }

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        parser::parse_source(source)
    }

    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
        manifest::analyze_manifest(path).await
    }

    fn handles_manifest(&self, filename: &str) -> bool {
        matches!(filename, "pom.xml" | "build.gradle" | "build.gradle.kts")
    }

    fn manifest_filename(&self) -> &'static str {
        // Default to pom.xml for manifest generation
        "pom.xml"
    }

    fn source_dir(&self) -> &'static str {
        // Standard Maven/Gradle source directory
        "src/main/java"
    }

    fn entry_point(&self) -> &'static str {
        // Java has no single entry point file, can be any class with a main method
        ""
    }

    fn module_separator(&self) -> &'static str {
        // Java uses "." as a separator for package and class names.
        "."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let plugin = JavaPlugin::new();
        assert_eq!(plugin.name(), "Java");
    }

    #[test]
    fn test_file_extensions() {
        let plugin = JavaPlugin::new();
        let extensions = plugin.file_extensions();
        assert!(!extensions.is_empty());
    }
}