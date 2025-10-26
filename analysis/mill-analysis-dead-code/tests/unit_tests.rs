//! Unit tests for the dead code analysis crate.

use async_trait::async_trait;
use mill_analysis_common::{AnalysisEngine, AnalysisError, LspProvider};
use mill_analysis_dead_code::{config::DeadCodeConfig, DeadCodeAnalyzer};
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

/// A mock LSP provider for testing purposes.
struct MockLspProvider;

#[async_trait]
impl LspProvider for MockLspProvider {
    async fn workspace_symbols(&self, _query: &str) -> Result<Vec<Value>, AnalysisError> {
        Ok(vec![])
    }

    async fn find_references(
        &self,
        _uri: &str,
        _line: u32,
        _character: u32,
    ) -> Result<Vec<Value>, AnalysisError> {
        Ok(vec![])
    }

    async fn document_symbols(&self, _uri: &str) -> Result<Vec<Value>, AnalysisError> {
        Ok(vec![])
    }
}

#[tokio::test]
async fn test_analyzer_runs_without_error() {
    let mock_lsp = Arc::new(MockLspProvider);
    let analyzer = DeadCodeAnalyzer;
    let config = DeadCodeConfig::default();
    let workspace_path = Path::new(".");

    let result = analyzer.analyze(mock_lsp, workspace_path, config).await;

    assert!(result.is_ok(), "Analysis should not fail");
    let report = result.unwrap();
    assert_eq!(
        report.dead_symbols.len(),
        0,
        "Should find no dead symbols in an empty workspace"
    );
}
