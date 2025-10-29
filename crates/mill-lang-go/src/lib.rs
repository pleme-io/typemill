//! Go Language Plugin for TypeMill
//!
//! This crate provides complete Go language support, implementing the
//! `LanguagePlugin` trait from `mill_plugin_api`.

pub mod import_support;
mod manifest;
pub mod parser;
pub mod refactoring;

use async_trait::async_trait;
use mill_plugin_api::{
    mill_plugin, CreatePackageConfig, CreatePackageResult, LanguagePlugin, ManifestData, PackageInfo,
    ParsedSource, PluginResult, ProjectFactory, ImportParser, ImportRenameSupport,
    ImportMoveSupport, ImportMutationSupport, ImportAdvancedSupport, LanguageMetadata,
    PluginCapabilities, LspConfig, PluginError,
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
        None
    }

    fn project_factory(&self) -> Option<&dyn ProjectFactory> {
        Some(self)
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
    use mill_plugin_api::Template;
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

        let go_mod_content = std::fs::read_to_string(tmp_dir.path().join("my-go-app/go.mod")).unwrap();
        assert!(go_mod_content.contains("module my-go-app"));
    }
}