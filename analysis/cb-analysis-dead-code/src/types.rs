// analysis/cb-analysis-dead-code/src/types.rs

use std::path::PathBuf;

/// Result of dead code analysis
#[derive(Debug, Clone, serde::Serialize)]
pub struct DeadCodeReport {
    pub workspace_path: PathBuf,
    pub dead_symbols: Vec<DeadSymbol>,
    pub stats: AnalysisStats,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DeadSymbol {
    pub name: String,
    pub kind: String,
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub reference_count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisStats {
    pub files_analyzed: usize,
    pub symbols_analyzed: usize,
    pub dead_symbols_found: usize,
    #[serde(rename = "analysisDurationMs")]
    pub duration_ms: u128,
}