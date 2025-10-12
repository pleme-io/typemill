/// Simple test to verify markdown plugin basics work
use cb_lang_markdown::MarkdownPlugin;
use cb_plugin_api::LanguagePlugin;

#[test]
fn test_plugin_handles_md_extension() {
    let plugin = MarkdownPlugin::new();
    assert!(plugin.handles_extension("md"), "Plugin should handle .md extension");
}

#[test]
fn test_plugin_has_import_support() {
    let plugin = MarkdownPlugin::new();
    assert!(plugin.import_support().is_some(), "Plugin should have import support");
}

#[test]
fn test_parse_inline_link() {
    let plugin = MarkdownPlugin::new();
    let import_support = plugin.import_support().expect("Should have import support");

    let content = "[link](ARCHITECTURE.md)";
    let imports = import_support.parse_imports(content);

    println!("Content: {}", content);
    println!("Imports: {:?}", imports);

    assert!(!imports.is_empty(), "Should find at least 1 import");
    assert!(imports.iter().any(|imp| imp.contains("ARCHITECTURE")),
            "Should contain ARCHITECTURE, got: {:?}", imports);
}
