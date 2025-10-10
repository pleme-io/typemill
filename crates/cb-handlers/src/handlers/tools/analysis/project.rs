use crate::handlers::tools::ToolHandlerContext;
use cb_core::model::mcp::ToolCall;
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tracing::info;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
enum ProjectReportFormat {
    Full,
    Hotspots,
}

/// Scans an entire project and generates either a full complexity report or a hotspots summary.
pub async fn handle_analyze_project(
    context: &ToolHandlerContext,
    tool_call: &ToolCall,
) -> ServerResult<Value> {
    use cb_ast::complexity::{
        aggregate_class_complexity, analyze_file_complexity, ComplexityHotspotsReport,
        FileComplexitySummary, FunctionHotspot, ProjectComplexityReport,
    };

    let args = tool_call.arguments.clone().unwrap_or(json!({}));

    // Parse parameters
    let directory_path = args
        .get("directory_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ServerError::InvalidRequest("Missing directory_path parameter".into()))?;

    let report_format: ProjectReportFormat =
        serde_json::from_value(args.get("report_format").cloned().unwrap_or(json!("full")))
            .unwrap_or(ProjectReportFormat::Full);

    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
    let metric = args
        .get("metric")
        .and_then(|v| v.as_str())
        .unwrap_or("cognitive");

    info!(
        directory_path = %directory_path,
        "Starting project analysis"
    );

    let dir_path = Path::new(directory_path);

    // List files and filter
    let files = context
        .app_state
        .file_service
        .list_files(dir_path, true)
        .await?;

    let supported_extensions: Vec<String> =
        context.app_state.language_plugins.supported_extensions();

    let analyzable_files: Vec<PathBuf> = files
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

    info!(
        analyzable_count = analyzable_files.len(),
        "Filtered to analyzable files"
    );

    // Analyze all files
    let mut all_file_summaries = Vec::new();
    let mut all_function_hotspots = Vec::new();
    let mut all_classes = Vec::new();
    let mut errors = Vec::new();

    for file_path in &analyzable_files {
        let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let plugin = match context.app_state.language_plugins.get_plugin(extension) {
            Some(p) => p,
            None => continue,
        };
        let language = plugin.metadata().name;

        let content = match context.app_state.file_service.read_file(file_path).await {
            Ok(c) => c,
            Err(e) => {
                errors.push(json!({"file": file_path.display().to_string(), "error": format!("Read error: {}", e)}));
                continue;
            }
        };
        let parsed = match plugin.parse(&content).await {
            Ok(p) => p,
            Err(e) => {
                errors.push(json!({"file": file_path.display().to_string(), "error": format!("Parse error: {}", e)}));
                continue;
            }
        };

        let report = analyze_file_complexity(
            &file_path.to_string_lossy(),
            &content,
            &parsed.symbols,
            &language,
        );

        let file_classes =
            aggregate_class_complexity(&file_path.to_string_lossy(), &report.functions, &language);

        all_file_summaries.push(FileComplexitySummary {
            file_path: file_path.to_string_lossy().to_string(),
            function_count: report.total_functions,
            class_count: file_classes.len(),
            average_complexity: report.average_complexity,
            average_cognitive_complexity: report.average_cognitive_complexity,
            max_complexity: report.max_complexity,
            total_issues: report.total_issues,
        });

        for func in &report.functions {
            all_function_hotspots.push(FunctionHotspot {
                name: func.name.clone(),
                file_path: file_path.to_string_lossy().to_string(),
                line: func.line,
                complexity: func.complexity.cyclomatic,
                cognitive_complexity: func.complexity.cognitive,
                rating: func.rating,
                sloc: func.metrics.sloc,
            });
        }
        all_classes.extend(file_classes);
    }

    // Generate the requested report
    match report_format {
        ProjectReportFormat::Full => {
            let total_files = all_file_summaries.len();
            let total_functions: usize = all_file_summaries.iter().map(|f| f.function_count).sum();
            let total_sloc: u32 = all_classes.iter().map(|c| c.total_sloc).sum();

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

            let max_complexity = all_file_summaries
                .iter()
                .map(|f| f.max_complexity)
                .max()
                .unwrap_or(0);
            let max_cognitive_complexity = all_function_hotspots
                .iter()
                .map(|f| f.cognitive_complexity)
                .max()
                .unwrap_or(0);

            let report = ProjectComplexityReport {
                directory: directory_path.to_string(),
                total_files,
                total_functions,
                total_classes: all_classes.len(),
                files: all_file_summaries,
                classes: all_classes,
                average_complexity,
                average_cognitive_complexity: average_cognitive,
                max_complexity,
                max_cognitive_complexity,
                total_sloc,
                hotspots_summary: format!("Analyzed {} files.", total_files),
            };
            let mut value = serde_json::to_value(report)?;
            if !errors.is_empty() {
                value["errors"] = json!(errors);
            }
            Ok(value)
        }
        ProjectReportFormat::Hotspots => {
            all_function_hotspots.sort_by(|a, b| {
                if metric == "cognitive" {
                    b.cognitive_complexity.cmp(&a.cognitive_complexity)
                } else {
                    b.complexity.cmp(&a.complexity)
                }
            });
            let top_functions: Vec<_> = all_function_hotspots.into_iter().take(limit).collect();

            all_classes.sort_by(|a, b| {
                if metric == "cognitive" {
                    b.total_cognitive_complexity
                        .cmp(&a.total_cognitive_complexity)
                } else {
                    b.total_complexity.cmp(&a.total_complexity)
                }
            });
            let top_classes: Vec<_> = all_classes.into_iter().take(limit).collect();

            let report = ComplexityHotspotsReport {
                directory: directory_path.to_string(),
                metric: metric.to_string(),
                top_functions,
                top_classes,
                summary: format!("Top {} hotspots identified.", limit),
            };
            let mut value = serde_json::to_value(report)?;
            if !errors.is_empty() {
                value["errors"] = json!(errors);
            }
            Ok(value)
        }
    }
}
