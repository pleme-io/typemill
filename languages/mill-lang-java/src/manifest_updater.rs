//! pom.xml manifest file updater for Java projects
//!
//! This module provides functionality for updating Maven pom.xml files,
//! including adding/removing dependencies and updating project metadata.

use async_trait::async_trait;
use mill_plugin_api::{ManifestUpdater, PluginError, PluginResult};
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use std::io::Cursor;
use std::path::Path;
use tracing::debug;

/// Java manifest updater for pom.xml files
#[derive(Default, Clone)]
pub struct JavaManifestUpdater;

#[async_trait]
impl ManifestUpdater for JavaManifestUpdater {
    async fn update_dependency(
        &self,
        manifest_path: &Path,
        old_name: &str,
        new_name: &str,
        new_version: Option<&str>,
    ) -> PluginResult<String> {
        // Read the manifest file
        let content = tokio::fs::read_to_string(manifest_path)
            .await
            .map_err(|e| PluginError::internal(format!("Failed to read manifest: {}", e)))?;
        debug!(
            manifest_path = %manifest_path.display(),
            old_name = %old_name,
            new_name = %new_name,
            new_version = ?new_version,
            "Updating Java dependency in pom.xml"
        );

        // Parse the pom.xml and update the specified dependency
        update_dependency_in_pom(&content, old_name, new_name, new_version)
    }

    fn generate_manifest(&self, package_name: &str, dependencies: &[String]) -> String {
        debug!(
            package_name = %package_name,
            dependency_count = dependencies.len(),
            "Generating new pom.xml manifest"
        );

        // Parse package name into groupId and artifactId
        let (group_id, artifact_id) = parse_package_name(package_name);

        // Generate a basic pom.xml structure
        let mut pom = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0
         http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>

    <groupId>{}</groupId>
    <artifactId>{}</artifactId>
    <version>1.0.0</version>
    <packaging>jar</packaging>

    <properties>
        <maven.compiler.source>11</maven.compiler.source>
        <maven.compiler.target>11</maven.compiler.target>
        <project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>
    </properties>
"#,
            group_id, artifact_id
        );

        if !dependencies.is_empty() {
            pom.push_str("\n    <dependencies>\n");
            for dep in dependencies {
                let (dep_group, dep_artifact, dep_version) = parse_dependency(dep);
                pom.push_str(&format!(
                    r#"        <dependency>
            <groupId>{}</groupId>
            <artifactId>{}</artifactId>
            <version>{}</version>
        </dependency>
"#,
                    dep_group, dep_artifact, dep_version
                ));
            }
            pom.push_str("    </dependencies>\n");
        }

        pom.push_str("</project>\n");
        pom
    }
}

/// Parse package name into groupId and artifactId
/// Format: "com.example:myproject" or "com.example.myproject"
fn parse_package_name(package_name: &str) -> (String, String) {
    if let Some((group, artifact)) = package_name.split_once(':') {
        (group.to_string(), artifact.to_string())
    } else {
        // Use reverse domain notation: com.example.myproject -> com.example:myproject
        let parts: Vec<&str> = package_name.rsplitn(2, '.').collect();
        if parts.len() == 2 {
            (parts[1].to_string(), parts[0].to_string())
        } else {
            ("com.example".to_string(), package_name.to_string())
        }
    }
}

/// Parse dependency string into groupId, artifactId, and version
/// Format: "groupId:artifactId:version" or "groupId:artifactId"
fn parse_dependency(dep: &str) -> (String, String, String) {
    let parts: Vec<&str> = dep.split(':').collect();
    match parts.len() {
        3 => (
            parts[0].to_string(),
            parts[1].to_string(),
            parts[2].to_string(),
        ),
        2 => (parts[0].to_string(), parts[1].to_string(), "1.0.0".to_string()),
        _ => (
            "com.example".to_string(),
            dep.to_string(),
            "1.0.0".to_string(),
        ),
    }
}

/// Update a dependency in pom.xml content
fn update_dependency_in_pom(
    content: &str,
    old_artifact_id: &str,
    new_artifact_id: &str,
    new_version: Option<&str>,
) -> PluginResult<String> {
    let mut reader = Reader::from_str(content);
    reader.trim_text(true);

    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    let mut in_dependency = false;
    let mut in_artifact_id = false;
    let mut in_version = false;
    let mut current_artifact_id = String::new();
    let mut found_target_dep = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                if name.as_ref() == b"dependency" {
                    in_dependency = true;
                    current_artifact_id.clear();
                    found_target_dep = false;
                } else if in_dependency && name.as_ref() == b"artifactId" {
                    in_artifact_id = true;
                } else if in_dependency && name.as_ref() == b"version" {
                    in_version = true;
                }
                writer
                    .write_event(Event::Start(e))
                    .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                if name.as_ref() == b"dependency" {
                    in_dependency = false;
                } else if name.as_ref() == b"artifactId" {
                    in_artifact_id = false;
                } else if name.as_ref() == b"version" {
                    in_version = false;
                }
                writer
                    .write_event(Event::End(e))
                    .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().map_err(|e| {
                    PluginError::manifest(format!("Failed to unescape XML text: {}", e))
                })?;

                if in_artifact_id {
                    current_artifact_id = text.to_string();
                    if current_artifact_id == old_artifact_id {
                        found_target_dep = true;
                        // Replace artifactId
                        writer
                            .write_event(Event::Text(BytesText::new(new_artifact_id)))
                            .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;
                    } else {
                        writer
                            .write_event(Event::Text(e))
                            .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;
                    }
                } else if in_version && found_target_dep {
                    if let Some(new_ver) = new_version {
                        // Replace version
                        writer
                            .write_event(Event::Text(BytesText::new(new_ver)))
                            .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;
                    } else {
                        writer
                            .write_event(Event::Text(e))
                            .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;
                    }
                } else {
                    writer
                        .write_event(Event::Text(e))
                        .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;
                }
            }
            Ok(Event::Eof) => break,
            Ok(e) => {
                writer
                    .write_event(e)
                    .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;
            }
            Err(e) => {
                return Err(PluginError::manifest(format!("Failed to parse pom.xml: {}", e)));
            }
        }
        buf.clear();
    }

    let result = writer.into_inner().into_inner();
    String::from_utf8(result)
        .map_err(|e| PluginError::internal(format!("Invalid UTF-8 in pom.xml: {}", e)))
}

/// Add a dependency to pom.xml content
pub(crate) fn add_dependency_to_pom(
    content: &str,
    group_id: &str,
    artifact_id: &str,
    version: &str,
) -> PluginResult<String> {
    let mut reader = Reader::from_str(content);
    reader.trim_text(true);

    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    let mut in_dependencies = false;
    let mut dependencies_found = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                if name.as_ref() == b"dependencies" {
                    in_dependencies = true;
                    dependencies_found = true;
                }
                writer
                    .write_event(Event::Start(e))
                    .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                if name.as_ref() == b"dependencies" && in_dependencies {
                    // Add new dependency before closing dependencies tag
                    write_dependency(&mut writer, group_id, artifact_id, version)?;
                    in_dependencies = false;
                }
                writer
                    .write_event(Event::End(e))
                    .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;
            }
            Ok(Event::Eof) => break,
            Ok(e) => {
                writer
                    .write_event(e)
                    .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;
            }
            Err(e) => {
                return Err(PluginError::manifest(format!("Failed to parse pom.xml: {}", e)));
            }
        }
        buf.clear();
    }

    if !dependencies_found {
        return Err(PluginError::manifest(
            "No <dependencies> section found in pom.xml",
        ));
    }

    let result = writer.into_inner().into_inner();
    String::from_utf8(result)
        .map_err(|e| PluginError::internal(format!("Invalid UTF-8 in pom.xml: {}", e)))
}

/// Write a dependency element to XML writer
fn write_dependency<W: std::io::Write>(
    writer: &mut Writer<W>,
    group_id: &str,
    artifact_id: &str,
    version: &str,
) -> PluginResult<()> {
    // Write dependency element with proper indentation
    writer
        .write_event(Event::Text(BytesText::new("\n        ")))
        .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::Start(BytesStart::new("dependency")))
        .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;

    // groupId
    writer
        .write_event(Event::Text(BytesText::new("\n            ")))
        .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::Start(BytesStart::new("groupId")))
        .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::Text(BytesText::new(group_id)))
        .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::End(BytesEnd::new("groupId")))
        .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;

    // artifactId
    writer
        .write_event(Event::Text(BytesText::new("\n            ")))
        .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::Start(BytesStart::new("artifactId")))
        .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::Text(BytesText::new(artifact_id)))
        .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::End(BytesEnd::new("artifactId")))
        .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;

    // version
    writer
        .write_event(Event::Text(BytesText::new("\n            ")))
        .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::Start(BytesStart::new("version")))
        .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::Text(BytesText::new(version)))
        .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::End(BytesEnd::new("version")))
        .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;

    // Close dependency
    writer
        .write_event(Event::Text(BytesText::new("\n        ")))
        .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;
    writer
        .write_event(Event::End(BytesEnd::new("dependency")))
        .map_err(|e| PluginError::internal(format!("XML write error: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_POM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0">
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>my-app</artifactId>
    <version>1.0.0</version>
    <dependencies>
        <dependency>
            <groupId>org.junit.jupiter</groupId>
            <artifactId>junit-jupiter</artifactId>
            <version>5.9.0</version>
        </dependency>
    </dependencies>
</project>"#;

    #[test]
    fn test_parse_package_name_with_colon() {
        let (group, artifact) = parse_package_name("com.example:myproject");
        assert_eq!(group, "com.example");
        assert_eq!(artifact, "myproject");
    }

    #[test]
    fn test_parse_package_name_without_colon() {
        let (group, artifact) = parse_package_name("com.example.myproject");
        assert_eq!(group, "com.example");
        assert_eq!(artifact, "myproject");
    }

    #[test]
    fn test_parse_dependency_full() {
        let (group, artifact, version) = parse_dependency("org.junit:junit-jupiter:5.9.0");
        assert_eq!(group, "org.junit");
        assert_eq!(artifact, "junit-jupiter");
        assert_eq!(version, "5.9.0");
    }

    #[test]
    fn test_parse_dependency_no_version() {
        let (group, artifact, version) = parse_dependency("org.junit:junit-jupiter");
        assert_eq!(group, "org.junit");
        assert_eq!(artifact, "junit-jupiter");
        assert_eq!(version, "1.0.0");
    }

    #[test]
    fn test_generate_manifest_simple() {
        let updater = JavaManifestUpdater;
        let pom = updater.generate_manifest("com.example:myapp", &[]);

        assert!(pom.contains("<groupId>com.example</groupId>"));
        assert!(pom.contains("<artifactId>myapp</artifactId>"));
        assert!(pom.contains("<version>1.0.0</version>"));
        assert!(pom.contains("<modelVersion>4.0.0</modelVersion>"));
    }

    #[test]
    fn test_generate_manifest_with_dependencies() {
        let updater = JavaManifestUpdater;
        let deps = vec![
            "org.junit.jupiter:junit-jupiter:5.9.0".to_string(),
            "com.google.guava:guava:31.1-jre".to_string(),
        ];
        let pom = updater.generate_manifest("com.example:myapp", &deps);

        assert!(pom.contains("<dependencies>"));
        assert!(pom.contains("<groupId>org.junit.jupiter</groupId>"));
        assert!(pom.contains("<artifactId>junit-jupiter</artifactId>"));
        assert!(pom.contains("<version>5.9.0</version>"));
        assert!(pom.contains("<groupId>com.google.guava</groupId>"));
        assert!(pom.contains("<artifactId>guava</artifactId>"));
        assert!(pom.contains("<version>31.1-jre</version>"));
    }

    #[test]
    fn test_update_dependency_artifact_id() {
        let result =
            update_dependency_in_pom(SIMPLE_POM, "junit-jupiter", "junit-jupiter-api", None)
                .unwrap();

        assert!(result.contains("<artifactId>junit-jupiter-api</artifactId>"));
        assert!(!result.contains("<artifactId>junit-jupiter</artifactId>"));
        // Version should remain unchanged
        assert!(result.contains("<version>5.9.0</version>"));
    }

    #[test]
    fn test_update_dependency_version() {
        let result =
            update_dependency_in_pom(SIMPLE_POM, "junit-jupiter", "junit-jupiter", Some("5.10.0"))
                .unwrap();

        assert!(result.contains("<artifactId>junit-jupiter</artifactId>"));
        assert!(result.contains("<version>5.10.0</version>"));
        assert!(!result.contains("<version>5.9.0</version>"));
    }

    #[test]
    fn test_update_dependency_both() {
        let result = update_dependency_in_pom(
            SIMPLE_POM,
            "junit-jupiter",
            "junit-jupiter-api",
            Some("5.10.0"),
        )
        .unwrap();

        assert!(result.contains("<artifactId>junit-jupiter-api</artifactId>"));
        assert!(result.contains("<version>5.10.0</version>"));
    }

    #[test]
    fn test_add_dependency() {
        let result =
            add_dependency_to_pom(SIMPLE_POM, "com.google.guava", "guava", "31.1-jre").unwrap();

        assert!(result.contains("<groupId>com.google.guava</groupId>"));
        assert!(result.contains("<artifactId>guava</artifactId>"));
        assert!(result.contains("<version>31.1-jre</version>"));
        // Original dependency should still exist
        assert!(result.contains("<artifactId>junit-jupiter</artifactId>"));
    }

    #[test]
    fn test_update_dependency_not_found() {
        // Should work without error even if dependency not found (no-op)
        let result = update_dependency_in_pom(SIMPLE_POM, "nonexistent", "new-name", None);
        assert!(result.is_ok());
        let content = result.unwrap();
        // Original should be preserved
        assert!(content.contains("<artifactId>junit-jupiter</artifactId>"));
    }

    #[test]
    fn test_add_dependency_no_dependencies_section() {
        let pom_without_deps = r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0">
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>my-app</artifactId>
</project>"#;

        let result = add_dependency_to_pom(pom_without_deps, "org.junit", "junit", "4.13.2");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No <dependencies> section"));
    }
}
