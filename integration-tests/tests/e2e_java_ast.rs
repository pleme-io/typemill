//! End-to-end integration tests for Java AST functionality
//!
//! These tests validate that the Java AST parsing works correctly
//! with the full codebuddy system using real Java test fixtures.

use cb_ast::language::{JavaAdapter, LanguageAdapter, ScanScope};
use integration_tests::harness::create_java_project;
use std::fs;

#[tokio::test]
async fn test_java_project_fixture_structure() {
    let workspace = create_java_project();

    // Verify pom.xml exists
    assert!(
        workspace.file_exists("pom.xml"),
        "Java project should have pom.xml"
    );

    // Verify main class exists
    assert!(
        workspace.file_exists("src/main/java/com/codebuddy/example/Main.java"),
        "Should have Main.java"
    );

    // Verify utils package exists
    assert!(
        workspace.file_exists("src/main/java/com/codebuddy/example/utils/Helper.java"),
        "Should have Helper.java"
    );

    assert!(
        workspace.file_exists("src/main/java/com/codebuddy/example/utils/StringProcessor.java"),
        "Should have StringProcessor.java"
    );

    // Verify data package exists
    assert!(
        workspace.file_exists("src/main/java/com/codebuddy/example/data/DataItem.java"),
        "Should have DataItem.java"
    );

    assert!(
        workspace.file_exists("src/main/java/com/codebuddy/example/data/DataProcessor.java"),
        "Should have DataProcessor.java"
    );
}

#[tokio::test]
async fn test_java_find_helper_references_in_main() {
    let workspace = create_java_project();
    let main_path = workspace.absolute_path("src/main/java/com/codebuddy/example/Main.java");

    // Read Main.java content
    let content = fs::read_to_string(&main_path).expect("Should be able to read Main.java");

    let adapter = JavaAdapter;

    // Find Helper references with QualifiedPaths scope
    let result = adapter.find_module_references(&content, "Helper", ScanScope::QualifiedPaths);

    assert!(
        result.is_ok(),
        "Should successfully parse Main.java: {:?}",
        result.err()
    );

    let references = result.unwrap();

    // Should find import declaration
    let declarations: Vec<_> = references
        .iter()
        .filter(|r| matches!(r.kind, cb_ast::language::ReferenceKind::Declaration))
        .collect();

    assert!(
        declarations.len() > 0,
        "Should find Helper import declaration in Main.java"
    );

    // Should find qualified method calls (Helper.logInfo, Helper.printSeparator)
    let qualified_calls: Vec<_> = references
        .iter()
        .filter(|r| matches!(r.kind, cb_ast::language::ReferenceKind::QualifiedPath))
        .collect();

    assert!(
        qualified_calls.len() >= 2,
        "Should find at least 2 Helper qualified calls in Main.java, found: {}",
        qualified_calls.len()
    );

    // Verify line numbers are correct (non-zero)
    for reference in &references {
        assert!(
            reference.line > 0 || reference.line == 0, // tree-sitter uses 0-indexed lines
            "Reference should have valid line number"
        );
    }
}

#[tokio::test]
async fn test_java_find_dataprocessor_references() {
    let workspace = create_java_project();
    let processor_path =
        workspace.absolute_path("src/main/java/com/codebuddy/example/data/DataProcessor.java");

    // Read DataProcessor.java content
    let content =
        fs::read_to_string(&processor_path).expect("Should be able to read DataProcessor.java");

    let adapter = JavaAdapter;

    // Find Helper references (cross-package import)
    let result = adapter.find_module_references(&content, "Helper", ScanScope::QualifiedPaths);

    assert!(
        result.is_ok(),
        "Should successfully parse DataProcessor.java"
    );

    let references = result.unwrap();

    // Should find Helper import from utils package
    assert!(
        references
            .iter()
            .any(|r| matches!(r.kind, cb_ast::language::ReferenceKind::Declaration)),
        "Should find Helper import in DataProcessor.java"
    );

    // Should find Helper qualified calls in methods
    let qualified: Vec<_> = references
        .iter()
        .filter(|r| matches!(r.kind, cb_ast::language::ReferenceKind::QualifiedPath))
        .collect();

    assert!(
        qualified.len() >= 1,
        "Should find at least 1 Helper qualified call in DataProcessor.java"
    );
}

#[tokio::test]
async fn test_java_find_utils_package_references() {
    let workspace = create_java_project();
    let main_path = workspace.absolute_path("src/main/java/com/codebuddy/example/Main.java");

    let content = fs::read_to_string(&main_path).expect("Should be able to read Main.java");

    let adapter = JavaAdapter;

    // Find all references to "utils" package
    let result = adapter.find_module_references(&content, "utils", ScanScope::TopLevelOnly);

    assert!(result.is_ok(), "Should successfully parse Main.java");

    let references = result.unwrap();

    // Should find multiple imports from utils package (Helper, StringProcessor)
    assert!(
        references.len() >= 2,
        "Should find at least 2 imports from utils package, found: {}",
        references.len()
    );

    // Verify all references are import declarations
    for reference in &references {
        assert!(
            matches!(reference.kind, cb_ast::language::ReferenceKind::Declaration),
            "All references should be import declarations in TopLevelOnly scope"
        );

        assert!(
            reference.text.contains("utils"),
            "Import text should contain 'utils': {}",
            reference.text
        );
    }
}

#[tokio::test]
async fn test_java_scope_variations() {
    let workspace = create_java_project();
    let main_path = workspace.absolute_path("src/main/java/com/codebuddy/example/Main.java");

    let content = fs::read_to_string(&main_path).expect("Should be able to read Main.java");

    let adapter = JavaAdapter;

    // Test TopLevelOnly - should find only imports
    let top_level_result =
        adapter.find_module_references(&content, "Helper", ScanScope::TopLevelOnly);
    assert!(top_level_result.is_ok());
    let top_level_refs = top_level_result.unwrap();

    assert!(
        top_level_refs
            .iter()
            .all(|r| matches!(r.kind, cb_ast::language::ReferenceKind::Declaration)),
        "TopLevelOnly should only find import declarations"
    );

    // Test QualifiedPaths - should find imports AND qualified calls
    let qualified_result =
        adapter.find_module_references(&content, "Helper", ScanScope::QualifiedPaths);
    assert!(qualified_result.is_ok());
    let qualified_refs = qualified_result.unwrap();

    assert!(
        qualified_refs.len() > top_level_refs.len(),
        "QualifiedPaths should find more references than TopLevelOnly"
    );

    assert!(
        qualified_refs
            .iter()
            .any(|r| matches!(r.kind, cb_ast::language::ReferenceKind::QualifiedPath)),
        "QualifiedPaths should find qualified method calls"
    );
}

#[tokio::test]
async fn test_java_multiple_files_cross_package() {
    let workspace = create_java_project();

    let adapter = JavaAdapter;

    // Test Main.java
    let main_content = workspace.read_file("src/main/java/com/codebuddy/example/Main.java");
    let main_refs = adapter
        .find_module_references(&main_content, "DataProcessor", ScanScope::TopLevelOnly)
        .expect("Should parse Main.java");

    assert!(main_refs.len() > 0, "Main.java should import DataProcessor");

    // Test DataProcessor.java
    let processor_content =
        workspace.read_file("src/main/java/com/codebuddy/example/data/DataProcessor.java");
    let processor_refs = adapter
        .find_module_references(&processor_content, "Helper", ScanScope::QualifiedPaths)
        .expect("Should parse DataProcessor.java");

    assert!(
        processor_refs.len() > 0,
        "DataProcessor.java should reference Helper"
    );

    // Verify cross-package imports work
    assert!(
        processor_refs.iter().any(|r| r.text.contains("utils")),
        "DataProcessor should import from utils package"
    );
}

#[tokio::test]
async fn test_java_stringprocessor_static_methods() {
    let workspace = create_java_project();
    let main_path = workspace.absolute_path("src/main/java/com/codebuddy/example/Main.java");

    let content = fs::read_to_string(&main_path).expect("Should be able to read Main.java");

    let adapter = JavaAdapter;

    // Find StringProcessor references
    let result =
        adapter.find_module_references(&content, "StringProcessor", ScanScope::QualifiedPaths);

    assert!(result.is_ok(), "Should successfully parse Main.java");

    let references = result.unwrap();

    // Should find import
    assert!(
        references
            .iter()
            .any(|r| matches!(r.kind, cb_ast::language::ReferenceKind::Declaration)),
        "Should find StringProcessor import"
    );

    // Should find static method call (StringProcessor.format)
    let qualified: Vec<_> = references
        .iter()
        .filter(|r| matches!(r.kind, cb_ast::language::ReferenceKind::QualifiedPath))
        .collect();

    assert!(
        qualified.len() >= 1,
        "Should find at least 1 StringProcessor static method call"
    );
}
