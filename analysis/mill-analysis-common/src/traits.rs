// analysis/mill-analysis-common/src/traits.rs

use crate::error::AnalysisError;
use crate::types::AnalysisMetadata;
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

/// Abstraction for LSP communication (dependency inversion)
#[async_trait]
pub trait LspProvider: Send + Sync {
    /// Query LSP workspace/symbol
    async fn workspace_symbols(&self, query: &str) -> Result<Vec<Value>, AnalysisError>;

    /// Query LSP textDocument/references
    async fn find_references(
        &self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<Value>, AnalysisError>;

    /// Query LSP textDocument/documentSymbol
    async fn document_symbols(&self, uri: &str) -> Result<Vec<Value>, AnalysisError>;

    /// Open a document in the LSP server (textDocument/didOpen)
    /// This is optional - default implementation does nothing
    async fn open_document(&self, _uri: &str, _content: &str) -> Result<(), AnalysisError> {
        Ok(()) // Default: no-op
    }
}

/// Core analysis engine trait
#[async_trait]
pub trait AnalysisEngine: Send + Sync {
    type Config;
    type Result;

    /// Run analysis with the given configuration
    async fn analyze(
        &self,
        lsp: Arc<dyn LspProvider>,
        workspace_path: &Path,
        config: Self::Config,
    ) -> Result<Self::Result, AnalysisError>;

    /// Get analysis metadata (name, version, capabilities)
    fn metadata(&self) -> AnalysisMetadata;
}
