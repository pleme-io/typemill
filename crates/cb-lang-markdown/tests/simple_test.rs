/// Simple integration test to verify markdown plugin works
use cb_lang_markdown::MarkdownPlugin;
use cb_plugin_api::LanguagePlugin;

#[test]
fn test_plugin_basics() {
    let plugin = MarkdownPlugin::new();

    // Test 1: Does it handle .md extension?
    println!("Test 1: Checking if plugin handles .md extension");
    assert!(
        plugin.handles_extension("md"),
        "Plugin should handle .md extension"
    );
    println!("✓ Plugin handles .md extension");

    // Test 2: Does it have import support?
    println!("\nTest 2: Checking if plugin has import support");
    assert!(
        plugin.import_support().is_some(),
        "Plugin should have import support"
    );
    println!("✓ Plugin has import support");
}

#[test]
fn test_parse_imports_real_content() {
    let plugin = MarkdownPlugin::new();
    let import_support = plugin.import_support().expect("Should have import support");

    // Real content from PRIMITIVES.md that references ARCHITECTURE.md
    let content = r#"
# Some Documentation

See [ARCHITECTURE.md](ARCHITECTURE.md) for details.

Also check [Architecture Documentation](../architecture/ARCHITECTURE.md).

Reference style links:
[arch-doc]: ARCHITECTURE.md

Autolink: <ARCHITECTURE.md>
"#;

    println!("\nTest 3: Parsing imports from markdown content");
    let imports = import_support.parse_imports(content);

    println!("Found imports: {:?}", imports);

    // Should find ARCHITECTURE.md (possibly multiple times with different paths)
    let has_architecture = imports.iter().any(|imp| imp.contains("ARCHITECTURE"));
    assert!(
        has_architecture,
        "Should find ARCHITECTURE.md reference. Found: {:?}",
        imports
    );

    println!("✓ Successfully parsed markdown imports");
    println!("  Imports found: {}", imports.len());
    for imp in &imports {
        println!("    - {}", imp);
    }
}

#[test]
fn test_inline_link_parsing() {
    let plugin = MarkdownPlugin::new();
    let import_support = plugin.import_support().expect("Should have import support");

    let content = "[link](ARCHITECTURE.md)";
    let imports = import_support.parse_imports(content);

    println!("\nTest 4: Inline link");
    println!("Content: {}", content);
    println!("Imports: {:?}", imports);

    assert_eq!(imports.len(), 1, "Should find exactly 1 import");
    assert!(
        imports[0].contains("ARCHITECTURE"),
        "Should contain ARCHITECTURE"
    );
}

#[test]
fn test_reference_style_link_parsing() {
    let plugin = MarkdownPlugin::new();
    let import_support = plugin.import_support().expect("Should have import support");

    let content = "[ref]: ARCHITECTURE.md";
    let imports = import_support.parse_imports(content);

    println!("\nTest 5: Reference-style link");
    println!("Content: {}", content);
    println!("Imports: {:?}", imports);

    assert_eq!(imports.len(), 1, "Should find exactly 1 import");
    assert!(
        imports[0].contains("ARCHITECTURE"),
        "Should contain ARCHITECTURE"
    );
}

#[test]
fn test_autolink_parsing() {
    let plugin = MarkdownPlugin::new();
    let import_support = plugin.import_support().expect("Should have import support");

    let content = "<ARCHITECTURE.md>";
    let imports = import_support.parse_imports(content);

    println!("\nTest 6: Autolink");
    println!("Content: {}", content);
    println!("Imports: {:?}", imports);

    assert_eq!(imports.len(), 1, "Should find exactly 1 import");
    assert!(
        imports[0].contains("ARCHITECTURE"),
        "Should contain ARCHITECTURE"
    );
}
