//! C language plugin for TypeMill

mod ast_parser;
mod cmake_parser;
mod import_support;
mod makefile_parser;

use async_trait::async_trait;
use mill_plugin_api::{
    import_support::{
        ImportAdvancedSupport, ImportMoveSupport, ImportMutationSupport, ImportParser,
        ImportRenameSupport,
    },
    mill_plugin, LanguagePlugin, LanguageMetadata, LspConfig, ManifestData, ParsedSource,
    PluginCapabilities, PluginResult,
};
use std::path::Path;

pub struct CPlugin {
    metadata: LanguageMetadata,
}

impl Default for CPlugin {
    fn default() -> Self {
        Self {
            metadata: LanguageMetadata {
                name: "C",
                extensions: &["c", "h"],
                manifest_filename: "Makefile",
                source_dir: "src",
                entry_point: "main.c",
                module_separator: "::", // Not really applicable to C
            },
        }
    }
}

#[async_trait]
impl LanguagePlugin for CPlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &self.metadata
    }

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        Ok(ast_parser::parse_source(source))
    }

    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
        let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or_default();
        if filename.starts_with("CMakeLists") {
            cmake_parser::analyze_cmake_manifest(path)
        } else if filename == "Makefile" {
            makefile_parser::analyze_makefile_manifest(path)
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
        Some(&import_support::CImportSupport)
    }

    fn import_rename_support(&self) -> Option<&dyn ImportRenameSupport> {
        Some(&import_support::CImportSupport)
    }

    fn import_move_support(&self) -> Option<&dyn ImportMoveSupport> {
        Some(&import_support::CImportSupport)
    }

    fn import_mutation_support(&self) -> Option<&dyn ImportMutationSupport> {
        Some(&import_support::CImportSupport)
    }

    fn import_advanced_support(&self) -> Option<&dyn ImportAdvancedSupport> {
        Some(&import_support::CImportSupport)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

mill_plugin! {
    name: "C",
    extensions: ["c", "h"],
    manifest: "Makefile",
    capabilities: PluginCapabilities::none().with_imports(),
    factory: || Box::new(CPlugin::default()),
    lsp: Some(LspConfig::new("clangd", &["clangd"]))
}

#[cfg(test)]
mod tests;