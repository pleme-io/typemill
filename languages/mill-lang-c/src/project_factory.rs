use mill_plugin_api::{CreatePackageConfig, CreatePackageResult, PackageInfo, ProjectFactory, PluginResult};
use std::fs;
use std::path::PathBuf;

pub struct CProjectFactory;

impl ProjectFactory for CProjectFactory {
    fn create_package(
        &self,
        config: &CreatePackageConfig,
    ) -> PluginResult<CreatePackageResult> {
        let package_path = PathBuf::from(&config.package_path);
        let src_path = package_path.join("src");
        fs::create_dir_all(&src_path).unwrap();

        let main_c_path = src_path.join("main.c");
        let main_c_content = r#"#include <stdio.h>

int main() {
    printf("Hello, World!\n");
    return 0;
}
"#;
        fs::write(&main_c_path, main_c_content).unwrap();

        let makefile_path = package_path.join("Makefile");
        let makefile_content = r#"CC = gcc
CFLAGS = -Wall -Wextra -std=c11
TARGET = main
SRCS = src/main.c

all: $(TARGET)

$(TARGET): $(SRCS)
	$(CC) $(CFLAGS) -o $(TARGET) $(SRCS)

clean:
	rm -f $(TARGET)
"#;
        fs::write(&makefile_path, makefile_content).unwrap();

        Ok(CreatePackageResult {
            created_files: vec![
                main_c_path.to_str().unwrap().to_string(),
                makefile_path.to_str().unwrap().to_string(),
            ],
            workspace_updated: false,
            package_info: PackageInfo {
                name: package_path.file_name().unwrap().to_str().unwrap().to_string(),
                version: "0.1.0".to_string(),
                manifest_path: makefile_path.to_str().unwrap().to_string(),
            },
        })
    }
}