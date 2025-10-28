use mill_plugin_api::{
    project_factory::{
        CreatePackageConfig, CreatePackageResult, PackageInfo, ProjectFactory,
    },
    PluginResult,
};
use std::fs;
use std::path::Path;

pub struct CppProjectFactory;

impl ProjectFactory for CppProjectFactory {
    fn create_package(&self, config: &CreatePackageConfig) -> PluginResult<CreatePackageResult> {
        let package_name = Path::new(&config.package_path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();
        let project_path = Path::new(&config.workspace_root).join(&config.package_path);
        fs::create_dir_all(project_path.join("src")).map_err(|e| {
            mill_plugin_api::PluginError::internal(format!("Failed to create src dir: {}", e))
        })?;
        fs::create_dir_all(project_path.join("include")).map_err(|e| {
            mill_plugin_api::PluginError::internal(format!("Failed to create include dir: {}", e))
        })?;

        let main_cpp_path = project_path.join("src/main.cpp");
        let cmake_path = project_path.join("CMakeLists.txt");

        let main_cpp_content = r#"#include <iostream>

int main() {
    std::cout << "Hello, world!" << std::endl;
    return 0;
}
"#;
        fs::write(&main_cpp_path, main_cpp_content).map_err(|e| {
            mill_plugin_api::PluginError::internal(format!("Failed to write main.cpp: {}", e))
        })?;

        let cmake_content = format!(
            r#"cmake_minimum_required(VERSION 3.10)
project({})

add_executable(${{PROJECT_NAME}} src/main.cpp)
"#,
            package_name
        );
        fs::write(&cmake_path, cmake_content).map_err(|e| {
            mill_plugin_api::PluginError::internal(format!("Failed to write CMakeLists.txt: {}", e))
        })?;

        Ok(CreatePackageResult {
            created_files: vec![
                main_cpp_path.to_str().unwrap().to_string(),
                cmake_path.to_str().unwrap().to_string(),
            ],
            workspace_updated: false,
            package_info: PackageInfo {
                name: package_name.to_string(),
                version: "0.1.0".to_string(),
                manifest_path: cmake_path.to_str().unwrap().to_string(),
            },
        })
    }
}