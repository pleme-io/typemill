//! Swift manifest file parsing
//!
//! Handles Package.swift files for Swift projects.
use cb_lang_common::ErrorBuilder;
use cb_plugin_api::{
    Dependency, DependencySource, ManifestData, PluginError, PluginResult,
};
use serde::Deserialize;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, error, warn};

#[derive(Deserialize, Debug)]
struct SwiftManifest {
    name: String,
    dependencies: Vec<SwiftDependency>,
    platforms: Vec<Platform>,
}

#[derive(Deserialize, Debug)]
struct Platform {
    #[serde(rename = "platformName")]
    platform_name: String,
    version: String,
}

#[derive(Deserialize, Debug)]
struct SwiftDependency {
    #[serde(rename = "sourceControl")]
    source_control: Vec<SourceControl>,
}

#[derive(Deserialize, Debug)]
struct SourceControl {
    identity: String,
    location: SourceControlLocation,
    requirement: Requirement,
}

#[derive(Deserialize, Debug)]
struct SourceControlLocation {
    remote: Vec<Remote>,
}

#[derive(Deserialize, Debug)]
struct Remote {
    url: String,
}

#[derive(Deserialize, Debug)]
struct Requirement {
    range: Vec<Range>,
}

#[derive(Deserialize, Debug)]
struct Range {
    #[serde(rename = "lowerBound")]
    lower_bound: String,
    #[serde(rename = "upperBound")]
    upper_bound: String,
}

/// Analyze Swift manifest file (`Package.swift`) by calling `swift package dump-package`.
pub async fn analyze_manifest(path: &Path) -> PluginResult<ManifestData> {
    let parent_dir = match path.parent() {
        Some(p) => p,
        None => {
            return Err(ErrorBuilder::manifest("Invalid manifest path")
                .with_path(path)
                .build())
        }
    };

    debug!(
        manifest_path = %path.display(),
        "Analyzing Swift manifest by running `swift package dump-package`"
    );

    let output = Command::new("swift")
        .arg("package")
        .arg("dump-package")
        .current_dir(parent_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to execute `swift package dump-package`");
            ErrorBuilder::manifest("Failed to execute `swift` command. Is the Swift toolchain installed and in your PATH?")
                .with_path(path)
                .with_context("io_error", e.to_string())
                .build()
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!(
            stderr = %stderr,
            "The `swift package dump-package` command failed"
        );
        return Err(ErrorBuilder::manifest("`swift package dump-package` failed")
            .with_path(path)
            .with_context("stderr", stderr.to_string())
            .build());
    }

    let manifest: SwiftManifest = serde_json::from_slice(&output.stdout).map_err(|e| {
        error!(error = %e, "Failed to deserialize Swift manifest JSON");
        ErrorBuilder::manifest("Failed to parse JSON from `swift package dump-package`")
            .with_path(path)
            .with_context("serde_error", e.to_string())
            .build()
    })?;

    let dependencies = manifest
        .dependencies
        .into_iter()
        .flat_map(|dep| dep.source_control)
        .map(|sc| {
            let version = sc
                .requirement
                .range
                .first()
                .map(|r| format!("{}..<{}", r.lower_bound, r.upper_bound))
                .unwrap_or_else(|| "any".to_string());

            Dependency {
                name: sc.identity,
                source: DependencySource::Version(version),
            }
        })
        .collect();

    let version = manifest.platforms.first().map(|p| p.version.clone()).unwrap_or_else(|| "0.0.0".to_string());

    Ok(ManifestData {
        name: manifest.name,
        version,
        dependencies,
        dev_dependencies: vec![], // Swift packages don't have a separate dev dependencies section
        raw_data: serde_json::from_slice(&output.stdout).map_err(|e| {
            error!(
                error = %e,
                "Failed to deserialize raw_data from Swift manifest JSON"
            );
            ErrorBuilder::manifest(
                "Failed to parse raw_data JSON from `swift package dump-package`",
            )
            .with_path(path)
            .with_context("serde_error", e.to_string())
            .build()
        })?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    const SAMPLE_PACKAGE_SWIFT: &str = r#"
// swift-tools-version:5.3
import PackageDescription

let package = Package(
    name: "MyAwesomePackage",
    platforms: [
        .macOS(.v10_15)
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-argument-parser.git", from: "1.0.0"),
    ],
    targets: [
        .target(
            name: "MyAwesomePackage",
            dependencies: [.product(name: "ArgumentParser", package: "swift-argument-parser")]),
        .testTarget(
            name: "MyAwesomePackageTests",
            dependencies: ["MyAwesomePackage"]),
    ]
)
"#;

    #[tokio::test]
    async fn test_analyze_valid_manifest() {
        // This test requires the `swift` command-line tool to be installed.
        let swift_cli_exists = Command::new("swift")
            .arg("--version")
            .output()
            .await
            .is_ok();

        if !swift_cli_exists {
            warn!("`swift` command not found, skipping test_analyze_valid_manifest");
            return;
        }

        // DEBUGGING: Print the PATH and find swift
        println!("PATH inside test: {}", std::env::var("PATH").unwrap_or_default());
        let _ = Command::new("which").arg("swift").status().await;

        let temp_dir = tempdir().unwrap();
        let package_path = temp_dir.path();

        // Create a valid Swift package structure
        fs::create_dir_all(package_path.join("Sources/MyAwesomePackage")).unwrap();
        fs::write(package_path.join("Sources/MyAwesomePackage/MyAwesomePackage.swift"), "public struct MyAwesomePackage {}").unwrap();

        let manifest_path = package_path.join("Package.swift");
        let mut temp_file = fs::File::create(&manifest_path).unwrap();
        writeln!(temp_file, "{}", SAMPLE_PACKAGE_SWIFT).unwrap();

        let result = analyze_manifest(&manifest_path).await;

        assert!(result.is_ok(), "analyze_manifest failed: {:?}", result.err());

        let manifest_data = result.unwrap();

        assert_eq!(manifest_data.name, "myawesomepackage");
        assert_eq!(manifest_data.version, "10.15");
        assert_eq!(manifest_data.dependencies.len(), 1);

        let dep = &manifest_data.dependencies[0];
        assert_eq!(dep.name, "swift-argument-parser");
    }

    #[tokio::test]
    async fn test_analyze_nonexistent_manifest() {
        let result = analyze_manifest(Path::new("/nonexistent/Package.swift")).await;
        assert!(result.is_err());
    }
}