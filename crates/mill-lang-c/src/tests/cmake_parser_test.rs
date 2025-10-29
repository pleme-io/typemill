use crate::cmake_parser::analyze_cmake_manifest;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_analyze_cmake_manifest() {
    let dir = tempdir().unwrap();
    let cmake_path = dir.path().join("CMakeLists.txt");
    let cmake_content = r#"
cmake_minimum_required(VERSION 3.10)
project(my_project)

add_executable(my_app main.c)
add_library(my_lib STATIC module1.c)
"#;
    fs::write(&cmake_path, cmake_content).unwrap();

    let manifest_data = analyze_cmake_manifest(&cmake_path).unwrap();

    assert_eq!(manifest_data.name, "my_project");
    let raw_data = manifest_data.raw_data;
    let executables = raw_data["executables"].as_array().unwrap();
    assert_eq!(executables.len(), 1);
    assert_eq!(executables[0], "my_app");
    let libraries = raw_data["libraries"].as_array().unwrap();
    assert_eq!(libraries.len(), 1);
    assert_eq!(libraries[0], "my_lib");
}