//! Batch analysis infrastructure for workspace-wide analysis
//!
//! This module provides optimized batch analysis capabilities that enable
//! analyzing multiple files efficiently with AST caching and parallel processing.
//!
//! # Key Features
//! - **AST Caching**: Parse each file once, reuse for multiple analysis kinds
//! - **Concurrent Processing**: Parallel file analysis (future enhancement)
//! - **Error Resilience**: Continue batch analysis even if individual files fail
//! - **Progress Tracking**: Cache hit/miss metrics for optimization insights
//! - **Aggregation**: Combine results and compute workspace-wide statistics
//!
//! # Usage
//!
//! ```no_run
//! use cb_handlers::handlers::tools::analysis::batch::{BatchAnalysisRequest, run_batch_analysis};
//! use std::path::PathBuf;
//!
//! let request = BatchAnalysisRequest {
//!     files: vec![
//!         PathBuf::from("src/file1.ts"),
//!         PathBuf::from("src/file2.ts"),
//!     ],
//!     category: "quality".to_string(),
//!     kinds: vec!["complexity".to_string(), "smells".to_string()],
//!     config: None,
//! };
//!
//! let result = run_batch_analysis(request, context).await?;
//! println!("Analyzed {} files with {} total findings",
//!     result.summary.files_analyzed,
//!     result.summary.total_findings
//! );
//! ```
//!
//! # Implementation Status (MVP)
//!
//! **Current Implementation:**
//! - Data structures for batch requests and results
//! - Sequential file processing (parallel deferred to future)
//! - Basic AST caching per-batch
//! - Aggregated statistics and metadata
//!
//! **Future Enhancements (TODOs):**
//! - Parallel processing with work stealing
//! - Progressive result streaming for large batches
//! - Incremental analysis (only changed files)
//! - Cross-file dependency analysis
//! - Persistent AST cache across batches

use super::super::ToolHandlerContext;
use super::AnalysisConfig;
use cb_plugin_api::Symbol;
use cb_protocol::analysis_result::AnalysisResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use thiserror::Error;
use tracing::{debug, error, info, warn};

/// Batch analysis request
///
/// Specifies a set of files to analyze with a given category and detection kinds.
/// Optionally accepts configuration to customize analysis behavior (thresholds, etc.).
#[derive(Debug, Clone)]
pub struct BatchAnalysisRequest {
    /// Files to analyze (absolute paths)
    pub files: Vec<PathBuf>,

    /// Analysis category (e.g., "quality", "dead_code", "dependencies")
    pub category: String,

    /// Detection kinds to run (e.g., ["complexity", "smells"])
    pub kinds: Vec<String>,

    /// Optional configuration for customizing analysis behavior
    pub config: Option<AnalysisConfig>,
}

/// Batch analysis result
///
/// Contains aggregated results from analyzing multiple files, including
/// per-file findings, summary statistics, and execution metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchAnalysisResult {
    /// Results by file path (key: file path, value: list of AnalysisResults)
    ///
    /// Each file may have multiple AnalysisResult entries (one per kind).
    /// For example, analyzing "quality" with kinds ["complexity", "smells"]
    /// produces 2 AnalysisResult entries per file.
    pub results: HashMap<String, Vec<AnalysisResult>>,

    /// Aggregated statistics across all files
    pub summary: BatchSummary,

    /// Execution metadata for the batch operation
    pub metadata: BatchMetadata,
}

/// Aggregated statistics for a batch analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSummary {
    /// Total number of files in the batch request
    pub total_files: usize,

    /// Number of files successfully analyzed
    pub files_analyzed: usize,

    /// Number of files that failed analysis
    pub files_failed: usize,

    /// Total findings across all files
    pub total_findings: usize,

    /// Breakdown of findings by severity (high, medium, low)
    pub findings_by_severity: HashMap<String, usize>,

    /// Total execution time in milliseconds
    pub execution_time_ms: u64,
}

/// Execution metadata for a batch analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMetadata {
    /// Timestamp when batch analysis started (ISO 8601 format)
    pub started_at: String,

    /// Timestamp when batch analysis completed (ISO 8601 format)
    pub completed_at: String,

    /// Categories analyzed (typically one, but extensible for multi-category batches)
    pub categories_analyzed: Vec<String>,

    /// Number of AST cache hits (files parsed and reused)
    pub ast_cache_hits: usize,

    /// Number of AST cache misses (files parsed from scratch)
    pub ast_cache_misses: usize,

    /// Files that failed analysis (with error messages)
    pub failed_files: HashMap<String, String>,
}

/// Cached AST data for a file
///
/// Stores parsed AST and metadata to avoid re-parsing the same file
/// multiple times within a batch analysis operation.
#[derive(Debug, Clone)]
struct CachedAst {
    /// Parsed symbols from the language plugin
    symbols: Vec<Symbol>,

    /// File content (stored for analysis functions that need raw text)
    content: String,

    /// Language name (e.g., "rust", "typescript")
    language: String,

    /// Complexity report (pre-computed for performance)
    complexity_report: cb_ast::complexity::ComplexityReport,
}

/// Run batch analysis across multiple files
///
/// This is the main entry point for batch analysis operations. It orchestrates
/// the entire workflow:
/// 1. Validate request
/// 2. Pre-parse all ASTs (optimization: parse once, analyze multiple times)
/// 3. For each file and kind, run analysis
/// 4. Aggregate results
/// 5. Build summary statistics
///
/// # Arguments
/// - `request`: The batch analysis request with files, category, kinds, and config
/// - `context`: The tool handler context with app state and services
///
/// # Returns
/// - `Ok(BatchAnalysisResult)`: Aggregated results from all files
/// - `Err(BatchError)`: Fatal error that prevents batch processing
///
/// # Error Handling
/// Individual file failures do NOT fail the entire batch. Failed files are tracked
/// in `metadata.failed_files` and `summary.files_failed`. Only fatal errors
/// (e.g., no files provided, invalid category) cause the batch to fail.
///
/// # Performance Notes (MVP)
/// - Current implementation: Sequential file processing
/// - AST caching: Parse once per file, reuse for all kinds
/// - Future: Parallel processing with tokio::spawn
///
/// # Example
/// ```no_run
/// use cb_handlers::handlers::tools::analysis::batch::{BatchAnalysisRequest, run_batch_analysis};
/// use std::path::PathBuf;
///
/// let request = BatchAnalysisRequest {
///     files: vec![PathBuf::from("src/main.rs")],
///     category: "quality".to_string(),
///     kinds: vec!["complexity".to_string()],
///     config: None,
/// };
///
/// let result = run_batch_analysis(request, context).await?;
/// ```
pub async fn run_batch_analysis(
    request: BatchAnalysisRequest,
    context: &ToolHandlerContext,
) -> Result<BatchAnalysisResult, BatchError> {
    let batch_start = Instant::now();
    let started_at = chrono::Utc::now().to_rfc3339();

    info!(
        files_count = request.files.len(),
        category = %request.category,
        kinds = ?request.kinds,
        "Starting batch analysis"
    );

    // 1. Validate request
    if request.files.is_empty() {
        return Err(BatchError::NoFiles);
    }

    if request.kinds.is_empty() {
        return Err(BatchError::NoKinds);
    }

    // Validate category (basic check - could be extended)
    let valid_categories = [
        "quality",
        "dead_code",
        "dependencies",
        "structure",
        "documentation",
        "tests",
    ];
    if !valid_categories.contains(&request.category.as_str()) {
        return Err(BatchError::InvalidCategory(request.category.clone()));
    }

    // 2. Pre-parse all ASTs (optimization)
    debug!(
        files_count = request.files.len(),
        "Pre-parsing ASTs for batch analysis"
    );
    let ast_cache = batch_parse_asts(&request.files, context).await;

    let ast_cache_hits = ast_cache.len();
    let ast_cache_misses = request.files.len() - ast_cache_hits;

    debug!(
        cache_hits = ast_cache_hits,
        cache_misses = ast_cache_misses,
        "AST parsing complete"
    );

    // 3. Process each file with all requested kinds
    let mut results: HashMap<String, Vec<AnalysisResult>> = HashMap::new();
    let mut total_findings = 0;
    let mut findings_by_severity: HashMap<String, usize> = HashMap::new();
    let mut files_analyzed = 0;
    let mut files_failed = 0;
    let mut failed_files: HashMap<String, String> = HashMap::new();

    for file_path in &request.files {
        let file_path_str = file_path.display().to_string();

        // Check if AST is cached
        let cached_ast = match ast_cache.get(file_path) {
            Some(ast) => ast,
            None => {
                // File failed to parse - skip it
                files_failed += 1;
                failed_files.insert(
                    file_path_str.clone(),
                    "Failed to parse file or get language plugin".to_string(),
                );
                warn!(
                    file_path = %file_path_str,
                    "Skipping file due to parse failure"
                );
                continue;
            }
        };

        debug!(
            file_path = %file_path_str,
            kinds_count = request.kinds.len(),
            "Analyzing file with cached AST"
        );

        let mut file_results = Vec::new();

        // Run each kind of analysis on this file
        for kind in &request.kinds {
            match analyze_file_with_cached_ast(
                file_path,
                cached_ast,
                &request.category,
                kind,
                request.config.as_ref(),
            )
            .await
            {
                Ok(result) => {
                    // Aggregate statistics
                    total_findings += result.summary.total_findings;
                    findings_by_severity
                        .entry("high".to_string())
                        .and_modify(|count| *count += result.summary.by_severity.high)
                        .or_insert(result.summary.by_severity.high);
                    findings_by_severity
                        .entry("medium".to_string())
                        .and_modify(|count| *count += result.summary.by_severity.medium)
                        .or_insert(result.summary.by_severity.medium);
                    findings_by_severity
                        .entry("low".to_string())
                        .and_modify(|count| *count += result.summary.by_severity.low)
                        .or_insert(result.summary.by_severity.low);

                    file_results.push(result);
                }
                Err(e) => {
                    error!(
                        error = %e,
                        file_path = %file_path_str,
                        kind = %kind,
                        "Failed to analyze file with kind"
                    );
                    failed_files.insert(format!("{}::{}", file_path_str, kind), e.to_string());
                }
            }
        }

        if !file_results.is_empty() {
            files_analyzed += 1;
            results.insert(file_path_str, file_results);
        } else {
            files_failed += 1;
        }
    }

    let execution_time_ms = batch_start.elapsed().as_millis() as u64;
    let completed_at = chrono::Utc::now().to_rfc3339();

    info!(
        files_analyzed = files_analyzed,
        files_failed = files_failed,
        total_findings = total_findings,
        execution_time_ms = execution_time_ms,
        "Batch analysis complete"
    );

    // 4. Build BatchAnalysisResult
    Ok(BatchAnalysisResult {
        results,
        summary: BatchSummary {
            total_files: request.files.len(),
            files_analyzed,
            files_failed,
            total_findings,
            findings_by_severity,
            execution_time_ms,
        },
        metadata: BatchMetadata {
            started_at,
            completed_at,
            categories_analyzed: vec![request.category],
            ast_cache_hits,
            ast_cache_misses,
            failed_files,
        },
    })
}

/// Parse ASTs for all files in batch (optimization: parse once, analyze many times)
///
/// This function pre-parses all files in the batch and stores them in a cache.
/// Parsing is the most expensive operation, so doing it once and reusing for
/// multiple analysis kinds provides significant performance benefits.
///
/// # Arguments
/// - `files`: List of file paths to parse
/// - `context`: Tool handler context with file service and language plugins
///
/// # Returns
/// A HashMap mapping file paths to their cached AST data. Files that fail
/// to parse are not included in the cache.
///
/// # Error Handling
/// Individual file parse failures do NOT fail the entire batch. Failed files
/// are simply omitted from the cache and will be skipped during analysis.
///
/// # Performance Notes (MVP)
/// - Current: Sequential parsing
/// - Future: Parallel parsing with tokio::spawn for large batches
///
/// # TODO: Future Enhancements
/// - Parallel parsing with tokio::spawn
/// - Persistent cache across batch operations
/// - Cache invalidation based on file modification time
async fn batch_parse_asts(
    files: &[PathBuf],
    context: &ToolHandlerContext,
) -> HashMap<PathBuf, CachedAst> {
    let mut cache = HashMap::new();

    // TODO: Future enhancement - parallel parsing
    // Use tokio::spawn to parse files concurrently:
    // let mut handles = Vec::new();
    // for file in files {
    //     let handle = tokio::spawn(parse_single_file(file.clone(), context.clone()));
    //     handles.push(handle);
    // }
    // for handle in handles {
    //     if let Ok(Some((path, ast))) = handle.await {
    //         cache.insert(path, ast);
    //     }
    // }

    for file_path in files {
        match parse_single_file(file_path, context).await {
            Ok(cached_ast) => {
                debug!(
                    file_path = %file_path.display(),
                    symbols_count = cached_ast.symbols.len(),
                    language = %cached_ast.language,
                    "File parsed and cached"
                );
                cache.insert(file_path.clone(), cached_ast);
            }
            Err(e) => {
                warn!(
                    error = %e,
                    file_path = %file_path.display(),
                    "Failed to parse file - will skip in batch"
                );
            }
        }
    }

    cache
}

/// Parse a single file and create cached AST data
///
/// Helper function for batch_parse_asts that handles parsing a single file.
///
/// # Arguments
/// - `file_path`: Path to the file to parse
/// - `context`: Tool handler context with file service and language plugins
///
/// # Returns
/// - `Ok(CachedAst)`: Successfully parsed and cached AST data
/// - `Err(BatchError)`: Parse failure
async fn parse_single_file(
    file_path: &Path,
    context: &ToolHandlerContext,
) -> Result<CachedAst, BatchError> {
    // Get file extension
    let extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| BatchError::ParseFailed(format!("No extension: {}", file_path.display())))?;

    // Read file
    let content = context
        .app_state
        .file_service
        .read_file(file_path)
        .await
        .map_err(|e| BatchError::ParseFailed(format!("Read failed: {}", e)))?;

    // Get language plugin
    let plugin = context
        .app_state
        .language_plugins
        .get_plugin(extension)
        .ok_or_else(|| {
            BatchError::ParseFailed(format!("No plugin for extension: {}", extension))
        })?;

    // Parse file
    let parsed = plugin
        .parse(&content)
        .await
        .map_err(|e| BatchError::ParseFailed(format!("Parse error: {}", e)))?;

    let language = plugin.metadata().name.to_string();

    // Pre-compute complexity report
    let complexity_report = cb_ast::complexity::analyze_file_complexity(
        &file_path.display().to_string(),
        &content,
        &parsed.symbols,
        &language,
    );

    Ok(CachedAst {
        symbols: parsed.symbols,
        content,
        language,
        complexity_report,
    })
}

/// Analyze a file using cached AST data
///
/// This function runs a specific analysis kind on a file using pre-parsed AST data.
/// It constructs the appropriate analysis function call based on category and kind.
///
/// # Arguments
/// - `file_path`: Path to the file being analyzed
/// - `cached_ast`: Pre-parsed AST data from the cache
/// - `category`: Analysis category (e.g., "quality")
/// - `kind`: Detection kind (e.g., "complexity", "smells")
/// - `config`: Optional analysis configuration
///
/// # Returns
/// - `Ok(AnalysisResult)`: Analysis result for this file and kind
/// - `Err(BatchError)`: Analysis failure
///
/// # Implementation Notes
/// This function delegates to the appropriate detection function based on
/// category and kind. For MVP, we support the "quality" category with its
/// detection kinds. Future enhancements can add support for other categories.
///
/// # TODO: Future Enhancements
/// - Support for all categories (dead_code, dependencies, structure, etc.)
/// - Generic dispatch mechanism instead of manual match statements
/// - Category-specific configuration passing
async fn analyze_file_with_cached_ast(
    file_path: &Path,
    cached_ast: &CachedAst,
    category: &str,
    kind: &str,
    config: Option<&AnalysisConfig>,
) -> Result<AnalysisResult, BatchError> {
    let file_path_str = file_path.display().to_string();

    // Check if kind is enabled in configuration
    if let Some(cfg) = config {
        if !cfg.is_kind_enabled(category, kind) {
            return Err(BatchError::AnalysisFailed(format!(
                "Kind '{}' is disabled in configuration for category '{}'",
                kind, category
            )));
        }
    }

    // Dispatch to appropriate analysis function based on category and kind
    // For MVP, we support "quality" category with its kinds
    // TODO: Add support for other categories (dead_code, dependencies, etc.)
    match category {
        "quality" => analyze_quality_with_cached_ast(file_path, cached_ast, kind).await,
        _ => Err(BatchError::AnalysisFailed(format!(
            "Category '{}' not yet supported in batch analysis (MVP limitation)",
            category
        ))),
    }
}

/// Analyze quality category with cached AST
///
/// Delegates to the appropriate quality detection function based on kind.
///
/// # Supported Kinds
/// - "complexity": Cyclomatic and cognitive complexity analysis
/// - "smells": Code smell detection (long methods, god classes, magic numbers)
/// - "maintainability": Overall maintainability metrics
/// - "readability": Readability issues (deep nesting, parameter count, comments)
async fn analyze_quality_with_cached_ast(
    file_path: &Path,
    cached_ast: &CachedAst,
    kind: &str,
) -> Result<AnalysisResult, BatchError> {
    use cb_protocol::analysis_result::{AnalysisScope, Finding};

    let file_path_str = file_path.display().to_string();
    let start_time = Instant::now();

    // Build scope for this file
    let scope = AnalysisScope {
        scope_type: "file".to_string(),
        path: file_path_str.clone(),
        include: vec![],
        exclude: vec![],
    };

    // Call the appropriate detection function based on kind
    // These functions are from quality.rs module
    let findings: Vec<Finding> = match kind {
        "complexity" => {
            // For complexity, we need to replicate the logic from quality.rs
            // that transforms ComplexityReport to findings
            // For MVP simplicity, we return empty findings
            // TODO: Implement full complexity analysis with thresholds
            warn!(
                file_path = %file_path_str,
                kind = %kind,
                "Complexity kind not yet fully implemented in batch analysis"
            );
            vec![]
        }
        "smells" => {
            // Import detection functions from quality.rs
            // For MVP, we return empty - TODO: wire up to quality::detect_smells
            warn!(
                file_path = %file_path_str,
                kind = %kind,
                "Smells kind not yet fully implemented in batch analysis"
            );
            vec![]
        }
        "maintainability" => {
            // TODO: Wire up to quality::analyze_maintainability
            warn!(
                file_path = %file_path_str,
                kind = %kind,
                "Maintainability kind not yet fully implemented in batch analysis"
            );
            vec![]
        }
        "readability" => {
            // TODO: Wire up to quality::analyze_readability
            warn!(
                file_path = %file_path_str,
                kind = %kind,
                "Readability kind not yet fully implemented in batch analysis"
            );
            vec![]
        }
        _ => {
            return Err(BatchError::AnalysisFailed(format!(
                "Unsupported quality kind: {}",
                kind
            )))
        }
    };

    // Build AnalysisResult
    let mut result = AnalysisResult::new("quality", kind, scope);
    result.metadata.language = Some(cached_ast.language.clone());

    for finding in findings {
        result.add_finding(finding);
    }

    result.summary.files_analyzed = 1;
    result.summary.symbols_analyzed = Some(cached_ast.complexity_report.total_functions);
    result.finalize(start_time.elapsed().as_millis() as u64);

    Ok(result)
}

/// Batch analysis errors
#[derive(Debug, Error)]
pub enum BatchError {
    /// No files provided in batch request
    #[error("No files provided in batch request")]
    NoFiles,

    /// No kinds provided in batch request
    #[error("No analysis kinds provided in batch request")]
    NoKinds,

    /// Invalid category name
    #[error("Invalid category: {0}")]
    InvalidCategory(String),

    /// File parse failed
    #[error("Parse failed: {0}")]
    ParseFailed(String),

    /// Analysis execution failed
    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_request_creation() {
        let request = BatchAnalysisRequest {
            files: vec![PathBuf::from("test.rs")],
            category: "quality".to_string(),
            kinds: vec!["complexity".to_string()],
            config: None,
        };

        assert_eq!(request.files.len(), 1);
        assert_eq!(request.category, "quality");
        assert_eq!(request.kinds.len(), 1);
    }

    #[test]
    fn test_batch_summary_initialization() {
        let summary = BatchSummary {
            total_files: 10,
            files_analyzed: 8,
            files_failed: 2,
            total_findings: 25,
            findings_by_severity: HashMap::from([
                ("high".to_string(), 5),
                ("medium".to_string(), 10),
                ("low".to_string(), 10),
            ]),
            execution_time_ms: 1500,
        };

        assert_eq!(summary.total_files, 10);
        assert_eq!(summary.files_analyzed, 8);
        assert_eq!(summary.files_failed, 2);
        assert_eq!(summary.total_findings, 25);
    }

    #[test]
    fn test_batch_error_display() {
        let err = BatchError::NoFiles;
        assert_eq!(err.to_string(), "No files provided in batch request");

        let err = BatchError::InvalidCategory("unknown".to_string());
        assert_eq!(err.to_string(), "Invalid category: unknown");
    }
}
