use mill_plugin_api::{CreatePackageConfig, CreatePackageResult, PluginError, PluginResult, ProjectFactory, PackageInfo, PackageType};
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Default)]
pub struct JavaProjectFactory;

impl JavaProjectFactory {
    pub fn new() -> Self {
        Self
    }
}

impl ProjectFactory for JavaProjectFactory {
    fn create_package(
        &self,
        config: &CreatePackageConfig,
    ) -> PluginResult<CreatePackageResult> {
        let package_dir = Path::new(&config.package_path);
        fs::create_dir_all(package_dir)
            .map_err(|e| PluginError::internal(format!("Failed to create package directory: {}", e)))?;

        let manifest_content;
        let manifest_filename;
        let package_name = package_dir
            .file_name()
            .ok_or_else(|| PluginError::invalid_input("Invalid package path"))?
            .to_str()
            .ok_or_else(|| PluginError::invalid_input("Invalid package path"))?;

        match config.package_type {
            PackageType::Library => {
                manifest_filename = "pom.xml";
                manifest_content = generate_pom_xml(package_name);
            }
            PackageType::Binary => {
                manifest_filename = "build.gradle";
                manifest_content = generate_build_gradle();
            }
        }

        let manifest_path = package_dir.join(manifest_filename);
        let mut file = fs::File::create(&manifest_path)
            .map_err(|e| PluginError::internal(format!("Failed to create manifest file: {}", e)))?;
        file.write_all(manifest_content.as_bytes())
            .map_err(|e| PluginError::internal(format!("Failed to write to manifest file: {}", e)))?;

        // Create standard directory structure
        let src_main_java = package_dir.join("src/main/java");
        let src_test_java = package_dir.join("src/test/java");
        fs::create_dir_all(&src_main_java).map_err(|e| PluginError::internal(format!("Failed to create src/main/java: {}", e)))?;
        fs::create_dir_all(&src_test_java).map_err(|e| PluginError::internal(format!("Failed to create src/test/java: {}", e)))?;

        Ok(CreatePackageResult {
            created_files: vec![manifest_path.to_string_lossy().into_owned()],
            workspace_updated: false,
            package_info: PackageInfo {
                name: package_name.to_string(),
                version: "1.0-SNAPSHOT".to_string(),
                manifest_path: manifest_path.to_string_lossy().into_owned(),
            },
        })
    }
}

fn generate_pom_xml(package_name: &str) -> String {
    format!(
        r#"<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>

    <groupId>com.example</groupId>
    <artifactId>{}</artifactId>
    <version>1.0-SNAPSHOT</version>
    <packaging>jar</packaging>

    <properties>
        <project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>
        <maven.compiler.source>1.8</maven.compiler.source>
        <maven.compiler.target>1.8</maven.compiler.target>
    </properties>

    <dependencies>
        <dependency>
            <groupId>junit</groupId>
            <artifactId>junit</artifactId>
            <version>4.11</version>
            <scope>test</scope>
        </dependency>
    </dependencies>
</project>"#,
        package_name
    )
}

fn generate_build_gradle() -> String {
    r#"plugins {
    id 'java'
    id 'application'
}

repositories {
    mavenCentral()
}

dependencies {
    testImplementation 'org.junit.jupiter:junit-jupiter-api:5.8.1'
    testRuntimeOnly 'org.junit.jupiter:junit-jupiter-engine:5.8.1'
}

application {
    mainClass = 'com.example.App'
}

tasks.named('test') {
    useJUnitPlatform()
}
"#.to_string()
}