mod ast_parser_test;
mod cmake_parser_test;
mod import_analyzer_test;
mod import_support_test;
mod makefile_parser_test;
mod manifest_updater_test;
mod module_reference_scanner_test;
mod project_factory_test;
mod refactoring_test;
mod workspace_support_test;

// Analysis metadata tests
#[cfg(test)]
mod analysis_metadata_tests {
    use crate::CPlugin;
    use mill_plugin_api::{AnalysisMetadata, LanguagePlugin, ScanScope};

    #[test]
    fn test_analysis_metadata_test_patterns() {
        let plugin = CPlugin::default();
        let patterns = plugin.test_patterns();

        // Should match CUnit/Unity style test functions
        let test_sample = "void test_something() {}";
        assert!(patterns.iter().any(|p| p.is_match(test_sample)));

        // Should match Google Test macros (if used with C)
        let gtest_sample = "TEST(Suite, TestName) {}";
        assert!(patterns.iter().any(|p| p.is_match(gtest_sample)));
    }

    #[test]
    fn test_analysis_metadata_assertion_patterns() {
        let plugin = CPlugin::default();
        let patterns = plugin.assertion_patterns();

        // Should match standard C assert
        let assert_sample = "assert(x == 5);";
        assert!(patterns.iter().any(|p| p.is_match(assert_sample)));

        // Should match CUnit assertions
        let cunit_sample = "CU_ASSERT_EQUAL(expected, actual);";
        assert!(patterns.iter().any(|p| p.is_match(cunit_sample)));

        // Should match Unity assertions
        let unity_sample = "TEST_ASSERT_TRUE(condition);";
        assert!(patterns.iter().any(|p| p.is_match(unity_sample)));
    }

    #[test]
    fn test_analysis_metadata_complexity_keywords() {
        let plugin = CPlugin::default();
        let keywords = plugin.complexity_keywords();

        // Should include C control flow keywords
        assert!(keywords.contains(&"if"));
        assert!(keywords.contains(&"else"));
        assert!(keywords.contains(&"switch"));
        assert!(keywords.contains(&"case"));
        assert!(keywords.contains(&"for"));
        assert!(keywords.contains(&"while"));
        assert!(keywords.contains(&"do"));
        assert!(keywords.contains(&"&&"));
        assert!(keywords.contains(&"||"));

        // Check nesting penalty
        assert_eq!(plugin.nesting_penalty(), 1.3);
    }

    // ========================================================================
    // EDGE CASE TESTS (8 tests)
    // ========================================================================

    #[tokio::test]
    async fn test_edge_parse_unicode_identifiers() {
        let plugin = CPlugin::default();
        let source = r#"
#include <stdio.h>
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
        let plugin = CPlugin::default();
        let long_string = "a".repeat(15000);
        let source = format!("char* x = \"{}\";\n", long_string);
        let result = plugin.parse(&source).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_edge_parse_no_newlines() {
        let plugin = CPlugin::default();
        let source = "int main() { printf(\"hello\"); return 0; }";
        let result = plugin.parse(source).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_edge_scan_mixed_line_endings() {
        let plugin = CPlugin::default();
        let scanner = plugin
            .module_reference_scanner()
            .expect("Should have scanner");
        let content = "#include <stdio.h>\r\n#include <stdlib.h>\n#include <string.h>";
        let refs = scanner
            .scan_references(content, "stdio", ScanScope::All)
            .expect("Should scan");

        // Debug output
        eprintln!("\n=== DEBUG: Mixed line endings test ===");
        eprintln!("Content bytes: {:?}", content.as_bytes());
        eprintln!("Number of references found: {}", refs.len());
        for (i, r) in refs.iter().enumerate() {
            eprintln!(
                "  Ref {}: text={:?}, line={}, column={}, length={}",
                i, r.text, r.line, r.column, r.length
            );
        }
        eprintln!("=== END DEBUG ===\n");

        assert_eq!(refs.len(), 1);
    }

    #[tokio::test]
    async fn test_edge_parse_empty_file() {
        let plugin = CPlugin::default();
        let result = plugin.parse("").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().symbols.len(), 0);
    }

    #[tokio::test]
    async fn test_edge_parse_whitespace_only() {
        let plugin = CPlugin::default();
        let result = plugin.parse("   \n\n\t\t\n   ").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().symbols.len(), 0);
    }

    #[test]
    fn test_edge_scan_special_regex_chars() {
        let plugin = CPlugin::default();
        let scanner = plugin
            .module_reference_scanner()
            .expect("Should have scanner");
        let content = "#include <stdio.h>";
        // Test with special regex characters
        let result = scanner.scan_references(content, "std.*", ScanScope::All);
        assert!(result.is_ok()); // Should not panic
    }

    #[test]
    fn test_edge_handle_null_bytes() {
        let plugin = CPlugin::default();
        let scanner = plugin
            .module_reference_scanner()
            .expect("Should have scanner");
        let content = "#include <stdio.h>\x00\n#include <stdlib.h>";
        let result = scanner.scan_references(content, "stdio", ScanScope::All);
        assert!(result.is_ok()); // Should not panic
    }

    // ========================================================================
    // PERFORMANCE TESTS (2 tests)
    // ========================================================================

    #[test]
    fn test_performance_parse_large_file() {
        use std::time::Instant;
        let plugin = CPlugin::default();

        // Create a large C file (~100KB, 5000 functions)
        let mut large_source = String::from("#include <stdio.h>\n\n");
        for i in 0..5000 {
            large_source.push_str(&format!("int function{}() {{ return {}; }}\n", i, i));
        }

        let start = Instant::now();
        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { plugin.parse(&large_source).await });
        let duration = start.elapsed();

        assert!(result.is_ok(), "Should parse large file");
        let symbols = result.unwrap().symbols;
        assert_eq!(symbols.len(), 5000, "Should find all 5000 functions");
        assert!(
            duration.as_secs() < 5,
            "Should parse within 5 seconds, took {:?}",
            duration
        );
    }

    #[test]
    fn test_performance_scan_many_references() {
        use std::time::Instant;
        let plugin = CPlugin::default();
        let scanner = plugin
            .module_reference_scanner()
            .expect("Should have scanner");

        // Create content with 10,000 references
        let mut content = String::from("#include <stdio.h>\n\n");
        for _ in 0..10000 {
            content.push_str("printf(\"test\");\n");
        }

        let start = Instant::now();
        let refs = scanner
            .scan_references(&content, "stdio", ScanScope::All)
            .expect("Should scan");
        let duration = start.elapsed();

        assert_eq!(
            refs.len(),
            1,
            "Should find include (C doesn't have qualified paths like other languages)"
        );
        assert!(
            duration.as_secs() < 10,
            "Should scan within 10 seconds, took {:?}",
            duration
        );
    }
}
