// analysis/cb-analysis-deep-dead-code/src/lib.rs

mod graph_builder;
mod dead_code_finder;

use async_trait::async_trait;
use cb_analysis_common::{
    AnalysisEngine, AnalysisError, AnalysisMetadata, LspProvider, SymbolNode,
};
use crate::dead_code_finder::DeadCodeFinder;
use crate::graph_builder::GraphBuilder;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

/// Configuration for deep dead code analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepDeadCodeConfig {
    /// When true, public symbols are considered as potential dead code.
    /// Default: false
    #[serde(default)]
    pub check_public_exports: bool,
    /// Glob patterns for files/directories to exclude from the analysis.
    pub exclude_patterns: Option<Vec<String>>,
}

impl Default for DeepDeadCodeConfig {
    fn default() -> Self {
        Self {
            check_public_exports: false,
            exclude_patterns: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepDeadCodeReport {
    pub dead_symbols: Vec<SymbolNode>,
}

pub struct DeepDeadCodeAnalyzer;

#[async_trait]
impl AnalysisEngine for DeepDeadCodeAnalyzer {
    type Config = DeepDeadCodeConfig;
    type Result = DeepDeadCodeReport;

    async fn analyze(
        &self,
        lsp: Arc<dyn LspProvider>,
        workspace_path: &Path,
        config: Self::Config,
    ) -> Result<Self::Result, AnalysisError> {
        info!("Starting deep dead code analysis...");

        let graph_builder = GraphBuilder::new(lsp, workspace_path.to_path_buf());
        let graph = graph_builder.build().await?;
        debug!("Constructed dependency graph: {:?}", graph);

        let dead_code_finder = DeadCodeFinder::new(&graph);
        let dead_symbols = dead_code_finder.find(&config);

        Ok(DeepDeadCodeReport { dead_symbols })
    }

    fn metadata(&self) -> AnalysisMetadata {
        AnalysisMetadata {
            name: "deep-dead-code",
            version: "1.0.0",
            description: "Finds dead code by building a workspace-wide dependency graph.",
            symbol_kinds_supported: vec![],
        }
    }
}