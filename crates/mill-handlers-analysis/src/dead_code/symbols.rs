use super::utils::is_symbol_used_in_code;
use crate::AnalysisConfig;
use mill_foundation::protocol::analysis_result::{
    Finding, FindingLocation, Position, Range, SafetyLevel, Severity, Suggestion,
};
use regex::Regex;
use serde_json::json;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Detect unused symbols (functions, classes, variables) in a file
///
/// This function identifies symbols that are defined but never referenced.
/// For MVP, it focuses on function definitions that are not called.
///
/// # Algorithm
/// 1. Get all functions from complexity report
/// 2. For each function, check if it's referenced in the code
/// 3. Skip exported/public functions (they may be used externally)
/// 4. Generate findings for unused private functions
///
/// # Heuristics
/// - Functions appearing in complexity_report are defined
/// - A function name appearing >1 time indicates it's called (first is definition)
/// - Public/exported functions are excluded (may be part of public API)
///
/// # Future Enhancements
/// TODO: Add support for detecting unused classes and variables
/// TODO: Use symbol visibility information from language plugins
/// TODO: Cross-reference with call hierarchy to detect call chains
///
/// # Parameters
/// - `complexity_report`: Used to get all function definitions
/// - `content`: The raw file content to search for references
/// - `symbols`: Parsed symbols from language plugin (for future enhancements)
/// - `language`: The language name (for language-specific patterns)
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A vector of findings for unused symbols, each with:
/// - Location with function name and range
/// - Metrics including symbol type
/// - Suggestions to remove or make private
pub(crate) fn detect_unused_symbols(
    complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    _symbols: &[mill_plugin_api::Symbol],
    language: &str,
    file_path: &str,
    _registry: &dyn mill_handler_api::LanguagePluginRegistry,
    _config: &AnalysisConfig,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Get exported functions for languages with explicit exports
    let exported_functions = get_exported_functions(content, language);

    // For MVP: Focus on unused functions
    for func in &complexity_report.functions {
        // Skip if function appears to be public/exported
        let is_exported = if let Some(ref exports) = exported_functions {
            exports.contains(&func.name)
        } else {
            is_public_by_convention(&func.name, language)
        };

        if is_exported {
            continue;
        }

        // Check if function is called anywhere in the code
        // We use a simple heuristic: if the function name appears more than once,
        // it's likely being called (first occurrence is the definition)
        if !is_symbol_used_in_code(content, &func.name) {
            let mut metrics = HashMap::new();
            metrics.insert("symbol_name".to_string(), json!(func.name));
            metrics.insert("symbol_type".to_string(), json!("function"));
            metrics.insert("line_count".to_string(), json!(func.metrics.sloc));

            findings.push(Finding {
                id: format!("unused-function-{}-{}", file_path, func.line),
                kind: "unused_function".to_string(),
                severity: Severity::Medium,
                location: FindingLocation {
                    file_path: file_path.to_string(),
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
                message: format!("Function '{}' is defined but never called", func.name),
                suggestions: vec![
                    Suggestion {
                        action: "remove_function".to_string(),
                        description: format!("Remove unused function '{}'", func.name),
                        target: None,
                        estimated_impact: format!(
                            "Reduces code by {} lines",
                            func.metrics.sloc
                        ),
                        safety: SafetyLevel::RequiresReview,
                        confidence: 0.75,
                        reversible: true,
                        refactor_call: Some(mill_foundation::protocol::analysis_result::RefactorCall {
                            command: "delete".to_string(),
                            arguments: json!({
                                "kind": "function",
                                "target": {
                                    "filePath": file_path,
                                    "range": {
                                        "start": { "line": func.line, "character": 0 },
                                        "end": { "line": func.line + func.metrics.sloc as usize, "character": 0 }
                                    }
                                }
                            }),
                        }),
                    },
                    Suggestion {
                        action: "make_private".to_string(),
                        description: format!(
                            "If needed for testing, make '{}' explicitly private/internal",
                            func.name
                        ),
                        target: None,
                        estimated_impact: "Documents intent for future maintainers".to_string(),
                        safety: SafetyLevel::Safe,
                        confidence: 0.90,
                        reversible: true,
                        refactor_call: None,
                    },
                ],
            });
        }
    }

    // TODO: Add detection for unused classes
    // Algorithm:
    // 1. Extract class definitions from symbols
    // 2. Check if class name is referenced (instantiated, inherited, etc.)
    // 3. Generate findings similar to unused functions

    // TODO: Add detection for unused variables/constants
    // Algorithm:
    // 1. Extract variable/constant declarations
    // 2. Check if variable is referenced in code
    // 3. Generate findings with suggestions to remove

    findings
}

/// Get all exported functions in a file (for languages with explicit export markers)
fn get_exported_functions(
    content: &str,
    language: &str,
) -> Option<std::collections::HashSet<String>> {
    let mut exported = std::collections::HashSet::new();

    match language.to_lowercase().as_str() {
        "rust" => {
            static RUST_PUB_FN: OnceLock<Regex> = OnceLock::new();
            let pattern = RUST_PUB_FN.get_or_init(|| {
                Regex::new(r"pub(?:\([^)]*\))?\s+fn\s+(\w+)").expect("Invalid regex")
            });
            for captures in pattern.captures_iter(content) {
                if let Some(name) = captures.get(1) {
                    exported.insert(name.as_str().to_string());
                }
            }
            Some(exported)
        }
        "typescript" | "javascript" => {
            static JS_EXPORT_FN: OnceLock<Regex> = OnceLock::new();
            let pattern = JS_EXPORT_FN.get_or_init(|| {
                Regex::new(r"export\s+(?:async\s+)?(?:function\s+)?(\w+)").expect("Invalid regex")
            });
            for captures in pattern.captures_iter(content) {
                if let Some(name) = captures.get(1) {
                    exported.insert(name.as_str().to_string());
                }
            }
            Some(exported)
        }
        _ => None,
    }
}

/// Check if a function is public by naming convention (for languages like Python/Go)
fn is_public_by_convention(func_name: &str, language: &str) -> bool {
    match language.to_lowercase().as_str() {
        "python" => !func_name.starts_with('_'),
        "go" => func_name.chars().next().is_some_and(|c| c.is_uppercase()),
        // For others (or if explicit scanning failed/not supported), be conservative
        _ => true,
    }
}
