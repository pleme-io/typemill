use crate::makefile_parser::analyze_makefile_manifest;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_analyze_makefile_manifest() {
    let dir = tempdir().unwrap();
    let makefile_path = dir.path().join("Makefile");
    let makefile_content = r#"
TARGET = my_program
SRCS = main.c module1.c module2.c
CFLAGS = -Wall -O2
"#;
    fs::write(&makefile_path, makefile_content).unwrap();

    let manifest_data = analyze_makefile_manifest(&makefile_path).unwrap();

    assert_eq!(manifest_data.name, "my_program");
    let raw_data = manifest_data.raw_data;
    let source_files = raw_data["source_files"].as_array().unwrap();
    assert_eq!(source_files.len(), 3);
    assert!(source_files.contains(&"main.c".into()));
    assert!(source_files.contains(&"module1.c".into()));
    assert!(source_files.contains(&"module2.c".into()));
    assert_eq!(raw_data["cflags"], "-Wall -O2");
}