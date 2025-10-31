// Swift Language Plugin for TypeMill

pub mod import_support;
pub mod lsp_installer;
pub mod project_factory;
pub mod refactoring;
pub mod workspace_support;

use async_trait::async_trait;
use mill_lang_common::{
    define_language_plugin, impl_capability_delegations, impl_language_plugin_basics,
};
use mill_plugin_api::{
    ImportAnalyzer, LanguagePlugin, ManifestData, ManifestUpdater, ModuleReference,
    ModuleReferenceScanner, ParsedSource, PluginError, PluginResult, RefactoringProvider,
    ScanScope,
};
use std::path::Path;
use lazy_static::lazy_static;

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
    capabilities: [with_imports, with_project_factory, with_workspace],
    fields: {
        import_support: import_support::SwiftImportSupport,
        project_factory: project_factory::SwiftProjectFactory,
        workspace_support: workspace_support::SwiftWorkspaceSupport,
        lsp_installer: lsp_installer::SwiftLspInstaller,
    },
    doc: "Swift language plugin implementation"
}

lazy_static! {
    static ref SYMBOL_REGEX: Regex =
        Regex::new(r"(?m)^\s*(func|class|struct|enum|protocol|extension)\s+([a-zA-Z0-9_]+)")
            .expect("Invalid regex for Swift symbol parsing");
    static ref MANIFEST_NAME_REGEX: Regex =
        Regex::new(r#"name:\s*"([^"]+)""#).expect("Invalid regex for Swift manifest name");
    static ref MANIFEST_VERSION_REGEX: Regex =
        Regex::new(r#"swift-tools-version:([0-9.]+)"#)
            .expect("Invalid regex for Swift manifest version");
    static ref MANIFEST_DEP_REGEX: Regex =
        Regex::new(r#"\.package\(\s*name:\s*"([^"]+)"[^)]+\)"#)
            .expect("Invalid regex for Swift manifest dependency");
    static ref IMPORT_REGEX: Regex =
        Regex::new(r"^\s*import\s+([a-zA-Z0-9_]+)").expect("Invalid regex for Swift import parsing");
}

#[async_trait]
impl LanguagePlugin for SwiftPlugin {
    impl_language_plugin_basics!();

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        let symbols = SYMBOL_REGEX
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
                let start = cap.get(0).map_or(0, |m| m.start());
                let line = source[..start].lines().count();
                let column = source[..start].lines().last().map_or(0, |l| l.len());

                mill_plugin_api::Symbol {
                    name: name.to_string(),
                    kind,
                    location: mill_plugin_api::SourceLocation { line, column },
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

        let name = MANIFEST_NAME_REGEX
            .captures(&content)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        let version = MANIFEST_VERSION_REGEX
            .captures(&content)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        let dependencies = MANIFEST_DEP_REGEX
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
        this => {
            refactoring_provider: RefactoringProvider,
            module_reference_scanner: ModuleReferenceScanner,
            import_analyzer: ImportAnalyzer,
            manifest_updater: ManifestUpdater,
        },
        import_support => {
            import_parser: ImportParser,
            import_rename_support: ImportRenameSupport,
            import_move_support: ImportMoveSupport,
            import_mutation_support: ImportMutationSupport,
            import_advanced_support: ImportAdvancedSupport,
        },
        project_factory => {
            project_factory: ProjectFactory,
        },
        workspace_support => {
            workspace_support: WorkspaceSupport,
        },
        lsp_installer => {
            lsp_installer: LspInstaller,
        }
    }
}

#[async_trait]
impl RefactoringProvider for SwiftPlugin {
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

use regex::Regex;

impl ModuleReferenceScanner for SwiftPlugin {
    fn scan_references(
        &self,
        content: &str,
        module_name: &str,
        scope: ScanScope,
    ) -> PluginResult<Vec<ModuleReference>> {
        let mut references = Vec::new();
        let import_pattern = format!(r"\bimport\s+{}\b", module_name);
        let import_re = Regex::new(&import_pattern)
            .map_err(|e| PluginError::internal(format!("Invalid regex: {}", e)))?;
        let qualified_pattern = format!(r"{}\.", module_name);
        let qualified_re = Regex::new(&qualified_pattern)
            .map_err(|e| PluginError::internal(format!("Invalid regex: {}", e)))?;

        for (line_idx, line) in content.lines().enumerate() {
            let line_num = line_idx + 1;

            if scope == ScanScope::All || scope == ScanScope::TopLevelOnly || scope == ScanScope::AllUseStatements {
                for mat in import_re.find_iter(line) {
                    references.push(ModuleReference {
                        line: line_num,
                        column: mat.start(),
                        length: mat.len(),
                        text: module_name.to_string(),
                        kind: mill_plugin_api::ReferenceKind::Declaration,
                    });
                }
            }

            if scope == ScanScope::All || scope == ScanScope::QualifiedPaths {
                for mat in qualified_re.find_iter(line) {
                    references.push(ModuleReference {
                        line: line_num,
                        column: mat.start(),
                        length: mat.len(),
                        text: module_name.to_string(),
                        kind: mill_plugin_api::ReferenceKind::QualifiedPath,
                    });
                }
            }
        }

        Ok(references)
    }
}

use mill_foundation::protocol::{ImportGraph, ImportGraphMetadata, ImportInfo, ImportType};

impl ImportAnalyzer for SwiftPlugin {
    fn build_import_graph(&self, file_path: &Path) -> PluginResult<ImportGraph> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| mill_plugin_api::PluginError::internal(e.to_string()))?;

        let imports = IMPORT_REGEX
            .captures_iter(&content)
            .map(|cap| {
                let line_number = content[..cap.get(0).map_or(0, |m| m.start())]
                    .lines()
                    .count() as u32;
                ImportInfo {
                    module_path: cap[1].to_string(),
                    import_type: ImportType::CInclude, // Using CInclude as a stand-in for Swift
                    named_imports: vec![],
                    default_import: None,
                    namespace_import: None,
                    type_only: false,
                    location: mill_foundation::protocol::SourceLocation {
                        start_line: line_number,
                        start_column: 0,
                        end_line: line_number,
                        end_column: cap[0].len() as u32,
                    },
                }
            })
            .collect();

        Ok(ImportGraph {
            source_file: file_path.to_string_lossy().into_owned(),
            imports,
            importers: vec![],
            metadata: ImportGraphMetadata {
                language: "swift".to_string(),
                parsed_at: chrono::Utc::now(),
                parser_version: "0.1.0".to_string(),
                circular_dependencies: vec![],
                external_dependencies: vec![],
            },
        })
    }
}

#[async_trait]
impl ManifestUpdater for SwiftPlugin {
    async fn update_dependency(
        &self,
        manifest_path: &Path,
        old_name: &str,
        new_name: &str,
        new_version: Option<&str>,
    ) -> PluginResult<String> {
        let content = std::fs::read_to_string(manifest_path)
            .map_err(|e| mill_plugin_api::PluginError::internal(e.to_string()))?;

        let pattern = format!(r#"(\.package\(\s*name:\s*"{}"[^)]*\))"#, old_name);
        let re = Regex::new(&pattern)
            .map_err(|e| PluginError::internal(format!("Invalid regex: {}", e)))?;

        if !re.is_match(&content) {
            return Ok(content);
        }

        let new_content = re.replace(&content, |caps: &regex::Captures| {
            let mut new_package_line = format!(".package(name: \"{}\"", new_name);
            if let Some(version) = new_version {
                // This is a simplification; a real implementation would need to handle
                // different versioning styles (.exact, .branch, etc.) and update existing ones.
                new_package_line.push_str(&format!(r#", .upToNextMajor(from: "{}"))"#, version));
            } else {
                 // Try to preserve existing versioning if possible, or just close the call
                if let Some(existing) = caps.get(0) {
                     if !existing.as_str().contains("version") && !existing.as_str().contains("from:") {
                         new_package_line.push(')');
                     } else {
                        // In a real scenario, you'd parse this part. For now, we just copy it.
                        if let Some(match_group) = caps.get(1) {
                            let rest = &existing.as_str()[match_group.end()..];
                            new_package_line.push_str(rest);
                        }
                     }
                } else {
                    new_package_line.push(')');
                }
            }
            new_package_line
        });

        Ok(new_content.to_string())
    }

    fn generate_manifest(&self, package_name: &str, dependencies: &[String]) -> String {
        let deps_str = if dependencies.is_empty() {
            "".to_string()
        } else {
            dependencies
                .iter()
                .map(|dep| format!(r#"        .package(url: "{}", from: "1.0.0")"#, dep))
                .collect::<Vec<_>>()
                .join(",\n")
        };

        format!(
            r#"// swift-tools-version:5.3
import PackageDescription

let package = Package(
    name: "{}",
    products: [
        .library(
            name: "{}",
            targets: ["{}"]),
    ],
    dependencies: [
        {}
    ],
    targets: [
        .target(
            name: "{}",
            dependencies: []),
        .testTarget(
            name: "{}Tests",
            dependencies: ["{}"]),
    ]
)
"#,
            package_name, package_name, package_name, deps_str, package_name, package_name, package_name
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mill_plugin_api::{ImportParser, ProjectFactory};

    #[tokio::test]
    async fn test_swift_plugin_basic() {
        let plugin = SwiftPlugin::new();
        assert_eq!(plugin.metadata().name, "swift");
        assert_eq!(plugin.metadata().extensions, &["swift"]);
        assert!(plugin.handles_extension("swift"));
        assert!(!plugin.handles_extension("rs"));
    }

    #[tokio::test]
    async fn test_parse_imports() {
        let plugin = SwiftPlugin::new();
        let swift_plugin = plugin
            .as_any()
            .downcast_ref::<SwiftPlugin>()
            .expect("Plugin should be SwiftPlugin");
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
        let swift_plugin = plugin
            .as_any()
            .downcast_ref::<SwiftPlugin>()
            .expect("Plugin should be SwiftPlugin");
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let path = temp_dir.path().to_path_buf();
        let config = mill_plugin_api::CreatePackageConfig {
            package_path: path.to_str().expect("Path should be valid UTF-8").to_string(),
            package_type: mill_plugin_api::PackageType::Library,
            template: mill_plugin_api::Template::Minimal,
            add_to_workspace: false,
            workspace_root: "".to_string(),
        };
        swift_plugin
            .project_factory
            .create_package(&config)
            .expect("create_package should succeed");

        assert!(path.join("Package.swift").exists());
        assert!(path.join("Sources").exists());
        assert!(path.join("Tests").exists());
    }

    #[test]
    fn test_workspace_support_add_remove() {
        let plugin = SwiftPlugin::new();
        let support = plugin
            .workspace_support()
            .expect("Plugin should have workspace support");
        let manifest_content = r#"
let package = Package(
    dependencies: [
        .package(url: "https://github.com/apple/swift-argument-parser", from: "1.0.0"),
    ]
)
"#;
        let new_content = support.add_workspace_member(manifest_content, "../MyOtherPackage");
        assert!(new_content.contains(r#".package(path: "../MyOtherPackage")"#));

        let final_content = support.remove_workspace_member(&new_content, "../MyOtherPackage");
        assert!(!final_content.contains(r#".package(path: "../MyOtherPackage")"#));
    }

    #[tokio::test]
    async fn test_refactoring_operations() {
        let plugin = SwiftPlugin::new();
        let provider = plugin
            .refactoring_provider()
            .expect("Plugin should have refactoring provider");

        // Test extract function
        let source = "func myFunc() {\n    print(\"hello\")\n}";
        let result = provider
            .plan_extract_function(source, 1, 1, "newFunc", "test.swift")
            .await;
        assert!(result.is_ok());
        let plan = result.expect("plan_extract_function should succeed");
        assert_eq!(plan.edits.len(), 2);

        // Test inline variable
        let source = "func myFunc() {\n    let x = 10\n    print(x)\n}";
        let result = provider
            .plan_inline_variable(source, 1, 0, "test.swift")
            .await;
        assert!(result.is_ok());
        let plan = result.expect("plan_inline_variable should succeed");
        assert_eq!(plan.edits.len(), 2);

        // Test extract variable
        let source = "func myFunc() {\n    print(10 + 20)\n}";
        let result = provider
            .plan_extract_variable(source, 1, 10, 1, 17, Some("myVar".to_string()), "test.swift")
            .await;
        assert!(result.is_ok());
        let plan = result.expect("plan_extract_variable should succeed");
        assert_eq!(plan.edits.len(), 2);
    }

    #[test]
    fn test_module_reference_scanner() {
        let plugin = SwiftPlugin::new();
        let scanner = plugin
            .module_reference_scanner()
            .expect("Plugin should have module reference scanner");
        let content = "import Foundation\nimport UIKit";
        let refs = scanner
            .scan_references(content, "Foundation", ScanScope::All)
            .expect("scan_references should succeed");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].text, "Foundation");
    }

    #[test]
    fn test_import_analyzer() {
        let plugin = SwiftPlugin::new();
        let analyzer = plugin
            .import_analyzer()
            .expect("Plugin should have import analyzer");
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("MyFile.swift");
        std::fs::write(&file_path, "import Foundation").expect("Failed to write to file");
        let graph = analyzer
            .build_import_graph(&file_path)
            .expect("build_import_graph should succeed");
        assert_eq!(graph.imports.len(), 1);
        assert_eq!(graph.imports[0].module_path, "Foundation");
    }

    #[test]
    fn test_manifest_updater_generate() {
        let plugin = SwiftPlugin::new();
        let updater = plugin
            .manifest_updater()
            .expect("Plugin should have manifest updater");
        let manifest =
            updater.generate_manifest("MyPackage", &["https://github.com/a/b".to_string()]);
        assert!(manifest.contains(r#"name: "MyPackage""#));
        assert!(manifest.contains(r#".package(url: "https://github.com/a/b", from: "1.0.0")"#));
    }

    #[test]
    fn test_lsp_installer_check() {
        let plugin = SwiftPlugin::new();
        let installer = plugin
            .lsp_installer()
            .expect("Plugin should have LSP installer");
        // This test is tricky as it depends on the test environment.
        // We'll just call the function to make sure it doesn't panic.
        let _ = installer.check_installed();
    }
}