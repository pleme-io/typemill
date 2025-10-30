use mill_lang_cpp::CppPlugin;
use mill_plugin_api::{LanguagePlugin, import_support::ImportParser};

#[tokio::test]
async fn test_parse_cpp20_imports() {
    let cpp_code = r#"
import std.core;
import my_module;
import "my_other_module";

int main() {
    return 0;
}
"#;
    let plugin = CppPlugin::default();
    let imports = plugin.import_parser().unwrap().parse_imports(cpp_code);

    assert_eq!(imports.len(), 3);
    assert!(imports.contains(&"std.core".to_string()));
    assert!(imports.contains(&"my_module".to_string()));
    assert!(imports.contains(&"my_other_module".to_string()));
}