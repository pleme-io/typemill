//! Reference gathering via LSP with AST fallback.

use crate::ast::RustSymbolExtractor;
use crate::error::Error;
use crate::types::{Reference, Symbol};
use mill_analysis_common::LspProvider;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, info, warn};

/// Gather all references between symbols via LSP with AST fallback.
///
/// This function:
/// 1. Opens all source files in LSP for proper indexing
/// 2. Queries LSP for cross-file references
/// 3. Uses AST to extract intra-file calls (reduces false positives without LSP)
pub(crate) async fn gather(
    lsp: &dyn LspProvider,
    symbols: &[Symbol],
) -> Result<Vec<Reference>, Error> {
    // Build a map from (file, line, col) to symbol ID for quick lookup
    let symbol_map = build_symbol_map(symbols);

    // Step 1: Open all source files in LSP for proper indexing
    // This ensures the LSP server knows about all files before we query references
    let opened_count = open_documents_in_lsp(lsp, symbols).await;
    if opened_count > 0 {
        debug!(count = opened_count, "Opened documents in LSP");
        // Give LSP a moment to process the opened documents
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    let mut references = Vec::new();

    // Step 2: Query references via LSP
    for symbol in symbols {
        let refs = get_symbol_references(lsp, symbol).await?;

        // For each reference location, find which symbol contains it
        for ref_location in refs {
            if let Some(from_id) = find_containing_symbol(&ref_location, &symbol_map, symbols) {
                // Skip self-references
                if from_id != symbol.id {
                    references.push(Reference {
                        from_id,
                        to_id: symbol.id.clone(),
                    });
                }
            }
        }
    }

    let lsp_ref_count = references.len();

    // Step 3: Augment with intra-file AST calls (helps when LSP returns empty)
    let ast_refs = gather_intra_file_calls(symbols)?;
    let ast_ref_count = ast_refs.len();

    // Merge, deduplicating
    let existing: HashSet<(String, String)> = references
        .iter()
        .map(|r| (r.from_id.clone(), r.to_id.clone()))
        .collect();

    for ast_ref in ast_refs {
        let key = (ast_ref.from_id.clone(), ast_ref.to_id.clone());
        if !existing.contains(&key) {
            references.push(ast_ref);
        }
    }

    info!(
        lsp_refs = lsp_ref_count,
        ast_refs = ast_ref_count,
        total = references.len(),
        "Gathered references"
    );

    Ok(references)
}

/// Open all unique source files in the LSP server for proper indexing.
///
/// This ensures rust-analyzer (or other LSP) knows about all files before
/// we query references, improving cross-file reference detection.
async fn open_documents_in_lsp(lsp: &dyn LspProvider, symbols: &[Symbol]) -> usize {
    let mut opened: HashSet<String> = HashSet::new();

    for symbol in symbols {
        if opened.contains(&symbol.uri) {
            continue;
        }

        // Extract file path from URI
        let file_path = symbol.uri.strip_prefix("file://").unwrap_or(&symbol.uri);
        let path = Path::new(file_path);

        if !path.exists() {
            continue;
        }

        // Read file content and open in LSP
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Err(e) = lsp.open_document(&symbol.uri, &content).await {
                debug!(
                    uri = %symbol.uri,
                    error = %e,
                    "Failed to open document in LSP"
                );
            } else {
                opened.insert(symbol.uri.clone());
            }
        }
    }

    opened.len()
}

/// Gather intra-file calls using AST parsing.
///
/// This finds function calls within the same file, reducing false positives
/// for local helper functions when LSP is unavailable.
fn gather_intra_file_calls(symbols: &[Symbol]) -> Result<Vec<Reference>, Error> {
    // Group symbols by URI (which has absolute path)
    let mut symbols_by_uri: HashMap<&str, Vec<&Symbol>> = HashMap::new();
    for symbol in symbols {
        symbols_by_uri
            .entry(&symbol.uri)
            .or_default()
            .push(symbol);
    }

    let rust_extractor = RustSymbolExtractor::new();
    let mut references = Vec::new();

    for (uri, file_symbols) in symbols_by_uri {
        // Only process Rust files for now
        if !uri.ends_with(".rs") {
            continue;
        }

        // Extract absolute path from file:// URI
        let file_path = uri.strip_prefix("file://").unwrap_or(uri);
        let path = Path::new(file_path);

        if !path.exists() {
            debug!(uri = uri, "Skipping non-existent file for call extraction");
            continue;
        }

        let calls = match rust_extractor.extract_calls(path) {
            Ok(c) => c,
            Err(e) => {
                debug!(
                    error = %e,
                    file = file_path,
                    "Failed to extract intra-file calls"
                );
                continue;
            }
        };

        // Build a map from function name to symbol ID
        let name_to_id: HashMap<&str, &str> = file_symbols
            .iter()
            .map(|s| (s.name.as_str(), s.id.as_str()))
            .collect();

        // Convert calls to references
        for call in calls {
            if let (Some(caller_id), Some(callee_id)) = (
                name_to_id.get(call.caller.as_str()),
                name_to_id.get(call.callee.as_str()),
            ) {
                references.push(Reference {
                    from_id: caller_id.to_string(),
                    to_id: callee_id.to_string(),
                });
            }
        }
    }

    debug!(count = references.len(), "Extracted intra-file call references");

    Ok(references)
}

/// Build a map for quick symbol lookup by location.
fn build_symbol_map(symbols: &[Symbol]) -> HashMap<String, Vec<&Symbol>> {
    let mut map: HashMap<String, Vec<&Symbol>> = HashMap::new();

    for symbol in symbols {
        map.entry(symbol.uri.clone()).or_default().push(symbol);
    }

    map
}

/// A reference location from LSP.
struct RefLocation {
    uri: String,
    line: u32,
    column: u32,
}

/// Get all reference locations for a symbol.
async fn get_symbol_references(
    lsp: &dyn LspProvider,
    symbol: &Symbol,
) -> Result<Vec<RefLocation>, Error> {
    match timeout(
        Duration::from_secs(5),
        lsp.find_references(&symbol.uri, symbol.line, symbol.column),
    )
    .await
    {
        Ok(Ok(values)) => Ok(parse_references(values)),
        Ok(Err(e)) => {
            debug!(
                error = %e,
                symbol = %symbol.name,
                "find_references failed"
            );
            Ok(vec![])
        }
        Err(_) => {
            warn!(symbol = %symbol.name, "find_references timed out");
            Ok(vec![])
        }
    }
}

/// Parse reference locations from LSP response.
fn parse_references(values: Vec<Value>) -> Vec<RefLocation> {
    values
        .into_iter()
        .filter_map(|v| {
            let uri = v.get("uri")?.as_str()?.to_string();
            let range = v.get("range")?;
            let start = range.get("start")?;
            let line = start.get("line")?.as_u64()? as u32;
            let column = start.get("character")?.as_u64()? as u32;

            Some(RefLocation { uri, line, column })
        })
        .collect()
}

/// Find which symbol contains a reference location.
///
/// Uses proper range containment: finds the smallest symbol whose range
/// completely contains the reference position.
fn find_containing_symbol(
    ref_loc: &RefLocation,
    symbol_map: &HashMap<String, Vec<&Symbol>>,
    _all_symbols: &[Symbol],
) -> Option<String> {
    let symbols_in_file = symbol_map.get(&ref_loc.uri)?;

    // Find the smallest symbol that completely contains the reference
    let mut best_match: Option<&Symbol> = None;
    let mut best_size = u32::MAX;

    for symbol in symbols_in_file {
        // Check if this symbol's range contains the reference position
        if range_contains_position(symbol, ref_loc.line, ref_loc.column) {
            // Calculate symbol size (smaller is more specific)
            let size = symbol.end_line.saturating_sub(symbol.line);
            if best_match.is_none() || size < best_size {
                best_match = Some(symbol);
                best_size = size;
            }
        }
    }

    // Fallback: if no range match, use the "closest preceding" heuristic
    if best_match.is_none() {
        let mut best_distance = u32::MAX;
        for symbol in symbols_in_file {
            if symbol.line <= ref_loc.line {
                let distance = ref_loc.line - symbol.line;
                if distance < best_distance {
                    best_distance = distance;
                    best_match = Some(symbol);
                }
            }
        }
    }

    best_match.map(|s| s.id.clone())
}

/// Check if a symbol's range contains a position.
fn range_contains_position(symbol: &Symbol, line: u32, column: u32) -> bool {
    // Position must be after start
    let after_start = line > symbol.line
        || (line == symbol.line && column >= symbol.column);

    // Position must be before end
    let before_end = line < symbol.end_line
        || (line == symbol.end_line && column <= symbol.end_column);

    after_start && before_end
}
