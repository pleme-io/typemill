// Example: Test Suite Pattern for Language Plugins
// Minimum 30+ tests required

#[cfg(test)]
mod tests {
    use super::*;

    // === Basic Plugin Tests ===

    #[tokio::test]
    async fn test_metadata() {
        let plugin = MyPlugin::new();
        assert_eq!(plugin.metadata().name, "MyLanguage");
        assert_eq!(plugin.metadata().extensions, &["ml"]);
    }

    #[test]
    fn test_capabilities() {
        let plugin = MyPlugin::new();
        let caps = plugin.capabilities();
        assert!(caps.imports);
        assert!(caps.workspace);
    }

    // === Symbol Extraction Tests (10+) ===

    #[tokio::test]
    async fn test_parse_function() {
        let plugin = MyPlugin::new();
        let source = "function hello() { return 'world'; }";

        let result = plugin.parse(source).await.unwrap();

        assert_eq!(result.symbols.len(), 1);
        assert_eq!(result.symbols[0].name, "hello");
        assert_eq!(result.symbols[0].kind, SymbolKind::Function);
    }

    #[tokio::test]
    async fn test_parse_class() {
        let source = "class MyClass { method() {} }";
        let result = plugin.parse(source).await.unwrap();

        let class_sym = result.symbols.iter()
            .find(|s| s.kind == SymbolKind::Class)
            .unwrap();
        assert_eq!(class_sym.name, "MyClass");
    }

    #[tokio::test]
    async fn test_parse_empty_file() {
        let result = plugin.parse("").await.unwrap();
        assert!(result.symbols.is_empty());
    }

    #[tokio::test]
    async fn test_parse_unicode() {
        let source = "function 日本語() { /* Unicode test */ }";
        let result = plugin.parse(source).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_parse_syntax_error() {
        let source = "function incomplete {";
        let result = plugin.parse(source).await;
        assert!(result.is_err());
    }

    // === Import Tests (10+) ===

    #[test]
    fn test_import_parsing_basic() {
        let imports = parse_imports("import foo").unwrap();
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].module_path, "foo");
    }

    #[test]
    fn test_import_parsing_with_alias() {
        let imports = parse_imports("import foo as bar").unwrap();
        assert_eq!(imports[0].module_path, "foo");
        assert_eq!(imports[0].default_import, Some("bar".to_string()));
    }

    #[test]
    fn test_import_parsing_multiple() {
        let source = "import foo\nimport bar\nimport baz";
        let imports = parse_imports(source).unwrap();
        assert_eq!(imports.len(), 3);
    }

    // === Manifest Tests (5+) ===

    #[tokio::test]
    async fn test_manifest_valid() {
        let manifest = r#"
            name = "test-package"
            version = "1.0.0"
        "#;

        let temp_file = write_temp_manifest(manifest);
        let result = plugin.analyze_manifest(&temp_file).await.unwrap();

        assert_eq!(result.name, "test-package");
        assert_eq!(result.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_manifest_missing_name() {
        let manifest = r#"version = "1.0.0""#;
        let temp_file = write_temp_manifest(manifest);

        let result = plugin.analyze_manifest(&temp_file).await;
        assert!(result.is_err());
    }

    // === Refactoring Tests (5+) ===

    #[test]
    fn test_rewrite_imports() {
        let source = "import old_module\nimport other";
        let (result, count) = rewrite_imports_for_rename(
            source, "old_module", "new_module"
        );

        assert_eq!(count, 1);
        assert!(result.contains("import new_module"));
        assert!(result.contains("import other"));
    }
}

// Helper functions
fn write_temp_manifest(content: &str) -> PathBuf {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("manifest.toml");
    std::fs::write(&path, content).unwrap();
    path
}
