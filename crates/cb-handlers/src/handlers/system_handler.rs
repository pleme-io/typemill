//! System operations tool handler
//!
//! Handles: health_check, notify_file_opened, notify_file_saved,
//!          notify_file_closed, find_dead_code, fix_imports

use super::compat::{ToolContext, ToolHandler};
use super::lsp_adapter::DirectLspAdapter;
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_plugins::LspService;
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};

// ============================================================================
// Dead Code Analysis - Private Implementation
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

/// Analyze workspace for dead code using a reference counting approach.
///
/// This uses the following algorithm:
/// 1. Collect all symbols from workspace via LSP workspace/symbol
/// 2. Filter to analyzable symbols (functions, classes, methods, interfaces)
/// 3. Check references for each symbol via LSP textDocument/references
/// 4. Symbols with ≤1 reference (just the declaration) are considered dead
async fn analyze_dead_code(
    lsp_config: cb_core::config::LspConfig,
    _workspace_path: &str,
    config: AnalysisConfig,
) -> ServerResult<Vec<DeadSymbol>> {
    let all_symbols = collect_workspace_symbols(&lsp_config).await?;
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
        check_symbol_references(&lsp_config, symbols_to_check, config.max_concurrent_checks)
            .await?;

    info!(
        dead_symbols_found = dead_symbols.len(),
        "Dead code analysis complete"
    );

    Ok(dead_symbols)
}

/// Collect workspace symbols from all configured language servers
async fn collect_workspace_symbols(
    lsp_config: &cb_core::config::LspConfig,
) -> ServerResult<Vec<Value>> {
    let mut all_symbols = Vec::new();

    for server_config in &lsp_config.servers {
        if server_config.extensions.is_empty() {
            continue;
        }

        let primary_ext = &server_config.extensions[0];
        let adapter = DirectLspAdapter::new(
            lsp_config.clone(),
            server_config.extensions.clone(),
            format!("dead-code-collector-{}", primary_ext),
        );

        match adapter
            .request("workspace/symbol", json!({ "query": "" }))
            .await
        {
            Ok(response) => {
                if let Some(symbols) = response.as_array() {
                    debug!(
                        extension = %primary_ext,
                        symbol_count = symbols.len(),
                        "Collected symbols"
                    );
                    all_symbols.extend_from_slice(symbols);
                }
            }
            Err(e) => {
                warn!(
                    extension = %primary_ext,
                    error = %e,
                    "Failed to get symbols from language server"
                );
            }
        }
    }

    Ok(all_symbols)
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
    lsp_config: &cb_core::config::LspConfig,
    symbols: Vec<&Value>,
    max_concurrent: usize,
) -> ServerResult<Vec<DeadSymbol>> {
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let mut tasks = Vec::new();

    for symbol in symbols {
        let sem = semaphore.clone();
        let lsp_config = lsp_config.clone();
        let symbol = symbol.clone();

        tasks.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.ok()?;
            check_single_symbol_references(&lsp_config, &symbol).await
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
    lsp_config: &cb_core::config::LspConfig,
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

    // Extract file path and extension
    let file_path = uri.strip_prefix("file://").unwrap_or(uri);
    let extension = std::path::Path::new(file_path)
        .extension()?
        .to_str()?
        .to_string();

    // Get LSP adapter for this file type
    let adapter = DirectLspAdapter::new(
        lsp_config.clone(),
        vec![extension.clone()],
        format!("ref-checker-{}", extension),
    );
    let client = adapter.get_or_create_client(&extension).await.ok()?;

    // Query references via LSP
    let params = json!({
        "textDocument": { "uri": uri },
        "position": { "line": line, "character": character },
        "context": { "includeDeclaration": true }
    });

    if let Ok(response) = client.send_request("textDocument/references", params).await {
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
// SystemHandler - Public Interface
// ============================================================================

pub struct SystemHandler;

impl SystemHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SystemHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for SystemHandler {
    fn supported_tools(&self) -> Vec<&'static str> {
        vec![
            "health_check",
            "notify_file_opened",
            "notify_file_saved",
            "notify_file_closed",
            "find_dead_code",
        ]
    }

    async fn handle_tool(&self, tool_call: ToolCall, context: &ToolContext) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling system operation");

        match tool_call.name.as_str() {
            "health_check" => self.handle_health_check(tool_call, context).await,
            "notify_file_opened" => self.handle_notify_file_opened(tool_call, context).await,
            "notify_file_saved" => self.handle_notify_file_saved(tool_call, context).await,
            "notify_file_closed" => self.handle_notify_file_closed(tool_call, context).await,
            "find_dead_code" => self.handle_find_dead_code(tool_call, context).await,
            _ => Err(ServerError::Unsupported(format!(
                "Unknown system operation: {}",
                tool_call.name
            ))),
        }
    }
}

impl SystemHandler {
    async fn handle_health_check(
        &self,
        _tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value> {
        info!("Handling health check request");

        let uptime_secs = context.app_state.start_time.elapsed().as_secs();
        let uptime_mins = uptime_secs / 60;
        let uptime_hours = uptime_mins / 60;

        // Get plugin count from plugin manager
        let plugin_count = context
            .plugin_manager
            .get_all_tool_definitions()
            .await
            .len();

        // Get paused workflow count from executor
        let paused_workflows = context
            .app_state
            .workflow_executor
            .get_paused_workflow_count();

        Ok(json!({
            "status": "healthy",
            "uptime": {
                "seconds": uptime_secs,
                "minutes": uptime_mins,
                "hours": uptime_hours,
                "formatted": format!("{}h {}m {}s", uptime_hours, uptime_mins % 60, uptime_secs % 60)
            },
            "plugins": {
                "loaded": plugin_count
            },
            "workflows": {
                "paused": paused_workflows
            }
        }))
    }

    async fn handle_notify_file_opened(
        &self,
        tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling notify_file_opened");

        let args = tool_call.arguments.unwrap_or(json!({}));
        let file_path_str = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'file_path' parameter".into()))?;

        let file_path = PathBuf::from(file_path_str);

        // Trigger plugin lifecycle hooks for all plugins that can handle this file
        if let Err(e) = context
            .plugin_manager
            .trigger_file_open_hooks(&file_path)
            .await
        {
            warn!(
                file_path = %file_path.display(),
                error = %e,
                "Failed to trigger plugin hooks (continuing)"
            );
        }

        // Get file extension to determine which LSP adapter to notify
        let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        // Load LSP config to create a temporary DirectLspAdapter for notification
        let app_config = cb_core::config::AppConfig::load()
            .map_err(|e| ServerError::Internal(format!("Failed to load app config: {}", e)))?;
        let lsp_config = app_config.lsp;

        // Find the server config for this extension
        if let Some(_server_config) = lsp_config
            .servers
            .iter()
            .find(|server| server.extensions.contains(&extension.to_string()))
        {
            // Create a temporary DirectLspAdapter to handle the notification
            let adapter = DirectLspAdapter::new(
                lsp_config,
                vec![extension.to_string()],
                format!("temp-{}-notifier", extension),
            );

            // Get or create LSP client and notify
            match adapter.get_or_create_client(extension).await {
                Ok(client) => match client.notify_file_opened(&file_path).await {
                    Ok(()) => {
                        debug!(
                            file_path = %file_path.display(),
                            "Successfully notified LSP server about file"
                        );
                        Ok(json!({
                            "success": true,
                            "message": format!("Notified LSP server about file: {}", file_path.display())
                        }))
                    }
                    Err(e) => {
                        warn!(
                            file_path = %file_path.display(),
                            error = %e,
                            "Failed to notify LSP server about file"
                        );
                        Err(ServerError::Runtime {
                            message: format!("Failed to notify LSP server: {}", e),
                        })
                    }
                },
                Err(e) => {
                    warn!(
                        extension = %extension,
                        error = %e,
                        "Failed to get LSP client for extension"
                    );
                    Err(ServerError::Runtime {
                        message: format!("Failed to get LSP client: {}", e),
                    })
                }
            }
        } else {
            debug!(extension = %extension, "No LSP server configured for extension");
            Ok(json!({
                "success": true,
                "message": format!("No LSP server configured for extension '{}'", extension)
            }))
        }
    }

    async fn handle_notify_file_saved(
        &self,
        tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling notify_file_saved");

        let args = tool_call.arguments.unwrap_or(json!({}));
        let file_path_str = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'file_path' parameter".into()))?;

        let file_path = PathBuf::from(file_path_str);

        // Trigger plugin lifecycle hooks for all plugins that can handle this file
        if let Err(e) = context
            .plugin_manager
            .trigger_file_save_hooks(&file_path)
            .await
        {
            warn!(
                file_path = %file_path.display(),
                error = %e,
                "Failed to trigger plugin save hooks (continuing)"
            );
        }

        debug!(
            file_path = %file_path.display(),
            "File saved notification processed"
        );

        Ok(json!({
            "success": true,
            "message": format!("Notified about saved file: {}", file_path.display())
        }))
    }

    async fn handle_notify_file_closed(
        &self,
        tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling notify_file_closed");

        let args = tool_call.arguments.unwrap_or(json!({}));
        let file_path_str = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'file_path' parameter".into()))?;

        let file_path = PathBuf::from(file_path_str);

        // Trigger plugin lifecycle hooks for all plugins that can handle this file
        if let Err(e) = context
            .plugin_manager
            .trigger_file_close_hooks(&file_path)
            .await
        {
            warn!(
                file_path = %file_path.display(),
                error = %e,
                "Failed to trigger plugin close hooks (continuing)"
            );
        }

        debug!(
            file_path = %file_path.display(),
            "File closed notification processed"
        );

        Ok(json!({
            "success": true,
            "message": format!("Notified about closed file: {}", file_path.display())
        }))
    }

    async fn handle_find_dead_code(
        &self,
        tool_call: ToolCall,
        _context: &ToolContext,
    ) -> ServerResult<Value> {
        let start_time = std::time::Instant::now();
        let args = tool_call.arguments.unwrap_or(json!({}));
        let workspace_path = args
            .get("workspace_path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        debug!(workspace_path = %workspace_path, "Handling find_dead_code request");

        // Load LSP configuration
        let app_config = cb_core::config::AppConfig::load()
            .map_err(|e| ServerError::Internal(format!("Failed to load config: {}", e)))?;

        // Run dead code analysis using local implementation
        let config = AnalysisConfig::default();
        let dead_symbols = analyze_dead_code(app_config.lsp, workspace_path, config).await?;

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
