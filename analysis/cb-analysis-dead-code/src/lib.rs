// analysis/cb-analysis-dead-code/src/lib.rs

pub mod config;
pub mod detector;
pub mod types;
pub mod utils;

use async_trait::async_trait;
use cb_analysis_common::{AnalysisEngine, AnalysisError, AnalysisMetadata, LspProvider};
pub use config::DeadCodeConfig;
use detector::run_analysis;
use std::path::Path;
pub use types::DeadCodeReport;

pub struct DeadCodeAnalyzer;

#[async_trait]
impl AnalysisEngine for DeadCodeAnalyzer {
    type Config = DeadCodeConfig;
    type Result = DeadCodeReport;

    async fn analyze(
        &self,
        lsp: std::sync::Arc<dyn LspProvider>,
        workspace_path: &Path,
        config: Self::Config,
    ) -> Result<Self::Result, AnalysisError> {
        run_analysis(lsp, workspace_path, &config).await
    }

    fn metadata(&self) -> AnalysisMetadata {
        AnalysisMetadata {
            name: "dead-code",
            version: "1.0.0",
            description: "Find unused functions, classes, and variables",
            symbol_kinds_supported: vec![5, 6, 9, 10, 11, 12, 13, 14, 22, 23],
        }
    }
}