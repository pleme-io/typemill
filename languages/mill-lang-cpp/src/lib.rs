//! CPP language plugin for TypeMill

mod analysis;
mod ast_parser;
mod cmake_parser;
mod conan_parser;
mod import_support;
mod project_factory;
mod refactoring;
mod vcpkg_parser;
mod workspace_support;
mod manifest_updater;
mod lsp_installer;

use async_trait::async_trait;
use crate::lsp_installer::CppLspInstaller;
use mill_plugin_api::{
    import_support::{
        ImportAdvancedSupport, ImportMoveSupport, ImportMutationSupport, ImportParser,
        ImportRenameSupport,
    },
    lsp_installer::LspInstaller,
    mill_plugin, LanguagePlugin, LanguageMetadata, LspConfig, ManifestData, ManifestUpdater,
    ParsedSource, PluginCapabilities, PluginResult,
};
use std::path::Path;

pub struct CppPlugin {
    metadata: LanguageMetadata,
    lsp_installer: CppLspInstaller,
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
            lsp_installer: CppLspInstaller,
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
        let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or_default();
        if filename.starts_with("CMakeLists") {
            cmake_parser::analyze_cmake_manifest(path)
        } else if filename == "conanfile.txt" || filename == "conanfile.py" {
            conan_parser::analyze_conan_manifest(path)
        } else if filename == "vcpkg.json" {
            let content = std::fs::read_to_string(path)
                .map_err(|e| mill_plugin_api::PluginError::manifest(format!("Failed to read manifest: {}", e)))?;
            vcpkg_parser::analyze_vcpkg_manifest(&content)
        } else {
            Err(mill_plugin_api::PluginError::not_supported(
                "Manifest analysis for this file type",
            ))
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::none()
            .with_imports()
            .with_workspace()
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

    fn project_factory(&self) -> Option<&dyn mill_plugin_api::ProjectFactory> {
        Some(&project_factory::CppProjectFactory)
    }

    fn workspace_support(&self) -> Option<&dyn mill_plugin_api::WorkspaceSupport> {
        Some(&workspace_support::CppWorkspaceSupport)
    }

    fn refactoring_provider(&self) -> Option<&dyn mill_plugin_api::RefactoringProvider> {
        Some(&refactoring::CppRefactoringProvider)
    }

    fn module_reference_scanner(&self) -> Option<&dyn mill_plugin_api::ModuleReferenceScanner> {
        Some(&analysis::CppAnalysisProvider)
    }

    fn import_analyzer(&self) -> Option<&dyn mill_plugin_api::ImportAnalyzer> {
        Some(&analysis::CppAnalysisProvider)
    }

    fn manifest_updater(&self) -> Option<&dyn ManifestUpdater> {
        Some(&manifest_updater::CppManifestUpdater)
    }

    fn lsp_installer(&self) -> Option<&dyn LspInstaller> {
        Some(&self.lsp_installer)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpp_plugin_creation() {
        let plugin = CppPlugin::default();
        assert_eq!(plugin.metadata().name, "C++");
    }
}
