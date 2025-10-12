// analysis/cb-analysis-deep-dead-code/src/graph_builder.rs

use cb_analysis_common::{
    graph::{DependencyGraph, SymbolKind, SymbolNode, UsageContext},
    AnalysisError, LspProvider,
};
use lsp_types::{Location, Range, SymbolKind as LspSymbolKind};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info, warn};

#[derive(Clone)]
struct ParsedSymbol {
    id: String,
    name: String,
    kind: SymbolKind,
    uri_str: String,
    range: Range,
    is_public: bool,
}

pub struct GraphBuilder {
    lsp: Arc<dyn LspProvider>,
    workspace_path: PathBuf,
}

impl GraphBuilder {
    pub fn new(lsp: Arc<dyn LspProvider>, workspace_path: PathBuf) -> Self {
        Self {
            lsp,
            workspace_path,
        }
    }

    pub async fn build(&self) -> Result<DependencyGraph, AnalysisError> {
        let mut graph = DependencyGraph::new();
        info!("Building dependency graph...");

        let symbols_val = self.lsp.workspace_symbols("").await?;
        if symbols_val.is_empty() {
            warn!("workspace/symbol returned no symbols. Cannot build dependency graph.");
            return Ok(graph);
        }
        info!("Found {} symbols.", symbols_val.len());

        let mut parsed_symbols = Vec::new();
        let mut file_symbol_map: HashMap<String, Vec<ParsedSymbol>> = HashMap::new();

        for symbol_val in &symbols_val {
            if let Some(parsed) = self.parse_symbol_value(symbol_val) {
                debug!("Adding symbol node: {:?}", parsed.id);
                graph.add_symbol(SymbolNode {
                    id: parsed.id.clone(),
                    name: parsed.name.clone(),
                    kind: parsed.kind.clone(),
                    file_path: parsed
                        .uri_str
                        .strip_prefix("file://")
                        .unwrap_or(&parsed.uri_str)
                        .to_string(),
                    is_public: parsed.is_public,
                });
                file_symbol_map
                    .entry(parsed.uri_str.clone())
                    .or_default()
                    .push(parsed.clone());
                parsed_symbols.push(parsed);
            }
        }

        info!("Populated graph with nodes. Now finding references to build edges...");
        for source_symbol in &parsed_symbols {
            debug!("Finding references for: {}", source_symbol.id);
            let references_val = self.lsp.find_references(
                &source_symbol.uri_str,
                source_symbol.range.start.line,
                source_symbol.range.start.character,
            ).await?;

            let locations: Vec<Location> = serde_json::from_value(Value::Array(references_val))
                .map_err(|e| AnalysisError::LspError(format!("Failed to parse references: {}", e)))?;

            debug!("Found {} references for {}", locations.len(), source_symbol.id);

            for loc in locations {
                if let Some(target_symbols) = file_symbol_map.get(loc.uri.as_str()) {
                    if let Some(target_symbol) = self.find_containing_symbol(target_symbols, loc.range) {
                        debug!("Adding dependency from {} to {}", target_symbol.id, source_symbol.id);
                        graph.add_dependency(
                            &target_symbol.id,
                            &source_symbol.id,
                            UsageContext::Unknown,
                        );
                    } else {
                        debug!("No containing symbol found for reference at {:?}", loc.range);
                    }
                }
            }
        }

        info!("Finished building dependency graph.");
        Ok(graph)
    }

    fn parse_symbol_value(&self, symbol_val: &Value) -> Option<ParsedSymbol> {
        let name = symbol_val.get("name")?.as_str()?.to_string();
        let location = symbol_val.get("location")?;
        let uri_str = location.get("uri")?.as_str()?.to_string();
        let range: Range = serde_json::from_value(location.get("range")?.clone()).ok()?;

        let file_path = PathBuf::from(uri_str.strip_prefix("file://").unwrap_or(&uri_str));
        let relative_path = pathdiff::diff_paths(&file_path, &self.workspace_path)
            .unwrap_or(file_path);

        let id = format!(
            "{}::{}@L{}",
            relative_path.display(),
            name,
            range.start.line
        );

        let lsp_kind: LspSymbolKind =
            serde_json::from_value(symbol_val.get("kind")?.clone()).ok()?;
        let kind: SymbolKind = lsp_kind.into();

        let is_public = matches!(
            lsp_kind,
            LspSymbolKind::FUNCTION
                | LspSymbolKind::CLASS
                | LspSymbolKind::INTERFACE
                | LspSymbolKind::CONSTRUCTOR
        );

        Some(ParsedSymbol {
            id,
            name,
            kind,
            uri_str,
            range,
            is_public,
        })
    }

    fn find_containing_symbol<'a>(
        &self,
        symbols: &'a [ParsedSymbol],
        reference_range: Range,
    ) -> Option<&'a ParsedSymbol> {
        let mut best_match: Option<&'a ParsedSymbol> = None;
        let mut best_match_size = u32::MAX;

        for symbol in symbols {
            if self.range_contains(symbol.range, reference_range) {
                let line_diff = symbol.range.end.line - symbol.range.start.line;
                if best_match.is_none() || line_diff < best_match_size {
                    best_match = Some(symbol);
                    best_match_size = line_diff;
                }
            }
        }
        best_match
    }

    fn range_contains(&self, container: Range, contained: Range) -> bool {
        container.start <= contained.start && container.end >= contained.end
    }
}