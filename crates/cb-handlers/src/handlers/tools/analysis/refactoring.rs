use crate::handlers::tools::ToolHandlerContext;
use cb_core::model::mcp::ToolCall;
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use tracing::{debug, info};

pub async fn handle_suggest_refactoring(
    context: &ToolHandlerContext,
    tool_call: &ToolCall,
) -> ServerResult<Value> {
    let args = tool_call.arguments.clone().unwrap_or(json!({}));

    let file_path_str = args
        .get("file_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ServerError::InvalidRequest("Missing file_path parameter".into()))?;

    debug!(
        file_path = %file_path_str,
        "Suggesting refactorings"
    );

    let file_path = Path::new(file_path_str);

    // Get file extension
    let extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| {
            ServerError::InvalidRequest(format!("File has no extension: {}", file_path_str))
        })?;

    // Read file content
    let content = context
        .app_state
        .file_service
        .read_file(file_path)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to read file: {}", e)))?;

    // Find language plugin
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

    // Parse file to get symbols
    let parsed = plugin
        .parse(&content)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to parse file: {}", e)))?;

    let language = plugin.metadata().name;

    info!(
        file_path = %file_path_str,
        language = %language,
        symbols_count = parsed.symbols.len(),
        "Analyzing for refactoring suggestions"
    );

    // Generate refactoring suggestions based on patterns
    let mut suggestions = Vec::new();

    // 1. Analyze complexity and code metrics
    let complexity_report = cb_ast::complexity::analyze_file_complexity(
        file_path_str,
        &content,
        &parsed.symbols,
        language,
    );

    for func in &complexity_report.functions {
        // Add all issues detected by complexity analysis
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

        // Add general complexity recommendation if provided
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

        // 2. Check for long functions using SLOC instead of total lines
        if func.metrics.sloc > 50 {
            suggestions.push(RefactoringSuggestion {
                kind: RefactoringKind::ExtractFunction,
                location: func.line,
                function_name: Some(func.name.clone()),
                description: format!(
                    "Function '{}' has {} source lines of code (>50 SLOC recommended)",
                    func.name,
                    func.metrics.sloc
                ),
                suggestion: "Consider breaking this function into smaller, more focused functions. Extract logical blocks into separate functions with descriptive names.".to_string(),
                priority: if func.metrics.sloc > 100 { "high" } else { "medium" }.to_string(),
            });
        }
    }

    // 3. Check for duplicate code patterns
    let duplicate_suggestions = detect_duplicate_patterns(&content, language);
    suggestions.extend(duplicate_suggestions);

    // 4. Check for magic numbers
    let magic_number_suggestions = detect_magic_numbers(&content, &parsed.symbols, language);
    suggestions.extend(magic_number_suggestions);

    // Sort suggestions by priority
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
        "complexity_report": {
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
    func: &cb_ast::complexity::FunctionComplexity,
) -> String {
    match kind {
        RefactoringKind::ReduceComplexity => {
            if func.complexity.cognitive > 20 {
                format!(
                    "This function has very high cognitive complexity ({}). Consider:
- Breaking it into smaller functions (extract method pattern)
- Using early returns to reduce nesting
- Extracting complex conditional logic into named boolean functions
- Simplifying nested if statements with guard clauses",
                    func.complexity.cognitive
                )
            } else {
                "Consider refactoring to reduce complexity. Break down complex logic into smaller, testable functions.".to_string()
            }
        }
        RefactoringKind::ReduceNesting => {
            format!(
                "Reduce nesting depth from {} to 2-3 levels using:
- Early returns (guard clauses): if (!condition) return;
- Extract nested blocks into separate functions
- Invert conditions to flatten structure
- Replace nested if-else with strategy pattern or lookup tables",
                func.complexity.max_nesting_depth
            )
        }
        RefactoringKind::ConsolidateParameters => {
            format!(
                "Consolidate {} parameters using:
- Create a configuration object/struct grouping related parameters
- Use the builder pattern for complex initialization
- Consider if this function is doing too much (Single Responsibility Principle)",
                func.metrics.parameters
            )
        }
        RefactoringKind::ImproveDocumentation => {
            format!(
                "Add documentation (current comment ratio: {:.2}):
- Add function/method docstring describing purpose
- Document parameters and return values
- Include usage examples for complex functions
- Explain non-obvious business logic",
                func.metrics.comment_ratio
            )
        }
        RefactoringKind::ExtractFunction => {
            "Extract logical blocks into separate functions with descriptive names. Each function should do one thing well.".to_string()
        }
        RefactoringKind::ExtractVariable => {
            "Extract complex expressions into named variables to improve readability.".to_string()
        }
        RefactoringKind::RemoveDuplication => {
            "Extract duplicate code into a shared function. Follow the DRY (Don't Repeat Yourself) principle.".to_string()
        }
        RefactoringKind::ReplaceMagicNumber => {
            "Replace magic numbers with named constants to improve code clarity.".to_string()
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
        "rust" | "go" | "java" | "typescript" | "javascript" => {
            Regex::new(r"\b(?:[2-9]|[1-9]\d+)(?:\.\d+)?\b").ok()
        }
        "python" => Regex::new(r"\b(?:[2-9]|[1-9]\d+)(?:\.\d+)?\b").ok(),
        _ => None,
    };

    if let Some(pattern) = number_pattern {
        let mut found_numbers = std::collections::HashMap::new();

        for line in content.lines() {
            if line.trim().starts_with("//") || line.trim().starts_with('#') {
                continue;
            }

            for cap in pattern.find_iter(line) {
                let number = cap.as_str();
                *found_numbers.entry(number.to_string()).or_insert(0) += 1;
            }
        }

        for (number, count) in found_numbers {
            if count >= 2 {
                suggestions.push(RefactoringSuggestion {
                    kind: RefactoringKind::ReplaceMagicNumber,
                    location: 1,
                    function_name: None,
                    description: format!("Magic number '{}' appears {} times", number, count),
                    suggestion: format!("Consider extracting '{}' to a named constant", number),
                    priority: if count > 3 { "medium" } else { "low" }.to_string(),
                });
            }
        }
    }

    suggestions
}