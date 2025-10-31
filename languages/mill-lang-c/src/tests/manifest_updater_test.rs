use crate::CPlugin;
use mill_plugin_api::{LanguagePlugin, ManifestUpdater};
use std::fs;
use tempfile::tempdir;

const MAKEFILE_CONTENT: &str = "LIBS = -lm";

#[tokio::test]
async fn test_update_dependency() {
    let plugin = CPlugin::default();
    let updater = plugin.manifest_updater().unwrap();

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("Makefile");
    fs::write(&file_path, MAKEFILE_CONTENT).unwrap();

    let new_content = updater
        .update_dependency(&file_path, "m", "curl", None)
        .await
        .unwrap();

    assert_eq!(new_content, "LIBS = -lm -lcurl");
}

#[test]
fn test_generate_manifest() {
    let plugin = CPlugin::default();
    let updater = plugin.manifest_updater().unwrap();

    let content = updater.generate_manifest("my_program", &["m".to_string(), "curl".to_string()]);

    assert!(content.contains("TARGET = my_program"));
    assert!(content.contains("LIBS = -lm -lcurl"));
}