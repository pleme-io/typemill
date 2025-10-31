//! Java manifest file parsing
//!
//! Handles manifest files for Java projects, including Maven (`pom.xml`) and
//! Gradle (`build.gradle`, `build.gradle.kts`).

use mill_plugin_api::{Dependency, DependencySource, ManifestData, PluginError, PluginResult};
use serde::Deserialize;
use std::path::Path;

// Structs for deserializing pom.xml files
#[derive(Debug, Deserialize, serde::Serialize)]
struct PomProject {
    #[serde(default, rename = "groupId")]
    group_id: String,
    #[serde(default, rename = "artifactId")]
    artifact_id: String,
    #[serde(default)]
    version: String,
    #[serde(default, rename = "dependencies")]
    dependencies: PomDependencies,
    #[serde(default, rename = "modules")]
    modules: PomModules,
}

#[derive(Debug, Deserialize, Default, serde::Serialize)]
struct PomDependencies {
    #[serde(default, rename = "dependency")]
    dependency: Vec<PomDependency>,
}

#[derive(Debug, Deserialize, Default, serde::Serialize)]
struct PomModules {
    #[serde(default, rename = "module")]
    module: Vec<String>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct PomDependency {
    #[serde(default, rename = "groupId")]
    group_id: String,
    #[serde(default, rename = "artifactId")]
    artifact_id: String,
    #[serde(default)]
    version: String,
}

/// Analyze a Java manifest file, dispatching to the correct parser.
pub async fn analyze_manifest(path: &Path) -> PluginResult<ManifestData> {
    let filename = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();

    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| PluginError::manifest(format!("Failed to read manifest: {}", e)))?;

    match filename {
        "pom.xml" => parse_pom_xml(&content),
        "build.gradle" | "build.gradle.kts" => parse_gradle_build(&content),
        _ => Err(PluginError::not_supported(format!(
            "Unsupported manifest file: {}",
            filename
        ))),
    }
}

/// Parses a `pom.xml` file content.
fn parse_pom_xml(content: &str) -> PluginResult<ManifestData> {
    let project: PomProject = quick_xml::de::from_str(content)
        .map_err(|e| PluginError::manifest(format!("Failed to parse pom.xml: {}", e)))?;

    // Serialize the project to JSON first, before moving fields out of it.
    let raw_data = serde_json::to_value(&project)
        .unwrap_or_else(|_| serde_json::json!({ "content": content }));

    let dependencies = project
        .dependencies
        .dependency
        .into_iter()
        .map(|dep| Dependency {
            name: format!("{}:{}", dep.group_id, dep.artifact_id),
            source: DependencySource::Version(dep.version),
        })
        .collect();

    Ok(ManifestData {
        name: project.artifact_id,
        version: project.version,
        dependencies,
        dev_dependencies: Vec::new(), // pom.xml doesn't have a standard dev dependency scope
        raw_data,
    })
}

/// Placeholder parser for Gradle build files.
fn parse_gradle_build(content: &str) -> PluginResult<ManifestData> {
    tracing::warn!("Gradle manifest parsing is not yet implemented. Returning placeholder data.");
    // TODO: Implement proper Gradle parsing. This is a complex task.
    // For now, we return a default object.
    Ok(ManifestData {
        name: "unknown-gradle-project".to_string(),
        version: "0.0.0".to_string(),
        dependencies: Vec::new(),
        dev_dependencies: Vec::new(),
        raw_data: serde_json::json!({ "content": content }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_parse_simple_pom_xml() {
        let pom_content = r#"
            <project>
                <groupId>com.mill</groupId>
                <artifactId>my-java-app</artifactId>
                <version>1.2.3</version>
                <dependencies>
                    <dependency>
                        <groupId>com.google.code.gson</groupId>
                        <artifactId>gson</artifactId>
                        <version>2.10.1</version>
                    </dependency>
                    <dependency>
                        <groupId>junit</groupId>
                        <artifactId>junit</artifactId>
                        <version>4.13.2</version>
                        <scope>test</scope>
                    </dependency>
                </dependencies>
            </project>
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", pom_content).unwrap();
        let path = temp_file.path().to_path_buf();
        // Manually rename to pom.xml so analyze_manifest dispatches correctly
        let pom_path = path.with_file_name("pom.xml");
        std::fs::rename(&path, &pom_path).unwrap();

        let result = analyze_manifest(&pom_path).await;
        assert!(result.is_ok());

        let manifest_data = result.unwrap();
        assert_eq!(manifest_data.name, "my-java-app");
        assert_eq!(manifest_data.version, "1.2.3".to_string());
        assert_eq!(manifest_data.dependencies.len(), 2);

        let gson_dep = manifest_data
            .dependencies
            .iter()
            .find(|d| d.name == "com.google.code.gson:gson")
            .unwrap();
        assert_eq!(
            gson_dep.source,
            DependencySource::Version("2.10.1".to_string())
        );
    }

    #[tokio::test]
    async fn test_analyze_gradle_placeholder() {
        let gradle_content = "plugins { id 'java' }";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", gradle_content).unwrap();
        let path = temp_file.path().to_path_buf();
        let gradle_path = path.with_file_name("build.gradle");
        std::fs::rename(&path, &gradle_path).unwrap();

        let result = analyze_manifest(&gradle_path).await;
        assert!(result.is_ok());
        let manifest_data = result.unwrap();
        assert_eq!(manifest_data.name, "unknown-gradle-project");
    }
}