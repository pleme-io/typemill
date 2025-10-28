//! Csharp manifest file parsing
//!
//! Handles *.csproj files for Csharp projects.
//!
//! This module can use cb-lang-common utilities:
//! - read_manifest for async file reading with error handling
//! - TomlWorkspace/JsonWorkspace for workspace operations
//! - ErrorBuilder for rich error context
use cb_lang_common::{read_manifest, ErrorBuilder};
use cb_plugin_api::{Dependency, DependencySource, ManifestData, PluginResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

// Structs for deserializing .csproj XML
#[derive(Debug, Deserialize, Serialize, PartialEq, Default)]
struct Project {
    #[serde(rename = "@Sdk", default)]
    sdk: Option<String>,
    #[serde(rename = "PropertyGroup", default)]
    property_groups: Vec<PropertyGroup>,
    #[serde(rename = "ItemGroup", default)]
    item_groups: Vec<ItemGroup>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Default)]
struct PropertyGroup {
    #[serde(rename = "OutputType", default)]
    output_type: Option<String>,
    #[serde(rename = "TargetFramework", default)]
    target_framework: Option<String>,
    #[serde(rename = "AssemblyName", default)]
    assembly_name: Option<String>,
    #[serde(rename = "Version", default)]
    version: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Default)]
struct ItemGroup {
    #[serde(rename = "PackageReference", default)]
    package_references: Vec<PackageReference>,
    #[serde(rename = "ProjectReference", default)]
    project_references: Vec<ProjectReference>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct PackageReference {
    #[serde(rename = "@Include")]
    name: String,
    #[serde(rename = "@Version")]
    version: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct ProjectReference {
    #[serde(rename = "@Include")]
    path: String,
}

/// Analyze Csharp manifest file
pub async fn analyze_manifest(path: &Path) -> PluginResult<ManifestData> {
    let content = read_manifest(path).await?;
    let project: Project =
        quick_xml::de::from_str(&content).map_err(|e| {
            ErrorBuilder::manifest("Invalid XML in .csproj file")
                .with_path(path)
                .with_context("xml_error", e.to_string())
                .build()
        })?;

    let name = project
        .property_groups
        .iter()
        .find_map(|p| p.assembly_name.as_ref()).cloned()
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string()
        });

    let version = project
        .property_groups
        .iter()
        .find_map(|p| p.version.as_ref()).cloned()
        .unwrap_or_else(|| "0.0.0".to_string());

    let mut dependencies = vec![];
    for item_group in &project.item_groups {
        for pkg_ref in &item_group.package_references {
            dependencies.push(Dependency {
                name: pkg_ref.name.clone(),
                source: DependencySource::Version(pkg_ref.version.clone()),
            });
        }
        for proj_ref in &item_group.project_references {
            let normalized_path_str = proj_ref.path.replace('\\', "/");
            let proj_path = Path::new(&normalized_path_str);
            let dep_name = proj_path
                .file_name()
                .and_then(|s| s.to_str())
                .and_then(|s| s.strip_suffix(".csproj"))
                .unwrap_or(&normalized_path_str)
                .to_string();

            dependencies.push(Dependency {
                name: dep_name,
                source: DependencySource::Path(normalized_path_str),
            });
        }
    }

    Ok(ManifestData {
        name,
        version,
        dependencies,
        dev_dependencies: vec![], // .csproj doesn't have a standard concept of dev dependencies
        raw_data: serde_json::to_value(&project).map_err(|e| {
            ErrorBuilder::manifest("Failed to serialize manifest data")
                .with_path(path)
                .with_context("serde_error", e.to_string())
                .build()
        })?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_csproj_file(content: &str) -> NamedTempFile {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "{}", content).unwrap();
        temp_file
    }

    #[tokio::test]
    async fn test_parse_basic_csproj() {
        let csproj_content = r#"
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net6.0</TargetFramework>
    <AssemblyName>MyAwesomeProject</AssemblyName>
    <Version>1.2.3</Version>
  </PropertyGroup>
  <ItemGroup>
    <PackageReference Include="Newtonsoft.Json" Version="13.0.1" />
  </ItemGroup>
  <ItemGroup>
    <ProjectReference Include="..\MyLibrary\MyLibrary.csproj" />
  </ItemGroup>
</Project>
"#;
        let temp_file = create_csproj_file(csproj_content);
        let result = analyze_manifest(temp_file.path()).await;
        assert!(result.is_ok(), "Parsing failed: {:?}", result.err());
        let manifest = result.unwrap();
        assert_eq!(manifest.name, "MyAwesomeProject");
        assert_eq!(manifest.version, "1.2.3");
        assert_eq!(manifest.dependencies.len(), 2);

        let pkg_dep = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "Newtonsoft.Json")
            .expect("Could not find package dependency 'Newtonsoft.Json'");
        assert_eq!(
            pkg_dep.source,
            DependencySource::Version("13.0.1".to_string())
        );

        let proj_dep = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "MyLibrary")
            .expect("Could not find project dependency 'MyLibrary'");
        assert_eq!(
            proj_dep.source,
            DependencySource::Path("../MyLibrary/MyLibrary.csproj".to_string())
        );
    }

    #[tokio::test]
    async fn test_fallback_to_filename_for_name() {
        let csproj_content = r#"
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <Version>1.0.0</Version>
  </PropertyGroup>
</Project>
"#;
        let mut temp_file = tempfile::Builder::new()
            .suffix(".csproj")
            .tempfile()
            .unwrap();
        write!(temp_file, "{}", csproj_content).unwrap();
        let path = temp_file.path().to_owned();
        let file_name = path.file_stem().unwrap().to_str().unwrap().to_owned();

        let result = analyze_manifest(&path).await;
        assert!(result.is_ok());
        let manifest = result.unwrap();
        assert_eq!(manifest.name, file_name);
    }

    #[tokio::test]
    async fn test_empty_csproj() {
        let csproj_content = "<Project></Project>";
        let temp_file = create_csproj_file(csproj_content);
        let result = analyze_manifest(temp_file.path()).await;
        assert!(result.is_ok());
        let manifest = result.unwrap();
        assert_eq!(manifest.version, "0.0.0");
        assert!(manifest.dependencies.is_empty());
    }

    #[tokio::test]
    async fn test_invalid_xml() {
        let csproj_content = "<Project><PropertyGroup></PropertyGroup>";
        let temp_file = create_csproj_file(csproj_content);
        let result = analyze_manifest(temp_file.path()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_analyze_nonexistent_manifest() {
        let result = analyze_manifest(Path::new("/nonexistent/manifest.csproj")).await;
        assert!(result.is_err());
    }
}