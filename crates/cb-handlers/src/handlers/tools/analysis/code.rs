use crate::handlers::tools::ToolHandlerContext;
use cb_core::model::mcp::ToolCall;
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use tracing::{debug, info};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
enum AnalysisReportFormat {
    Full,
    Complexity,
}

pub async fn handle_analyze_code(
    context: &ToolHandlerContext,
    tool_call: &ToolCall,
) -> ServerResult<Value> {
    let args = tool_call.arguments.clone().unwrap_or(json!({}));

    let file_path_str = args
        .get("file_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ServerError::InvalidRequest("Missing file_path parameter".into()))?;

    let report_format: AnalysisReportFormat =
        serde_json::from_value(args.get("report_format").cloned().unwrap_or(json!("full")))
            .unwrap_or(AnalysisReportFormat::Full);

    debug!(
        file_path = %file_path_str,
        "Analyzing code"
    );

    let file_path = Path::new(file_path_str);

    let extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| {
            ServerError::InvalidRequest(format!("File has no extension: {}", file_path_str))
        })?;

    let content = context
        .app_state
        .file_service
        .read_file(file_path)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to read file: {}", e)))?;

    let plugin = context
        .app_state
        .language_plugins
        .get_plugin(extension)
        .ok_or_else(|| {
            ServerError::Unsupported(format!(
                "No language plugin found for extension: {}",
                extension
            ))
        })?;

    let parsed = plugin
        .parse(&content)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to parse file: {}", e)))?;

    let language = plugin.metadata().name;

    info!(
        file_path = %file_path_str,
        language = %language,
        symbols_count = parsed.symbols.len(),
        "Generating code analysis report"
    );

    let complexity_report = cb_ast::complexity::analyze_file_complexity(
        file_path_str,
        &content,
        &parsed.symbols,
        &language,
    );

    if matches!(report_format, AnalysisReportFormat::Complexity) {
        return serde_json::to_value(complexity_report)
            .map_err(|e| ServerError::Internal(format!("Failed to serialize report: {}", e)));
    }

    // Full report (formerly suggest_refactoring)
    let mut suggestions = Vec::new();

    for func in &complexity_report.functions {
        for issue in &func.issues {
            let (kind, priority) = if issue.contains("cognitive complexity") {
                (RefactoringKind::ReduceComplexity, "high")
            } else if issue.contains("parameters") {
                (RefactoringKind::ConsolidateParameters, "medium")
            } else if issue.contains("nesting") {
                (RefactoringKind::ReduceNesting, "high")
            } else if issue.contains("comment ratio") {
                (RefactoringKind::ImproveDocumentation, "low")
            } else {
                (RefactoringKind::ReduceComplexity, "medium")
            };

            suggestions.push(RefactoringSuggestion {
                kind,
                location: func.line,
                function_name: Some(func.name.clone()),
                description: format!("Function '{}': {}", func.name, issue),
                suggestion: generate_suggestion_text(&kind, func),
                priority: priority.to_string(),
            });
        }

        if let Some(recommendation) = &func.recommendation {
            suggestions.push(RefactoringSuggestion {
                kind: RefactoringKind::ReduceComplexity,
                location: func.line,
                function_name: Some(func.name.clone()),
                description: format!(
                    "Function '{}' has cognitive complexity of {} (cyclomatic: {}, rating: {})",
                    func.name,
                    func.complexity.cognitive,
                    func.complexity.cyclomatic,
                    func.rating.description()
                ),
                suggestion: recommendation.clone(),
                priority: match func.rating {
                    cb_ast::complexity::ComplexityRating::VeryComplex => "high",
                    cb_ast::complexity::ComplexityRating::Complex => "medium",
                    _ => "low",
                }
                .to_string(),
            });
        }

        if func.metrics.sloc > 50 {
            suggestions.push(RefactoringSuggestion {
                kind: RefactoringKind::ExtractFunction,
                location: func.line,
                function_name: Some(func.name.clone()),
                description: format!(
                    "Function '{}' has {} source lines of code (>50 SLOC recommended)",
                    func.name, func.metrics.sloc
                ),
                suggestion: "Consider breaking this function into smaller, more focused functions."
                    .to_string(),
                priority: if func.metrics.sloc > 100 {
                    "high"
                } else {
                    "medium"
                }
                .to_string(),
            });
        }
    }

    let duplicate_suggestions = detect_duplicate_patterns(&content, &language);
    suggestions.extend(duplicate_suggestions);

    let magic_number_suggestions = detect_magic_numbers(&content, &parsed.symbols, &language);
    suggestions.extend(magic_number_suggestions);

    suggestions.sort_by(|a, b| {
        let priority_order = |p: &str| match p {
            "high" => 0,
            "medium" => 1,
            "low" => 2,
            _ => 3,
        };
        priority_order(&a.priority).cmp(&priority_order(&b.priority))
    });

    info!(
        file_path = %file_path_str,
        suggestions_count = suggestions.len(),
        "Refactoring analysis complete"
    );

    Ok(json!({
        "file_path": file_path_str,
        "language": language,
        "suggestions": suggestions,
        "total_suggestions": suggestions.len(),
        "complexity_summary": {
            "average_complexity": complexity_report.average_complexity,
            "average_cognitive_complexity": complexity_report.average_cognitive_complexity,
            "max_complexity": complexity_report.max_complexity,
            "max_cognitive_complexity": complexity_report.max_cognitive_complexity,
            "total_functions": complexity_report.total_functions,
            "total_sloc": complexity_report.total_sloc,
            "average_sloc": complexity_report.average_sloc,
            "total_issues": complexity_report.total_issues,
        }
    }))
}

#[derive(Debug, Serialize, Deserialize)]
struct RefactoringSuggestion {
    kind: RefactoringKind,
    location: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    function_name: Option<String>,
    description: String,
    suggestion: String,
    priority: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RefactoringKind {
    ReduceComplexity,
    ReduceNesting,
    ConsolidateParameters,
    ImproveDocumentation,
    ExtractFunction,
    ExtractVariable,
    RemoveDuplication,
    ReplaceMagicNumber,
}

fn generate_suggestion_text(
    kind: &RefactoringKind,
    _func: &cb_ast::complexity::FunctionComplexity,
) -> String {
    match kind {
        RefactoringKind::ReduceComplexity => {
            "Consider refactoring to reduce complexity.".to_string()
        }
        RefactoringKind::ReduceNesting => {
            "Reduce nesting depth using early returns or guard clauses.".to_string()
        }
        RefactoringKind::ConsolidateParameters => {
            "Consolidate parameters using a configuration object/struct.".to_string()
        }
        RefactoringKind::ImproveDocumentation => {
            "Add documentation to explain the function's purpose.".to_string()
        }
        RefactoringKind::ExtractFunction => {
            "Extract logical blocks into separate functions.".to_string()
        }
        RefactoringKind::ExtractVariable => {
            "Extract complex expressions into named variables.".to_string()
        }
        RefactoringKind::RemoveDuplication => {
            "Extract duplicate code into a shared function.".to_string()
        }
        RefactoringKind::ReplaceMagicNumber => {
            "Replace magic numbers with named constants.".to_string()
        }
    }
}

fn detect_duplicate_patterns(_content: &str, _language: &str) -> Vec<RefactoringSuggestion> {
    Vec::new()
}

fn detect_magic_numbers(
    content: &str,
    _symbols: &[cb_plugin_api::Symbol],
    language: &str,
) -> Vec<RefactoringSuggestion> {
    let mut suggestions = Vec::new();
    let number_pattern = match language.to_lowercase().as_str() {
        "rust" | "go" | "java" | "typescript" | "javascript" | "python" => {
            Regex::new(r"\b(?:[2-9]|[1-9]\d+)(?:\.\d+)?\b").ok()
        }
        _ => None,
    };

    if let Some(pattern) = number_pattern {
        let mut found_numbers = std::collections::HashMap::new();
        for (i, line) in content.lines().enumerate() {
            if line.trim().starts_with("//") || line.trim().starts_with('#') {
                continue;
            }
            for cap in pattern.find_iter(line) {
                let number = cap.as_str();
                found_numbers
                    .entry(number.to_string())
                    .or_insert_with(Vec::new)
                    .push(i + 1);
            }
        }

        for (number, lines) in found_numbers {
            if lines.len() >= 2 {
                suggestions.push(RefactoringSuggestion {
                    kind: RefactoringKind::ReplaceMagicNumber,
                    location: lines[0],
                    function_name: None,
                    description: format!("Magic number '{}' appears {} times", number, lines.len()),
                    suggestion: format!("Consider extracting '{}' to a named constant", number),
                    priority: if lines.len() > 3 { "medium" } else { "low" }.to_string(),
                });
            }
        }
    }
    suggestions
}
