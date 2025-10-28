use mill_lang_cpp::CppPlugin;
use mill_plugin_api::import_support::ImportParser;
use mill_plugin_api::LanguagePlugin;

#[test]
fn test_parse_imports() {
    let plugin = CppPlugin::default();
    let import_parser = plugin.import_parser().unwrap();

    let source = r#"
#include <iostream>
#include "my_header.h"
#include <vector>

int main() {
    std::cout << "Hello, world!" << std::endl;
    return 0;
}
"#;

    let imports = import_parser.parse_imports(source);

    assert_eq!(imports.len(), 3);
    assert_eq!(imports[0], "iostream");
    assert_eq!(imports[1], "my_header.h");
    assert_eq!(imports[2], "vector");
}