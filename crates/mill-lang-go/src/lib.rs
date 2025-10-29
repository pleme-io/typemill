pub mod import_support;
mod manifest;
pub mod parser;
pub mod refactoring;
pub mod workspace_support;

use async_trait::async_trait;
use mill_plugin_api::{
    mill_plugin, CreatePackageConfig, CreatePackageResult, ImportAdvancedSupport,
    ImportMoveSupport, ImportMutationSupport, ImportParser, ImportRenameSupport, LanguageMetadata,
    LanguagePlugin, ManifestData, ParsedSource, PluginCapabilities, PluginError, PluginResult,
    ProjectFactory, WorkspaceSupport, LspConfig, PackageInfo,
};
use std::any::Any;
use std::path::Path;

pub const METADATA: LanguageMetadata = LanguageMetadata {
    name: "Go",
    extensions: &["go"],
    manifest_filename: "go.mod",
    source_dir: ".",
    entry_point: "main.go",
    module_separator: "/",
};

pub const CAPABILITIES: PluginCapabilities = PluginCapabilities {
    imports: true,
    workspace: true,
    project_factory: true,
    path_alias_resolver: false,
};

#[derive(Default)]
pub struct GoPlugin {
    import_support: import_support::GoImportSupport,
    workspace_support: workspace_support::GoWorkspaceSupport,
}

impl GoPlugin {
    pub fn new() -> Box<dyn LanguagePlugin> {
        Box::new(Self::default())
    }
}

#[async_trait]
impl LanguagePlugin for GoPlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &METADATA
    }

    fn capabilities(&self) -> PluginCapabilities {
        CAPABILITIES
    }

    fn as_any(&self) -> &dyn Any {
        self
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

    fn project_factory(&self) -> Option<&dyn ProjectFactory> {
        Some(self)
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

    fn workspace_support(&self) -> Option<&dyn WorkspaceSupport> {
        Some(&self.workspace_support)
    }
}

impl ProjectFactory for GoPlugin {
    fn create_package(
        &self,
        _config: &CreatePackageConfig,
    ) -> PluginResult<CreatePackageResult> {
        todo!("Project creation for Go is not yet implemented.");
    }
}

mill_plugin! {
    name: "go",
    extensions: ["go"],
    manifest: "go.mod",
    capabilities: CAPABILITIES,
    factory: GoPlugin::new,
    lsp: Some(LspConfig {
        command: "gopls",
        arguments: &[""],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{PathBuf};
    use mill_plugin_api::{PackageType, Template};

    #[test]
    fn test_go_capabilities() {
        let plugin = GoPlugin::new();
        let caps = plugin.capabilities();

        assert!(caps.imports);
        assert!(caps.workspace);
        assert!(caps.project_factory);
    }

    #[test]
    fn test_go_workspace_support() {
        let plugin = GoPlugin::new();
        assert!(
            plugin.workspace_support().is_some(),
            "Go should have workspace support"
        );
    }

    #[test]
    #[ignore]
    fn test_go_project_factory() {
        let plugin = GoPlugin::new();
        let factory = plugin.project_factory().unwrap();
        let config = CreatePackageConfig {
            package_path: "my-go-app".to_string(),
            workspace_root: ".".to_string(),
            package_type: PackageType::Binary,
            template: Template::Minimal,
            add_to_workspace: false,
        };
        let result = factory.create_package(&config).unwrap();
        assert_eq!(result.created_files.len(), 2);
        assert_eq!(result.created_files[0], "my-go-app/go.mod".to_string());
        assert_eq!(result.created_files[1], "my-go-app/main.go".to_string());
    }
}