use mill_lang_c::CPlugin;
use mill_plugin_api::LanguagePlugin;
use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn test_makefile_project() {
    let dir = tempdir().unwrap();
    let makefile_path = dir.path().join("Makefile");
    let makefile_content = r#"
TARGET = my_program
SRCS = main.c module1.c
"#;
    fs::write(&makefile_path, makefile_content).unwrap();

    let plugin = CPlugin::default();
    let manifest_data = plugin.analyze_manifest(&makefile_path).await.unwrap();

    assert_eq!(manifest_data.name, "my_program");
}

#[tokio::test]
async fn test_cmake_project() {
    let dir = tempdir().unwrap();
    let cmake_path = dir.path().join("CMakeLists.txt");
    let cmake_content = r#"
project(my_cmake_project)
add_executable(my_app main.c)
"#;
    fs::write(&cmake_path, cmake_content).unwrap();

    let plugin = CPlugin::default();
    let manifest_data = plugin.analyze_manifest(&cmake_path).await.unwrap();

    assert_eq!(manifest_data.name, "my_cmake_project");
}