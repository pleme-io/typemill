// analysis/mill-analysis-deep-dead-code/tests/deep_dead_code_test.rs

use async_trait::async_trait;
use mill_analysis_common::{ AnalysisEngine , AnalysisError , LspProvider };
use mill_analysis_deep_dead_code::{ DeepDeadCodeAnalyzer , DeepDeadCodeConfig };
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::{tempdir, TempDir};

struct MockLspProvider {
    references: HashMap<String, Vec<Value>>,
}

#[async_trait]
impl LspProvider for MockLspProvider {
    async fn workspace_symbols(&self, _query: &str) -> Result<Vec<Value>, AnalysisError> {
        // This is no longer used by the AST-based approach for .rs files.
        Ok(vec![])
    }

    async fn find_references(
        &self,
        uri: &str,
        line: u32,
        _character: u32,
    ) -> Result<Vec<Value>, AnalysisError> {
        let key = format!("{}@L{}", uri, line);
        Ok(self.references.get(&key).cloned().unwrap_or_default())
    }

    async fn document_symbols(&self, _uri: &str) -> Result<Vec<Value>, AnalysisError> {
        Ok(vec![])
    }
}

/// A helper struct to manage the temporary test workspace.
struct TestWorkspace {
    dir: TempDir,
    files: HashMap<String, PathBuf>,
}

impl TestWorkspace {
    fn new() -> Self {
        Self {
            dir: tempdir().unwrap(),
            files: HashMap::new(),
        }
    }

    fn add_file(&mut self, name: &str, content: &str) {
        let path = self.dir.path().join(name);
        fs::write(&path, content).unwrap();
        self.files.insert(name.to_string(), path);
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn file_uri(&self, name: &str) -> String {
        format!("file://{}", self.files[name].to_str().unwrap())
    }

    fn create_location(&self, file_name: &str, line: u32, character: u32) -> Value {
        serde_json::json!({
            "uri": self.file_uri(file_name),
            "range": {
                "start": { "line": line, "character": character },
                "end": { "line": line, "character": character + 1 }
            }
        })
    }
}

#[tokio::test]
async fn test_deep_dead_code_analysis() {
    let mut workspace = TestWorkspace::new();
    workspace.add_file("main.rs", "mod lib; fn main() { lib::used_function(); }");
    workspace.add_file(
        "lib.rs",
        "pub fn used_function() {}\npub fn unused_function() {}",
    );

    let mut references = HashMap::new();
    references.insert(
        format!("{}@L0", workspace.file_uri("lib.rs")), // used_function
        vec![workspace.create_location("main.rs", 0, 29)],
    );

    let mock_lsp = Arc::new(MockLspProvider { references });
    let analyzer = DeepDeadCodeAnalyzer;
    let config = DeepDeadCodeConfig::default();

    let result = analyzer
        .analyze(mock_lsp, workspace.path(), config)
        .await
        .unwrap();

    // In default (non-aggressive) mode, `main` and all public symbols are entry points.
    // The only unreferenced symbol is the `mod lib` declaration itself.
    assert_eq!(result.dead_symbols.len(), 1);
    assert_eq!(result.dead_symbols[0].name, "lib");
}

#[tokio::test]
async fn test_deep_dead_code_analysis_with_aggressive_mode() {
    let mut workspace = TestWorkspace::new();
    workspace.add_file("main.rs", "mod lib; fn main() {}");
    workspace.add_file("lib.rs", "pub fn uncalled_public_function() {}");

    let mock_lsp = Arc::new(MockLspProvider {
        references: HashMap::new(),
    });

    let analyzer = DeepDeadCodeAnalyzer;
    let aggressive_config = DeepDeadCodeConfig {
        check_public_exports: true,
        ..Default::default()
    };
    let result = analyzer
        .analyze(mock_lsp, workspace.path(), aggressive_config)
        .await
        .unwrap();

    // In aggressive mode, only `main` is an entry point.
    // `uncalled_public_function` and the `lib` module declaration are dead.
    assert_eq!(result.dead_symbols.len(), 2);
    let dead_names: HashSet<_> = result.dead_symbols.iter().map(|s| &s.name).collect();
    assert!(dead_names.contains(&"uncalled_public_function".to_string()));
    assert!(dead_names.contains(&"lib".to_string()));
}

#[tokio::test]
async fn test_deep_dead_code_with_ast_extractor() {
    let mut workspace = TestWorkspace::new();
    workspace.add_file("main.rs", "mod lib; fn main() { lib::used_function(); }");
    workspace.add_file(
        "lib.rs",
        "pub fn used_function() {}\npub fn unused_function() {}",
    );

    let mut references = HashMap::new();
    references.insert(
        format!("{}@L0", workspace.file_uri("lib.rs")), // used_function
        vec![workspace.create_location("main.rs", 0, 29)],
    );

    let mock_lsp = Arc::new(MockLspProvider { references });
    let analyzer = DeepDeadCodeAnalyzer;
    let config = DeepDeadCodeConfig {
        check_public_exports: true, // Aggressive mode
        ..Default::default()
    };

    let result = analyzer
        .analyze(mock_lsp, workspace.path(), config)
        .await
        .unwrap();

    // In aggressive mode, `main` is the entry, it calls `used_function`.
    // `unused_function` and the `lib` module declaration are dead.
    assert_eq!(result.dead_symbols.len(), 2);
    let dead_names: HashSet<_> = result.dead_symbols.iter().map(|s| &s.name).collect();
    assert!(dead_names.contains(&"unused_function".to_string()));
    assert!(dead_names.contains(&"lib".to_string()));
}