use mill_foundation::core::rename_scope::RenameScope;
use std::path::Path;

#[test]
fn test_gitignore_detection() {
    let scope = RenameScope::standard();

    // .gitignore should be detected by filename
    assert!(scope.should_include_file(Path::new(".gitignore")));
    assert!(scope.should_include_file(Path::new("/project/.gitignore")));
    assert!(scope.should_include_file(Path::new("/project/subdir/.gitignore")));

    // Similar names should not match
    assert!(!scope.should_include_file(Path::new(".gitignore.backup")));
    assert!(!scope.should_include_file(Path::new("gitignore")));
}

#[test]
fn test_gitignore_flag_control() {
    // Standard scope includes gitignore
    let standard = RenameScope::standard();
    assert!(standard.update_gitignore);
    assert!(standard.should_include_file(Path::new(".gitignore")));

    // Code scope excludes gitignore
    let code = RenameScope::code();
    assert!(!code.update_gitignore);
    assert!(!code.should_include_file(Path::new(".gitignore")));

    // Everything scope includes gitignore
    let everything = RenameScope::everything();
    assert!(everything.update_gitignore);
    assert!(everything.should_include_file(Path::new(".gitignore")));
}

#[test]
fn test_update_all_includes_gitignore() {
    let scope = RenameScope {
        update_code: false,
        update_string_literals: false,
        update_docs: false,
        update_configs: false,
        update_gitignore: false,
        update_comments: false,
        update_markdown_prose: false,
        update_exact_matches: false,
        exclude_patterns: vec![],
        update_all: true,
    }
    .resolve_update_all();

    assert!(scope.update_gitignore);
    assert!(scope.update_code);
    assert!(scope.update_docs);
}
