use mill_plugin_api::{
    CreatePackageConfig, CreatePackageResult, PackageInfo, ProjectFactory, PluginResult,
};
use std::fs;
use std::path::PathBuf;

#[derive(Default)]
pub struct SwiftProjectFactory;

impl ProjectFactory for SwiftProjectFactory {
    fn create_package(
        &self,
        config: &CreatePackageConfig,
    ) -> PluginResult<CreatePackageResult> {
        let path: PathBuf = config.package_path.clone().into();
        let package_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                mill_plugin_api::PluginError::invalid_input("Invalid package path")
            })?;

        // Create root directory
        fs::create_dir_all(&path)
            .map_err(|e| mill_plugin_api::PluginError::internal(e.to_string()))?;

        // Create Package.swift
        let package_swift_content = format!(
            r#"
// swift-tools-version:5.3
import PackageDescription

let package = Package(
    name: "{}",
    products: [
        .library(
            name: "{}",
            targets: ["{}"]),
    ],
    dependencies: [],
    targets: [
        .target(
            name: "{}",
            dependencies: []),
        .testTarget(
            name: "{}Tests",
            dependencies: ["{}"]),
    ]
)
"#,
            package_name, package_name, package_name, package_name, package_name, package_name
        );
        let manifest_path = path.join("Package.swift");
        fs::write(&manifest_path, package_swift_content)
            .map_err(|e| mill_plugin_api::PluginError::internal(e.to_string()))?;

        // Create Sources directory and a file inside
        let sources_dir = path.join("Sources").join(&package_name);
        fs::create_dir_all(&sources_dir)
            .map_err(|e| mill_plugin_api::PluginError::internal(e.to_string()))?;
        let entry_point = sources_dir.join(format!("{}.swift", package_name));
        fs::write(&entry_point, "public struct MyLibrary {}")
            .map_err(|e| mill_plugin_api::PluginError::internal(e.to_string()))?;

        // Create Tests directory and a file inside
        let tests_dir = path.join("Tests").join(format!("{}Tests", package_name));
        fs::create_dir_all(&tests_dir)
            .map_err(|e| mill_plugin_api::PluginError::internal(e.to_string()))?;
        fs::write(
            tests_dir.join(format!("{}Tests.swift", package_name)),
            r#"
import XCTest
@testable import MyLibrary

final class MyLibraryTests: XCTestCase {
    func testExample() {
        // This is an example of a functional test case.
        // Use XCTAssert and related functions to verify your tests produce the correct
        // results.
        XCTAssertNotNil(MyLibrary())
    }
}
"#,
        )
        .map_err(|e| mill_plugin_api::PluginError::internal(e.to_string()))?;

        Ok(CreatePackageResult {
            created_files: vec![
                entry_point.to_str().unwrap().to_string(),
                manifest_path.to_str().unwrap().to_string(),
            ],
            workspace_updated: false,
            package_info: PackageInfo {
                name: package_name.to_string(),
                version: "0.1.0".to_string(),
                manifest_path: manifest_path.to_str().unwrap().to_string(),
            },
        })
    }
}