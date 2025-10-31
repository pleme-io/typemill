//! C# project creation using the `dotnet` CLI.
use async_trait::async_trait;
use mill_plugin_api::{
    CreatePackageConfig, CreatePackageResult, PackageInfo, ProjectFactory, PluginResult,
};
use std::process::Command;
use std::path::Path;

#[derive(Default)]
pub struct CsharpProjectFactory;

#[async_trait]
impl ProjectFactory for CsharpProjectFactory {
    fn create_package(
        &self,
        config: &CreatePackageConfig,
    ) -> PluginResult<CreatePackageResult> {
        let package_name = Path::new(&config.package_path)
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();
        let project_path = Path::new(&config.workspace_root).join(&config.package_path);

        if project_path.exists() {
            return Err(mill_plugin_api::PluginError::internal(format!(
                "Directory '{}' already exists.",
                project_path.display()
            )));
        }

        // Use `dotnet new console` to create a new project
        let output = Command::new("dotnet")
            .arg("new")
            .arg("console")
            .arg("-n")
            .arg(package_name)
            .arg("-o")
            .arg(&project_path)
            .output()
            .map_err(|e| {
                mill_plugin_api::PluginError::internal(format!(
                    "Failed to execute 'dotnet new': {}",
                    e
                ))
            })?;

        if !output.status.success() {
            return Err(mill_plugin_api::PluginError::internal(format!(
                "Failed to create C# project: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let manifest_path = project_path.join(format!("{}.csproj", package_name));
        let entry_point = project_path.join("Program.cs");

        Ok(CreatePackageResult {
            created_files: vec![
                manifest_path.to_str().unwrap_or_default().to_string(),
                entry_point.to_str().unwrap_or_default().to_string(),
            ],
            workspace_updated: false, // We don't have a workspace file to update for C#
            package_info: PackageInfo {
                name: package_name.to_string(),
                manifest_path: manifest_path.to_str().unwrap_or_default().to_string(),
                version: "".to_string(),
            },
        })
    }
}