use crate::handlers::tools::ToolHandlerContext;
use cb_core::model::mcp::ToolCall;
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Handle analyze_project_complexity tool call
///
/// Scans an entire directory or project for complexity metrics across all supported files.
pub async fn handle_analyze_project_complexity(
    context: &ToolHandlerContext,
    tool_call: &ToolCall,
) -> ServerResult<Value> {
    use cb_ast::complexity::{
        aggregate_class_complexity, analyze_file_complexity, FileComplexitySummary,
        ProjectComplexityReport,
    };

    let args = tool_call.arguments.clone().unwrap_or(json!({}));

    // Parse parameters
    let directory_path = args
        .get("directory_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ServerError::InvalidRequest("Missing directory_path parameter".into()))?;

    let pattern = args.get("pattern").and_then(|v| v.as_str());
    let include_tests = args
        .get("include_tests")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    info!(
        directory_path = %directory_path,
        pattern = ?pattern,
        include_tests = include_tests,
        "Starting project complexity analysis"
    );

    let dir_path = Path::new(directory_path);

    // List files in directory
    let files = context
        .app_state
        .file_service
        .list_files_with_pattern(dir_path, true, pattern)
        .await?;

    info!(files_count = files.len(), "Found files to analyze");

    // Filter by supported extensions
    let supported_extensions: Vec<String> = context
        .app_state
        .language_plugins
        .supported_extensions()
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    let mut analyzable_files: Vec<PathBuf> = files
        .iter()
        .filter_map(|file| {
            let path = if file.starts_with('/') {
                PathBuf::from(file)
            } else {
                dir_path.join(file)
            };

            // Check extension
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if supported_extensions.contains(&ext.to_string()) {
                    // Filter out test files if requested
                    if !include_tests {
                        let file_str = path.to_string_lossy();
                        if file_str.contains("test") || file_str.contains("spec") {
                            return None;
                        }
                    }
                    return Some(path);
                }
            }
            None
        })
        .collect();

    analyzable_files.sort();

    info!(
        analyzable_count = analyzable_files.len(),
        "Filtered to analyzable files"
    );

    // Analyze each file sequentially (to avoid AST cache thrashing)
    let mut all_file_summaries = Vec::new();
    let mut all_classes = Vec::new();
    let mut total_functions = 0;
    let mut total_sloc = 0;
    let mut project_max_complexity = 0;
    let mut project_max_cognitive = 0;
    let mut errors = Vec::new();

    for file_path in &analyzable_files {
        // Get file extension to determine language
        let extension = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown");

        // Get language plugin
        let plugin = match context.app_state.language_plugins.get_plugin(extension) {
            Some(p) => p,
            None => {
                warn!(
                    file_path = %file_path.display(),
                    extension = %extension,
                    "No language plugin found"
                );
                continue;
            }
        };

        let language = plugin.metadata().name;

        // Read file content
        let content = match context.app_state.file_service.read_file(file_path).await {
            Ok(c) => c,
            Err(e) => {
                warn!(
                    file_path = %file_path.display(),
                    error = %e,
                    "Failed to read file"
                );
                errors.push(json!({
                    "file": file_path.to_string_lossy(),
                    "error": format!("Failed to read: {}", e)
                }));
                continue;
            }
        };

        // Parse file
        let parsed = match plugin.parse(&content).await {
            Ok(p) => p,
            Err(e) => {
                warn!(
                    file_path = %file_path.display(),
                    error = %e,
                    "Failed to parse file"
                );
                errors.push(json!({
                    "file": file_path.to_string_lossy(),
                    "error": format!("Failed to parse: {}", e)
                }));
                continue;
            }
        };

        // Analyze complexity
        let report =
            analyze_file_complexity(&file_path.to_string_lossy(), &content, &parsed.symbols, language);

        // Aggregate class-level complexity
        let file_classes =
            aggregate_class_complexity(&file_path.to_string_lossy(), &report.functions, language);

        // Update project-level stats
        total_functions += report.total_functions;
        total_sloc += report.total_sloc;
        project_max_complexity = project_max_complexity.max(report.max_complexity);
        project_max_cognitive = project_max_cognitive.max(report.max_cognitive_complexity);

        // Create file summary
        all_file_summaries.push(FileComplexitySummary {
            file_path: file_path.to_string_lossy().to_string(),
            function_count: report.total_functions,
            class_count: file_classes.len(),
            average_complexity: report.average_complexity,
            average_cognitive_complexity: report.average_cognitive_complexity,
            max_complexity: report.max_complexity,
            total_issues: report.total_issues,
        });

        // Collect classes
        all_classes.extend(file_classes);
    }

    // Calculate project-wide averages
    let total_files = all_file_summaries.len();
    let total_classes = all_classes.len();

    let average_complexity = if total_functions > 0 {
        all_file_summaries
            .iter()
            .map(|f| f.average_complexity * f.function_count as f64)
            .sum::<f64>()
            / total_functions as f64
    } else {
        0.0
    };

    let average_cognitive = if total_functions > 0 {
        all_file_summaries
            .iter()
            .map(|f| f.average_cognitive_complexity * f.function_count as f64)
            .sum::<f64>()
            / total_functions as f64
    } else {
        0.0
    };

    // Generate summary
    let total_issues: usize = all_file_summaries.iter().map(|f| f.total_issues).sum();
    let hotspots_summary = if total_issues == 0 {
        format!(
            "{} functions analyzed across {} files. No issues detected.",
            total_functions, total_files
        )
    } else {
        format!(
            "{} functions analyzed across {} files. {} issue{} detected that need{} attention.",
            total_functions,
            total_files,
            total_issues,
            if total_issues == 1 { "" } else { "s" },
            if total_issues == 1 { "s" } else { "" }
        )
    };

    let report = ProjectComplexityReport {
        directory: directory_path.to_string(),
        total_files,
        total_functions,
        total_classes,
        files: all_file_summaries,
        classes: all_classes,
        average_complexity,
        average_cognitive_complexity: average_cognitive,
        max_complexity: project_max_complexity,
        max_cognitive_complexity: project_max_cognitive,
        total_sloc,
        hotspots_summary,
    };

    info!(
        total_files = total_files,
        total_functions = total_functions,
        total_classes = total_classes,
        errors_count = errors.len(),
        "Project complexity analysis complete"
    );

    let mut result = serde_json::to_value(report)
        .map_err(|e| ServerError::Internal(format!("Failed to serialize report: {}", e)))?;

    if !errors.is_empty() {
        if let Some(obj) = result.as_object_mut() {
            obj.insert("errors".to_string(), json!(errors));
        }
    }

    Ok(result)
}

/// Handle find_complexity_hotspots tool call
///
/// Finds the most complex functions and classes in a project (top N worst offenders).
pub async fn handle_find_complexity_hotspots(
    context: &ToolHandlerContext,
    tool_call: &ToolCall,
) -> ServerResult<Value> {
    use cb_ast::complexity::{
        aggregate_class_complexity, analyze_file_complexity, ComplexityHotspotsReport,
        FunctionHotspot,
    };

    let args = tool_call.arguments.clone().unwrap_or(json!({}));

    // Parse parameters
    let directory_path = args
        .get("directory_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ServerError::InvalidRequest("Missing directory_path parameter".into()))?;

    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

    let metric = args
        .get("metric")
        .and_then(|v| v.as_str())
        .unwrap_or("cognitive");

    if metric != "cognitive" && metric != "cyclomatic" {
        return Err(ServerError::InvalidRequest(
            "metric must be 'cognitive' or 'cyclomatic'".into(),
        ));
    }

    info!(
        directory_path = %directory_path,
        limit = limit,
        metric = %metric,
        "Starting hotspot analysis"
    );

    let dir_path = Path::new(directory_path);

    // List all files recursively
    let files = context
        .app_state
        .file_service
        .list_files(dir_path, true)
        .await?;

    // Filter by supported extensions
    let supported_extensions: Vec<String> = context
        .app_state
        .language_plugins
        .supported_extensions()
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    let mut analyzable_files: Vec<PathBuf> = files
        .iter()
        .filter_map(|file| {
            let path = if file.starts_with('/') {
                PathBuf::from(file)
            } else {
                dir_path.join(file)
            };

            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if supported_extensions.contains(&ext.to_string()) {
                    return Some(path);
                }
            }
            None
        })
        .collect();

    analyzable_files.sort();

    info!(
        analyzable_count = analyzable_files.len(),
        "Filtered to analyzable files"
    );

    // Collect all function hotspots
    let mut all_hotspots = Vec::new();
    let mut all_classes = Vec::new();

    for file_path in &analyzable_files {
        // Get file extension to determine language
        let extension = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown");

        // Get language plugin
        let plugin = match context.app_state.language_plugins.get_plugin(extension) {
            Some(p) => p,
            None => continue,
        };

        let language = plugin.metadata().name;

        // Read and parse file
        let content = match context.app_state.file_service.read_file(file_path).await {
            Ok(c) => c,
            Err(_) => continue,
        };

        let parsed = match plugin.parse(&content).await {
            Ok(p) => p,
            Err(_) => continue,
        };

        // Analyze complexity
        let report =
            analyze_file_complexity(&file_path.to_string_lossy(), &content, &parsed.symbols, language);

        // Convert functions to hotspots
        for func in &report.functions {
            all_hotspots.push(FunctionHotspot {
                name: func.name.clone(),
                file_path: file_path.to_string_lossy().to_string(),
                line: func.line,
                complexity: func.complexity.cyclomatic,
                cognitive_complexity: func.complexity.cognitive,
                rating: func.rating,
                sloc: func.metrics.sloc,
            });
        }

        // Aggregate class complexity
        let file_classes =
            aggregate_class_complexity(&file_path.to_string_lossy(), &report.functions, language);
        all_classes.extend(file_classes);
    }

    // Sort and take top N functions
    all_hotspots.sort_by(|a, b| {
        if metric == "cognitive" {
            b.cognitive_complexity.cmp(&a.cognitive_complexity)
        } else {
            b.complexity.cmp(&a.complexity)
        }
    });
    let top_functions: Vec<_> = all_hotspots.into_iter().take(limit).collect();

    // Sort and take top N classes
    all_classes.sort_by(|a, b| {
        if metric == "cognitive" {
            b.total_cognitive_complexity
                .cmp(&a.total_cognitive_complexity)
        } else {
            b.total_complexity.cmp(&a.total_complexity)
        }
    });
    let top_classes: Vec<_> = all_classes.into_iter().take(limit).collect();

    // Generate summary
    let very_complex_count = top_functions
        .iter()
        .filter(|f| matches!(f.rating, cb_ast::complexity::ComplexityRating::VeryComplex))
        .count();

    let summary = if very_complex_count > 0 {
        format!(
            "Top {} complexity hotspots identified. {} very complex function{} require{} immediate refactoring.",
            limit,
            very_complex_count,
            if very_complex_count == 1 { "" } else { "s" },
            if very_complex_count == 1 { "s" } else { "" }
        )
    } else {
        format!(
            "Top {} complexity hotspots identified. No critical issues found.",
            limit
        )
    };

    let report = ComplexityHotspotsReport {
        directory: directory_path.to_string(),
        metric: metric.to_string(),
        top_functions,
        top_classes,
        summary,
    };

    info!(
        hotspots_count = report.top_functions.len(),
        classes_count = report.top_classes.len(),
        "Hotspot analysis complete"
    );

    serde_json::to_value(report)
        .map_err(|e| ServerError::Internal(format!("Failed to serialize report: {}", e)))
}