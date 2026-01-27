//! Batch analysis infrastructure for workspace-wide analysis
use crate::ToolHandlerContext;
use crate::{
    dependencies as dependencies_handler, documentation as documentation_handler,
    quality as quality_handler, structure as structure_handler,
    suggestions::{
        ActionableSuggestion, AnalysisContext, EvidenceStrength, Location, RefactorType,
        RefactoringCandidate, Scope, SuggestionConfig, SuggestionGenerator,
    },
    tests_handler, AnalysisConfig,
};
use ignore::WalkBuilder;
use mill_foundation::errors::MillError;
use mill_foundation::protocol::analysis_result::{
    AnalysisResult, AnalysisScope, Finding, FindingLocation, Position, Range, Severity,
};
use mill_plugin_api::Symbol;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;
use thiserror::Error;
use tracing::{info, warn};

/// Helper to downcast AnalysisConfigTrait to concrete AnalysisConfig
fn get_analysis_config(context: &ToolHandlerContext) -> Result<&AnalysisConfig, MillError> {
    context
        .analysis_config
        .as_any()
        .downcast_ref::<AnalysisConfig>()
        .ok_or_else(|| {
            MillError::internal("Failed to downcast AnalysisConfigTrait to AnalysisConfig")
        })
}

// --- New Data Structures for Multi-Query Batching ---

#[derive(Debug, Deserialize, Clone)]
/// Analysis query for batch operations
#[serde(rename_all = "camelCase")]
pub struct AnalysisQuery {
    pub command: String,
    pub kind: String,
    pub scope: QueryScope,
    #[serde(default)]
    #[allow(dead_code)]
    pub options: Option<Value>,
}

#[derive(Debug, Deserialize, Clone)]
/// Scope for analysis queries
#[serde(rename_all = "camelCase")]
pub struct QueryScope {
    #[serde(rename = "type")]
    pub scope_type: String,
    pub path: Option<String>,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// Request for batch analysis operations
#[derive(Debug, Clone)]
pub struct BatchAnalysisRequest {
    pub queries: Vec<AnalysisQuery>,
    pub config: Option<AnalysisConfig>,
    pub no_suggestions: bool,
    pub max_suggestions: Option<usize>,
}

/// Result from a single query in batch
#[derive(Debug, Clone, Serialize)]
pub struct SingleQueryResult {
    pub command: String,
    pub kind: String,
    pub result: AnalysisResult,
}

/// Batch analysis result
#[derive(Debug, Clone, Serialize)]
pub struct BatchAnalysisResult {
    pub results: Vec<SingleQueryResult>,
    pub summary: BatchSummary,
    pub metadata: BatchMetadata,
    pub suggestions: Vec<ActionableSuggestion>,
}

/// File-level analysis result (internal to mill-handlers)
#[derive(Debug, Clone, Serialize)]
pub(crate) struct FileAnalysisResult {
    pub file_path: PathBuf,
    pub findings: Vec<Finding>,
    pub suggestions: Vec<ActionableSuggestion>,
}

// --- Shared Data Structures (mostly unchanged) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Summary of batch analysis
#[serde(rename_all = "camelCase")]
pub struct BatchSummary {
    pub total_queries: usize,
    pub total_files_scanned: usize,
    pub files_analyzed: usize,
    pub files_failed: usize,
    pub total_findings: usize,
    pub total_suggestions: usize,
    pub findings_by_severity: HashMap<String, usize>,
    pub suggestions_by_safety: HashMap<String, usize>,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Batch analysis metadata
#[serde(rename_all = "camelCase")]
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
    complexity_report: mill_ast::complexity::ComplexityReport,
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

    // 2. Load suggestion config and create generator
    let suggestion_config = SuggestionConfig::load().unwrap_or_default();
    let suggestion_generator = SuggestionGenerator::with_config(suggestion_config);

    // 3. Pre-parse all ASTs for optimization
    let ast_cache = batch_parse_asts(&all_files_vec, context).await;
    let ast_cache_hits = ast_cache.len();
    let ast_cache_misses = all_files_vec.len() - ast_cache_hits;

    // 3. Process each query
    let mut query_results = Vec::new();
    let mut failed_files_map = HashMap::new();
    let mut all_categories = HashSet::new();
    // Cache key: (file_path, category, kind) to allow same file analyzed with different queries
    let mut all_file_results: HashMap<(PathBuf, String, String), FileAnalysisResult> =
        HashMap::new();

    for query in &request.queries {
        let category = query
            .command
            .split('.')
            .next_back()
            .unwrap_or("")
            .to_string();
        if category.is_empty() {
            warn!(command = %query.command, "Skipping query with invalid command format");
            continue;
        }
        all_categories.insert(category.clone());

        let files_for_query = resolve_scope_to_files(&query.scope).await?;
        for file_path in &files_for_query {
            if let Some(cached_ast) = ast_cache.get(file_path) {
                let cache_key = (file_path.clone(), category.clone(), query.kind.clone());
                if let std::collections::hash_map::Entry::Vacant(entry) =
                    all_file_results.entry(cache_key)
                {
                    let default_config = get_analysis_config(context)
                        .map_err(|e| BatchError::AnalysisFailed(e.to_string()))?;
                    let config = request.config.as_ref().unwrap_or(default_config);
                    match analyze_file_with_cached_ast(
                        file_path,
                        cached_ast,
                        &category,
                        &query.kind,
                        config,
                        &suggestion_generator,
                        context,
                    )
                    .await
                    {
                        Ok(result_for_file) => {
                            entry.insert(result_for_file);
                        }
                        Err(e) => {
                            let file_path_str = file_path.display().to_string();
                            warn!(file_path=%file_path_str, error=%e, "Analysis failed for file in query");
                            failed_files_map.insert(file_path_str, e.to_string());
                        }
                    }
                }
            } else {
                let file_path_str = file_path.display().to_string();
                failed_files_map
                    .entry(file_path_str)
                    .or_insert_with(|| "File failed to parse".to_string());
            }
        }

        let files_for_query_set: HashSet<_> = files_for_query.iter().collect();
        let all_findings_for_query: Vec<Finding> = all_file_results
            .iter()
            .filter(|((path, cat, kind), _)| {
                files_for_query_set.contains(path) && cat == &category && kind == &query.kind
            })
            .flat_map(|(_, r)| r.findings.clone())
            .collect();

        let symbols_analyzed_for_query: usize = all_file_results
            .iter()
            .filter(|((path, cat, kind), _)| {
                files_for_query_set.contains(path) && cat == &category && kind == &query.kind
            })
            .filter_map(|((path, _, _), _)| ast_cache.get(path))
            .map(|ast| ast.symbols.len())
            .sum();

        let scope_path = query.scope.path.clone().unwrap_or_default();
        let scope = AnalysisScope {
            scope_type: query.scope.scope_type.clone(),
            path: scope_path,
            include: query.scope.include.clone(),
            exclude: query.scope.exclude.clone(),
        };

        let mut query_analysis_result = AnalysisResult::new(&category, &query.kind, scope);
        query_analysis_result.findings = all_findings_for_query;
        query_analysis_result.summary.files_analyzed = files_for_query.len();
        query_analysis_result.summary.symbols_analyzed = Some(symbols_analyzed_for_query);
        query_analysis_result.finalize(0); // Timings are for the whole batch

        query_results.push(SingleQueryResult {
            command: query.command.clone(),
            kind: query.kind.clone(),
            result: query_analysis_result,
        });
    }

    let mut all_suggestions: Vec<ActionableSuggestion> = if request.no_suggestions {
        Vec::new()
    } else {
        all_file_results
            .values()
            .flat_map(|r| r.suggestions.clone())
            .collect()
    };

    if !request.no_suggestions {
        // 4. Generate workspace-level suggestions
        let all_findings: Vec<Finding> = all_file_results
            .values()
            .flat_map(|r| r.findings.clone())
            .collect();
        let workspace_suggestions = generate_workspace_suggestions(&all_findings, context);
        all_suggestions.extend(workspace_suggestions);

        // 5. Deduplicate and rank suggestions
        deduplicate_suggestions(&mut all_suggestions);
        rank_suggestions(&mut all_suggestions);

        if let Some(max) = request.max_suggestions {
            all_suggestions.truncate(max);
        }
    }

    // 6. Build final summary and metadata
    let execution_time_ms = batch_start.elapsed().as_millis() as u64;
    let completed_at = chrono::Utc::now().to_rfc3339();

    let mut total_findings = 0;
    let mut findings_by_severity = HashMap::new();
    for res in &query_results {
        total_findings += res.result.findings.len();
        for finding in &res.result.findings {
            let severity_str = serde_json::to_string(&finding.severity)
                .unwrap_or_else(|_| "unknown".to_string())
                .replace('"', "");
            *findings_by_severity.entry(severity_str).or_insert(0) += 1;
        }
    }

    let total_suggestions = all_suggestions.len();
    let mut suggestions_by_safety = HashMap::new();
    for suggestion in &all_suggestions {
        let safety_str = serde_json::to_string(&suggestion.safety)
            .unwrap_or_else(|_| "unknown".to_string())
            .replace('"', "");
        *suggestions_by_safety.entry(safety_str).or_insert(0) += 1;
    }

    Ok(BatchAnalysisResult {
        results: query_results,
        summary: BatchSummary {
            total_queries: request.queries.len(),
            total_files_scanned: all_files_vec.len(),
            files_analyzed: ast_cache_hits,
            files_failed: all_files_vec.len() - ast_cache_hits,
            total_findings,
            total_suggestions,
            findings_by_severity,
            suggestions_by_safety,
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
        suggestions: all_suggestions,
    })
}

use globset::{Glob, GlobSetBuilder};

async fn resolve_scope_to_files(scope: &QueryScope) -> Result<Vec<PathBuf>, BatchError> {
    let root_path = scope.path.as_ref().ok_or_else(|| {
        BatchError::InvalidScope(format!("'{}' scope requires a 'path'", scope.scope_type))
    })?;

    match scope.scope_type.as_str() {
        "file" => Ok(vec![PathBuf::from(root_path)]),
        "directory" | "workspace" => {
            let mut files = Vec::new();
            let walker = WalkBuilder::new(root_path).build();

            let mut include_builder = GlobSetBuilder::new();
            for pattern in &scope.include {
                include_builder
                    .add(Glob::new(pattern).map_err(|e| BatchError::InvalidScope(e.to_string()))?);
            }
            let include_set = include_builder
                .build()
                .map_err(|e| BatchError::InvalidScope(e.to_string()))?;

            let mut exclude_builder = GlobSetBuilder::new();
            for pattern in &scope.exclude {
                exclude_builder
                    .add(Glob::new(pattern).map_err(|e| BatchError::InvalidScope(e.to_string()))?);
            }
            let exclude_set = exclude_builder
                .build()
                .map_err(|e| BatchError::InvalidScope(e.to_string()))?;

            for entry in walker.flatten() {
                if entry.file_type().is_some_and(|ft| ft.is_file()) {
                    let path = entry.path();
                    if !exclude_set.is_match(path)
                        && (include_set.is_empty() || include_set.is_match(path))
                    {
                        files.push(path.to_path_buf());
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

    let complexity_report = mill_ast::complexity::analyze_file_complexity(
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
    config: &AnalysisConfig,
    _suggestion_generator: &SuggestionGenerator,
    context: &ToolHandlerContext,
) -> Result<FileAnalysisResult, BatchError> {
    let file_path_str = file_path.display().to_string();

    let findings: Vec<Finding> = match category {
        "quality" => match kind {
            "complexity" => {
                let mut findings = vec![];
                for func in &cached_ast.complexity_report.functions {
                    if func.complexity.cognitive >= 10 || func.complexity.cyclomatic >= 15 {
                        let severity = match func.rating {
                            mill_ast::complexity::ComplexityRating::VeryComplex => Severity::High,
                            mill_ast::complexity::ComplexityRating::Complex => Severity::Medium,
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
                                    start: Position {
                                        line: func.line as u32,
                                        character: 0,
                                    },
                                    end: Position {
                                        line: (func.line + func.metrics.sloc as usize) as u32,
                                        character: 0,
                                    },
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
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            "maintainability" => quality_handler::analyze_maintainability(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            "readability" => quality_handler::analyze_readability(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            _ => {
                return Err(BatchError::AnalysisFailed(format!(
                    "Unsupported quality kind: {}",
                    kind
                )))
            }
        },
        "dead_code" => {
            // Dead code analysis now uses workspace-level LSP + call graph reachability.
            // This cannot be done on a per-file basis with cached AST data.
            // Use the dedicated `analyze.dead_code` tool instead.
            return Err(BatchError::AnalysisFailed(
                "Dead code analysis requires workspace-level analysis. \
                 Use the 'analyze.dead_code' tool instead of batch analysis."
                    .to_string(),
            ));
        }
        "dependencies" => match kind {
            "imports" => dependencies_handler::detect_imports(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            "graph" => dependencies_handler::detect_graph(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            "circular" => dependencies_handler::detect_circular(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            "coupling" => dependencies_handler::detect_coupling(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            "cohesion" => dependencies_handler::detect_cohesion(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            "depth" => dependencies_handler::detect_depth(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            _ => {
                return Err(BatchError::AnalysisFailed(format!(
                    "Unsupported dependencies kind: {}",
                    kind
                )))
            }
        },
        "structure" => match kind {
            "symbols" => structure_handler::detect_symbols(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            "hierarchy" => structure_handler::detect_hierarchy(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            "interfaces" => structure_handler::detect_interfaces(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            "inheritance" => structure_handler::detect_inheritance(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            "modules" => structure_handler::detect_modules(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            _ => {
                return Err(BatchError::AnalysisFailed(format!(
                    "Unsupported structure kind: {}",
                    kind
                )))
            }
        },
        "documentation" => match kind {
            "coverage" => documentation_handler::detect_coverage(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            "quality" => documentation_handler::detect_quality(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            "style" => documentation_handler::detect_style(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            "examples" => documentation_handler::detect_examples(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            "todos" => documentation_handler::detect_todos(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            _ => {
                return Err(BatchError::AnalysisFailed(format!(
                    "Unsupported documentation kind: {}",
                    kind
                )))
            }
        },
        "tests" => match kind {
            "coverage" => tests_handler::detect_coverage(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            "quality" => tests_handler::detect_quality(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            "assertions" => tests_handler::detect_assertions(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            "organization" => tests_handler::detect_organization(
                &cached_ast.complexity_report,
                &cached_ast.content,
                &cached_ast.symbols,
                &cached_ast.language,
                &file_path_str,
                context.app_state.language_plugins.as_ref(),
                config,
            ),
            _ => {
                return Err(BatchError::AnalysisFailed(format!(
                    "Unsupported tests kind: {}",
                    kind
                )))
            }
        },
        _ => {
            return Err(BatchError::AnalysisFailed(format!(
                "Unsupported category '{}' in batch analysis",
                category
            )))
        }
    };

    let context = AnalysisContext {
        file_path: file_path_str.clone(),
        has_full_type_info: false, // Assume no type info for now
        has_partial_type_info: false,
        ast_parse_errors: 0,
    };

    let candidates = findings
        .iter()
        .flat_map(finding_to_candidate)
        .collect::<Vec<_>>();

    let suggestions = _suggestion_generator.generate_multiple(candidates, &context);

    let result = FileAnalysisResult {
        file_path: file_path.to_path_buf(),
        findings,
        suggestions,
    };

    Ok(result)
}

fn finding_to_candidate(finding: &Finding) -> Option<RefactoringCandidate> {
    let (refactor_type, scope) = match finding.kind.as_str() {
        "long_method" => (RefactorType::ExtractMethod, Scope::Function),
        "complexity_hotspot" => (RefactorType::ExtractMethod, Scope::Function),
        "unused_import" | "unused_imports" => (RefactorType::RemoveUnusedImport, Scope::File),
        "unused_symbols" => (RefactorType::RemoveDeadCode, Scope::File),
        _ => return None,
    };

    let location = finding.location.range.as_ref().map(|r| Location {
        file: finding.location.file_path.clone(),
        line: r.start.line as usize,
        character: r.start.character as usize,
    })?;

    let candidate = RefactoringCandidate {
        refactor_type,
        message: finding.message.clone(),
        scope,
        has_side_effects: false,
        reference_count: None,
        is_unreachable: false,
        is_recursive: false,
        involves_generics: false,
        involves_macros: false,
        evidence_strength: EvidenceStrength::Medium,
        location,
        refactor_call_args: json!({
            "file_path": finding.location.file_path,
            "range": finding.location.range,
        }),
    };

    Some(candidate)
}

fn deduplicate_suggestions(suggestions: &mut Vec<ActionableSuggestion>) {
    let mut seen = HashSet::new();
    suggestions.retain(|s| {
        let key = (
            s.message.clone(),
            s.refactor_call
                .as_ref()
                .map(|rc| rc.tool.clone())
                .unwrap_or_default(),
        );
        seen.insert(key)
    });
}

fn rank_suggestions(suggestions: &mut [ActionableSuggestion]) {
    suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
}

fn generate_workspace_suggestions(
    _all_findings: &[Finding],
    _context: &ToolHandlerContext,
) -> Vec<ActionableSuggestion> {
    // TODO: Implement workspace-level suggestion generation
    vec![]
}
