//! Code analysis tool handlers
//!
//! Handles: find_unused_imports, analyze_complexity, suggest_refactoring
//!
//! These tools provide deep code analysis capabilities:
//! - Import analysis (dead imports, optimization opportunities)
//! - Complexity metrics (cyclomatic complexity)
//! - Refactoring suggestions (pattern-based + LSP)

use super::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use tracing::{debug, info, warn};

// ============================================================================
// Analysis Handler
// ============================================================================

pub struct AnalysisHandler;

impl AnalysisHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ToolHandler for AnalysisHandler {
    fn tool_names(&self) -> &[&str] {
        &[
            "find_unused_imports",
            "analyze_complexity",
            "suggest_refactoring",
            "analyze_project_complexity",
            "find_complexity_hotspots",
        ]
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        match tool_call.name.as_str() {
            "find_unused_imports" => {
                self.handle_find_unused_imports(context, tool_call).await
            }
            "analyze_complexity" => self.handle_analyze_complexity(context, tool_call).await,
            "suggest_refactoring" => self.handle_suggest_refactoring(context, tool_call).await,
            "analyze_project_complexity" => {
                self.handle_analyze_project_complexity(context, tool_call)
                    .await
            }
            "find_complexity_hotspots" => {
                self.handle_find_complexity_hotspots(context, tool_call)
                    .await
            }
            _ => Err(ServerError::InvalidRequest(format!(
                "Unknown analysis tool: {}",
                tool_call.name
            ))),
        }
    }
}

// ============================================================================
// find_unused_imports Implementation
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct UnusedImport {
    line: usize,
    source: String,
    imported: Vec<String>,
    suggestion: String,
}

impl AnalysisHandler {
    async fn handle_find_unused_imports(
        &self,
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
            "Finding unused imports"
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

        // Get import support
        let import_support = plugin.import_support().ok_or_else(|| {
            ServerError::Unsupported(format!(
                "Language {} does not support import operations",
                plugin.metadata().name
            ))
        })?;

        // Parse imports
        let imports = import_support.parse_imports(&content);

        debug!(
            imports_count = imports.len(),
            file_path = %file_path_str,
            "Parsed imports"
        );

        // Analyze each import for usage
        let mut unused_imports = Vec::new();
        let mut line_num = 1;

        for import_path in &imports {
            // Extract symbols from import path
            let symbols = extract_imported_symbols(&content, import_path);

            if symbols.is_empty() {
                // Check if the import path itself is used (side-effect imports)
                if !is_module_used_in_code(&content, import_path) {
                    unused_imports.push(UnusedImport {
                        line: line_num,
                        source: import_path.clone(),
                        imported: vec![],
                        suggestion: format!("Remove unused import: {}", import_path),
                    });
                }
            } else {
                // Check each imported symbol
                let mut unused_symbols = Vec::new();
                for symbol in &symbols {
                    if !is_symbol_used_in_code(&content, symbol) {
                        unused_symbols.push(symbol.clone());
                    }
                }

                if !unused_symbols.is_empty() {
                    let all_unused = unused_symbols.len() == symbols.len();
                    let suggestion = if all_unused {
                        format!("Remove entire import from {}", import_path)
                    } else {
                        format!(
                            "Remove unused symbols: {} from {}",
                            unused_symbols.join(", "),
                            import_path
                        )
                    };

                    unused_imports.push(UnusedImport {
                        line: line_num,
                        source: import_path.clone(),
                        imported: unused_symbols,
                        suggestion,
                    });
                }
            }

            line_num += 1;
        }

        info!(
            file_path = %file_path_str,
            unused_count = unused_imports.len(),
            total_imports = imports.len(),
            "Found unused imports"
        );

        Ok(json!({
            "file_path": file_path_str,
            "unused_imports": unused_imports,
            "total_unused": unused_imports.len(),
            "total_imports": imports.len(),
            "analysis_complete": true,
        }))
    }

    async fn handle_analyze_complexity(
        &self,
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
            "Analyzing cyclomatic complexity"
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

        // Get language name for complexity patterns
        let language = plugin.metadata().name;

        info!(
            file_path = %file_path_str,
            language = %language,
            symbols_count = parsed.symbols.len(),
            "Analyzing complexity for file"
        );

        // Analyze complexity using cb-ast module
        let report =
            cb_ast::complexity::analyze_file_complexity(file_path_str, &content, &parsed.symbols, language);

        info!(
            file_path = %file_path_str,
            total_functions = report.total_functions,
            average_complexity = report.average_complexity,
            max_complexity = report.max_complexity,
            "Complexity analysis complete"
        );

        serde_json::to_value(report)
            .map_err(|e| ServerError::Internal(format!("Failed to serialize report: {}", e)))
    }

    async fn handle_suggest_refactoring(
        &self,
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
        let complexity_report =
            cb_ast::complexity::analyze_file_complexity(file_path_str, &content, &parsed.symbols, language);

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
                    }.to_string(),
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
}

// ============================================================================
// Refactoring Suggestion Types
// ============================================================================

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

// ============================================================================
// Helper Functions for Refactoring Analysis
// ============================================================================

/// Generate actionable suggestion text based on refactoring kind and function metrics
fn generate_suggestion_text(kind: &RefactoringKind, func: &cb_ast::complexity::FunctionComplexity) -> String {
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

/// Extract function body for refactoring analysis (simplified version)
#[allow(dead_code)]
fn extract_function_body_for_refactoring(content: &str, location: &cb_plugin_api::SourceLocation) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let start_line = location.line.saturating_sub(1);

    if start_line >= lines.len() {
        return String::new();
    }

    let mut body_lines = Vec::new();
    let mut brace_count = 0;
    let mut started = false;

    for (idx, line) in lines.iter().enumerate().skip(start_line) {
        body_lines.push(*line);

        for ch in line.chars() {
            match ch {
                '{' => {
                    brace_count += 1;
                    started = true;
                }
                '}' => {
                    brace_count -= 1;
                    if started && brace_count == 0 {
                        return body_lines.join("\n");
                    }
                }
                _ => {}
            }
        }

        if idx - start_line > 500 {
            break;
        }
    }

    body_lines.join("\n")
}

/// Detect duplicate code patterns
fn detect_duplicate_patterns(_content: &str, _language: &str) -> Vec<RefactoringSuggestion> {
    // Simple heuristic-based duplicate detection
    // This is a placeholder for more sophisticated duplicate detection
    // Could be enhanced with:
    // - Abstract Syntax Tree-based similarity detection
    // - Token-based duplicate detection
    // - Structural similarity analysis

    // For now, return empty vec - this is a complex feature that would need significant implementation
    Vec::new()
}

/// Detect magic numbers (numeric literals that should be named constants)
fn detect_magic_numbers(content: &str, _symbols: &[cb_plugin_api::Symbol], language: &str) -> Vec<RefactoringSuggestion> {
    let mut suggestions = Vec::new();

    // Language-specific numeric literal patterns
    let number_pattern = match language.to_lowercase().as_str() {
        "rust" | "go" | "java" | "typescript" | "javascript" => {
            // Match numeric literals: 42, 3.14, 0x1A, etc.
            // Exclude: 0, 1 (commonly used and acceptable)
            Regex::new(r"\b(?:[2-9]|[1-9]\d+)(?:\.\d+)?\b").ok()
        }
        "python" => {
            Regex::new(r"\b(?:[2-9]|[1-9]\d+)(?:\.\d+)?\b").ok()
        }
        _ => None,
    };

    if let Some(pattern) = number_pattern {
        let mut found_numbers = std::collections::HashMap::new();

        for line in content.lines() {
            // Skip comments and strings (simple heuristic)
            if line.trim().starts_with("//") || line.trim().starts_with('#') {
                continue;
            }

            for cap in pattern.find_iter(line) {
                let number = cap.as_str();
                *found_numbers.entry(number.to_string()).or_insert(0) += 1;
            }
        }

        // Suggest extracting numbers that appear multiple times
        for (number, count) in found_numbers {
            if count >= 2 {
                suggestions.push(RefactoringSuggestion {
                    kind: RefactoringKind::ReplaceMagicNumber,
                    location: 1, // Would need better location tracking
                    function_name: None,
                    description: format!(
                        "Magic number '{}' appears {} times",
                        number, count
                    ),
                    suggestion: format!(
                        "Consider extracting '{}' to a named constant",
                        number
                    ),
                    priority: if count > 3 { "medium" } else { "low" }.to_string(),
                });
            }
        }
    }

    suggestions
}

// ============================================================================
// Helper Functions for Import Analysis
// ============================================================================

/// Extract imported symbols from an import statement in the file
///
/// This function looks for the actual import statement in the source code
/// and extracts the symbols being imported.
fn extract_imported_symbols(content: &str, import_path: &str) -> Vec<String> {
    let mut symbols = Vec::new();

    // Language-specific import patterns
    // Rust: use std::collections::{HashMap, HashSet};
    let rust_pattern = format!(
        r"use\s+{}::\{{([^}}]+)\}}|use\s+{}::(\w+)",
        regex::escape(import_path),
        regex::escape(import_path)
    );

    // TypeScript/JavaScript: import { foo, bar } from './module'
    let ts_pattern = format!(
        r#"import\s*\{{\s*([^}}]+)\s*\}}\s*from\s*['"]{}['"]|import\s+(\w+)\s+from\s*['"]{}['"]"#,
        regex::escape(import_path),
        regex::escape(import_path)
    );

    // Python: from module import foo, bar
    let python_pattern = format!(
        r"from\s+{}\s+import\s+([^;\n]+)",
        regex::escape(import_path)
    );

    // Go: import "module" (uses full module typically)
    let go_pattern = format!(r#"import\s+"{}""#, regex::escape(import_path));

    // Try each pattern
    for pattern_str in &[rust_pattern, ts_pattern, python_pattern, go_pattern] {
        if let Ok(pattern) = Regex::new(pattern_str) {
            for captures in pattern.captures_iter(content) {
                // Get the first non-empty capture group
                for i in 1..captures.len() {
                    if let Some(matched) = captures.get(i) {
                        let matched_str = matched.as_str().trim();
                        if !matched_str.is_empty() {
                            // Split by commas and clean up
                            for symbol in matched_str.split(',') {
                                let clean_symbol = symbol
                                    .split_whitespace()
                                    .next()
                                    .unwrap_or("")
                                    .to_string();
                                if !clean_symbol.is_empty() {
                                    symbols.push(clean_symbol);
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }
    }

    symbols
}

/// Check if a symbol is actually used in the code (excluding the import statement)
fn is_symbol_used_in_code(content: &str, symbol: &str) -> bool {
    // Create pattern that matches the symbol as a word boundary
    let pattern_str = format!(r"\b{}\b", regex::escape(symbol));

    if let Ok(pattern) = Regex::new(&pattern_str) {
        let occurrences = pattern.find_iter(content).count();

        // If the symbol appears more than once, it's used (first occurrence is the import)
        // This is a heuristic - may have false positives/negatives but works for most cases
        occurrences > 1
    } else {
        // If regex fails, assume it's used (conservative approach)
        true
    }
}

/// Check if a module path is referenced in the code (for side-effect imports)
fn is_module_used_in_code(content: &str, module_path: &str) -> bool {
    // For side-effect imports (no symbols), check if module path appears outside import
    // This is a simplified heuristic
    let lines: Vec<&str> = content.lines().collect();

    let mut found_import_line = false;
    for line in lines {
        // Skip the import line itself
        if line.contains(module_path) && (line.contains("import") || line.contains("use")) {
            found_import_line = true;
            continue;
        }

        // If module path appears elsewhere, it's used
        if found_import_line && line.contains(module_path) {
            return true;
        }
    }

    false
}

impl AnalysisHandler {
    /// Handle analyze_project_complexity tool call
    ///
    /// Scans an entire directory or project for complexity metrics across all supported files.
    async fn handle_analyze_project_complexity(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        use cb_ast::complexity::{
            aggregate_class_complexity, analyze_file_complexity, FileComplexitySummary,
            ProjectComplexityReport,
        };
        use std::path::{Path, PathBuf};

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
            let report = analyze_file_complexity(
                &file_path.to_string_lossy(),
                &content,
                &parsed.symbols,
                language,
            );

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

        // Add errors if any
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
    async fn handle_find_complexity_hotspots(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        use cb_ast::complexity::{
            aggregate_class_complexity, analyze_file_complexity, ComplexityHotspotsReport,
            FunctionHotspot,
        };
        use std::path::{Path, PathBuf};

        let args = tool_call.arguments.clone().unwrap_or(json!({}));

        // Parse parameters
        let directory_path = args
            .get("directory_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing directory_path parameter".into()))?;

        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

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
            let report = analyze_file_complexity(
                &file_path.to_string_lossy(),
                &content,
                &parsed.symbols,
                language,
            );

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
            .filter(|f| {
                matches!(
                    f.rating,
                    cb_ast::complexity::ComplexityRating::VeryComplex
                )
            })
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_imported_symbols_typescript() {
        let content = r#"
import { useState, useEffect } from 'react';
import Button from './components/Button';
"#;

        let symbols = extract_imported_symbols(content, "react");
        assert!(symbols.contains(&"useState".to_string()));
        assert!(symbols.contains(&"useEffect".to_string()));

        let button_symbols = extract_imported_symbols(content, "./components/Button");
        assert!(button_symbols.contains(&"Button".to_string()));
    }

    #[test]
    fn test_extract_imported_symbols_rust() {
        let content = r#"
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
"#;

        let symbols = extract_imported_symbols(content, "std::collections");
        assert!(symbols.contains(&"HashMap".to_string()));
        assert!(symbols.contains(&"HashSet".to_string()));

        let path_symbols = extract_imported_symbols(content, "std::path");
        assert!(path_symbols.contains(&"PathBuf".to_string()));
    }

    #[test]
    fn test_is_symbol_used_in_code() {
        let content = r#"
import { useState, useEffect } from 'react';

function App() {
    const [count, setCount] = useState(0);
    return <div>{count}</div>;
}
"#;

        // useState is used
        assert!(is_symbol_used_in_code(content, "useState"));

        // useEffect is imported but not used
        assert!(!is_symbol_used_in_code(content, "useEffect"));
    }

    #[test]
    fn test_is_symbol_used_multiple_occurrences() {
        let content = r#"
import { Button } from './components';

export function Page() {
    return <Button>Click</Button>;
}
"#;

        // Button appears twice (import + usage)
        assert!(is_symbol_used_in_code(content, "Button"));
    }
}
