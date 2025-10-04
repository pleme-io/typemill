//! Code analysis tool handler
//!
//! Handles: find_dead_code
//!
//! This module contains deep static analysis tools that examine code quality,
//! identify unused code, and provide insights into codebase health.

use super::compat::{ToolContext, ToolHandler};
use super::lsp_adapter::DirectLspAdapter;
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_plugins::LspService;
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};

// ============================================================================
// Dead Code Analysis - Configuration & Types
// ============================================================================

/// Configuration for dead code analysis
#[derive(Debug, Clone)]
struct AnalysisConfig {
    /// Maximum number of concurrent LSP reference checks
    max_concurrent_checks: usize,
    /// Symbol kinds to analyze (LSP SymbolKind numbers)
    analyzed_kinds: Vec<u64>,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            max_concurrent_checks: 20,
            // LSP SymbolKind: Function=12, Class=5, Method=6, Interface=11
            analyzed_kinds: vec![5, 6, 11, 12],
        }
    }
}

/// Result of dead code analysis
#[derive(Debug, Clone)]
struct DeadSymbol {
    name: String,
    kind: String,
    file_path: String,
    line: u32,
    column: u32,
    reference_count: usize,
}

// ============================================================================
// Dead Code Analysis - Core Algorithm
// ============================================================================

/// Analyze workspace for dead code using a reference counting approach.
///
/// This uses the following algorithm:
/// 1. Collect all symbols from workspace via LSP workspace/symbol
/// 2. Filter to analyzable symbols (functions, classes, methods, interfaces)
/// 3. Check references for each symbol via LSP textDocument/references
/// 4. Symbols with ≤1 reference (just the declaration) are considered dead
async fn analyze_dead_code(
    lsp_adapter: Arc<DirectLspAdapter>,
    _workspace_path: &str,
    config: AnalysisConfig,
) -> ServerResult<Vec<DeadSymbol>> {
    let all_symbols = collect_workspace_symbols(&lsp_adapter).await?;
    debug!(
        total_symbols = all_symbols.len(),
        "Collected symbols from language servers"
    );

    if all_symbols.is_empty() {
        return Ok(Vec::new());
    }

    let symbols_to_check: Vec<_> = all_symbols
        .iter()
        .filter(|s| should_analyze_symbol(s, &config))
        .collect();
    debug!(
        symbols_to_check = symbols_to_check.len(),
        "Filtered to analyzable symbols"
    );

    let dead_symbols =
        check_symbol_references(&lsp_adapter, symbols_to_check, config.max_concurrent_checks)
            .await?;

    info!(
        dead_symbols_found = dead_symbols.len(),
        "Dead code analysis complete"
    );

    Ok(dead_symbols)
}

/// Collect workspace symbols using the shared LSP adapter
///
/// Note: Some LSP servers (like rust-analyzer) don't support empty workspace/symbol queries
/// and will return 0 symbols. This is a known limitation.
async fn collect_workspace_symbols(
    lsp_adapter: &Arc<DirectLspAdapter>,
) -> ServerResult<Vec<Value>> {
    // Use the adapter's built-in method to query all servers
    // Try with wildcard query first (more compatible)
    let query_attempts = vec!["*", ""];

    for query in query_attempts {
        match lsp_adapter
            .request("workspace/symbol", json!({ "query": query }))
            .await
        {
            Ok(response) => {
                if let Some(symbols) = response.as_array() {
                    if !symbols.is_empty() {
                        debug!(
                            symbol_count = symbols.len(),
                            query = query,
                            "Collected symbols from workspace"
                        );
                        return Ok(symbols.clone());
                    }
                }
            }
            Err(e) => {
                debug!(
                    error = %e,
                    query = query,
                    "Failed to get workspace symbols with query"
                );
            }
        }
    }

    warn!(
        "No workspace symbols found. Note: Some LSP servers (like rust-analyzer) don't support workspace/symbol queries and will return 0 symbols."
    );
    Ok(Vec::new())
}

/// Check if a symbol should be analyzed based on configuration
fn should_analyze_symbol(symbol: &Value, config: &AnalysisConfig) -> bool {
    symbol
        .get("kind")
        .and_then(|k| k.as_u64())
        .is_some_and(|kind| config.analyzed_kinds.contains(&kind))
}

/// Check references for symbols in parallel with concurrency limiting
async fn check_symbol_references(
    lsp_adapter: &Arc<DirectLspAdapter>,
    symbols: Vec<&Value>,
    max_concurrent: usize,
) -> ServerResult<Vec<DeadSymbol>> {
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let mut tasks = Vec::new();

    for symbol in symbols {
        let sem = semaphore.clone();
        let adapter = lsp_adapter.clone();
        let symbol = symbol.clone();

        tasks.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.ok()?;
            check_single_symbol_references(&adapter, &symbol).await
        }));
    }

    let mut dead_symbols = Vec::new();
    for task in tasks {
        if let Ok(Some(dead_symbol)) = task.await {
            dead_symbols.push(dead_symbol);
        }
    }

    Ok(dead_symbols)
}

/// Check references for a single symbol using LSP textDocument/references
async fn check_single_symbol_references(
    lsp_adapter: &Arc<DirectLspAdapter>,
    symbol: &Value,
) -> Option<DeadSymbol> {
    // Extract symbol metadata
    let name = symbol.get("name")?.as_str()?.to_string();
    let kind = symbol.get("kind")?.as_u64()?;
    let location = symbol.get("location")?;
    let uri = location.get("uri")?.as_str()?;
    let start = location.get("range")?.get("start")?;
    let line = start.get("line")?.as_u64()? as u32;
    let character = start.get("character")?.as_u64()? as u32;

    // Extract file path from URI
    let file_path = uri.strip_prefix("file://").unwrap_or(uri);

    // Query references via shared LSP adapter
    let params = json!({
        "textDocument": { "uri": uri },
        "position": { "line": line, "character": character },
        "context": { "includeDeclaration": true }
    });

    if let Ok(response) = lsp_adapter.request("textDocument/references", params).await {
        let ref_count = response.as_array().map_or(0, |a| a.len());

        // Symbol is dead if it has ≤1 reference (just the declaration itself)
        if ref_count <= 1 {
            return Some(DeadSymbol {
                name,
                kind: lsp_kind_to_string(kind),
                file_path: file_path.to_string(),
                line,
                column: character,
                reference_count: ref_count,
            });
        }
    }

    None
}

/// Convert LSP SymbolKind number to human-readable string
fn lsp_kind_to_string(kind: u64) -> String {
    match kind {
        5 => "class",
        6 => "method",
        11 => "interface",
        12 => "function",
        _ => "symbol",
    }
    .to_string()
}

// ============================================================================
// AnalysisHandler - Public Interface
// ============================================================================

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
    fn supported_tools(&self) -> Vec<&'static str> {
        vec!["find_dead_code"]
    }

    async fn handle_tool(&self, tool_call: ToolCall, context: &ToolContext) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling code analysis operation");

        match tool_call.name.as_str() {
            "find_dead_code" => self.handle_find_dead_code(tool_call, context).await,
            _ => Err(ServerError::Unsupported(format!(
                "Unknown analysis operation: {}",
                tool_call.name
            ))),
        }
    }
}

impl AnalysisHandler {
    async fn handle_find_dead_code(
        &self,
        tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value> {
        let start_time = std::time::Instant::now();
        let args = tool_call.arguments.unwrap_or(json!({}));
        let workspace_path = args
            .get("workspace_path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        debug!(workspace_path = %workspace_path, "Handling find_dead_code request");

        // Get shared LSP adapter from context
        let lsp_adapter = context.lsp_adapter.lock().await;
        let adapter = lsp_adapter.as_ref().ok_or_else(|| {
            ServerError::Internal("LSP adapter not initialized".to_string())
        })?;

        // Run dead code analysis using shared LSP adapter
        let config = AnalysisConfig::default();
        let dead_symbols = analyze_dead_code(adapter.clone(), workspace_path, config).await?;

        // Format response with complete stats
        let dead_symbols_json: Vec<Value> = dead_symbols
            .iter()
            .map(|s| {
                json!({
                    "name": s.name,
                    "kind": s.kind,
                    "file": s.file_path,
                    "line": s.line,
                    "column": s.column,
                    "referenceCount": s.reference_count,
                })
            })
            .collect();

        let files_analyzed = dead_symbols
            .iter()
            .map(|s| s.file_path.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len();

        Ok(json!({
            "workspacePath": workspace_path,
            "deadSymbols": dead_symbols_json,
            "analysisStats": {
                "filesAnalyzed": files_analyzed,
                "symbolsAnalyzed": dead_symbols_json.len(),
                "deadSymbolsFound": dead_symbols.len(),
                "analysisDurationMs": start_time.elapsed().as_millis(),
            }
        }))
    }
}
