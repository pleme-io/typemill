use super::manifest::extract_dependencies;
use mill_plugin_api::{
    LanguagePlugin, LanguageMetadata, PluginCapabilities, PluginResult, ParsedSource, ManifestData, ImportParser,
};
use async_trait::async_trait;
use std::path::{Path};
use tempfile::tempdir;
use tokio::fs;
use std::time::Instant;

struct MockImportParser;

impl ImportParser for MockImportParser {
    fn parse_imports(&self, content: &str) -> Vec<String> {
        // Simulate some work
        let mut deps = Vec::new();
        for line in content.lines() {
            if line.starts_with("import ") {
                deps.push(line.trim_start_matches("import ").to_string());
            }
        }
        // Simulate CPU load
        let _ = (0..1000).map(|i| i * i).sum::<i32>();
        deps
    }

    fn contains_import(&self, _content: &str, _module: &str) -> bool {
        false
    }
}

struct MockPlugin;

#[async_trait]
impl LanguagePlugin for MockPlugin {
    fn metadata(&self) -> &LanguageMetadata {
        unimplemented!()
    }

    async fn parse(&self, _source: &str) -> PluginResult<ParsedSource> {
        unimplemented!()
    }

    async fn analyze_manifest(&self, _path: &Path) -> PluginResult<ManifestData> {
        unimplemented!()
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::default()
    }

    fn import_parser(&self) -> Option<&dyn ImportParser> {
        Some(&MockImportParser)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[tokio::test]
async fn benchmark_extract_dependencies() {
    let temp_dir = tempdir().unwrap();
    let num_files = 500;
    let mut files = Vec::new();

    // Create many files
    for i in 0..num_files {
        let file_path = temp_dir.path().join(format!("file_{}.txt", i));
        fs::write(&file_path, format!("import dep_{};\nimport common_dep;", i)).await.unwrap();
        files.push(file_path);
    }

    let plugin = MockPlugin;

    let start = Instant::now();
    let deps = extract_dependencies(&plugin, &files).await;
    let duration = start.elapsed();

    println!("BENCHMARK_RESULT: Time taken for {} files: {:?}", num_files, duration);
    assert!(deps.len() > 0);
}
