//! C language plugin for TypeMill

mod ast_parser;
mod cmake_parser;
pub mod constants;
mod import_support;
mod makefile_parser;
mod lsp_installer;
mod project_factory;
mod refactoring;
mod workspace_support;

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

use self::{
    lsp_installer::CLspInstaller, project_factory::CProjectFactory,
    workspace_support::CWorkspaceSupport,
};
use mill_foundation::protocol::{ImportGraph, ImportInfo, ImportType};
use mill_plugin_api::{
    ImportAnalyzer, LspInstaller, ManifestUpdater, ModuleReference, ModuleReferenceScanner,
    ProjectFactory, ReferenceKind, ScanScope, WorkspaceSupport,
};

use crate::constants::{assertion_patterns, test_patterns, INCLUDE_PATTERN, LIBS_PATTERN};

pub struct CPlugin {
    metadata: LanguageMetadata,
    project_factory: CProjectFactory,
    workspace_support: CWorkspaceSupport,
    lsp_installer: CLspInstaller,
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
            project_factory: CProjectFactory,
            workspace_support: CWorkspaceSupport,
            lsp_installer: CLspInstaller,
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

    async fn list_functions(&self, source: &str) -> PluginResult<Vec<String>> {
        Ok(ast_parser::list_functions(source))
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
        PluginCapabilities::none()
            .with_imports()
            .with_workspace()
            .with_project_factory()
    }

    fn analyze_detailed_imports(
        &self,
        source: &str,
        file_path: Option<&Path>,
    ) -> PluginResult<mill_foundation::protocol::ImportGraph> {
        import_support::CImportSupport.analyze_detailed_imports(source, file_path)
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

    fn refactoring_provider(&self) -> Option<&dyn mill_plugin_api::RefactoringProvider> {
        Some(self)
    }

    fn project_factory(&self) -> Option<&dyn ProjectFactory> {
        Some(&self.project_factory)
    }

    fn workspace_support(&self) -> Option<&dyn WorkspaceSupport> {
        Some(&self.workspace_support)
    }

    fn module_reference_scanner(&self) -> Option<&dyn ModuleReferenceScanner> {
        Some(self)
    }

    fn import_analyzer(&self) -> Option<&dyn ImportAnalyzer> {
        Some(self)
    }

    fn manifest_updater(&self) -> Option<&dyn ManifestUpdater> {
        Some(self)
    }

    fn lsp_installer(&self) -> Option<&dyn LspInstaller> {
        Some(&self.lsp_installer)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[async_trait]
impl ManifestUpdater for CPlugin {
    async fn update_dependency(
        &self,
        manifest_path: &Path,
        _old_name: &str,
        new_name: &str,
        _new_version: Option<&str>,
    ) -> PluginResult<String> {
        let content = std::fs::read_to_string(manifest_path).unwrap();
        if let Some(caps) = LIBS_PATTERN.captures(&content) {
            let existing_libs = caps.get(1).unwrap().as_str();
            let new_libs = format!("{} -l{}", existing_libs, new_name);
            Ok(content.replace(existing_libs, &new_libs))
        } else {
            Ok(format!("{}\nLIBS = -l{}", content, new_name))
        }
    }

    fn generate_manifest(&self, package_name: &str, dependencies: &[String]) -> String {
        let libs = dependencies
            .iter()
            .map(|d| format!("-l{}", d))
            .collect::<Vec<String>>()
            .join(" ");

        format!(
            "CC = gcc\nCFLAGS = -Wall -Wextra -std=c11\nTARGET = {}\nSRCS = src/main.c\nLIBS = {}\n\nall: $(TARGET)\n\n$(TARGET): $(SRCS)\n\t$(CC) $(CFLAGS) -o $(TARGET) $(SRCS) $(LIBS)\n\nclean:\n\trm -f $(TARGET)\n",
            package_name, libs
        )
    }
}

impl ImportAnalyzer for CPlugin {
    fn build_import_graph(&self, file_path: &Path) -> PluginResult<ImportGraph> {
        let content = std::fs::read_to_string(file_path).unwrap();
        let mut imports = Vec::new();

        for (i, line) in content.lines().enumerate() {
            for cap in INCLUDE_PATTERN.captures_iter(line) {
                imports.push(ImportInfo {
                    module_path: cap.get(2).unwrap().as_str().to_string(),
                    import_type: ImportType::CInclude,
                    named_imports: vec![],
                    default_import: None,
                    namespace_import: None,
                    type_only: false,
                    location: mill_foundation::protocol::SourceLocation {
                        start_line: i as u32,
                        start_column: cap.get(2).unwrap().start() as u32,
                        end_line: i as u32,
                        end_column: cap.get(2).unwrap().end() as u32,
                    },
                });
            }
        }

        use chrono::Utc;
        Ok(ImportGraph {
            source_file: file_path.to_str().unwrap().to_string(),
            imports,
            importers: vec![],
            metadata: mill_foundation::protocol::ImportGraphMetadata {
                language: "c".to_string(),
                parsed_at: Utc::now(),
                parser_version: "0.1.0".to_string(),
                circular_dependencies: vec![],
                external_dependencies: vec![],
            },
        })
    }
}

impl ModuleReferenceScanner for CPlugin {
    fn scan_references(
        &self,
        content: &str,
        _module_name: &str,
        scope: ScanScope,
    ) -> PluginResult<Vec<ModuleReference>> {
        let mut references = Vec::new();

        for (i, line) in content.lines().enumerate() {
            if scope == ScanScope::AllUseStatements && (line.trim().starts_with("//") || line.trim().starts_with("/*")) {
                continue;
            }

            for cap in INCLUDE_PATTERN.captures_iter(line) {
                references.push(ModuleReference {
                    text: cap.get(2).unwrap().as_str().to_string(),
                    line: i + 1,
                    column: cap.get(2).unwrap().start(),
                    length: cap.get(2).unwrap().as_str().len(),
                    kind: ReferenceKind::Declaration,
                });
            }
        }

        Ok(references)
    }
}

// Implement RefactoringProvider trait for CPlugin
#[async_trait]
impl mill_plugin_api::RefactoringProvider for CPlugin {
    fn supports_inline_variable(&self) -> bool {
        true
    }

    async fn plan_inline_variable(
        &self,
        source: &str,
        variable_line: u32,
        variable_col: u32,
        file_path: &str,
    ) -> PluginResult<mill_foundation::protocol::EditPlan> {
        refactoring::plan_inline_variable(source, variable_line, variable_col, file_path)
    }

    fn supports_extract_function(&self) -> bool {
        true
    }

    async fn plan_extract_function(
        &self,
        source: &str,
        start_line: u32,
        end_line: u32,
        function_name: &str,
        file_path: &str,
    ) -> PluginResult<mill_foundation::protocol::EditPlan> {
        refactoring::plan_extract_function(source, start_line, end_line, function_name, file_path)
    }

    fn supports_extract_variable(&self) -> bool {
        true
    }

    async fn plan_extract_variable(
        &self,
        source: &str,
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
        variable_name: Option<String>,
        file_path: &str,
    ) -> PluginResult<mill_foundation::protocol::EditPlan> {
        refactoring::plan_extract_variable(
            source,
            start_line,
            start_col,
            end_line,
            end_col,
            variable_name,
            file_path,
        )
    }
}

impl mill_plugin_api::AnalysisMetadata for CPlugin {
    fn test_patterns(&self) -> Vec<regex::Regex> {
        test_patterns()
    }

    fn assertion_patterns(&self) -> Vec<regex::Regex> {
        assertion_patterns()
    }

    fn doc_comment_style(&self) -> mill_plugin_api::DocCommentStyle {
        mill_plugin_api::DocCommentStyle::JavaDoc
    }

    fn visibility_keywords(&self) -> Vec<&'static str> {
        vec!["static", "extern"]
    }

    fn interface_keywords(&self) -> Vec<&'static str> {
        vec!["struct", "enum", "typedef"]
    }

    fn complexity_keywords(&self) -> Vec<&'static str> {
        vec!["if", "else", "switch", "case", "for", "while", "do", "&&", "||"]
    }

    fn nesting_penalty(&self) -> f32 {
        1.3
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