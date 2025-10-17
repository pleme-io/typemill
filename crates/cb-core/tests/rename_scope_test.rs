use cb_core::rename_scope::RenameScope;
use std::path::Path;

#[test]
fn test_rename_scope_code_only() {
    let scope = RenameScope::code_only();

    // Code files should be included
    assert!(scope.should_include_file(Path::new("src/main.rs")));
    assert!(scope.should_include_file(Path::new("lib/utils.ts")));

    // Documentation and config files should be excluded
    assert!(!scope.should_include_file(Path::new("README.md")));
    assert!(!scope.should_include_file(Path::new("Cargo.toml")));
    assert!(!scope.should_include_file(Path::new("config.yaml")));
}

#[test]
fn test_rename_scope_all() {
    let scope = RenameScope::all();

    // All files should be included
    assert!(scope.should_include_file(Path::new("src/main.rs")));
    assert!(scope.should_include_file(Path::new("README.md")));
    assert!(scope.should_include_file(Path::new("Cargo.toml")));
    assert!(scope.should_include_file(Path::new("config.yaml")));
}

#[test]
fn test_rename_scope_exclude_patterns() {
    let scope = RenameScope {
        update_code: true,
        update_string_literals: true,
        update_docs: true,
        update_configs: true,
        update_examples: true,
        update_comments: false,
        exclude_patterns: vec!["**/test_*".to_string(), "**/fixtures/**".to_string()],
    };

    // Normal files should be included
    assert!(scope.should_include_file(Path::new("src/main.rs")));
    assert!(scope.should_include_file(Path::new("README.md")));

    // Excluded patterns should be filtered out
    assert!(!scope.should_include_file(Path::new("src/test_utils.rs")));
    assert!(!scope.should_include_file(Path::new("fixtures/example.md")));
    assert!(!scope.should_include_file(Path::new("tests/fixtures/data.toml")));
}

#[test]
fn test_rename_scope_custom() {
    let scope = RenameScope {
        update_code: true,
        update_string_literals: false,
        update_docs: true,
        update_configs: false,
        update_examples: true,
        update_comments: false,
        exclude_patterns: vec![],
    };

    // Code and docs included
    assert!(scope.should_include_file(Path::new("src/lib.rs")));
    assert!(scope.should_include_file(Path::new("docs/guide.md")));

    // Configs excluded
    assert!(!scope.should_include_file(Path::new("config.toml")));
    assert!(!scope.should_include_file(Path::new("settings.yaml")));
}
