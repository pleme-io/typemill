// analysis/mill-analysis-dead-code/src/config.rs

/// Configuration for dead code analysis
#[derive(Debug, Clone)]
pub struct DeadCodeConfig {
    pub symbol_kinds: Vec<u64>,
    pub max_concurrency: usize,
    pub min_reference_threshold: usize,
    pub include_exported: bool,
    pub file_types: Option<Vec<String>>,
    pub max_results: Option<usize>,
    pub timeout: Option<std::time::Duration>,
}

impl Default for DeadCodeConfig {
    fn default() -> Self {
        Self {
            // Comprehensive default: classes, methods, constructors, enums, interfaces,
            // functions, variables, constants, enum members, structs
            symbol_kinds: vec![5, 6, 9, 10, 11, 12, 13, 14, 22, 23],
            max_concurrency: 20,
            min_reference_threshold: 1,
            include_exported: true,
            file_types: None,
            max_results: None,
            timeout: None,
        }
    }
}
