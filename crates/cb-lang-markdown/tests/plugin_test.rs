/// Simple test to verify markdown plugin basics work
use cb_lang_markdown::MarkdownPlugin;
use cb_plugin_api::{import_support::ImportParser, LanguagePlugin};

#[test]
fn test_plugin_handles_md_extension() {
    let plugin = MarkdownPlugin::new();
    assert!(
        plugin.handles_extension("md"),
        "Plugin should handle .md extension"
    );
}

#[test]
fn test_plugin_has_import_support() {
    let plugin = MarkdownPlugin::new();
    assert!(
        plugin.import_parser().is_some(),
        "Plugin should have import parser"
    );
}

#[test]
fn test_parse_inline_link() {
    let plugin = MarkdownPlugin::new();
    let import_parser = plugin.import_parser().expect("Should have import parser");

    let content = "[link](ARCHITECTURE.md)";
    let imports = ImportParser::parse_imports(import_parser, content);

    println!("Content: {}", content);
    println!("Imports: {:?}", imports);

    assert!(!imports.is_empty(), "Should find at least 1 import");
    assert!(
        imports.iter().any(|imp| imp.contains("ARCHITECTURE")),
        "Should contain ARCHITECTURE, got: {:?}",
        imports
    );
}
