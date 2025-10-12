// analysis/cb-analysis-deep-dead-code/tests/deep_dead_code_test.rs

use async_trait::async_trait;
use cb_analysis_common::{AnalysisEngine, AnalysisError, LspProvider};
use cb_analysis_deep_dead_code::{DeepDeadCodeAnalyzer, DeepDeadCodeConfig};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

struct MockLspProvider {
    symbols: Vec<Value>,
    references: HashMap<String, Vec<Value>>,
}

#[async_trait]
impl LspProvider for MockLspProvider {
    async fn workspace_symbols(&self, _query: &str) -> Result<Vec<Value>, AnalysisError> {
        Ok(self.symbols.clone())
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

fn create_symbol(uri: &str, name: &str, line: u32, kind: u64) -> Value {
    json!({
        "name": name,
        "kind": kind,
        "location": {
            "uri": uri,
            "range": {
                "start": { "line": line, "character": 0 },
                "end": { "line": line, "character": 10 }
            }
        }
    })
}

fn create_location(uri: &str, line: u32) -> Value {
    json!({
        "uri": uri,
        "range": {
            "start": { "line": line, "character": 0 },
            "end": { "line": line, "character": 10 }
        }
    })
}

#[tokio::test]
async fn test_deep_dead_code_analysis() {
    let main_uri = "file:///main.rs";
    let lib_uri = "file:///lib.rs";

    let symbols = vec![
        create_symbol(main_uri, "main", 0, 12),      // public function
        create_symbol(lib_uri, "used_function", 2, 12), // public function
        create_symbol(lib_uri, "unused_function", 5, 12), // public function
    ];

    let mut references = HashMap::new();
    references.insert(
        format!("{}@L2", lib_uri), // used_function
        vec![
            create_location(main_uri, 0), // main references used_function (within main's range)
        ],
    );
    references.insert(
        format!("{}@L5", lib_uri), // unused_function
        vec![], // No references
    );

    let mock_lsp = Arc::new(MockLspProvider { symbols, references });
    let analyzer = DeepDeadCodeAnalyzer;
    let config = DeepDeadCodeConfig::default();
    let workspace_path = Path::new(".");

    let result = analyzer.analyze(mock_lsp, workspace_path, config).await;

    assert!(result.is_ok());
    let report = result.unwrap();

    // In default (non-aggressive) mode, public symbols are entry points, so nothing should be dead.
    assert_eq!(report.dead_symbols.len(), 0);
}

#[tokio::test]
async fn test_deep_dead_code_analysis_with_aggressive_mode() {
    let main_uri = "file:///main.rs";
    let lib_uri = "file:///lib.rs";

    let symbols = vec![
        create_symbol(main_uri, "main", 0, 12), // public function
        create_symbol(lib_uri, "uncalled_public_function", 2, 12), // public function
    ];

    let references = HashMap::new();

    let mock_lsp = Arc::new(MockLspProvider {
        symbols,
        references,
    });
    let analyzer = DeepDeadCodeAnalyzer;
    let workspace_path = Path::new(".");

    // Test with aggressive mode enabled
    let aggressive_config = DeepDeadCodeConfig {
        check_public_exports: true,
        ..Default::default()
    };
    let result = analyzer
        .analyze(
            mock_lsp.clone(),
            workspace_path,
            aggressive_config,
        )
        .await;
    assert!(result.is_ok());
    let report = result.unwrap();
    assert_eq!(report.dead_symbols.len(), 1);
    assert_eq!(report.dead_symbols[0].name, "uncalled_public_function");

    // Test with aggressive mode disabled
    let default_config = DeepDeadCodeConfig::default();
    let result = analyzer
        .analyze(mock_lsp, workspace_path, default_config)
        .await;
    assert!(result.is_ok());
    let report = result.unwrap();
    assert_eq!(report.dead_symbols.len(), 0);
}