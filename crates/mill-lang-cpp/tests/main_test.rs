use mill_lang_cpp::CppPlugin;
use mill_plugin_api::LanguagePlugin;
use tempfile::Builder;
use std::io::Write;

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

    println!("Found symbols: {:?}", symbols.iter().map(|s| &s.name).collect::<Vec<_>>());

    // TODO: Improve symbol parsing to correctly handle nested symbols.
    // The current implementation only finds top-level symbols.
    assert_eq!(symbols.len(), 4, "Should find namespace, class, method, and main function");
    let names: Vec<_> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"MyNamespace"));
    assert!(names.contains(&"MyClass"));
    assert!(names.contains(&"myMethod"));
    assert!(names.contains(&"main"));
}

#[tokio::test]
async fn test_analyze_cmake_manifest() {
    let plugin = CppPlugin::default();
    let content = "project(MyAwesomeProject)";

    let mut temp_file = Builder::new()
        .prefix("CMakeLists")
        .suffix(".txt")
        .tempfile()
        .unwrap();
    writeln!(temp_file, "{}", content).unwrap();
    let path = temp_file.into_temp_path();

    let manifest_data = plugin.analyze_manifest(&path).await.unwrap();

    assert_eq!(manifest_data.name, "MyAwesomeProject".to_string());
}