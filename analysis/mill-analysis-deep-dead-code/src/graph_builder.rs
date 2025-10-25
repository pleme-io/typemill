// analysis/mill-analysis-deep-dead-code/src/graph_builder.rs

use crate::ast_parser::{typescript::TypeScriptSymbolExtractor, RustSymbolExtractor};
use mill_analysis_common::{ graph::{ DependencyGraph , SymbolNode , UsageContext } , AnalysisError , LspProvider , };
use lsp_types::{Location, Range};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

pub struct GraphBuilder {
    lsp: Arc<dyn LspProvider>,
    workspace_path: PathBuf,
    rust_symbol_extractor: RustSymbolExtractor,
    typescript_symbol_extractor: TypeScriptSymbolExtractor,
}

impl GraphBuilder {
    pub fn new(lsp: Arc<dyn LspProvider>, workspace_path: PathBuf) -> Self {
        Self {
            lsp,
            workspace_path,
            rust_symbol_extractor: RustSymbolExtractor::new(),
            typescript_symbol_extractor: TypeScriptSymbolExtractor::new(),
        }
    }

    pub async fn build(&self) -> Result<DependencyGraph, AnalysisError> {
        let mut graph = DependencyGraph::new();
        info!("Building dependency graph using AST parser for symbol extraction...");

        // Step 1: Extract symbols from all source files using AST parsers.
        let mut all_symbols = Vec::new();
        let mut file_symbol_map: HashMap<String, Vec<&SymbolNode>> = HashMap::new();

        let source_file_extensions: Vec<&str> = vec!["rs", "ts", "tsx", "js", "jsx"];

        for entry in WalkDir::new(&self.workspace_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .is_some_and(|ext| source_file_extensions.contains(&ext))
            })
        {
            let path = entry.path();
            let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

            let extracted_symbols = match extension {
                "rs" => self
                    .rust_symbol_extractor
                    .extract_symbols(path, &self.workspace_path),
                "ts" | "tsx" | "js" | "jsx" => self
                    .typescript_symbol_extractor
                    .extract_symbols(path, &self.workspace_path),
                _ => continue,
            };

            match extracted_symbols {
                Ok(symbols) => {
                    all_symbols.extend(symbols);
                }
                Err(e) => {
                    warn!("Could not extract symbols from {:?}: {}", path, e);
                }
            }
        }

        info!("Extracted {} symbols from source files.", all_symbols.len());

        // Step 2: Add all extracted symbols as nodes to the graph.
        for symbol in &all_symbols {
            graph.add_symbol(symbol.clone());
        }

        // Create a map for efficient lookup of symbols by file URI.
        for symbol in &all_symbols {
            let absolute_path = self.workspace_path.join(&symbol.file_path);
            let uri = format!("file://{}", absolute_path.to_str().unwrap());
            file_symbol_map.entry(uri).or_default().push(symbol);
        }

        // Step 3: Find references to build the edges of the graph.
        info!("Populated graph with nodes. Now finding references to build edges...");
        for source_symbol in &all_symbols {
            let absolute_path = self.workspace_path.join(&source_symbol.file_path);
            let uri_str = format!("file://{}", absolute_path.to_str().unwrap());
            debug!("Finding references for: {}", source_symbol.id);

            let references_val = self
                .lsp
                .find_references(
                    &uri_str,
                    source_symbol.range.start.line,
                    source_symbol.range.start.character,
                )
                .await?;

            let locations: Vec<Location> = serde_json::from_value(Value::Array(references_val))
                .map_err(|e| {
                    AnalysisError::LspError(format!("Failed to parse references: {}", e))
                })?;

            debug!(
                "Found {} references for {}",
                locations.len(),
                source_symbol.id
            );

            for loc in locations {
                if let Some(target_symbols) = file_symbol_map.get(loc.uri.as_str()) {
                    if let Some(target_symbol) =
                        self.find_containing_symbol(target_symbols, loc.range)
                    {
                        debug!(
                            "Adding dependency from {} to {}",
                            target_symbol.id, source_symbol.id
                        );
                        graph.add_dependency(
                            &target_symbol.id,
                            &source_symbol.id,
                            UsageContext::Unknown,
                        );
                    } else {
                        debug!(
                            "No containing symbol found for reference at {:?}",
                            loc.range
                        );
                    }
                }
            }
        }

        info!("Finished building dependency graph.");
        Ok(graph)
    }

    /// Finds the smallest symbol in a file that completely contains the given range.
    fn find_containing_symbol<'a>(
        &self,
        symbols: &'a [&'a SymbolNode],
        reference_range: Range,
    ) -> Option<&'a SymbolNode> {
        let mut best_match: Option<&'a SymbolNode> = None;
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