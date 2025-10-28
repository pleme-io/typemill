//! CPP language plugin for TypeMill

mod ast_parser;
mod cmake_parser;
mod import_support;

use async_trait::async_trait;
use mill_plugin_api::{
    import_support::{
        ImportAdvancedSupport, ImportMoveSupport, ImportMutationSupport, ImportParser,
        ImportRenameSupport,
    },
    mill_plugin, LanguagePlugin, LanguageMetadata, LspConfig, ManifestData, ParsedSource,
    PluginCapabilities, PluginResult, Symbol, SymbolKind,
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

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        Ok(ast_parser::parse_source(source))
    }

    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
        if path.file_name().unwrap_or_default().to_str().unwrap_or_default().starts_with("CMakeLists") {
            cmake_parser::analyze_cmake_manifest(path)
        } else {
            Err(mill_plugin_api::PluginError::not_supported(
                "Manifest analysis for this file type",
            ))
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::none().with_imports()
    }

    fn import_parser(&self) -> Option<&dyn ImportParser> {
        Some(&import_support::CppImportSupport)
    }

    fn import_rename_support(&self) -> Option<&dyn ImportRenameSupport> {
        Some(&import_support::CppImportSupport)
    }

    fn import_move_support(&self) -> Option<&dyn ImportMoveSupport> {
        Some(&import_support::CppImportSupport)
    }

    fn import_mutation_support(&self) -> Option<&dyn ImportMutationSupport> {
        Some(&import_support::CppImportSupport)
    }

    fn import_advanced_support(&self) -> Option<&dyn ImportAdvancedSupport> {
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