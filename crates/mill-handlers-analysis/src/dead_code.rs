//! Dead code analysis handler.
//!
//! Thin wrapper around mill-analysis-dead-code that exposes the MCP tool interface.

use crate::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use serde_json::{json, Value};
use tracing::info;

/// Handler for the `analyze.dead_code` MCP tool.
pub struct DeadCodeHandler;

impl Default for DeadCodeHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl DeadCodeHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ToolHandler for DeadCodeHandler {
    fn tool_names(&self) -> &[&str] {
        &["analyze.dead_code"]
    }

    fn is_internal(&self) -> bool {
        false // PUBLIC tool
    }

    #[cfg(feature = "analysis-dead-code")]
    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        use crate::lsp_provider_adapter::LspProviderAdapter;
        use mill_analysis_dead_code::{Config, DeadCodeAnalyzer, EntryPoints};
        use std::path::Path;

        let args = tool_call.arguments.clone().unwrap_or(json!({}));

        // Extract path from scope
        let path = args
            .get("scope")
            .and_then(|s| s.get("path"))
            .and_then(|p| p.as_str())
            .or_else(|| args.get("path").and_then(|p| p.as_str()))
            .ok_or_else(|| ServerError::invalid_request("Missing 'path' in scope or arguments"))?;

        let path = Path::new(path);

        // Get file extension for LSP adapter (default to Rust)
        let file_extension = args
            .get("file_extension")
            .and_then(|v| v.as_str())
            .unwrap_or("rs")
            .to_string();

        info!(
            path = %path.display(),
            extension = %file_extension,
            "Starting dead code analysis"
        );

        // Create LSP provider adapter
        let lsp_adapter = LspProviderAdapter::new(context.lsp_adapter.clone(), file_extension);

        // Build config from args
        let config = Config {
            entry_points: EntryPoints {
                include_main: args
                    .get("include_main")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                include_tests: args
                    .get("include_tests")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                include_pub_exports: args
                    .get("include_pub_exports")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                custom: args
                    .get("entry_points")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
            },
            min_confidence: args
                .get("min_confidence")
                .and_then(|v| v.as_f64())
                .map(|f| f as f32)
                .unwrap_or(0.7),
            file_extensions: args.get("file_extensions").and_then(|v| v.as_array()).map(
                |arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                },
            ),
            max_symbols: args
                .get("max_symbols")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize),
        };

        // Run analysis
        let report = DeadCodeAnalyzer::analyze(&lsp_adapter, path, config)
            .await
            .map_err(|e| ServerError::analysis(format!("Dead code analysis failed: {}", e)))?;

        info!(
            dead_found = report.stats.dead_found,
            files_analyzed = report.stats.files_analyzed,
            duration_ms = report.stats.duration_ms,
            "Dead code analysis complete"
        );

        serde_json::to_value(report)
            .map_err(|e| ServerError::internal(format!("Failed to serialize result: {}", e)))
    }

    #[cfg(not(feature = "analysis-dead-code"))]
    async fn handle_tool_call(
        &self,
        _context: &ToolHandlerContext,
        _tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        Err(ServerError::not_supported(
            "Dead code analysis requires the 'analysis-dead-code' feature to be enabled."
                .to_string(),
        ))
    }
}
