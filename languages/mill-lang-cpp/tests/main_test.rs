use mill_foundation::protocol::{DependencyUpdate, DependencyUpdateType};
use mill_lang_cpp::CppPlugin;
use mill_plugin_api::{
    capabilities::{ImportAnalyzer, ModuleReferenceScanner, RefactoringProvider},
    import_support::{
        ImportAdvancedSupport, ImportMoveSupport, ImportMutationSupport, ImportRenameSupport,
    },
    LanguagePlugin, ManifestUpdater, ScanScope, WorkspaceSupport,
};
use std::io::Write;
use std::path::Path;
use tempfile::Builder;

#[test]
fn test_update_import_reference() {
    let plugin = CppPlugin::default();
    let advanced_support = plugin.import_advanced_support().unwrap();
    let source = r#"#include "old/path/to/header.h""#;
    let update = DependencyUpdate {
        target_file: "dummy.cpp".to_string(),
        update_type: DependencyUpdateType::ImportPath,
        old_reference: "old/path/to/header.h".to_string(),
        new_reference: "new/path/to/header.h".to_string(),
    };

    let new_source = advanced_support
        .update_import_reference(Path::new("dummy.cpp"), source, &update)
        .unwrap();

    assert_eq!(new_source, r#"#include "new/path/to/header.h""#);
}

#[test]
fn test_rewrite_imports_for_move() {
    let plugin = CppPlugin::default();
    let move_support = plugin.import_move_support().unwrap();
    let source = r#"#include "./relative/header.h""#;

    let old_path = Path::new("/project/src/old_dir/my_file.cpp");
    let new_path = Path::new("/project/src/new_dir/my_file.cpp");

    let (new_source, changes) = move_support.rewrite_imports_for_move(source, old_path, new_path);

    assert_eq!(changes, 1);
    assert!(new_source.contains("../old_dir/relative/header.h"));
}

#[test]
fn test_rewrite_imports_for_move_to_root() {
    let plugin = CppPlugin::default();
    let move_support = plugin.import_move_support().unwrap();
    let source = r#"#include "../common/header.h""#;

    let old_path = Path::new("/project/src/my_file.cpp");
    let new_path = Path::new("/project/my_file.cpp");

    let (new_source, changes) = move_support.rewrite_imports_for_move(source, old_path, new_path);

    assert_eq!(changes, 1);
    assert!(new_source.contains("common/header.h"));
    assert!(!new_source.contains(".."));
}

#[test]
fn test_rewrite_imports_for_move_sibling_dirs() {
    let plugin = CppPlugin::default();
    let move_support = plugin.import_move_support().unwrap();
    let source = r#"#include "../lib/utils.h""#;

    let old_path = Path::new("/project/src/app/main.cpp");
    let new_path = Path::new("/project/src/cli/main.cpp");

    let (new_source, changes) = move_support.rewrite_imports_for_move(source, old_path, new_path);

    assert_eq!(changes, 1);
    assert!(new_source.contains("../lib/utils.h"));
}

#[test]
fn test_rewrite_imports_for_rename() {
    let plugin = CppPlugin::default();
    let rename_support = plugin.import_rename_support().unwrap();
    let source = r#"#include "old/path/to/header.h""#;
    let (new_source, changes) = rename_support.rewrite_imports_for_rename(
        source,
        "old/path/to/header.h",
        "new/path/to/header.h",
    );
    assert_eq!(changes, 1);
    assert_eq!(new_source, r#"#include "new/path/to/header.h""#);
}

mod import_mutation_tests {
    use super::*;

    #[test]
    fn test_add_import_to_empty_file() {
        let plugin = CppPlugin::default();
        let mutation_support = plugin.import_mutation_support().unwrap();
        let source = "";
        let new_source = mutation_support.add_import(source, "new_header.h");
        assert_eq!(new_source, "#include \"new_header.h\"");
    }

    #[test]
    fn test_add_import_to_existing_imports() {
        let plugin = CppPlugin::default();
        let mutation_support = plugin.import_mutation_support().unwrap();
        let source = "#include <iostream>\n#include \"my_header.h\"";
        let new_source = mutation_support.add_import(source, "another.h");
        let expected = "#include <iostream>\n#include \"my_header.h\"\n#include \"another.h\"";
        assert_eq!(new_source.trim(), expected.trim());
    }

    #[test]
    fn test_add_duplicate_import() {
        let plugin = CppPlugin::default();
        let mutation_support = plugin.import_mutation_support().unwrap();
        let source = "#include <iostream>";
        let new_source = mutation_support.add_import(source, "iostream");
        assert_eq!(new_source, source);
    }

    #[test]
    fn test_remove_import() {
        let plugin = CppPlugin::default();
        let mutation_support = plugin.import_mutation_support().unwrap();
        let source = "#include <iostream>\n#include \"my_header.h\"";
        let new_source = mutation_support.remove_import(source, "my_header.h");
        assert_eq!(new_source.trim(), "#include <iostream>");
    }

    #[test]
    fn test_remove_nonexistent_import() {
        let plugin = CppPlugin::default();
        let mutation_support = plugin.import_mutation_support().unwrap();
        let source = "#include <iostream>";
        let new_source = mutation_support.remove_import(source, "nonexistent.h");
        assert_eq!(new_source, source);
    }

    #[test]
    fn test_remove_import_removes_line() {
        let plugin = CppPlugin::default();
        let mutation_support = plugin.import_mutation_support().unwrap();
        let source = "#include <iostream>\n#include \"my_header.h\"\n#include <vector>";
        let new_source = mutation_support.remove_import(source, "my_header.h");
        let expected = "#include <iostream>\n#include <vector>";
        assert_eq!(new_source.trim(), expected.trim());
        assert_eq!(new_source.lines().count(), 2);
    }
}

#[test]
fn test_parse_imports() {
    let plugin = CppPlugin::default();
    let import_parser = plugin.import_parser().unwrap();

    let source = r#"
#include <iostream>
#include "my_header.h"
"#;

    let imports = import_parser.parse_imports(source);

    assert_eq!(imports.len(), 2);
    assert!(imports.contains(&"iostream".to_string()));
    assert!(imports.contains(&"my_header.h".to_string()));
}

#[tokio::test]
async fn test_parse_symbols() {
    let plugin = CppPlugin::default();
    let source = r#"
namespace MyNamespace {
    class MyClass {
    public:
        void myMethod() {}
    };
}

int main() {
    return 0;
}
"#;
    let parsed_source = plugin.parse(source).await.unwrap();
    let symbols = parsed_source.symbols;

    println!(
        "Found symbols: {:?}",
        symbols.iter().map(|s| &s.name).collect::<Vec<_>>()
    );

    // TODO: Improve symbol parsing to correctly handle nested symbols.
    // The current implementation only finds top-level symbols.
    assert_eq!(
        symbols.len(),
        4,
        "Should find namespace, class, method, and main function"
    );
    let names: Vec<_> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"MyNamespace"));
    assert!(names.contains(&"MyClass"));
    assert!(names.contains(&"myMethod"));
    assert!(names.contains(&"main"));
}

#[tokio::test]
async fn test_analyze_cmake_manifest_advanced() {
    let plugin = CppPlugin::default();
    let content = r#"
        project(MyAwesomeProject)
        add_executable(my_app main.cpp)
        add_library(my_lib my_lib.cpp)
        target_link_libraries(my_app my_lib)
        target_link_libraries(my_app another_lib)
    "#;

    let mut temp_file = Builder::new()
        .prefix("CMakeLists")
        .suffix(".txt")
        .tempfile()
        .unwrap();
    writeln!(temp_file, "{}", content).unwrap();
    let path = temp_file.into_temp_path();

    let manifest_data = plugin.analyze_manifest(&path).await.unwrap();

    assert_eq!(manifest_data.name, "MyAwesomeProject".to_string());
    assert_eq!(manifest_data.dependencies.len(), 2);
    assert!(manifest_data
        .dependencies
        .iter()
        .any(|d| d.name == "my_lib"));
    assert!(manifest_data
        .dependencies
        .iter()
        .any(|d| d.name == "another_lib"));

    let raw_data = manifest_data.raw_data;
    assert_eq!(raw_data["executables"].as_array().unwrap().len(), 1);
    assert_eq!(raw_data["executables"][0], "my_app");
    assert_eq!(raw_data["libraries"].as_array().unwrap().len(), 1);
    assert_eq!(raw_data["libraries"][0], "my_lib");

    let linked_libs = raw_data["linked_libraries"].as_array().unwrap();
    assert_eq!(linked_libs.len(), 2);
    assert_eq!(linked_libs[0]["library"], "my_lib");
    assert_eq!(linked_libs[0]["linkage"], "PRIVATE");
    assert_eq!(linked_libs[1]["library"], "another_lib");
    assert_eq!(linked_libs[1]["linkage"], "PRIVATE");
}

#[tokio::test]
async fn test_analyze_conan_manifest() {
    let plugin = CppPlugin::default();
    let content = r#"
        [requires]
        fmt/10.2.1
        gtest/1.14.0

        [generators]
        CMakeDeps
        CMakeToolchain
    "#;

    let temp_dir = Builder::new().prefix("conan-test").tempdir().unwrap();
    let path = temp_dir.path().join("conanfile.txt");
    let mut temp_file = std::fs::File::create(&path).unwrap();
    writeln!(temp_file, "{}", content).unwrap();

    let manifest_data = plugin.analyze_manifest(&path).await.unwrap();

    assert_eq!(manifest_data.dependencies.len(), 2);
    assert!(manifest_data.dependencies.iter().any(|d| d.name == "fmt"));
    assert!(manifest_data.dependencies.iter().any(|d| d.name == "gtest"));
}

#[tokio::test]
async fn test_manifest_updater() {
    let plugin = CppPlugin::default();
    let updater = plugin.manifest_updater().unwrap();

    let initial_content = "project(TestProject)\nadd_executable(app main.cpp)";

    // Add a dependency
    let updated_content = updater
        .update_dependency(Path::new(""), initial_content, "fmt", Some("10.2.1"))
        .await
        .unwrap();
    assert!(updated_content.contains("target_link_libraries(app PRIVATE fmt)"));

    // Generate a manifest
    let generated_content = updater.generate_manifest("new_project", &[]);
    assert!(generated_content.contains("project(new_project VERSION 1.0)"));
}

#[test]
fn test_workspace_support() {
    let plugin = CppPlugin::default();
    let workspace_support = plugin.workspace_support().unwrap();
    let initial_content = "project(MyProject)\n";

    // Add a workspace member
    let content_after_add = workspace_support.add_workspace_member(initial_content, "my_lib");
    println!("Content after add: {}", content_after_add);
    assert!(content_after_add.contains("add_subdirectory(my_lib)"));

    // Remove a workspace member
    let content_after_remove =
        workspace_support.remove_workspace_member(&content_after_add, "my_lib");
    assert!(!content_after_remove.contains("add_subdirectory(my_lib)"));
}

#[tokio::test]
async fn test_refactoring_extract_function() {
    let plugin = CppPlugin::default();
    let refactoring_provider = plugin.refactoring_provider().unwrap();
    let source = "void foo() {\n  int x = 1;\n  int y = 2;\n}";
    let plan = refactoring_provider
        .plan_extract_function(source, 1, 2, "new_function", "dummy.cpp")
        .await;
    assert!(plan.is_ok());
    assert!(!plan.unwrap().edits.is_empty());
}

#[tokio::test]
async fn test_refactoring_inline_variable() {
    let plugin = CppPlugin::default();
    let refactoring_provider = plugin.refactoring_provider().unwrap();
    let source = "void foo() {\n  int x = 1;\n  int y = x;\n}";
    let plan = refactoring_provider
        .plan_inline_variable(source, 2, 10, "dummy.cpp")
        .await;
    assert!(plan.is_ok());
    assert!(!plan.unwrap().edits.is_empty());
}

#[tokio::test]
async fn test_refactoring_extract_variable() {
    let plugin = CppPlugin::default();
    let refactoring_provider = plugin.refactoring_provider().unwrap();
    let source = "int main() {\n  int x = 1 + 2;\n  return x;\n}";
    // Extract "1 + 2" from line 2, columns 10-15 (1-indexed)
    let plan = refactoring_provider
        .plan_extract_variable(source, 2, 10, 2, 15, Some("y".to_string()), "dummy.cpp")
        .await;
    assert!(plan.is_ok(), "Failed to create extract variable plan");
    let unwrapped_plan = plan.unwrap();
    assert!(!unwrapped_plan.edits.is_empty());
    assert!(unwrapped_plan
        .edits
        .iter()
        .any(|e| e.new_text.contains("auto y = 1 + 2;")));
}

#[test]
fn test_module_reference_scanner() {
    let plugin = CppPlugin::default();
    let scanner = plugin.module_reference_scanner().unwrap();
    let source = "#include <iostream>\n#include \"my_header.h\"";
    let references = scanner
        .scan_references(source, "my_header.h", ScanScope::All)
        .unwrap();
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].text, "my_header.h");
}

#[tokio::test]
async fn test_import_analyzer() {
    let plugin = CppPlugin::default();
    let analyzer = plugin.import_analyzer().unwrap();
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.cpp");
    let source = "#include <iostream>\n#include \"my_header.h\"";
    std::fs::write(&file_path, source).unwrap();
    let graph = analyzer.build_import_graph(&file_path).unwrap();
    assert_eq!(graph.imports.len(), 2);
    assert!(graph.imports.iter().any(|i| i.module_path == "iostream"));
    assert!(graph.imports.iter().any(|i| i.module_path == "my_header.h"));
}

#[tokio::test]
async fn test_analyze_vcpkg_manifest() {
    let plugin = CppPlugin::default();
    let content = r#"
{
    "name": "my-project",
    "version-string": "1.0.0",
    "dependencies": [
        "fmt",
        "gtest"
    ]
}
"#;

    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("vcpkg.json");
    std::fs::write(&path, content).unwrap();

    let manifest_data = plugin.analyze_manifest(&path).await.unwrap();

    assert_eq!(manifest_data.name, "my-project");
    assert_eq!(manifest_data.version, "1.0.0");
    assert_eq!(manifest_data.dependencies.len(), 2);
    assert!(manifest_data.dependencies.iter().any(|d| d.name == "fmt"));
    assert!(manifest_data.dependencies.iter().any(|d| d.name == "gtest"));
}

use mill_plugin_api::project_factory::{PackageType, Template};

#[test]
fn test_project_factory() {
    let plugin = CppPlugin::default();
    let factory = plugin.project_factory().unwrap();
    let temp_dir = Builder::new().prefix("cpp-project-test").tempdir().unwrap();

    let config = mill_plugin_api::project_factory::CreatePackageConfig {
        package_path: "my-cpp-project".to_string(),
        package_type: PackageType::Binary,
        template: Template::Minimal,
        add_to_workspace: false,
        workspace_root: temp_dir.path().to_str().unwrap().to_string(),
    };

    let result = factory.create_package(&config).unwrap();

    let project_path = temp_dir.path().join("my-cpp-project");
    assert!(project_path.exists());
    assert!(project_path.join("src/main.cpp").exists());
    assert!(project_path.join("include").exists());
    assert!(project_path.join("CMakeLists.txt").exists());

    let main_cpp_content = std::fs::read_to_string(project_path.join("src/main.cpp")).unwrap();
    assert!(main_cpp_content.contains("Hello, world!"));

    let cmake_content = std::fs::read_to_string(project_path.join("CMakeLists.txt")).unwrap();
    assert!(cmake_content.contains("project(my-cpp-project)"));

    assert_eq!(result.package_info.name, "my-cpp-project");
    assert!(result.created_files.len() >= 2);
}

#[tokio::test]
async fn test_analyze_cmake_manifest_with_sources() {
    let plugin = CppPlugin::default();
    let manifest_content = r#"
        cmake_minimum_required(VERSION 3.10)
        project(MyProject)

        add_executable(my_app src/main.cpp src/other.cpp)
        add_library(my_lib src/lib.cpp)
    "#;

    let dir = tempfile::tempdir().unwrap();
    let manifest_path = dir.path().join("CMakeLists.txt");
    std::fs::write(&manifest_path, manifest_content).unwrap();

    let manifest_data = plugin.analyze_manifest(&manifest_path).await.unwrap();
    let source_files = manifest_data.raw_data["source_files"].as_array().unwrap();

    assert_eq!(source_files.len(), 3);
    assert!(source_files.contains(&"src/main.cpp".into()));
    assert!(source_files.contains(&"src/other.cpp".into()));
    assert!(source_files.contains(&"src/lib.cpp".into()));
}

#[tokio::test]
async fn test_analyze_conanfile_py() {
    let plugin = CppPlugin::default();
    let content = r#"
from conan import ConanFile

class MyProject(ConanFile):
    name = "my_project"
    version = "1.0"
    requires = [
        "fmt/10.2.1",
        "gtest/1.14.0"
    ]
"#;

    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("conanfile.py");
    std::fs::write(&path, content).unwrap();

    let manifest_data = plugin.analyze_manifest(&path).await.unwrap();

    assert_eq!(manifest_data.dependencies.len(), 2);
    assert!(manifest_data.dependencies.iter().any(|d| d.name == "fmt"));
    assert!(manifest_data.dependencies.iter().any(|d| d.name == "gtest"));
}
