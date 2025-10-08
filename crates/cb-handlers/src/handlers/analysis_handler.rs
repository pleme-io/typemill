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
use futures::stream::{FuturesUnordered, StreamExt};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};

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
    /// Minimum reference count threshold (symbols with ≤ this many refs are considered dead)
    min_reference_threshold: usize,
    /// Include exported symbols in analysis
    include_exported: bool,
    /// File extensions to analyze (e.g., [".ts", ".tsx"]). None = all files
    file_types: Option<Vec<String>>,
    /// Maximum number of dead symbols to find before stopping
    max_results: Option<usize>,
    /// Maximum analysis duration
    timeout: Option<std::time::Duration>,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            max_concurrent_checks: 20,
            // Comprehensive default: classes, methods, constructors, enums, interfaces,
            // functions, variables, constants, enum members, structs
            analyzed_kinds: vec![5, 6, 9, 10, 11, 12, 13, 14, 22, 23],
            min_reference_threshold: 1,
            include_exported: true,
            file_types: None,
            max_results: None,
            timeout: None,
        }
    }
}

/// Build AnalysisConfig from tool call arguments
fn config_from_params(args: &Value) -> AnalysisConfig {
    let mut config = AnalysisConfig::default();

    // Parse symbol_kinds parameter
    if let Some(kinds) = args.get("symbol_kinds").and_then(|v| v.as_array()) {
        let mut parsed_kinds = Vec::new();
        for kind in kinds {
            if let Some(kind_str) = kind.as_str() {
                if let Some(kind_num) = parse_symbol_kind(kind_str) {
                    parsed_kinds.push(kind_num);
                }
            }
        }
        if !parsed_kinds.is_empty() {
            config.analyzed_kinds = parsed_kinds;
        }
    }

    // Parse max_concurrency parameter (clamped to 1-100)
    if let Some(max_conc) = args.get("max_concurrency").and_then(|v| v.as_u64()) {
        config.max_concurrent_checks = (max_conc as usize).clamp(1, 100);
    }

    // Parse min_references parameter
    if let Some(min_refs) = args.get("min_references").and_then(|v| v.as_u64()) {
        config.min_reference_threshold = min_refs as usize;
    }

    // Parse file_types parameter
    if let Some(types) = args.get("file_types").and_then(|v| v.as_array()) {
        let file_types: Vec<String> = types
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        if !file_types.is_empty() {
            config.file_types = Some(file_types);
        }
    }

    // Parse include_exported parameter
    if let Some(inc_exp) = args.get("include_exported").and_then(|v| v.as_bool()) {
        config.include_exported = inc_exp;
    }

    // Parse max_results parameter
    if let Some(max_res) = args.get("max_results").and_then(|v| v.as_u64()) {
        config.max_results = Some(max_res as usize);
    }

    // Parse timeout_seconds parameter
    if let Some(timeout_sec) = args.get("timeout_seconds").and_then(|v| v.as_u64()) {
        config.timeout = Some(std::time::Duration::from_secs(timeout_sec));
    }

    config
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
///    - If empty, fall back to per-file textDocument/documentSymbol (for rust-analyzer)
/// 2. Filter to analyzable symbols (functions, classes, methods, interfaces)
/// 3. Check references for each symbol via LSP textDocument/references
/// 4. Symbols with ≤1 reference (just the declaration) are considered dead
async fn analyze_dead_code(
    lsp_adapter: Arc<DirectLspAdapter>,
    workspace_path: &str,
    config: AnalysisConfig,
) -> ServerResult<Vec<DeadSymbol>> {
    // Try workspace/symbol first (fast path for most LSP servers)
    let mut all_symbols = collect_workspace_symbols(&lsp_adapter).await?;
    debug!(
        total_symbols = all_symbols.len(),
        "Collected symbols from workspace/symbol"
    );

    // If workspace/symbol returned nothing, use fallback
    if all_symbols.is_empty() {
        warn!("workspace/symbol returned 0 symbols - using per-file fallback");
        all_symbols =
            collect_symbols_by_document(&lsp_adapter, workspace_path, config.file_types.as_ref())
                .await?;
        debug!(
            total_symbols = all_symbols.len(),
            "Collected symbols via fallback (textDocument/documentSymbol)"
        );

        if all_symbols.is_empty() {
            return Ok(Vec::new());
        }
    }

    let symbols_to_check: Vec<_> = all_symbols
        .iter()
        .filter(|s| should_analyze_symbol(s, &config))
        .collect();
    debug!(
        symbols_to_check = symbols_to_check.len(),
        "Filtered to analyzable symbols"
    );

    let dead_symbols = check_symbol_references(&lsp_adapter, symbols_to_check, &config).await?;

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

/// Fallback: Collect symbols by querying textDocument/documentSymbol for each file.
///
/// This is used when workspace/symbol returns no results (e.g., rust-analyzer).
/// It discovers all source files in the workspace and queries each individually.
async fn collect_symbols_by_document(
    lsp_adapter: &Arc<DirectLspAdapter>,
    workspace_path: &str,
    file_types_filter: Option<&Vec<String>>,
) -> ServerResult<Vec<Value>> {
    debug!(
        workspace_path = %workspace_path,
        "Using fallback: collecting symbols via textDocument/documentSymbol"
    );

    // Discover all source files in the workspace
    let source_files = discover_source_files(workspace_path, file_types_filter)?;
    debug!(
        file_count = source_files.len(),
        "Discovered source files for symbol collection"
    );

    if source_files.is_empty() {
        return Ok(Vec::new());
    }

    let mut all_symbols = Vec::new();
    let file_count = source_files.len();

    // Query each file for its document symbols
    for file_path in &source_files {
        // Convert to absolute path and create proper file:// URI
        let absolute_path = file_path.canonicalize().unwrap_or_else(|_| file_path.clone());
        let uri = format!("file://{}", absolute_path.display());

        match lsp_adapter
            .request(
                "textDocument/documentSymbol",
                json!({
                    "textDocument": { "uri": uri }
                }),
            )
            .await
        {
            Ok(response) => {
                // documentSymbol can return either DocumentSymbol[] or SymbolInformation[]
                // We need to handle both and convert to workspace symbol format
                if let Some(symbols) = response.as_array() {
                    for symbol in symbols {
                        // Flatten nested symbols and convert to workspace symbol format
                        flatten_document_symbol(symbol, &uri, &mut all_symbols);
                    }
                }
            }
            Err(e) => {
                debug!(
                    error = %e,
                    file_path = %file_path.display(),
                    "Failed to get document symbols for file"
                );
            }
        }
    }

    info!(
        symbol_count = all_symbols.len(),
        file_count = file_count,
        "Collected symbols via document-by-document fallback"
    );

    Ok(all_symbols)
}

/// Discover source files in the workspace that should be analyzed.
///
/// Optionally filters by file types if provided.
fn discover_source_files(
    workspace_path: &str,
    file_types_filter: Option<&Vec<String>>,
) -> ServerResult<Vec<PathBuf>> {
    let workspace_dir = Path::new(workspace_path);
    if !workspace_dir.exists() {
        return Err(ServerError::InvalidRequest(format!(
            "Workspace path does not exist: {}",
            workspace_path
        )));
    }

    let mut source_files = Vec::new();

    // Use provided filter or default extensions
    let default_extensions = ["rs", "ts", "tsx", "js", "jsx", "py", "go"];
    let extensions_to_check: Vec<String> = if let Some(filter) = file_types_filter {
        filter
            .iter()
            .map(|ext| ext.trim_start_matches('.').to_string())
            .collect()
    } else {
        default_extensions.iter().map(|s| s.to_string()).collect()
    };

    // Walk the directory tree
    for entry in walkdir::WalkDir::new(workspace_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                if extensions_to_check.contains(&ext.to_string()) {
                    source_files.push(entry.path().to_path_buf());
                }
            }
        }
    }

    Ok(source_files)
}

/// Flatten a potentially nested DocumentSymbol into workspace symbol format.
///
/// DocumentSymbol can be hierarchical (e.g., class -> methods). We flatten this
/// into individual symbols, converting to the same format as workspace/symbol.
fn flatten_document_symbol(symbol: &Value, uri: &str, output: &mut Vec<Value>) {
    // If it's already in SymbolInformation format (has "location"), use it directly
    if symbol.get("location").is_some() {
        output.push(symbol.clone());
        return;
    }

    // Otherwise, it's DocumentSymbol format (has "range")
    // Convert to workspace symbol (SymbolInformation) format
    if let (Some(name), Some(kind), Some(range)) =
        (symbol.get("name"), symbol.get("kind"), symbol.get("range"))
    {
        output.push(json!({
            "name": name,
            "kind": kind,
            "location": {
                "uri": uri,
                "range": range
            }
        }));
    }

    // Recursively process children if present
    if let Some(children) = symbol.get("children").and_then(|c| c.as_array()) {
        for child in children {
            flatten_document_symbol(child, uri, output);
        }
    }
}

/// Check if a symbol should be analyzed based on configuration
fn should_analyze_symbol(symbol: &Value, config: &AnalysisConfig) -> bool {
    symbol
        .get("kind")
        .and_then(|k| k.as_u64())
        .is_some_and(|kind| config.analyzed_kinds.contains(&kind))
}

/// Check references for symbols in parallel with concurrency limiting
///
/// Uses FuturesUnordered for efficient streaming and early termination support.
async fn check_symbol_references(
    lsp_adapter: &Arc<DirectLspAdapter>,
    symbols: Vec<&Value>,
    config: &AnalysisConfig,
) -> ServerResult<Vec<DeadSymbol>> {
    let semaphore = Arc::new(Semaphore::new(config.max_concurrent_checks));
    let mut dead_symbols = Vec::new();
    let mut futures = FuturesUnordered::new();
    let mut symbols_iter = symbols.into_iter();
    let start_time = Instant::now();

    // Fill initial batch
    for symbol in symbols_iter.by_ref().take(config.max_concurrent_checks) {
        let sem = semaphore.clone();
        let adapter = lsp_adapter.clone();
        let symbol = symbol.clone();

        let min_refs = config.min_reference_threshold;
        let include_exported = config.include_exported;
        futures.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.ok()?;
            check_single_symbol_references(&adapter, &symbol, min_refs, include_exported).await
        }));
    }

    // Stream results and spawn more tasks as they complete
    while let Some(result) = futures.next().await {
        // Check timeout
        if let Some(timeout) = config.timeout {
            if start_time.elapsed() > timeout {
                debug!(
                    timeout_seconds = timeout.as_secs(),
                    dead_symbols_found = dead_symbols.len(),
                    "Analysis timeout reached, stopping early"
                );
                break;
            }
        }

        // Check max_results
        if let Some(max_res) = config.max_results {
            if dead_symbols.len() >= max_res {
                debug!(
                    max_results = max_res,
                    "Reached max_results limit, stopping early"
                );
                break;
            }
        }

        // Process result
        if let Ok(Some(dead_symbol)) = result {
            dead_symbols.push(dead_symbol);
        }

        // Spawn next task if more symbols remain
        if let Some(symbol) = symbols_iter.next() {
            let sem = semaphore.clone();
            let adapter = lsp_adapter.clone();
            let symbol = symbol.clone();
            let min_refs = config.min_reference_threshold;
            let include_exported = config.include_exported;

            futures.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.ok()?;
                check_single_symbol_references(&adapter, &symbol, min_refs, include_exported).await
            }));
        }
    }

    Ok(dead_symbols)
}

/// Check if a symbol appears to be exported based on heuristic analysis.
///
/// Uses lightweight heuristics to detect exported/public symbols:
/// - Reads the declaration line from the file
/// - Checks for common export/visibility keywords
///
/// This is not 100% accurate but good enough for filtering purposes.
fn is_symbol_exported(file_path: &str, line: u32) -> bool {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    // Try to read the specific line
    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(_) => return false, // Can't read file, assume not exported
    };

    let reader = BufReader::new(file);
    if let Some(Ok(line_content)) = reader.lines().nth(line as usize) {
        let line_lower = line_content.to_lowercase();
        // Check for common export/public keywords
        return line_lower.contains("export ")
            || line_lower.contains("pub ")
            || line_lower.contains("public ");
    }

    false // Couldn't read line, assume not exported
}

/// Check references for a single symbol using LSP textDocument/references
async fn check_single_symbol_references(
    lsp_adapter: &Arc<DirectLspAdapter>,
    symbol: &Value,
    min_reference_threshold: usize,
    include_exported: bool,
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

    // Skip exported symbols if include_exported is false
    if !include_exported && is_symbol_exported(file_path, line) {
        return None;
    }

    // Query references via shared LSP adapter
    let params = json!({
        "textDocument": { "uri": uri },
        "position": { "line": line, "character": character },
        "context": { "includeDeclaration": true }
    });

    if let Ok(response) = lsp_adapter.request("textDocument/references", params).await {
        let ref_count = response.as_array().map_or(0, |a| a.len());

        // Symbol is dead if it has ≤ min_reference_threshold references
        if ref_count <= min_reference_threshold {
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
        1 => "file",
        2 => "module",
        3 => "namespace",
        4 => "package",
        5 => "class",
        6 => "method",
        7 => "property",
        8 => "field",
        9 => "constructor",
        10 => "enum",
        11 => "interface",
        12 => "function",
        13 => "variable",
        14 => "constant",
        15 => "string",
        16 => "number",
        17 => "boolean",
        18 => "array",
        19 => "object",
        20 => "key",
        21 => "null",
        22 => "enum_member",
        23 => "struct",
        24 => "event",
        25 => "operator",
        26 => "type_parameter",
        _ => "unknown",
    }
    .to_string()
}

/// Convert string symbol kind name to LSP SymbolKind number
fn parse_symbol_kind(kind_str: &str) -> Option<u64> {
    match kind_str.to_lowercase().as_str() {
        "file" | "files" => Some(1),
        "module" | "modules" => Some(2),
        "namespace" | "namespaces" => Some(3),
        "package" | "packages" => Some(4),
        "class" | "classes" => Some(5),
        "method" | "methods" => Some(6),
        "property" | "properties" => Some(7),
        "field" | "fields" => Some(8),
        "constructor" | "constructors" => Some(9),
        "enum" | "enums" => Some(10),
        "interface" | "interfaces" => Some(11),
        "function" | "functions" => Some(12),
        "variable" | "variables" => Some(13),
        "constant" | "constants" => Some(14),
        "string" | "strings" => Some(15),
        "number" | "numbers" => Some(16),
        "boolean" | "booleans" => Some(17),
        "array" | "arrays" => Some(18),
        "object" | "objects" => Some(19),
        "key" | "keys" => Some(20),
        "null" => Some(21),
        "enum_member" | "enum_members" | "enummember" | "enummembers" => Some(22),
        "struct" | "structs" => Some(23),
        "event" | "events" => Some(24),
        "operator" | "operators" => Some(25),
        "type_parameter" | "type_parameters" | "typeparameter" | "typeparameters" => Some(26),
        _ => None,
    }
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
        let start_time = Instant::now();
        let args = tool_call.arguments.unwrap_or(json!({}));
        let workspace_path = args
            .get("workspace_path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        // Build configuration from parameters
        let config = config_from_params(&args);

        debug!(
            workspace_path = %workspace_path,
            symbol_kinds = ?config.analyzed_kinds,
            max_concurrency = config.max_concurrent_checks,
            "Handling find_dead_code request"
        );

        // Get shared LSP adapter from context
        let lsp_adapter = context.lsp_adapter.lock().await;
        let adapter = lsp_adapter
            .as_ref()
            .ok_or_else(|| ServerError::Internal("LSP adapter not initialized".to_string()))?;

        // Run dead code analysis using shared LSP adapter
        let dead_symbols =
            analyze_dead_code(adapter.clone(), workspace_path, config.clone()).await?;

        // Compute statistics
        let files_analyzed = dead_symbols
            .iter()
            .map(|s| s.file_path.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len();

        // Group symbols by kind for detailed stats
        let mut symbol_kind_stats: std::collections::HashMap<String, (usize, usize)> =
            std::collections::HashMap::new();

        for symbol in &dead_symbols {
            let entry = symbol_kind_stats
                .entry(symbol.kind.clone())
                .or_insert((0, 0));
            entry.1 += 1; // dead count
        }

        // Check if analysis was truncated
        let is_truncated = config
            .max_results
            .is_some_and(|max| dead_symbols.len() >= max)
            || config
                .timeout
                .is_some_and(|timeout| start_time.elapsed() >= timeout);

        // Format dead symbols for response
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

        // Build symbol kinds analyzed list
        let symbol_kinds_analyzed: Vec<String> = config
            .analyzed_kinds
            .iter()
            .map(|k| lsp_kind_to_string(*k))
            .collect();

        // Build symbols by kind stats
        let symbols_by_kind: Value = symbol_kind_stats
            .into_iter()
            .map(|(kind, (_total, dead))| {
                (
                    kind,
                    json!({
                        "dead": dead
                    }),
                )
            })
            .collect();

        Ok(json!({
            "workspacePath": workspace_path,
            "deadSymbols": dead_symbols_json,
            "analysisStats": {
                "filesAnalyzed": files_analyzed,
                "symbolsAnalyzed": dead_symbols_json.len(),
                "deadSymbolsFound": dead_symbols.len(),
                "analysisDurationMs": start_time.elapsed().as_millis(),
                "symbolKindsAnalyzed": symbol_kinds_analyzed,
                "truncated": is_truncated,
                "symbolsByKind": symbols_by_kind,
            },
            "configUsed": {
                "symbolKinds": symbol_kinds_analyzed,
                "maxConcurrency": config.max_concurrent_checks,
                "minReferences": config.min_reference_threshold,
                "includeExported": config.include_exported,
                "fileTypes": config.file_types,
            }
        }))
    }
}
