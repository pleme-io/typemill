// Swift Language Plugin for TypeMill

pub mod import_support;
pub mod project_factory;

use async_trait::async_trait;
use mill_lang_common::{
    define_language_plugin, impl_capability_delegations, impl_language_plugin_basics,
};
use mill_plugin_api::{LanguagePlugin, PluginResult, ParsedSource, ManifestData};
use std::path::Path;

define_language_plugin! {
    struct: SwiftPlugin,
    name: "swift",
    extensions: ["swift"],
    manifest: "Package.swift",
    lsp_command: "sourcekit-lsp",
    lsp_args: [""],
    source_dir: "Sources",
    entry_point: "main.swift",
    module_separator: ".",
    capabilities: [with_imports, with_project_factory],
    fields: {
        import_support: import_support::SwiftImportSupport,
        project_factory: project_factory::SwiftProjectFactory,
    },
    doc: "Swift language plugin implementation"
}

#[async_trait]
impl LanguagePlugin for SwiftPlugin {
    impl_language_plugin_basics!();

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        let re = regex::Regex::new(r"(?m)^\s*(func|class|struct|enum|protocol|extension)\s+([a-zA-Z0-9_]+)").unwrap();
        let symbols = re
            .captures_iter(source)
            .map(|cap| {
                let kind_str = &cap[1];
                let name = &cap[2];
                let kind = match kind_str {
                    "func" => mill_plugin_api::SymbolKind::Function,
                    "class" => mill_plugin_api::SymbolKind::Class,
                    "struct" => mill_plugin_api::SymbolKind::Struct,
                    "enum" => mill_plugin_api::SymbolKind::Enum,
                    "protocol" => mill_plugin_api::SymbolKind::Interface,
                    "extension" => mill_plugin_api::SymbolKind::Module,
                    _ => mill_plugin_api::SymbolKind::Function,
                };
                let start = cap.get(0).unwrap().start();
                let line = source[..start].lines().count();
                let column = source[..start].lines().last().map_or(0, |l| l.len());

                mill_plugin_api::Symbol {
                    name: name.to_string(),
                    kind,
                    location: mill_plugin_api::SourceLocation {
                        line,
                        column,
                    },
                    documentation: None,
                }
            })
            .collect();
        Ok(ParsedSource {
            data: serde_json::Value::Null,
            symbols,
        })
    }

    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| mill_plugin_api::PluginError::internal(e.to_string()))?;
        let name_re = regex::Regex::new(r#"name:\s*"([^"]+)""#).unwrap();
        let version_re = regex::Regex::new(r#"swift-tools-version:([0-9.]+)"#).unwrap();
        let dep_re = regex::Regex::new(r#"\.package\(\s*name:\s*"([^"]+)"[^)]+\)"#).unwrap();

        let name = name_re
            .captures(&content)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        let version = version_re
            .captures(&content)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        let dependencies = dep_re
            .captures_iter(&content)
            .map(|caps| mill_plugin_api::Dependency {
                name: caps[1].to_string(),
                source: mill_plugin_api::DependencySource::Version("".to_string()),
            })
            .collect();

        Ok(ManifestData {
            name,
            version,
            dependencies,
            dev_dependencies: vec![],
            raw_data: serde_json::Value::Null,
        })
    }

    impl_capability_delegations! {
        import_support => {
            import_parser: ImportParser,
            import_rename_support: ImportRenameSupport,
            import_move_support: ImportMoveSupport,
            import_mutation_support: ImportMutationSupport,
            import_advanced_support: ImportAdvancedSupport,
        },
        project_factory => {
            project_factory: ProjectFactory,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_swift_plugin_basic() {
        let plugin = SwiftPlugin::new();
        assert_eq!(plugin.metadata().name, "swift");
        assert_eq!(plugin.metadata().extensions, &["swift"]);
        assert!(plugin.handles_extension("swift"));
        assert!(!plugin.handles_extension("rs"));
    }

    use mill_plugin_api::{ImportParser, ProjectFactory};

    #[tokio::test]
    async fn test_parse_imports() {
        let plugin = SwiftPlugin::new();
        let swift_plugin = plugin.as_any().downcast_ref::<SwiftPlugin>().unwrap();
        let source = r#"
import Foundation
import UIKit
"#;
        let imports = swift_plugin.import_support.parse_imports(source);
        assert_eq!(imports, vec!["Foundation", "UIKit"]);
    }

    #[tokio::test]
    async fn test_create_package() {
        let plugin = SwiftPlugin::new();
        let swift_plugin = plugin.as_any().downcast_ref::<SwiftPlugin>().unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().to_path_buf();
        let config = mill_plugin_api::CreatePackageConfig {
            package_path: path.to_str().unwrap().to_string(),
            package_type: mill_plugin_api::PackageType::Library,
            template: mill_plugin_api::Template::Minimal,
            add_to_workspace: false,
            workspace_root: "".to_string(),
        };
        swift_plugin
            .project_factory
            .create_package(&config)
            .unwrap();

        assert!(path.join("Package.swift").exists());
        assert!(path.join("Sources").exists());
        assert!(path.join("Tests").exists());
    }
}