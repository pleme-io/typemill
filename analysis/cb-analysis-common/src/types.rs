// analysis/cb-analysis-common/src/types.rs

/// Metadata about an analysis engine
#[derive(Debug, Clone)]
pub struct AnalysisMetadata {
    pub name: &'static str,
    pub version: &'static str,
    pub description: &'static str,
    pub symbol_kinds_supported: Vec<u64>,
}
