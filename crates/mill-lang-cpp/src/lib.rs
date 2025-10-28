//! CPP language plugin for TypeMill

mod import_support;

use async_trait::async_trait;
use mill_plugin_api::{
    import_support::ImportParser, mill_plugin, LanguagePlugin, LanguageMetadata, LspConfig,
    ManifestData, ParsedSource, PluginCapabilities, PluginResult,
};
use std::path::Path;

pub struct CppPlugin {
    metadata: LanguageMetadata,
}

impl Default for CppPlugin {
    fn default() -> Self {
        Self {
            metadata: LanguageMetadata {
                name: "C++",
                extensions: &["cpp", "cc", "cxx", "h", "hpp"],
                manifest_filename: "CMakeLists.txt",
                source_dir: "src",
                entry_point: "main.cpp",
                module_separator: "::",
            },
        }
    }
}

#[async_trait]
impl LanguagePlugin for CppPlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &self.metadata
    }

    async fn parse(&self, _source: &str) -> PluginResult<ParsedSource> {
        Ok(ParsedSource {
            data: serde_json::Value::Null,
            symbols: vec![],
        })
    }

    async fn analyze_manifest(&self, _path: &Path) -> PluginResult<ManifestData> {
        Err(mill_plugin_api::PluginError::not_supported(
            "Manifest analysis for C++",
        ))
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::none().with_imports()
    }

    fn import_parser(&self) -> Option<&dyn ImportParser> {
        Some(&import_support::CppImportSupport)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

mill_plugin! {
    name: "C++",
    extensions: ["cpp", "cc", "cxx", "h", "hpp"],
    manifest: "CMakeLists.txt",
    capabilities: PluginCapabilities::none().with_imports(),
    factory: || Box::new(CppPlugin::default()),
    lsp: Some(LspConfig::new("clangd", &["clangd"]))
}