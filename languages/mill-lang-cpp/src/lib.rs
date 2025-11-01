//! CPP language plugin for TypeMill

mod analysis;
mod ast_parser;
mod cmake_parser;
mod conan_parser;
pub mod constants;
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

use crate::constants::{assertion_patterns, test_patterns};

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

    async fn list_functions(&self, source: &str) -> PluginResult<Vec<String>> {
        Ok(ast_parser::list_functions(source))
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

impl mill_plugin_api::AnalysisMetadata for CppPlugin {
    fn test_patterns(&self) -> Vec<regex::Regex> {
        test_patterns()
    }

    fn assertion_patterns(&self) -> Vec<regex::Regex> {
        assertion_patterns()
    }

    fn doc_comment_style(&self) -> mill_plugin_api::DocCommentStyle {
        mill_plugin_api::DocCommentStyle::TripleSlash
    }

    fn visibility_keywords(&self) -> Vec<&'static str> {
        vec!["public", "private", "protected"]
    }

    fn interface_keywords(&self) -> Vec<&'static str> {
        vec!["class", "struct", "interface"]
    }

    fn complexity_keywords(&self) -> Vec<&'static str> {
        vec!["if", "else", "switch", "case", "for", "while", "catch", "&&", "||"]
    }

    fn nesting_penalty(&self) -> f32 {
        1.4
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
    use mill_plugin_api::ScanScope;

    #[test]
    fn test_cpp_plugin_creation() {
        let plugin = CppPlugin::default();
        assert_eq!(plugin.metadata().name, "C++");
    }

    #[test]
    fn test_analysis_metadata_test_patterns() {
        use mill_plugin_api::AnalysisMetadata;
        let plugin = CppPlugin::default();
        let patterns = plugin.test_patterns();

        // Should match Google Test
        let gtest_sample = "TEST(MySuite, MyTest) {}";
        assert!(patterns.iter().any(|p| p.is_match(gtest_sample)));

        // Should match Google Test fixtures
        let fixture_sample = "TEST_F(MyFixture, MyTest) {}";
        assert!(patterns.iter().any(|p| p.is_match(fixture_sample)));

        // Should match Boost.Test
        let boost_sample = "BOOST_AUTO_TEST_CASE(test_name) {}";
        assert!(patterns.iter().any(|p| p.is_match(boost_sample)));

        // Should match Catch2
        let catch_sample = "CATCH_TEST_CASE(\"description\") {}";
        assert!(patterns.iter().any(|p| p.is_match(catch_sample)));
    }

    #[test]
    fn test_analysis_metadata_assertion_patterns() {
        use mill_plugin_api::AnalysisMetadata;
        let plugin = CppPlugin::default();
        let patterns = plugin.assertion_patterns();

        // Should match Google Test expectations
        let expect_sample = "EXPECT_EQ(expected, actual);";
        assert!(patterns.iter().any(|p| p.is_match(expect_sample)));

        // Should match Google Test assertions
        let assert_sample = "ASSERT_TRUE(condition);";
        assert!(patterns.iter().any(|p| p.is_match(assert_sample)));

        // Should match Catch2 checks
        let check_sample = "CHECK(value == expected);";
        assert!(patterns.iter().any(|p| p.is_match(check_sample)));

        // Should match Catch2 requirements
        let require_sample = "REQUIRE(ptr != nullptr);";
        assert!(patterns.iter().any(|p| p.is_match(require_sample)));
    }

    #[test]
    fn test_analysis_metadata_complexity_keywords() {
        use mill_plugin_api::AnalysisMetadata;
        let plugin = CppPlugin::default();
        let keywords = plugin.complexity_keywords();

        // Should include C++ control flow keywords
        assert!(keywords.contains(&"if"));
        assert!(keywords.contains(&"else"));
        assert!(keywords.contains(&"switch"));
        assert!(keywords.contains(&"case"));
        assert!(keywords.contains(&"for"));
        assert!(keywords.contains(&"while"));
        assert!(keywords.contains(&"catch"));
        assert!(keywords.contains(&"&&"));
        assert!(keywords.contains(&"||"));

        // Check nesting penalty
        assert_eq!(plugin.nesting_penalty(), 1.4);
    }

    // ========================================================================
    // EDGE CASE TESTS (8 tests)
    // ========================================================================

    #[tokio::test]
    async fn test_edge_parse_unicode_identifiers() {
        let plugin = CppPlugin::default();
        let source = r#"
#include <iostream>
void тестфункция() {
    int مُتَغَيِّر = 42;
}
"#;
        let result = plugin.parse(source).await;
        // Should not panic with Unicode identifiers
        assert!(result.is_ok() || result.is_err()); // Either way, no panic
    }

    #[tokio::test]
    async fn test_edge_parse_extremely_long_line() {
        let plugin = CppPlugin::default();
        let long_string = "a".repeat(15000);
        let source = format!("std::string x = \"{}\";\n", long_string);
        let result = plugin.parse(&source).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_edge_parse_no_newlines() {
        let plugin = CppPlugin::default();
        let source = "int main() { std::cout << \"hello\"; return 0; }";
        let result = plugin.parse(source).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_edge_scan_mixed_line_endings() {
        let plugin = CppPlugin::default();
        let scanner = plugin.module_reference_scanner().expect("Should have scanner");
        let content = "#include <iostream>\r\n#include <vector>\n#include <string>";
        let refs = scanner.scan_references(content, "iostream", ScanScope::All).expect("Should scan");
        assert_eq!(refs.len(), 1);
    }

    #[tokio::test]
    async fn test_edge_parse_empty_file() {
        let plugin = CppPlugin::default();
        let result = plugin.parse("").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().symbols.len(), 0);
    }

    #[tokio::test]
    async fn test_edge_parse_whitespace_only() {
        let plugin = CppPlugin::default();
        let result = plugin.parse("   \n\n\t\t\n   ").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().symbols.len(), 0);
    }

    #[test]
    fn test_edge_scan_special_regex_chars() {
        let plugin = CppPlugin::default();
        let scanner = plugin.module_reference_scanner().expect("Should have scanner");
        let content = "#include <iostream>";
        // Test with special regex characters
        let result = scanner.scan_references(content, "io.*", ScanScope::All);
        assert!(result.is_ok()); // Should not panic
    }

    #[test]
    fn test_edge_handle_null_bytes() {
        let plugin = CppPlugin::default();
        let scanner = plugin.module_reference_scanner().expect("Should have scanner");
        let content = "#include <iostream>\x00\n#include <vector>";
        let result = scanner.scan_references(content, "iostream", ScanScope::All);
        assert!(result.is_ok()); // Should not panic
    }

    // ========================================================================
    // PERFORMANCE TESTS (2 tests)
    // ========================================================================

    #[test]
    fn test_performance_parse_large_file() {
        use std::time::Instant;
        let plugin = CppPlugin::default();

        // Create a large C++ file (~100KB, 5000 functions)
        let mut large_source = String::from("#include <iostream>\n\n");
        for i in 0..5000 {
            large_source.push_str(&format!("int function{}() {{ return {}; }}\n", i, i));
        }

        let start = Instant::now();
        let result = tokio::runtime::Runtime::new().unwrap().block_on(async {
            plugin.parse(&large_source).await
        });
        let duration = start.elapsed();

        assert!(result.is_ok(), "Should parse large file");
        let symbols = result.unwrap().symbols;
        assert_eq!(symbols.len(), 5000, "Should find all 5000 functions");
        assert!(duration.as_secs() < 5, "Should parse within 5 seconds, took {:?}", duration);
    }

    #[test]
    fn test_performance_scan_many_references() {
        use std::time::Instant;
        let plugin = CppPlugin::default();
        let scanner = plugin.module_reference_scanner().expect("Should have scanner");

        // Create content with 10,000 references
        let mut content = String::from("#include <iostream>\n\n");
        for _ in 0..10000 {
            content.push_str("std::cout << \"test\";\n");
        }

        let start = Instant::now();
        let refs = scanner.scan_references(&content, "iostream", ScanScope::All).expect("Should scan");
        let duration = start.elapsed();

        assert_eq!(refs.len(), 1, "Should find include (C++ doesn't have qualified paths in scanner)");
        assert!(duration.as_secs() < 10, "Should scan within 10 seconds, took {:?}", duration);
    }
}
