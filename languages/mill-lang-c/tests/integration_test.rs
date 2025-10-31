use mill_lang_c::CPlugin;
use mill_plugin_api::LanguagePlugin;

#[tokio::test]
async fn test_plugin_metadata() {
    let plugin = CPlugin::default();
    let metadata = plugin.metadata();
    assert_eq!(metadata.name, "C");
    assert_eq!(metadata.extensions, &["c", "h"]);
}

#[tokio::test]
async fn test_parse_empty_file() {
    let plugin = CPlugin::default();
    let source = "";
    let result = plugin.parse(source).await;
    assert!(result.is_ok());
    let parsed_source = result.unwrap();
    assert!(parsed_source.symbols.is_empty());
}

#[tokio::test]
async fn test_simple_function() {
    let plugin = CPlugin::default();
    let source = r#"
int main() {
    return 0;
}
"#;
    let result = plugin.parse(source).await;
    assert!(result.is_ok());
    let parsed_source = result.unwrap();
    assert_eq!(parsed_source.symbols.len(), 1);
    let symbol = &parsed_source.symbols[0];
    assert_eq!(symbol.name, "main");
    assert_eq!(symbol.kind, mill_plugin_api::SymbolKind::Function);
}