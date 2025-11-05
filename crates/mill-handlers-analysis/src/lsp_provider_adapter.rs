//! Adapter to bridge mill-handler-api::LspAdapter to mill-analysis-common::LspProvider
//!
//! This adapter allows the analysis engine to use LSP services through the
//! handler API's LspAdapter trait abstraction.

use async_trait::async_trait;
use mill_analysis_common::{AnalysisError, LspProvider};
use mill_handler_api::LspAdapter;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;

/// Adapter that implements LspProvider using LspAdapter from handler API
pub struct LspProviderAdapter {
    lsp_adapter: Arc<Mutex<Option<Arc<dyn LspAdapter>>>>,
    /// File extension to use for LSP client (e.g., "rs", "ts")
    file_extension: String,
}

impl LspProviderAdapter {
    /// Create a new LspProviderAdapter
    pub fn new(
        lsp_adapter: Arc<Mutex<Option<Arc<dyn LspAdapter>>>>,
        file_extension: String,
    ) -> Self {
        Self {
            lsp_adapter,
            file_extension,
        }
    }

    /// Get the LSP client for the configured file extension
    async fn get_client(&self) -> Result<Arc<mill_lsp::lsp_system::LspClient>, AnalysisError> {
        let adapter_guard = self.lsp_adapter.lock().await;
        let adapter = adapter_guard
            .as_ref()
            .ok_or_else(|| AnalysisError::LspError("No LSP adapter available".to_string()))?;

        adapter
            .get_or_create_client(&self.file_extension)
            .await
            .map_err(|e| AnalysisError::LspError(format!("Failed to get LSP client: {}", e)))
    }
}

#[async_trait]
impl LspProvider for LspProviderAdapter {
    async fn workspace_symbols(&self, query: &str) -> Result<Vec<Value>, AnalysisError> {
        debug!(
            file_extension = %self.file_extension,
            query = %query,
            "LspProviderAdapter::workspace_symbols"
        );

        let client = self.get_client().await?;

        let params = json!({ "query": query });

        let response = client
            .send_request("workspace/symbol", params)
            .await
            .map_err(|e| AnalysisError::LspError(format!("workspace/symbol failed: {}", e)))?;

        // Extract symbols array from response
        let symbols = response
            .as_array()
            .cloned()
            .unwrap_or_default();

        debug!(
            symbols_count = symbols.len(),
            "workspace_symbols returned {} symbols",
            symbols.len()
        );

        Ok(symbols)
    }

    async fn find_references(
        &self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<Value>, AnalysisError> {
        debug!(
            uri = %uri,
            line = line,
            character = character,
            "LspProviderAdapter::find_references"
        );

        let client = self.get_client().await?;

        let params = json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
            "context": { "includeDeclaration": true }
        });

        let response = client
            .send_request("textDocument/references", params)
            .await
            .map_err(|e| {
                AnalysisError::LspError(format!("textDocument/references failed: {}", e))
            })?;

        // Extract references array from response
        let references = response
            .as_array()
            .cloned()
            .unwrap_or_default();

        debug!(
            references_count = references.len(),
            "find_references returned {} references",
            references.len()
        );

        Ok(references)
    }

    async fn document_symbols(&self, uri: &str) -> Result<Vec<Value>, AnalysisError> {
        debug!(
            uri = %uri,
            "LspProviderAdapter::document_symbols"
        );

        let client = self.get_client().await?;

        let params = json!({
            "textDocument": { "uri": uri }
        });

        let response = client
            .send_request("textDocument/documentSymbol", params)
            .await
            .map_err(|e| {
                AnalysisError::LspError(format!("textDocument/documentSymbol failed: {}", e))
            })?;

        // Extract symbols array from response
        let symbols = response
            .as_array()
            .cloned()
            .unwrap_or_default();

        debug!(
            symbols_count = symbols.len(),
            "document_symbols returned {} symbols",
            symbols.len()
        );

        Ok(symbols)
    }

    async fn open_document(&self, uri: &str, content: &str) -> Result<(), AnalysisError> {
        debug!(
            uri = %uri,
            content_length = content.len(),
            "LspProviderAdapter::open_document"
        );

        let client = self.get_client().await?;

        // Determine language ID from file extension
        let language_id = if uri.ends_with(".rs") {
            "rust"
        } else if uri.ends_with(".ts") || uri.ends_with(".tsx") {
            "typescript"
        } else if uri.ends_with(".js") || uri.ends_with(".jsx") {
            "javascript"
        } else {
            "plaintext"
        };

        let params = json!({
            "textDocument": {
                "uri": uri,
                "languageId": language_id,
                "version": 1,
                "text": content
            }
        });

        client
            .send_notification("textDocument/didOpen", params)
            .await
            .map_err(|e| {
                AnalysisError::LspError(format!("textDocument/didOpen failed: {}", e))
            })?;

        debug!(
            uri = %uri,
            "Successfully sent didOpen notification"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_adapter_creation() {
        // Test that adapter can be created with None LSP adapter
        let lsp_adapter: Arc<Mutex<Option<Arc<dyn LspAdapter>>>> = Arc::new(Mutex::new(None));
        let adapter = LspProviderAdapter::new(lsp_adapter, "rs".to_string());

        // Should fail to get client when no adapter is available
        let result = adapter.get_client().await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("No LSP adapter available"));
        }
    }

    #[tokio::test]
    async fn test_adapter_with_missing_client() {
        // Test that adapter properly handles missing LSP adapter
        let lsp_adapter: Arc<Mutex<Option<Arc<dyn LspAdapter>>>> = Arc::new(Mutex::new(None));
        let adapter = LspProviderAdapter::new(lsp_adapter, "rs".to_string());

        // workspace_symbols should fail gracefully
        let result = adapter.workspace_symbols("test").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), mill_analysis_common::AnalysisError::LspError(_)));
    }

    #[tokio::test]
    async fn test_adapter_with_missing_client_find_references() {
        // Test find_references with missing adapter
        let lsp_adapter: Arc<Mutex<Option<Arc<dyn LspAdapter>>>> = Arc::new(Mutex::new(None));
        let adapter = LspProviderAdapter::new(lsp_adapter, "rs".to_string());

        let result = adapter.find_references("file:///test.rs", 0, 0).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), mill_analysis_common::AnalysisError::LspError(_)));
    }

    #[tokio::test]
    async fn test_adapter_with_missing_client_document_symbols() {
        // Test document_symbols with missing adapter
        let lsp_adapter: Arc<Mutex<Option<Arc<dyn LspAdapter>>>> = Arc::new(Mutex::new(None));
        let adapter = LspProviderAdapter::new(lsp_adapter, "rs".to_string());

        let result = adapter.document_symbols("file:///test.rs").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), mill_analysis_common::AnalysisError::LspError(_)));
    }
}
