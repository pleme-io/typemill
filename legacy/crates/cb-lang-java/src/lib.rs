//! Java language plugin for Codebuddy
//!
//! Provides AST parsing, symbol extraction, and manifest analysis for Java.

pub mod import_support;
mod manifest;
mod parser;
pub mod refactoring;
pub mod workspace_support;

use async_trait::async_trait;
use cb_plugin_api::{
    ImportSupport, LanguageCapabilities, LanguageMetadata, LanguagePlugin, ManifestData,
    ParsedSource, PluginResult, WorkspaceSupport,
};
use std::path::Path;

/// Java language plugin
pub struct JavaPlugin {
    metadata: LanguageMetadata,
    import_support: import_support::JavaImportSupport,
    workspace_support: workspace_support::JavaWorkspaceSupport,
}

impl JavaPlugin {
    pub fn new() -> Self {
        Self {
            metadata: LanguageMetadata::JAVA,
            import_support: import_support::JavaImportSupport::new(),
            workspace_support: workspace_support::JavaWorkspaceSupport::new(),
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
            imports: true,   // ✅ AST-based import support via JavaParser
            workspace: true, // ✅ Maven multi-module workspace support via quick-xml
        }
    }

    fn import_support(&self) -> Option<&dyn ImportSupport> {
        Some(&self.import_support)
    }

    fn workspace_support(&self) -> Option<&dyn WorkspaceSupport> {
        Some(&self.workspace_support)
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

        assert!(caps.imports, "Java plugin should support imports via AST");
        assert!(
            caps.workspace,
            "Java plugin should support workspace via quick-xml"
        );
    }

    #[test]
    fn test_java_import_support() {
        let plugin = JavaPlugin::new();
        assert!(
            plugin.import_support().is_some(),
            "Java should have import support"
        );
    }

    #[test]
    fn test_java_workspace_support() {
        let plugin = JavaPlugin::new();
        assert!(
            plugin.workspace_support().is_some(),
            "Java should have workspace support"
        );
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
