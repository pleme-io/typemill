//! Go Language Plugin for TypeMill
//!
//! This crate provides complete Go language support, implementing the
//! `LanguagePlugin` trait from `mill_plugin_api`.

pub mod import_support;
pub mod lsp_installer;
mod manifest;
pub mod parser;
pub mod refactoring;
pub mod workspace_support;

use async_trait::async_trait;
use mill_foundation::protocol::EditPlan;
use mill_foundation::protocol::ImportGraph;
use mill_plugin_api::{
    mill_plugin, CreatePackageConfig, CreatePackageResult, ImportAdvancedSupport, ImportAnalyzer,
    ImportMoveSupport, ImportMutationSupport, ImportParser, ImportRenameSupport, LanguageMetadata,
    LanguagePlugin, LspConfig, LspInstaller, ManifestData, ManifestUpdater, ModuleReference,
    ModuleReferenceScanner, PackageInfo, ParsedSource, PluginCapabilities, PluginError,
    PluginResult, ProjectFactory, ReferenceKind, RefactoringProvider, ScanScope, WorkspaceSupport,
};
use std::any::Any;
use std::path::{Path, PathBuf};

pub const METADATA: LanguageMetadata = LanguageMetadata {
    name: "Go",
    extensions: &["go"],
    manifest_filename: "go.mod",
    module_separator: "/",
    source_dir: ".",
    entry_point: "main.go",
};

pub const CAPABILITIES: PluginCapabilities = PluginCapabilities {
    imports: true,
    workspace: true,
    project_factory: true,
    path_alias_resolver: false,
};

/// Go language plugin implementation.
#[derive(Default)]
pub struct GoPlugin {
    import_support: import_support::GoImportSupport,
    workspace_support: workspace_support::GoWorkspaceSupport,
    lsp_installer: lsp_installer::GoLspInstaller,
}

#[async_trait]
impl LanguagePlugin for GoPlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &METADATA
    }

    fn capabilities(&self) -> PluginCapabilities {
        CAPABILITIES
    }

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        let symbols = parser::extract_symbols(source)?;

        Ok(ParsedSource {
            data: serde_json::json!({
                "language": "go",
                "symbols_count": symbols.len()
            }),
            symbols,
        })
    }

    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
        manifest::load_go_mod(path).await
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn import_parser(&self) -> Option<&dyn ImportParser> {
        Some(&self.import_support)
    }

    fn import_rename_support(&self) -> Option<&dyn ImportRenameSupport> {
        Some(&self.import_support)
    }

    fn import_move_support(&self) -> Option<&dyn ImportMoveSupport> {
        Some(&self.import_support)
    }

    fn import_mutation_support(&self) -> Option<&dyn ImportMutationSupport> {
        Some(&self.import_support)
    }

    fn import_advanced_support(&self) -> Option<&dyn ImportAdvancedSupport> {
        Some(&self.import_support)
    }

    fn project_factory(&self) -> Option<&dyn ProjectFactory> {
        Some(self)
    }

    fn refactoring_provider(&self) -> Option<&dyn RefactoringProvider> {
        Some(self)
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
}

#[async_trait]
impl ManifestUpdater for GoPlugin {
    async fn update_dependency(
        &self,
        manifest_path: &Path,
        old_name: &str,
        new_name: &str,
        new_version: Option<&str>,
    ) -> PluginResult<String> {
        let content = tokio::fs::read_to_string(manifest_path)
            .await
            .map_err(|e| PluginError::internal(e.to_string()))?;
        manifest::update_dependency(&content, old_name, new_name, new_version)
    }

    fn generate_manifest(&self, package_name: &str, _dependencies: &[String]) -> String {
        manifest::generate_manifest(package_name, "1.21")
    }
}

impl ImportAnalyzer for GoPlugin {
    fn build_import_graph(&self, file_path: &Path) -> PluginResult<ImportGraph> {
        let content =
            std::fs::read_to_string(file_path).map_err(|e| PluginError::internal(e.to_string()))?;
        parser::analyze_imports(&content, Some(file_path))
    }
}

impl ModuleReferenceScanner for GoPlugin {
    fn scan_references(
        &self,
        content: &str,
        module_name: &str,
        scope: ScanScope,
    ) -> PluginResult<Vec<ModuleReference>> {
        let mut references = Vec::new();
        let import_pattern = format!("\"([^\"]*?{})\"", regex::escape(module_name));
        let import_re = regex::Regex::new(&import_pattern)
            .map_err(|e| PluginError::internal(format!("Invalid regex: {}", e)))?;

        for (i, line) in content.lines().enumerate() {
            if line.trim().starts_with("import") || line.trim().starts_with('"') {
                for mat in import_re.find_iter(line) {
                    references.push(ModuleReference {
                        line: i,
                        column: mat.start(),
                        length: mat.len(),
                        text: mat.as_str().to_string(),
                        kind: ReferenceKind::Declaration,
                    });
                }
            }

            if scope == ScanScope::All || scope == ScanScope::QualifiedPaths {
                let qualified_pattern = format!(r"\b{}\.", regex::escape(module_name));
                let qualified_re = regex::Regex::new(&qualified_pattern)
                    .map_err(|e| PluginError::internal(format!("Invalid regex: {}", e)))?;
                for mat in qualified_re.find_iter(line) {
                    references.push(ModuleReference {
                        line: i,
                        column: mat.start(),
                        length: mat.len(),
                        text: mat.as_str().to_string(),
                        kind: ReferenceKind::QualifiedPath,
                    });
                }
            }
        }

        Ok(references)
    }
}

#[async_trait]
impl RefactoringProvider for GoPlugin {
    // extract_function
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
    ) -> PluginResult<EditPlan> {
        refactoring::plan_extract_function(source, start_line, end_line, function_name, file_path)
    }

    // inline_variable
    fn supports_inline_variable(&self) -> bool {
        true
    }

    async fn plan_inline_variable(
        &self,
        source: &str,
        line: u32,
        col: u32,
        file_path: &str,
    ) -> PluginResult<EditPlan> {
        refactoring::plan_inline_variable(source, line, col, file_path)
    }

    // extract_variable
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
    ) -> PluginResult<EditPlan> {
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

impl ProjectFactory for GoPlugin {
    fn create_package(&self, config: &CreatePackageConfig) -> PluginResult<CreatePackageResult> {
        let package_path = Path::new(&config.package_path);
        let absolute_package_path = PathBuf::from(&config.workspace_root).join(package_path);
        std::fs::create_dir_all(&absolute_package_path).map_err(|e| PluginError::internal(e.to_string()))?;

        let module_name = package_path.to_string_lossy();
        let go_mod_content = manifest::generate_manifest(&module_name, "1.21");
        let go_mod_path = absolute_package_path.join("go.mod");
        std::fs::write(&go_mod_path, go_mod_content).map_err(|e| PluginError::internal(e.to_string()))?;

        let main_go_content = format!("package main\n\nimport \"fmt\"\n\nfunc main() {{\n\tfmt.Println(\"Hello, {}!\")\n}}\n", module_name);
        let main_go_path = absolute_package_path.join("main.go");
        std::fs::write(&main_go_path, main_go_content).map_err(|e| PluginError::internal(e.to_string()))?;

        let created_files = vec![
            go_mod_path.to_string_lossy().into_owned(),
            main_go_path.to_string_lossy().into_owned(),
        ];

        Ok(CreatePackageResult {
            created_files,
            package_info: PackageInfo {
                name: module_name.into_owned(),
                version: "1.0.0".to_string(),
                manifest_path: go_mod_path.to_string_lossy().into_owned(),
            },
            workspace_updated: false,
        })
    }
}

mill_plugin! {
    name: "Go",
    extensions: METADATA.extensions,
    manifest: "go.mod",
    capabilities: CAPABILITIES,
    factory: || {
        Box::new(GoPlugin::default())
    },
    lsp: Some(LspConfig::new("gopls", &["gopls"]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mill_plugin_api::{LanguagePlugin, Template, WorkspaceSupport};
    use tempfile::tempdir;

    #[test]
    fn test_go_metadata() {
        let plugin = GoPlugin::default();
        let metadata = plugin.metadata();
        assert_eq!(metadata.name, "Go");
    }

    #[test]
    fn test_go_capabilities() {
        let plugin = GoPlugin::default();
        let caps = plugin.capabilities();
        assert!(caps.imports);
        assert!(caps.workspace);
    }

    #[test]
    fn test_create_package() {
        let plugin = GoPlugin::default();
        let tmp_dir = tempdir().unwrap();
        let config = CreatePackageConfig {
            package_path: "my-go-app".to_string(),
            workspace_root: tmp_dir.path().to_str().unwrap().to_string(),
            add_to_workspace: false,
            package_type: mill_plugin_api::PackageType::Binary,
            template: Template::Minimal,
        };

        let result = plugin.create_package(&config).unwrap();
        assert_eq!(result.created_files.len(), 2);
        assert!(result.created_files[0].contains("go.mod"));
        assert!(result.created_files[1].contains("main.go"));

        let go_mod_content =
            std::fs::read_to_string(tmp_dir.path().join("my-go-app/go.mod")).unwrap();
        assert!(go_mod_content.contains("module my-go-app"));
    }

    #[test]
    fn test_import_advanced_support_returns_some() {
        let plugin = GoPlugin::default();
        assert!(plugin.import_advanced_support().is_some());
    }

    #[tokio::test]
    async fn test_refactoring_provider_extract_function() {
        let plugin = GoPlugin::default();
        let provider = plugin.refactoring_provider().unwrap();
        let source = "package main\n\nfunc main() {\n\tprintln(\"hello\")\n}";
        let plan = provider
            .plan_extract_function(source, 3, 3, "greet", "main.go")
            .await
            .unwrap();
        assert_eq!(plan.edits.len(), 2);
    }

    #[tokio::test]
    async fn test_refactoring_provider_inline_variable() {
        let plugin = GoPlugin::default();
        let provider = plugin.refactoring_provider().unwrap();
        let source = "package main\n\nfunc main() {\n\tconst x = \"hello\"\n\tprintln(x)\n}";
        let plan = provider
            .plan_inline_variable(source, 3, 8, "main.go")
            .await
            .unwrap();
        assert_eq!(plan.edits.len(), 2);
    }

    #[tokio::test]
    async fn test_refactoring_provider_extract_variable() {
        let plugin = GoPlugin::default();
        let provider = plugin.refactoring_provider().unwrap();
        let source = "package main\n\nfunc main() {\n\tprintln(\"hello\")\n}";
        let plan = provider
            .plan_extract_variable(source, 3, 9, 3, 16, Some("greeting".to_string()), "main.go")
            .await
            .unwrap();
        assert_eq!(plan.edits.len(), 2);
    }

    #[test]
    fn test_workspace_support_add_and_list_members() {
        let support = workspace_support::GoWorkspaceSupport::default();
        let initial_content = "go 1.21\n";
        let with_member = support.add_workspace_member(initial_content, "my-go-app");
        assert!(with_member.contains("use ./my-go-app"));
        let members = support.list_workspace_members(&with_member);
        assert_eq!(members, vec!["my-go-app"]);
    }

    #[test]
    fn test_module_reference_scanner() {
        let plugin = GoPlugin::default();
        let scanner = plugin.module_reference_scanner().unwrap();
        let content = "package main\n\nimport \"fmt\"";
        let refs = scanner
            .scan_references(content, "fmt", ScanScope::All)
            .unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].line, 2);
    }

    #[test]
    fn test_import_analyzer() {
        let plugin = GoPlugin::default();
        let analyzer = plugin.import_analyzer().unwrap();
        let tmp_dir = tempdir().unwrap();
        let file_path = tmp_dir.path().join("main.go");
        std::fs::write(&file_path, "package main\n\nimport \"fmt\"").unwrap();
        let graph = analyzer.build_import_graph(&file_path).unwrap();
        assert_eq!(graph.imports.len(), 1);
        assert_eq!(graph.imports[0].module_path, "fmt");
    }

    #[tokio::test]
    async fn test_manifest_updater() {
        let plugin = GoPlugin::default();
        let updater = plugin.manifest_updater().unwrap();
        let tmp_dir = tempdir().unwrap();
        let file_path = tmp_dir.path().join("go.mod");
        std::fs::write(&file_path, "module my-go-app\n\nrequire example.com/pkg v1.2.3")
            .unwrap();
        let updated = updater
            .update_dependency(
                &file_path,
                "example.com/pkg",
                "example.com/newpkg",
                Some("v1.2.4"),
            )
            .await
            .unwrap();
        assert!(updated.contains("example.com/newpkg v1.2.4"));
    }

    #[tokio::test]
    async fn test_lsp_installer() {
        let installer = lsp_installer::GoLspInstaller::default();
        // This test can't easily install the real gopls in a hermetic way.
        // We'll just check that the name is correct. In a real CI environment,
        // we would mock the `go install` command.
        assert_eq!(installer.lsp_name(), "gopls");
    }
}