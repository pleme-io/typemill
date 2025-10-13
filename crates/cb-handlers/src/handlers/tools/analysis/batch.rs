//! Batch analysis infrastructure for workspace-wide analysis
use super::super::ToolHandlerContext;
use super::{
    AnalysisConfig,
    dead_code as dead_code_handler,
    dependencies as dependencies_handler,
    documentation as documentation_handler,
    quality as quality_handler,
    structure as structure_handler,
    tests_handler,
};
use cb_plugin_api::Symbol;
use cb_protocol::analysis_result::{AnalysisResult, AnalysisScope, Finding, FindingLocation, Severity, Range, Position};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;
use thiserror::Error;
use tracing::{error, info, warn};

// --- New Data Structures for Multi-Query Batching ---

#[derive(Debug, Deserialize, Clone)]
pub struct AnalysisQuery {
    pub command: String,
    pub kind: String,
    pub scope: QueryScope,
    #[serde(default)]
    pub options: Option<Value>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct QueryScope {
    #[serde(rename = "type")]
    pub scope_type: String,
    pub path: Option<String>,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BatchAnalysisRequest {
    pub queries: Vec<AnalysisQuery>,
    pub config: Option<AnalysisConfig>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SingleQueryResult {
    pub command: String,
    pub kind: String,
    pub result: AnalysisResult,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchAnalysisResult {
    pub results: Vec<SingleQueryResult>,
    pub summary: BatchSummary,
    pub metadata: BatchMetadata,
}

// --- Shared Data Structures (mostly unchanged) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSummary {
    pub total_queries: usize,
    pub total_files_scanned: usize,
    pub files_analyzed: usize,
    pub files_failed: usize,
    pub total_findings: usize,
    pub findings_by_severity: HashMap<String, usize>,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMetadata {
    pub started_at: String,
    pub completed_at: String,
    pub categories_analyzed: Vec<String>,
    pub ast_cache_hits: usize,
    pub ast_cache_misses: usize,
    pub failed_files: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct CachedAst {
    symbols: Vec<Symbol>,
    content: String,
    language: String,
    complexity_report: cb_ast::complexity::ComplexityReport,
}

#[derive(Debug, Error)]
pub enum BatchError {
    #[error("No queries provided in batch request")]
    NoQueries,
    #[error("Invalid command in query: {0}")]
    InvalidCommand(String),
    #[error("Invalid scope: {0}")]
    InvalidScope(String),
    #[error("IO Error: {0}")]
    IoError(String),
    #[error("Parse failed: {0}")]
    ParseFailed(String),
    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),
}

// --- Core Batch Logic ---

pub async fn run_batch_analysis(
    request: BatchAnalysisRequest,
    context: &ToolHandlerContext,
) -> Result<BatchAnalysisResult, BatchError> {
    let batch_start = Instant::now();
    let started_at = chrono::Utc::now().to_rfc3339();

    if request.queries.is_empty() {
        return Err(BatchError::NoQueries);
    }

    info!(
        queries_count = request.queries.len(),
        "Starting multi-query batch analysis"
    );

    // 1. Collect all unique files from all query scopes
    let mut all_files_to_parse = HashSet::new();
    for query in &request.queries {
        let files_for_query = resolve_scope_to_files(&query.scope).await?;
        all_files_to_parse.extend(files_for_query);
    }
    let all_files_vec: Vec<PathBuf> = all_files_to_parse.into_iter().collect();

    // 2. Pre-parse all ASTs for optimization
    let ast_cache = batch_parse_asts(&all_files_vec, context).await;
    let ast_cache_hits = ast_cache.len();
    let ast_cache_misses = all_files_vec.len() - ast_cache_hits;

    // 3. Process each query
    let mut query_results = Vec::new();
    let mut failed_files_map = HashMap::new();
    let mut all_categories = HashSet::new();

    for query in &request.queries {
        let category = query.command.split('.').last().unwrap_or("").to_string();
        if category.is_empty() {
            warn!(command = %query.command, "Skipping query with invalid command format");
            continue;
        }
        all_categories.insert(category.clone());

        let files_for_query = resolve_scope_to_files(&query.scope).await?;
        let mut all_findings_for_query: Vec<Finding> = Vec::new();
        let mut symbols_analyzed_for_query = 0;
        let mut files_analyzed_in_query = 0;

        for file_path in &files_for_query {
            if let Some(cached_ast) = ast_cache.get(file_path) {
                match analyze_file_with_cached_ast(
                    file_path,
                    cached_ast,
                    &category,
                    &query.kind,
                    request.config.as_ref(),
                )
                .await
                {
                    Ok(result_for_file) => {
                        all_findings_for_query.extend(result_for_file.findings);
                        symbols_analyzed_for_query +=
                            result_for_file.summary.symbols_analyzed.unwrap_or(0);
                    }
                    Err(e) => {
                        let file_path_str = file_path.display().to_string();
                        warn!(file_path=%file_path_str, error=%e, "Analysis failed for file in query");
                        failed_files_map.insert(file_path_str, e.to_string());
                    }
                }
                files_analyzed_in_query += 1;
            } else {
                 let file_path_str = file_path.display().to_string();
                 if !failed_files_map.contains_key(&file_path_str) {
                    failed_files_map.insert(file_path_str, "File failed to parse".to_string());
                 }
            }
        }

        let scope_path = query.scope.path.clone().unwrap_or_default();
        let scope = AnalysisScope {
            scope_type: query.scope.scope_type.clone(),
            path: scope_path,
            include: query.scope.include.clone(),
            exclude: query.scope.exclude.clone(),
        };

        let mut query_analysis_result = AnalysisResult::new(&category, &query.kind, scope);
        query_analysis_result.findings = all_findings_for_query;
        query_analysis_result.summary.files_analyzed = files_analyzed_in_query;
        query_analysis_result.summary.symbols_analyzed = Some(symbols_analyzed_for_query);
        query_analysis_result.finalize(0); // Timings are for the whole batch

        query_results.push(SingleQueryResult {
            command: query.command.clone(),
            kind: query.kind.clone(),
            result: query_analysis_result,
        });
    }

    // 4. Build final summary and metadata
    let execution_time_ms = batch_start.elapsed().as_millis() as u64;
    let completed_at = chrono::Utc::now().to_rfc3339();

    let mut total_findings = 0;
    let mut findings_by_severity = HashMap::new();
    for res in &query_results {
        total_findings += res.result.summary.total_findings;
        findings_by_severity.entry("high".to_string()).or_insert(0);
        findings_by_severity.entry("medium".to_string()).or_insert(0);
        findings_by_severity.entry("low".to_string()).or_insert(0);
    }

    Ok(BatchAnalysisResult {
        results: query_results,
        summary: BatchSummary {
            total_queries: request.queries.len(),
            total_files_scanned: all_files_vec.len(),
            files_analyzed: ast_cache_hits,
            files_failed: all_files_vec.len() - ast_cache_hits,
            total_findings,
            findings_by_severity,
            execution_time_ms,
        },
        metadata: BatchMetadata {
            started_at,
            completed_at,
            categories_analyzed: all_categories.into_iter().collect(),
            ast_cache_hits,
            ast_cache_misses,
            failed_files: failed_files_map,
        },
    })
}

use globset::{Glob, GlobSetBuilder};

async fn resolve_scope_to_files(scope: &QueryScope) -> Result<Vec<PathBuf>, BatchError> {
    let root_path = scope.path.as_ref().ok_or_else(|| {
        BatchError::InvalidScope(format!(
            "'{}' scope requires a 'path'",
            scope.scope_type
        ))
    })?;

    match scope.scope_type.as_str() {
        "file" => Ok(vec![PathBuf::from(root_path)]),
        "directory" | "workspace" => {
            let mut files = Vec::new();
            let walker = WalkBuilder::new(root_path).build();

            let mut include_builder = GlobSetBuilder::new();
            for pattern in &scope.include {
                include_builder.add(Glob::new(pattern).map_err(|e| BatchError::InvalidScope(e.to_string()))?);
            }
            let include_set = include_builder.build().map_err(|e| BatchError::InvalidScope(e.to_string()))?;

            let mut exclude_builder = GlobSetBuilder::new();
            for pattern in &scope.exclude {
                exclude_builder.add(Glob::new(pattern).map_err(|e| BatchError::InvalidScope(e.to_string()))?);
            }
            let exclude_set = exclude_builder.build().map_err(|e| BatchError::InvalidScope(e.to_string()))?;

            for result in walker {
                if let Ok(entry) = result {
                    if entry.file_type().map_or(false, |ft| ft.is_file()) {
                        let path = entry.path();
                        if !exclude_set.is_match(path) && (include_set.is_empty() || include_set.is_match(path)) {
                            files.push(path.to_path_buf());
                        }
                    }
                }
            }
            Ok(files)
        }
        _ => Err(BatchError::InvalidScope(format!(
            "Unsupported scope type: '{}'",
            scope.scope_type
        ))),
    }
}

async fn batch_parse_asts(
    files: &[PathBuf],
    context: &ToolHandlerContext,
) -> HashMap<PathBuf, CachedAst> {
    let mut cache = HashMap::new();
    for file_path in files {
        match parse_single_file(file_path, context).await {
            Ok(cached_ast) => {
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

async fn parse_single_file(
    file_path: &Path,
    context: &ToolHandlerContext,
) -> Result<CachedAst, BatchError> {
    let extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| BatchError::ParseFailed(format!("No extension: {}", file_path.display())))?;

    let content = context
        .app_state
        .file_service
        .read_file(file_path)
        .await
        .map_err(|e| BatchError::ParseFailed(format!("Read failed: {}", e)))?;

    let plugin = context
        .app_state
        .language_plugins
        .get_plugin(extension)
        .ok_or_else(|| {
            BatchError::ParseFailed(format!("No plugin for extension: {}", extension))
        })?;

    let parsed = plugin
        .parse(&content)
        .await
        .map_err(|e| BatchError::ParseFailed(format!("Parse error: {}", e)))?;

    let language = plugin.metadata().name.to_string();

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

async fn analyze_file_with_cached_ast(
    file_path: &Path,
    cached_ast: &CachedAst,
    category: &str,
    kind: &str,
    _config: Option<&AnalysisConfig>,
) -> Result<AnalysisResult, BatchError> {
    let file_path_str = file_path.display().to_string();
    let start_time = Instant::now();

    let scope = AnalysisScope {
        scope_type: "file".to_string(),
        path: file_path_str.clone(),
        include: vec![],
        exclude: vec![],
    };

    let findings: Vec<Finding> = match category {
        "quality" => match kind {
            "complexity" => {
                let mut findings = vec![];
                for func in &cached_ast.complexity_report.functions {
                    if func.complexity.cognitive >= 10 || func.complexity.cyclomatic >= 15 {
                        let severity = match func.rating {
                            cb_ast::complexity::ComplexityRating::VeryComplex => Severity::High,
                            cb_ast::complexity::ComplexityRating::Complex => Severity::Medium,
                            _ => Severity::Low,
                        };

                        let mut metrics = HashMap::new();
                        metrics.insert(
                            "cyclomatic_complexity".to_string(),
                            json!(func.complexity.cyclomatic),
                        );
                        metrics.insert(
                            "cognitive_complexity".to_string(),
                            json!(func.complexity.cognitive),
                        );

                        findings.push(Finding {
                            id: format!("complexity-{}-{}", file_path_str, func.line),
                            kind: "complexity_hotspot".to_string(),
                            severity,
                            location: FindingLocation {
                                file_path: file_path_str.clone(),
                                range: Some(Range {
                                    start: Position { line: func.line as u32, character: 0 },
                                    end: Position { line: (func.line + func.metrics.sloc as usize) as u32, character: 0 },
                                }),
                                symbol: Some(func.name.clone()),
                                symbol_kind: Some("function".to_string()),
                            },
                            metrics: Some(metrics),
                            message: format!("Function '{}' has high complexity", func.name),
                            suggestions: vec![],
                        });
                    }
                }
                findings
            }
            "smells" => quality_handler::detect_smells(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "maintainability" => quality_handler::analyze_maintainability(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "readability" => quality_handler::analyze_readability(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            _ => return Err(BatchError::AnalysisFailed(format!("Unsupported quality kind: {}", kind))),
        },
        "dead_code" => match kind {
            "unused_imports" => dead_code_handler::detect_unused_imports(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "unused_symbols" => dead_code_handler::detect_unused_symbols(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "unreachable_code" => dead_code_handler::detect_unreachable_code(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "unused_parameters" => dead_code_handler::detect_unused_parameters(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "unused_types" => dead_code_handler::detect_unused_types(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "unused_variables" => dead_code_handler::detect_unused_variables(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            _ => return Err(BatchError::AnalysisFailed(format!("Unsupported dead_code kind: {}", kind))),
        },
        "dependencies" => match kind {
            "imports" => dependencies_handler::detect_imports(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "graph" => dependencies_handler::detect_graph(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "circular" => dependencies_handler::detect_circular(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "coupling" => dependencies_handler::detect_coupling(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "cohesion" => dependencies_handler::detect_cohesion(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "depth" => dependencies_handler::detect_depth(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            _ => return Err(BatchError::AnalysisFailed(format!("Unsupported dependencies kind: {}", kind))),
        },
        "structure" => match kind {
            "symbols" => structure_handler::detect_symbols(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "hierarchy" => structure_handler::detect_hierarchy(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "interfaces" => structure_handler::detect_interfaces(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "inheritance" => structure_handler::detect_inheritance(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "modules" => structure_handler::detect_modules(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            _ => return Err(BatchError::AnalysisFailed(format!("Unsupported structure kind: {}", kind))),
        },
        "documentation" => match kind {
            "coverage" => documentation_handler::detect_coverage(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "quality" => documentation_handler::detect_quality(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "style" => documentation_handler::detect_style(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "examples" => documentation_handler::detect_examples(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "todos" => documentation_handler::detect_todos(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            _ => return Err(BatchError::AnalysisFailed(format!("Unsupported documentation kind: {}", kind))),
        },
        "tests" => match kind {
            "coverage" => tests_handler::detect_coverage(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "quality" => tests_handler::detect_quality(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "assertions" => tests_handler::detect_assertions(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            "organization" => tests_handler::detect_organization(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
            ),
            _ => return Err(BatchError::AnalysisFailed(format!("Unsupported tests kind: {}", kind))),
        },
        _ => {
            return Err(BatchError::AnalysisFailed(format!(
                "Unsupported category '{}' in batch analysis",
                category
            )))
        }
    };

    let mut result = AnalysisResult::new(category, kind, scope);
    result.metadata.language = Some(cached_ast.language.clone());
    for finding in findings {
        result.add_finding(finding);
    }
    result.summary.files_analyzed = 1;
    result.summary.symbols_analyzed = Some(cached_ast.complexity_report.total_functions);
    result.finalize(start_time.elapsed().as_millis() as u64);

    Ok(result)
}
