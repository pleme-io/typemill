// analysis/cb-analysis-deep-dead-code/src/ast_parser/tests.rs

use super::*;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_extract_symbols_from_rust_file() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.rs");
    let workspace_root = dir.path();

    let source_code = r#"
        pub struct MyStruct {
            field: i32,
        }

        enum MyEnum {
            Variant1,
            Variant2,
        }

        fn my_function() -> bool {
            true
        }

        pub(crate) trait MyTrait {
            fn do_something(&self);
        }

        const MY_CONST: u8 = 42;
    "#;

    fs::write(&file_path, source_code).unwrap();

    let extractor = SymbolExtractor::new();
    let symbols = extractor.extract_symbols(&file_path, workspace_root).unwrap();

    assert_eq!(symbols.len(), 5);

    // Test MyStruct
    let my_struct = symbols.iter().find(|s| s.name == "MyStruct").unwrap();
    assert_eq!(my_struct.kind, SymbolKind::Struct);
    assert!(my_struct.is_public);
    assert_eq!(my_struct.range.start.line, 1);

    // Test MyEnum
    let my_enum = symbols.iter().find(|s| s.name == "MyEnum").unwrap();
    assert_eq!(my_enum.kind, SymbolKind::Enum);
    assert!(!my_enum.is_public);
    assert_eq!(my_enum.range.start.line, 5);

    // Test my_function
    let my_function = symbols.iter().find(|s| s.name == "my_function").unwrap();
    assert_eq!(my_function.kind, SymbolKind::Function);
    assert!(!my_function.is_public);
    assert_eq!(my_function.range.start.line, 10);

    // Test MyTrait
    let my_trait = symbols.iter().find(|s| s.name == "MyTrait").unwrap();
    assert_eq!(my_trait.kind, SymbolKind::Trait);
    assert!(!my_trait.is_public); // pub(crate) is not considered public for our purposes
    assert_eq!(my_trait.range.start.line, 14);

    // Test MY_CONST
    let my_const = symbols.iter().find(|s| s.name == "MY_CONST").unwrap();
    assert_eq!(my_const.kind, SymbolKind::Constant);
    assert!(!my_const.is_public);
    assert_eq!(my_const.range.start.line, 18);
}

#[test]
fn test_handle_parse_error_gracefully() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("invalid.rs");
    let workspace_root = dir.path();

    let source_code = "pub struct MyStruct {"; // Invalid Rust code
    fs::write(&file_path, source_code).unwrap();

    let extractor = SymbolExtractor::new();
    let symbols = extractor.extract_symbols(&file_path, workspace_root).unwrap();

    // Should return an empty Vec, not an error
    assert!(symbols.is_empty());
}
