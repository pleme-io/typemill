//! Core dead code analysis algorithm.

use crate::config::DeadCodeConfig;
use crate::types::{AnalysisStats, DeadCodeReport, DeadSymbol};
use crate::utils::{is_symbol_exported, lsp_kind_to_string};
use futures::stream::{FuturesUnordered, StreamExt};
use mill_analysis_common::{AnalysisError, LspProvider};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::time::timeout;
use tracing::{debug, info, warn};

/// Analyze workspace for dead code using a reference counting approach.
///
/// This is the main entry point for the dead code analysis engine.
pub async fn run_analysis(
    lsp: Arc<dyn LspProvider>,
    workspace_path: &Path,
    config: &DeadCodeConfig,
) -> Result<DeadCodeReport, AnalysisError> {
    let start_time = Instant::now();
    #[allow(unused_assignments)] // Value is assigned before first read
    let mut files_analyzed = 0;

    // 1. Collect all symbols from the workspace
    let mut all_symbols = collect_workspace_symbols(lsp.clone()).await?;
    debug!(
        total_symbols = all_symbols.len(),
        "Collected symbols from workspace/symbol"
    );

    if !all_symbols.is_empty() {
        // If we got symbols from the workspace, we still need to know how many files were considered.
        // We also need to filter by file_types if provided.
        let source_files = discover_source_files(workspace_path, config.file_types.as_ref())?;
        files_analyzed = source_files.len();

        if let Some(filter) = &config.file_types {
            let extensions: HashSet<String> = filter
                .iter()
                .map(|s| s.trim_start_matches('.').to_lowercase())
                .collect();
            all_symbols.retain(|symbol| {
                symbol
                    .get("location")
                    .and_then(|loc| loc.get("uri"))
                    .and_then(|uri| uri.as_str())
                    .and_then(|uri_str| {
                        Path::new(uri_str.strip_prefix("file://").unwrap_or(uri_str)).extension()
                    })
                    .and_then(|ext| ext.to_str())
                    .map(|ext_str| extensions.contains(&ext_str.to_lowercase()))
                    .unwrap_or(false)
            });
        }
    } else {
        warn!("workspace/symbol returned 0 symbols - using per-file fallback");
        let (symbols, num_files) =
            collect_symbols_by_document(lsp.clone(), workspace_path, config.file_types.as_ref())
                .await?;
        all_symbols = symbols;
        files_analyzed = num_files;
        debug!(
            total_symbols = all_symbols.len(),
            "Collected symbols via fallback (textDocument/documentSymbol)"
        );
    }

    if all_symbols.is_empty() {
        return Ok(DeadCodeReport {
            workspace_path: workspace_path.to_path_buf(),
            dead_symbols: vec![],
            stats: AnalysisStats {
                files_analyzed,
                symbols_analyzed: 0,
                dead_symbols_found: 0,
                duration_ms: start_time.elapsed().as_millis(),
            },
        });
    }

    // 2. Filter to analyzable symbols
    let symbols_to_check: Vec<_> = all_symbols
        .iter()
        .filter(|s| should_analyze_symbol(s, config))
        .collect();
    let total_symbols_to_analyze = symbols_to_check.len();
    debug!(
        symbols_to_check = total_symbols_to_analyze,
        "Filtered to analyzable symbols"
    );

    // 3. Check references and find dead symbols
    let dead_symbols = check_symbol_references(lsp, symbols_to_check, config).await?;
    info!(
        dead_symbols_found = dead_symbols.len(),
        "Dead code analysis complete"
    );

    // 4. Compute stats and build the final report
    let report = DeadCodeReport {
        workspace_path: workspace_path.to_path_buf(),
        stats: AnalysisStats {
            files_analyzed,
            symbols_analyzed: total_symbols_to_analyze,
            dead_symbols_found: dead_symbols.len(),
            duration_ms: start_time.elapsed().as_millis(),
        },
        dead_symbols,
    };

    Ok(report)
}

/// Collect workspace symbols using the provided LSP provider.
async fn collect_workspace_symbols(lsp: Arc<dyn LspProvider>) -> Result<Vec<Value>, AnalysisError> {
    let query_attempts = vec!["*", ""];
    for query in query_attempts {
        let lsp_call = lsp.workspace_symbols(query);
        match timeout(Duration::from_secs(30), lsp_call).await {
            Ok(Ok(symbols)) if !symbols.is_empty() => {
                debug!(
                    symbol_count = symbols.len(),
                    query, "Collected symbols from workspace"
                );
                return Ok(symbols);
            }
            Ok(Ok(_)) => continue, // Empty result, try next query
            Ok(Err(e)) => {
                debug!(error = %e, query, "Failed to get workspace symbols");
                // Don't return error, just try next query
            }
            Err(_) => {
                warn!(query, "Timeout getting workspace symbols");
                // Timeout, try next query
            }
        }
    }
    warn!("No workspace symbols found after trying all queries.");
    Ok(Vec::new())
}

/// Fallback: Collect symbols by querying textDocument/documentSymbol for each file.
async fn collect_symbols_by_document(
    lsp: Arc<dyn LspProvider>,
    workspace_path: &Path,
    file_types_filter: Option<&Vec<String>>,
) -> Result<(Vec<Value>, usize), AnalysisError> {
    let source_files = discover_source_files(workspace_path, file_types_filter)?;
    let num_source_files = source_files.len();
    debug!(
        file_count = num_source_files,
        "Discovered source files for symbol collection"
    );

    let mut all_symbols = Vec::new();
    for file_path in &source_files {
        let uri = format!("file://{}", file_path.display());
        let lsp_call = lsp.document_symbols(&uri);
        match timeout(Duration::from_secs(5), lsp_call).await {
            Ok(Ok(symbols)) => {
                for symbol in symbols {
                    flatten_document_symbol(&symbol, &uri, &mut all_symbols);
                }
            }
            Ok(Err(e)) => {
                debug!(error = %e, file_path = %file_path.display(), "Failed to get document symbols")
            }
            Err(_) => warn!(file_path = %file_path.display(), "Timeout getting document symbols"),
        }
    }
    Ok((all_symbols, num_source_files))
}

/// Discover source files in the workspace, optionally filtering by file types.
fn discover_source_files(
    workspace_path: &Path,
    file_types_filter: Option<&Vec<String>>,
) -> Result<Vec<PathBuf>, AnalysisError> {
    if !workspace_path.exists() {
        return Err(AnalysisError::FileSystemError(format!(
            "Workspace path does not exist: {}",
            workspace_path.display()
        )));
    }
    let default_extensions = ["rs", "ts", "tsx", "js", "jsx", "py", "go"];
    let extensions_to_check: HashSet<String> = if let Some(filter) = file_types_filter {
        filter
            .iter()
            .map(|ext| ext.trim_start_matches('.').to_string())
            .collect()
    } else {
        default_extensions.iter().map(|s| s.to_string()).collect()
    };
    let source_files = walkdir::WalkDir::new(workspace_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .and_then(|ext_str| {
                    if extensions_to_check.contains(ext_str) {
                        Some(e.path().to_path_buf())
                    } else {
                        None
                    }
                })
        })
        .collect();
    Ok(source_files)
}

/// Flatten a potentially nested DocumentSymbol into workspace symbol format.
fn flatten_document_symbol(symbol: &Value, uri: &str, output: &mut Vec<Value>) {
    if symbol.get("location").is_some() {
        output.push(symbol.clone());
        return;
    }
    if let (Some(name), Some(kind), Some(range)) =
        (symbol.get("name"), symbol.get("kind"), symbol.get("range"))
    {
        output.push(json!({
            "name": name,
            "kind": kind,
            "location": { "uri": uri, "range": range }
        }));
    }
    if let Some(children) = symbol.get("children").and_then(|c| c.as_array()) {
        for child in children {
            flatten_document_symbol(child, uri, output);
        }
    }
}

/// Check if a symbol should be analyzed based on configuration.
fn should_analyze_symbol(symbol: &Value, config: &DeadCodeConfig) -> bool {
    symbol
        .get("kind")
        .and_then(|k| k.as_u64())
        .is_some_and(|kind| config.symbol_kinds.contains(&kind))
}

/// Check references for symbols in parallel with concurrency limiting.
async fn check_symbol_references(
    lsp: Arc<dyn LspProvider>,
    symbols: Vec<&Value>,
    config: &DeadCodeConfig,
) -> Result<Vec<DeadSymbol>, AnalysisError> {
    let semaphore = Arc::new(Semaphore::new(config.max_concurrency));
    let mut dead_symbols = Vec::new();
    let mut futures = FuturesUnordered::new();
    let mut symbols_iter = symbols.into_iter();
    let start_time = Instant::now();

    let min_refs = config.min_reference_threshold;
    let inc_exp = config.include_exported;

    for symbol in symbols_iter.by_ref().take(config.max_concurrency) {
        let lsp_clone = lsp.clone();
        let sem_clone = semaphore.clone();
        let symbol_clone = symbol.clone();
        futures.push(tokio::spawn(async move {
            let _permit = sem_clone.acquire().await.ok()?;
            check_single_symbol_references(lsp_clone, &symbol_clone, min_refs, inc_exp).await
        }));
    }

    while let Some(result) = futures.next().await {
        if let Some(timeout_val) = config.timeout {
            if start_time.elapsed() > timeout_val {
                warn!("Analysis timeout reached");
                break;
            }
        }
        if let Some(max_res) = config.max_results {
            if dead_symbols.len() >= max_res {
                debug!("Reached max_results limit");
                break;
            }
        }

        if let Ok(Some(dead_symbol)) = result {
            dead_symbols.push(dead_symbol);
        }

        if let Some(symbol) = symbols_iter.next() {
            let lsp_clone = lsp.clone();
            let sem_clone = semaphore.clone();
            let symbol_clone = symbol.clone();
            futures.push(tokio::spawn(async move {
                let _permit = sem_clone.acquire().await.ok()?;
                check_single_symbol_references(lsp_clone, &symbol_clone, min_refs, inc_exp).await
            }));
        }
    }

    Ok(dead_symbols)
}

/// Check references for a single symbol using LSP.
async fn check_single_symbol_references(
    lsp: Arc<dyn LspProvider>,
    symbol: &Value,
    min_reference_threshold: usize,
    include_exported: bool,
) -> Option<DeadSymbol> {
    let name = symbol.get("name")?.as_str()?.to_string();
    let kind_num = symbol.get("kind")?.as_u64()?;
    let location = symbol.get("location")?;
    let uri = location.get("uri")?.as_str()?;
    let start = location.get("range")?.get("start")?;
    let line = start.get("line")?.as_u64()? as u32;
    let character = start.get("character")?.as_u64()? as u32;

    let file_path = uri.strip_prefix("file://").unwrap_or(uri);
    if !include_exported && is_symbol_exported(file_path, line) {
        return None;
    }

    let references_result = timeout(
        Duration::from_secs(5),
        lsp.find_references(uri, line, character),
    )
    .await;

    match references_result {
        Ok(Ok(references)) => {
            let ref_count = references.len();
            if ref_count <= min_reference_threshold {
                return Some(DeadSymbol {
                    name,
                    kind: lsp_kind_to_string(kind_num),
                    file_path: file_path.to_string(),
                    line,
                    column: character,
                    reference_count: ref_count,
                });
            }
        }
        _ => warn!(symbol_name = %name, "Timeout or error checking references"),
    }

    None
}
