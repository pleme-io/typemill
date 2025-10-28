//! Swift language plugin for Codebuddy
//!
//! Provides AST parsing, symbol extraction, and manifest analysis for Swift.

mod import_support;
mod manifest;
mod parser;
pub mod refactoring;

use crate::import_support::SwiftImportSupport;
use async_trait::async_trait;
use cb_plugin_api::{
    ImportSupport, LanguageCapabilities, LanguageMetadata, LanguagePlugin, ManifestData,
    ParsedSource, PluginResult,
};
use std::path::Path;

/// Swift language plugin implementation
pub struct SwiftPlugin {
    metadata: LanguageMetadata,
    import_support: SwiftImportSupport,
}

impl SwiftPlugin {
    /// Create a new Swift plugin instance
    pub fn new() -> Self {
        Self {
            metadata: LanguageMetadata::SWIFT,
            import_support: SwiftImportSupport,
        }
    }
}

impl Default for SwiftPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguagePlugin for SwiftPlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> LanguageCapabilities {
        LanguageCapabilities {
            imports: true,
            workspace: false, // TODO: Set to true when workspace support is implemented
        }
    }

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        parser::parse_source(source)
    }

    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
        manifest::analyze_manifest(path).await
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn import_support(&self) -> Option<&dyn ImportSupport> {
        Some(&self.import_support)
    }

    // Optional: Override workspace_support() when ready
    // fn workspace_support(&self) -> Option<&dyn WorkspaceSupport> {
    //     Some(&self.workspace_support)
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let plugin = SwiftPlugin::new();
        assert_eq!(plugin.metadata().name, "Swift");
    }

    #[test]
    fn test_file_extensions() {
        let plugin = SwiftPlugin::new();
        let extensions = plugin.metadata().extensions;
        assert!(!extensions.is_empty());
    }

    #[test]
    fn test_capabilities() {
        let plugin = SwiftPlugin::new();
        let caps = plugin.capabilities();
        // Update these assertions as capabilities are implemented
        assert!(caps.imports);
        assert!(!caps.workspace);
    }
}
