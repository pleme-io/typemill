//! Dead code analysis using AST + LSP + call graph reachability.
//!
//! This crate finds unused code by:
//! 1. Extracting symbols via AST parsing (more accurate than LSP for visibility)
//! 2. Building a call graph from LSP references + AST intra-file calls
//! 3. Finding entry points (main, tests, pub exports)
//! 4. Marking unreachable symbols as dead
//!
//! # LSP vs AST-Only Analysis
//!
//! The analyzer works in two modes:
//!
//! - **With LSP**: Full cross-file analysis. LSP provides references between
//!   files, enabling detection of unused functions that are never called from
//!   other modules.
//!
//! - **Without LSP** (using `NoOpLspProvider`): Intra-file analysis only.
//!   AST parsing detects function calls within the same file, which is enough
//!   to find dead code in single-file scenarios or when LSP isn't available.
//!
//! # Example
//!
//! ```ignore
//! use mill_analysis_dead_code::{DeadCodeAnalyzer, Config, EntryPoints};
//! use std::path::Path;
//!
//! // With LSP (full cross-file analysis)
//! let report = DeadCodeAnalyzer::analyze(
//!     lsp.as_ref(),
//!     Path::new("src/"),
//!     Config::default(),
//! ).await?;
//!
//! // Without LSP (intra-file only, useful for quick analysis)
//! let report = DeadCodeAnalyzer::analyze_without_lsp(
//!     Path::new("src/lib.rs"),
//!     Config::default(),
//! ).await?;
//!
//! for dead in &report.dead_code {
//!     println!("{}: {} at {}:{}", dead.kind, dead.name, dead.location.file.display(), dead.location.line);
//! }
//! ```

mod ast;
mod collect;
mod error;
mod graph;
mod reachability;
mod references;
mod report;
mod types;

pub use error::Error;
pub use types::*;

use async_trait::async_trait;
use mill_analysis_common::{AnalysisError, LspProvider};
use serde_json::Value;
use std::path::Path;
use std::time::Instant;
use tracing::info;

/// A no-op LSP provider that returns empty results.
///
/// Use this when you don't have an LSP server available. The analyzer will
/// still work using AST-based intra-file call detection, which is sufficient
/// for single-file analysis.
pub struct NoOpLspProvider;

#[async_trait]
impl LspProvider for NoOpLspProvider {
    async fn workspace_symbols(&self, _query: &str) -> Result<Vec<Value>, AnalysisError> {
        Ok(vec![])
    }

    async fn find_references(
        &self,
        _uri: &str,
        _line: u32,
        _character: u32,
    ) -> Result<Vec<Value>, AnalysisError> {
        Ok(vec![])
    }

    async fn document_symbols(&self, _uri: &str) -> Result<Vec<Value>, AnalysisError> {
        Ok(vec![])
    }
}

/// Dead code analyzer using LSP + call graph reachability.
pub struct DeadCodeAnalyzer;

impl DeadCodeAnalyzer {
    /// Analyze a path for dead code without requiring an LSP server.
    ///
    /// This uses AST-based intra-file call detection only. Suitable for:
    /// - Single-file analysis
    /// - Quick analysis without LSP setup
    /// - Environments where LSP servers aren't available
    ///
    /// For full cross-file analysis, use `analyze()` with an LSP provider.
    pub async fn analyze_without_lsp(path: &Path, config: Config) -> Result<Report, Error> {
        Self::analyze(&NoOpLspProvider, path, config).await
    }

    /// Analyze a path for dead code.
    ///
    /// - File path: analyzes that file
    /// - Directory path: analyzes all files recursively
    ///
    /// The LSP provider is used for cross-file reference detection. If you
    /// don't have an LSP server available, use `analyze_without_lsp()` instead.
    pub async fn analyze(
        lsp: &dyn LspProvider,
        path: &Path,
        config: Config,
    ) -> Result<Report, Error> {
        let start = Instant::now();

        info!(path = %path.display(), "Starting dead code analysis");

        // 1. Collect all symbols
        let symbols = collect::symbols(lsp, path).await?;
        info!(count = symbols.len(), "Collected symbols");

        if symbols.is_empty() {
            return Ok(Report {
                dead_code: vec![],
                stats: Stats {
                    files_analyzed: 0,
                    symbols_analyzed: 0,
                    dead_found: 0,
                    duration_ms: start.elapsed().as_millis() as u64,
                },
            });
        }

        // 2. Get references for each symbol
        let references = references::gather(lsp, &symbols).await?;
        info!(edges = references.len(), "Gathered references");

        // 3. Build call graph
        let call_graph = graph::build(&symbols, &references);
        info!(
            nodes = call_graph.node_count(),
            edges = call_graph.edge_count(),
            "Built call graph"
        );

        // 4. Find entry points and do reachability analysis
        let entry_points = reachability::find_entry_points(&symbols, &config.entry_points);
        info!(count = entry_points.len(), "Found entry points");

        let reachable = reachability::analyze(&call_graph, &entry_points);
        info!(count = reachable.len(), "Found reachable symbols");

        // 5. Build report from unreachable symbols
        let dead_code = report::build(&symbols, &reachable, &references, &config);
        info!(count = dead_code.len(), "Found dead code");

        let files_analyzed = symbols
            .iter()
            .map(|s| &s.file_path)
            .collect::<std::collections::HashSet<_>>()
            .len();

        Ok(Report {
            stats: Stats {
                files_analyzed,
                symbols_analyzed: symbols.len(),
                dead_found: dead_code.len(),
                duration_ms: start.elapsed().as_millis() as u64,
            },
            dead_code,
        })
    }
}
