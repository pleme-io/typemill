//! Symbol collection via AST parsing with LSP fallback.

use crate::ast::{RustSymbolExtractor, SymbolExtractor, TypeScriptSymbolExtractor};
use crate::error::Error;
use crate::types::{Kind, Symbol, SymbolVisibility};
use mill_analysis_common::graph::SymbolNode;
use mill_analysis_common::LspProvider;
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

/// Default file extensions to analyze.
const DEFAULT_EXTENSIONS: &[&str] = &["rs", "ts", "tsx", "js", "jsx"];

/// Collect all symbols from the given path using AST parsing.
/// Falls back to LSP if AST parsing fails.
pub(crate) async fn symbols(lsp: &dyn LspProvider, path: &Path) -> Result<Vec<Symbol>, Error> {
    if !path.exists() {
        return Err(Error::PathNotFound(path.display().to_string()));
    }

    // Primary: Extract symbols via AST parsing (more accurate for visibility)
    let ast_symbols = collect_ast_symbols(path)?;

    if !ast_symbols.is_empty() {
        info!(count = ast_symbols.len(), "Extracted symbols via AST");
        let symbols = ast_symbols.into_iter().map(symbol_node_to_symbol).collect();
        return Ok(symbols);
    }

    // Fallback: Try LSP workspace symbols
    warn!("AST extraction returned empty, falling back to LSP");
    let workspace_symbols = collect_workspace_symbols(lsp).await?;

    if !workspace_symbols.is_empty() {
        debug!(
            count = workspace_symbols.len(),
            "Got symbols from workspace/symbol"
        );

        // Filter to only symbols in our target path
        let path_str = path.to_string_lossy();
        let filtered: Vec<Symbol> = workspace_symbols
            .into_iter()
            .filter(|s| s.file_path.starts_with(path_str.as_ref()))
            .collect();

        return Ok(filtered);
    }

    // Last resort: collect per-document
    warn!("workspace/symbol returned empty, falling back to per-document collection");
    collect_document_symbols(lsp, path).await
}

/// Collect symbols via AST parsing.
fn collect_ast_symbols(path: &Path) -> Result<Vec<SymbolNode>, Error> {
    let files = discover_files(path)?;
    debug!(count = files.len(), "Discovered source files for AST parsing");

    let workspace_root = if path.is_file() {
        path.parent().unwrap_or(path)
    } else {
        path
    };

    let rust_extractor = RustSymbolExtractor::new();
    let ts_extractor = TypeScriptSymbolExtractor::new();

    let mut all_symbols = Vec::new();

    for file_path in files {
        let extension = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let symbols = match extension {
            "rs" => match rust_extractor.extract_symbols(&file_path, workspace_root) {
                Ok(s) => s,
                Err(e) => {
                    warn!(error = %e, file = %file_path.display(), "Failed to parse Rust file");
                    continue;
                }
            },
            "ts" | "tsx" | "js" | "jsx" => {
                match ts_extractor.extract_symbols(&file_path, workspace_root) {
                    Ok(s) => s,
                    Err(e) => {
                        warn!(error = %e, file = %file_path.display(), "Failed to parse TS/JS file");
                        continue;
                    }
                }
            }
            _ => continue,
        };

        all_symbols.extend(symbols);
    }

    Ok(all_symbols)
}

/// Convert SymbolNode to our internal Symbol type.
fn symbol_node_to_symbol(node: SymbolNode) -> Symbol {
    // AST extraction sets is_public = true only for fully public (`pub`) symbols
    // Other visibility levels (pub(crate), pub(super), private) have is_public = false
    let visibility = if node.is_public {
        SymbolVisibility::Public
    } else {
        SymbolVisibility::Private
    };

    Symbol {
        id: node.id.clone(),
        name: node.name,
        kind: symbol_kind_to_kind(&node.kind),
        file_path: node.file_path.clone(),
        uri: format!("file://{}", node.file_path),
        line: node.range.start.line,
        column: node.range.start.character,
        end_line: node.range.end.line,
        end_column: node.range.end.character,
        visibility,
    }
}

/// Convert common SymbolKind to our Kind type.
fn symbol_kind_to_kind(sk: &mill_analysis_common::graph::SymbolKind) -> Kind {
    use mill_analysis_common::graph::SymbolKind;
    match sk {
        SymbolKind::Function => Kind::Function,
        SymbolKind::Struct | SymbolKind::Type => Kind::Struct,
        SymbolKind::Enum => Kind::Enum,
        SymbolKind::Trait | SymbolKind::Interface => Kind::Interface,
        SymbolKind::Constant => Kind::Const,
        SymbolKind::Module => Kind::Module,
        SymbolKind::TypeAlias => Kind::TypeAlias,
        SymbolKind::Lsp(lsp_kind) => {
            // Convert LspSymbolKind to u64 via serde
            if let Ok(value) = serde_json::to_value(lsp_kind) {
                if let Some(num) = value.as_u64() {
                    return Kind::from_lsp(num);
                }
            }
            Kind::Unknown
        }
        SymbolKind::Unknown => Kind::Unknown,
    }
}

/// Try to collect symbols via workspace/symbol LSP request.
async fn collect_workspace_symbols(lsp: &dyn LspProvider) -> Result<Vec<Symbol>, Error> {
    // Try different queries - some LSPs want "*", some want ""
    for query in ["*", ""] {
        match timeout(Duration::from_secs(30), lsp.workspace_symbols(query)).await {
            Ok(Ok(values)) if !values.is_empty() => {
                return Ok(parse_symbols(values));
            }
            Ok(Ok(_)) => continue, // Empty, try next query
            Ok(Err(e)) => {
                debug!(error = %e, query, "workspace/symbol failed");
            }
            Err(_) => {
                warn!(query, "workspace/symbol timed out");
            }
        }
    }

    Ok(vec![])
}

/// Collect symbols by walking files and querying documentSymbol for each.
async fn collect_document_symbols(
    lsp: &dyn LspProvider,
    path: &Path,
) -> Result<Vec<Symbol>, Error> {
    let files = discover_files(path)?;
    debug!(count = files.len(), "Discovered source files");

    let mut all_symbols = Vec::new();

    for file_path in files {
        let uri = format!("file://{}", file_path.display());

        match timeout(Duration::from_secs(5), lsp.document_symbols(&uri)).await {
            Ok(Ok(values)) => {
                let symbols = parse_document_symbols(values, &uri);
                all_symbols.extend(symbols);
            }
            Ok(Err(e)) => {
                debug!(error = %e, file = %file_path.display(), "documentSymbol failed");
            }
            Err(_) => {
                warn!(file = %file_path.display(), "documentSymbol timed out");
            }
        }
    }

    Ok(all_symbols)
}

/// Discover source files in the given path.
fn discover_files(path: &Path) -> Result<Vec<std::path::PathBuf>, Error> {
    let extensions: HashSet<&str> = DEFAULT_EXTENSIONS.iter().copied().collect();

    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }

    let files: Vec<_> = WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| extensions.contains(ext))
        })
        // Skip target directories and node_modules
        .filter(|e| {
            !e.path()
                .components()
                .any(|c| c.as_os_str() == "target" || c.as_os_str() == "node_modules")
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    Ok(files)
}

/// Parse workspace symbols from LSP response.
fn parse_symbols(values: Vec<Value>) -> Vec<Symbol> {
    values.into_iter().filter_map(parse_symbol).collect()
}

/// Parse a single symbol from LSP JSON.
fn parse_symbol(value: Value) -> Option<Symbol> {
    let name = value.get("name")?.as_str()?.to_string();
    let kind_num = value.get("kind")?.as_u64()?;
    let kind = Kind::from_lsp(kind_num);

    let location = value.get("location")?;
    let uri = location.get("uri")?.as_str()?;
    let range = location.get("range")?;
    let start = range.get("start")?;
    let end = range.get("end")?;
    let line = start.get("line")?.as_u64()? as u32;
    let column = start.get("character")?.as_u64()? as u32;
    let end_line = end.get("line").and_then(|v| v.as_u64()).unwrap_or(line as u64) as u32;
    let end_column = end.get("character").and_then(|v| v.as_u64()).unwrap_or(column as u64) as u32;

    let file_path = uri.strip_prefix("file://").unwrap_or(uri).to_string();

    // Generate unique ID
    let id = format!("{}::{}:{}", file_path, line, name);

    // Heuristic: check if symbol might be public
    // This is imperfect but LSP doesn't give us visibility directly
    let visibility = infer_visibility(&name, &file_path);

    Some(Symbol {
        id,
        name,
        kind,
        file_path,
        uri: uri.to_string(),
        line,
        column,
        end_line,
        end_column,
        visibility,
    })
}

/// Parse document symbols (which may be nested) from LSP response.
fn parse_document_symbols(values: Vec<Value>, uri: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    for value in values {
        flatten_document_symbol(&value, uri, &mut symbols);
    }
    symbols
}

/// Flatten potentially nested document symbols.
fn flatten_document_symbol(value: &Value, uri: &str, output: &mut Vec<Symbol>) {
    // DocumentSymbol format (nested)
    if let (Some(name), Some(kind_num), Some(range)) = (
        value.get("name").and_then(|v| v.as_str()),
        value.get("kind").and_then(|v| v.as_u64()),
        value.get("range"),
    ) {
        let start = range.get("start");
        let end = range.get("end");
        if let Some(start) = start {
            let line = start.get("line").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let column = start
                .get("character")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            let end_line = end
                .and_then(|e| e.get("line"))
                .and_then(|v| v.as_u64())
                .unwrap_or(line as u64) as u32;
            let end_column = end
                .and_then(|e| e.get("character"))
                .and_then(|v| v.as_u64())
                .unwrap_or(column as u64) as u32;

            let file_path = uri.strip_prefix("file://").unwrap_or(uri).to_string();
            let id = format!("{}::{}:{}", file_path, line, name);
            let visibility = infer_visibility(name, &file_path);

            output.push(Symbol {
                id,
                name: name.to_string(),
                kind: Kind::from_lsp(kind_num),
                file_path,
                uri: uri.to_string(),
                line,
                column,
                end_line,
                end_column,
                visibility,
            });
        }

        // Recurse into children
        if let Some(children) = value.get("children").and_then(|c| c.as_array()) {
            for child in children {
                flatten_document_symbol(child, uri, output);
            }
        }
    }
    // SymbolInformation format (flat, has location)
    else if value.get("location").is_some() {
        if let Some(symbol) = parse_symbol(value.clone()) {
            output.push(symbol);
        }
    }
}

/// Heuristic to infer symbol visibility when AST parsing isn't available.
/// LSP doesn't provide visibility info, so we use naming conventions.
fn infer_visibility(name: &str, file_path: &str) -> SymbolVisibility {
    // In Rust, symbols in lib.rs or main.rs at top level are often public
    let is_lib_or_main = file_path.ends_with("lib.rs") || file_path.ends_with("main.rs");

    // Convention: SCREAMING_CASE is usually public constants
    let is_screaming_case = name.chars().all(|c| c.is_uppercase() || c == '_') && name.len() > 1;

    // Starts with uppercase usually means public type (struct, enum, trait)
    let starts_upper = name.chars().next().is_some_and(|c| c.is_uppercase());

    if is_lib_or_main || is_screaming_case || starts_upper {
        SymbolVisibility::Public
    } else {
        SymbolVisibility::Private
    }
}
