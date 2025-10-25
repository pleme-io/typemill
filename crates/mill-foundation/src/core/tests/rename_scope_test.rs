use mill_foundation::core::rename_scope::RenameScope;
use std::path::Path;

#[test]
fn test_rename_scope_code() {
    let scope = RenameScope::code();

    // Code files should be included
    assert!(scope.should_include_file(Path::new("src/main.rs")));
    assert!(scope.should_include_file(Path::new("lib/utils.ts")));

    // Documentation and config files should be excluded
    assert!(!scope.should_include_file(Path::new("README.md")));
    assert!(!scope.should_include_file(Path::new("Cargo.toml")));
    assert!(!scope.should_include_file(Path::new("config.yaml")));
    assert!(!scope.should_include_file(Path::new(".gitignore")));
}

#[test]
fn test_rename_scope_project() {
    let scope = RenameScope::project();

    // All files should be included
    assert!(scope.should_include_file(Path::new("src/main.rs")));
    assert!(scope.should_include_file(Path::new("README.md")));
    assert!(scope.should_include_file(Path::new("Cargo.toml")));
    assert!(scope.should_include_file(Path::new("config.yaml")));
    assert!(scope.should_include_file(Path::new(".gitignore")));
}

#[test]
fn test_rename_scope_comments() {
    let scope = RenameScope::comments();

    // Code files should be included
    assert!(scope.should_include_file(Path::new("src/main.rs")));
    assert!(scope.should_include_file(Path::new("lib/utils.ts")));

    // Documentation and config files should be included
    assert!(scope.should_include_file(Path::new("README.md")));
    assert!(scope.should_include_file(Path::new("Cargo.toml")));

    // Comments enabled
    assert!(scope.update_comments);

    // Prose still opt-in
    assert!(!scope.update_markdown_prose);
}

#[test]
fn test_rename_scope_everything() {
    let scope = RenameScope::everything();

    // All files should be included
    assert!(scope.should_include_file(Path::new("src/main.rs")));
    assert!(scope.should_include_file(Path::new("README.md")));
    assert!(scope.should_include_file(Path::new("Cargo.toml")));

    // All options enabled
    assert!(scope.update_comments);
    assert!(scope.update_markdown_prose);
}

#[test]
fn test_rename_scope_exclude_patterns() {
    let scope = RenameScope {
        update_code: true,
        update_string_literals: true,
        update_docs: true,
        update_configs: true,
        update_gitignore: true,
        update_comments: false,
        update_markdown_prose: false,
        update_exact_matches: false,
        exclude_patterns: vec!["**/test_*".to_string(), "**/fixtures/**".to_string()],
        update_all: false,
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
        update_gitignore: false,
        update_comments: false,
        update_markdown_prose: false,
        update_exact_matches: false,
        exclude_patterns: vec![],
        update_all: false,
    };

    // Code and docs included
    assert!(scope.should_include_file(Path::new("src/lib.rs")));
    assert!(scope.should_include_file(Path::new("docs/guide.md")));

    // Configs excluded
    assert!(!scope.should_include_file(Path::new("config.toml")));
    assert!(!scope.should_include_file(Path::new("settings.yaml")));
}