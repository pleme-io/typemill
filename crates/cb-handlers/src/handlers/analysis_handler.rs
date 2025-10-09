//! Code analysis tool handler
//!
//! Handles: find_dead_code
//!
//! This module contains deep static analysis tools that examine code quality,
//! identify unused code, and provide insights into codebase health.

use super::tools::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
use serde_json::Value;
use tracing::debug;

// Feature-gated implementation module for the new analysis subsystem.
#[cfg(feature = "analysis-dead-code")]
mod analysis_impl {
    use super::super::lsp_adapter::DirectLspAdapter;
    use super::super::tools::ToolHandlerContext;
    use async_trait::async_trait;
    use cb_analysis_common::{AnalysisEngine, AnalysisError, LspProvider};
    use cb_analysis_dead_code::{DeadCodeAnalyzer, DeadCodeConfig, DeadCodeReport};
    use cb_core::model::mcp::ToolCall;
    use cb_plugins::LspService;
    use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
    use serde_json::{json, Value};
    use std::path::Path;
    use std::sync::Arc;
    use tracing::debug;

    /// Adapter to make DirectLspAdapter compatible with LspProvider trait.
    pub struct DirectLspProviderAdapter {
        adapter: Arc<DirectLspAdapter>,
    }

    impl DirectLspProviderAdapter {
        pub fn new(adapter: Arc<DirectLspAdapter>) -> Self {
            Self { adapter }
        }
    }

    #[async_trait]
    impl LspProvider for DirectLspProviderAdapter {
        async fn workspace_symbols(&self, query: &str) -> Result<Vec<Value>, AnalysisError> {
            self.adapter
                .request("workspace/symbol", json!({ "query": query }))
                .await
                .map(|v| v.as_array().cloned().unwrap_or_default())
                .map_err(|e| AnalysisError::LspError(e.to_string()))
        }

        async fn find_references(
            &self,
            uri: &str,
            line: u32,
            character: u32,
        ) -> Result<Vec<Value>, AnalysisError> {
            let params = json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
                "context": { "includeDeclaration": true }
            });

            self.adapter
                .request("textDocument/references", params)
                .await
                .map(|v| v.as_array().cloned().unwrap_or_default())
                .map_err(|e| AnalysisError::LspError(e.to_string()))
        }

        async fn document_symbols(&self, uri: &str) -> Result<Vec<Value>, AnalysisError> {
            self.adapter
                .request(
                    "textDocument/documentSymbol",
                    json!({ "textDocument": { "uri": uri } }),
                )
                .await
                .map(|v| v.as_array().cloned().unwrap_or_default())
                .map_err(|e| AnalysisError::LspError(e.to_string()))
        }
    }

    /// Parses tool call arguments into the analysis configuration.
    fn config_from_params(args: &Value) -> DeadCodeConfig {
        let mut config = DeadCodeConfig::default();

        if let Some(file_types) = args.get("file_types").and_then(|v| v.as_array()) {
            let types: Vec<String> = file_types.iter().filter_map(|v| v.as_str().map(String::from)).collect();
            if !types.is_empty() {
                config.file_types = Some(types);
            }
        }

        if let Some(include_exported) = args.get("include_exported").and_then(|v| v.as_bool()) {
            config.include_exported = include_exported;
        }

        if let Some(max_results) = args.get("max_results").and_then(|v| v.as_u64()) {
            config.max_results = Some(max_results as usize);
        }

        if let Some(min_refs) = args.get("min_reference_threshold").and_then(|v| v.as_u64()) {
            config.min_reference_threshold = min_refs as usize;
        }

        if let Some(timeout_secs) = args.get("timeout_seconds").and_then(|v| v.as_u64()) {
            config.timeout = Some(std::time::Duration::from_secs(timeout_secs));
        }

        config
    }

    /// Formats the analysis report into the final JSON response for the user.
    fn format_mcp_response(
        report: DeadCodeReport,
        workspace_path: &Path,
    ) -> ServerResult<Value> {
        let dead_symbols_json: Vec<Value> = report
            .dead_symbols
            .iter()
            .map(|s| {
                json!({
                    "name": s.name,
                    "kind": s.kind,
                    "file": s.file_path,
                    "line": s.line,
                    "column": s.column,
                })
            })
            .collect();

        Ok(json!({
            "workspacePath": workspace_path.display().to_string(),
            "deadSymbols": dead_symbols_json,
            "analysisStats": report.stats,
        }))
    }

    /// The new implementation of find_dead_code using the analysis crate.
    pub async fn handle_find_dead_code_impl(
        tool_call: ToolCall,
        context: &ToolHandlerContext,
    ) -> ServerResult<Value> {
        let args = tool_call.arguments.unwrap_or_default();
        let workspace_path_str = args
            .get("workspace_path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        let workspace_path = Path::new(workspace_path_str);
        let config = config_from_params(&args);

        debug!(?config, "Handling find_dead_code request");

        let lsp_adapter_lock = context.lsp_adapter.lock().await;
        let lsp_adapter = lsp_adapter_lock
            .as_ref()
            .ok_or_else(|| ServerError::Internal("LSP adapter not initialized".to_string()))?
            .clone();

        let lsp_provider = Arc::new(DirectLspProviderAdapter::new(lsp_adapter));
        let analyzer = DeadCodeAnalyzer;
        let report = analyzer
            .analyze(lsp_provider, workspace_path, config)
            .await
            .map_err(|e| ServerError::Internal(e.to_string()))?;

        format_mcp_response(report, workspace_path)
    }
}

pub struct AnalysisHandler;

impl AnalysisHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AnalysisHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for AnalysisHandler {
    fn tool_names(&self) -> &[&str] {
        &["find_dead_code"]
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling code analysis operation");

        match tool_call.name.as_str() {
            "find_dead_code" => self.handle_find_dead_code(tool_call.clone(), context).await,
            _ => Err(ServerError::Unsupported(format!(
                "Unknown analysis operation: {}",
                tool_call.name
            ))),
        }
    }
}

impl AnalysisHandler {
    #[cfg(feature = "analysis-dead-code")]
    async fn handle_find_dead_code(
        &self,
        tool_call: ToolCall,
        context: &ToolHandlerContext,
    ) -> ServerResult<Value> {
        analysis_impl::handle_find_dead_code_impl(tool_call, context).await
    }

    #[cfg(not(feature = "analysis-dead-code"))]
    async fn handle_find_dead_code(
        &self,
        _tool_call: ToolCall,
        _context: &ToolHandlerContext,
    ) -> ServerResult<Value> {
        Err(ServerError::Unsupported(
            "The 'find_dead_code' tool is not available because the 'analysis-dead-code' feature is not enabled.".to_string(),
        ))
    }
}