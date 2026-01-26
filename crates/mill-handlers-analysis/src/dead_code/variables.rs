use super::utils::is_symbol_used_in_code;
use crate::AnalysisConfig;
use mill_foundation::protocol::analysis_result::{
    Finding, FindingLocation, Position, Range, SafetyLevel, Severity, Suggestion,
};
use regex::Regex;
use serde_json::json;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Detect unused local variables
///
/// This function identifies local variable declarations that are never
/// read after being assigned.
///
/// # Algorithm
/// 1. Use language-specific patterns to find variable declarations
/// 2. For each declaration, check if variable is used later in code
/// 3. Generate findings for unused variables
///
/// # Heuristics
/// - Simple regex-based variable extraction
/// - Text search for variable usage (>1 occurrence means used)
/// - Does not perform scope analysis (may have false positives)
///
/// # Future Enhancements
/// TODO: Use AST-based scope analysis for accurate detection
/// TODO: Distinguish between write-only and read usage
/// TODO: Handle shadowing and nested scopes correctly
/// TODO: Detect variables used only for debugging
///
/// # Parameters
/// - `complexity_report`: Not used for unused variables detection
/// - `content`: The raw file content to analyze
/// - `symbols`: Not used for unused variables detection
/// - `language`: The language name for language-specific patterns
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A vector of findings for unused variables, each with:
/// - Location with variable declaration line
/// - Metrics including variable name and scope
/// - Suggestion to remove the variable
pub(crate) fn detect_unused_variables(
    complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    _symbols: &[mill_plugin_api::Symbol],
    language: &str,
    file_path: &str,
    _registry: &dyn mill_handler_api::LanguagePluginRegistry,
    _config: &AnalysisConfig,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    // Language-specific variable declaration patterns
    static RUST_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    static JS_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    static PYTHON_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    static GO_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    static DEFAULT_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

    let var_patterns: &Vec<Regex> = match language.to_lowercase().as_str() {
        "rust" => RUST_PATTERNS.get_or_init(|| {
            vec![
                Regex::new(r"let\s+mut\s+(\w+)\s*[=:]").expect("Invalid regex"), // let mut x =
                Regex::new(r"let\s+(\w+)\s*[=:]").expect("Invalid regex"),       // let x =
            ]
        }),
        "typescript" | "javascript" => JS_PATTERNS.get_or_init(|| {
            vec![
                Regex::new(r"const\s+(\w+)\s*=").expect("Invalid regex"), // const x =
                Regex::new(r"let\s+(\w+)\s*=").expect("Invalid regex"),   // let x =
                Regex::new(r"var\s+(\w+)\s*=").expect("Invalid regex"),   // var x =
            ]
        }),
        "python" => PYTHON_PATTERNS.get_or_init(|| {
            vec![
                Regex::new(r"^\s*(\w+)\s*=\s*").expect("Invalid regex"), // x = (at start of line)
            ]
        }),
        "go" => GO_PATTERNS.get_or_init(|| {
            vec![
                Regex::new(r"(\w+)\s*:=").expect("Invalid regex"),     // x :=
                Regex::new(r"var\s+(\w+)\s+").expect("Invalid regex"), // var x Type
            ]
        }),
        _ => DEFAULT_PATTERNS.get_or_init(|| {
            vec![Regex::new(r"let\s+(\w+)\s*=").expect("Invalid regex")]
        }),
    };

    // Analyze within each function scope
    for func in &complexity_report.functions {
        if func.line == 0 || func.line > lines.len() {
            continue;
        }

        let func_start = func.line - 1;
        let func_end = (func_start + func.metrics.sloc as usize).min(lines.len());

        // Collect variables declared in this function
        for i in func_start..func_end {
            if i >= lines.len() {
                break;
            }

            let line = lines[i];

            for pattern in var_patterns {
                if let Some(captures) = pattern.captures(line) {
                    if let Some(var_match) = captures.get(1) {
                        let var_name = var_match.as_str();

                        // Skip special variable names
                        if var_name == "_" || var_name.starts_with('_') {
                            continue;
                        }

                        // Skip if it's a parameter (already covered by unused_parameters)
                        // This is a simple heuristic - full AST would be more accurate
                        if line.contains("fn ")
                            || line.contains("function ")
                            || line.contains("def ")
                        {
                            continue;
                        }

                        // Get the rest of the function after this declaration
                        let mut remaining_code = String::new();
                        for j in (i + 1)..func_end {
                            if j < lines.len() {
                                remaining_code.push_str(lines[j]);
                                remaining_code.push('\n');
                            }
                        }

                        // Check if variable is used after declaration
                        if !is_symbol_used_in_code(&remaining_code, var_name) {
                            let mut metrics = HashMap::new();
                            metrics.insert("variable_name".to_string(), json!(var_name));
                            metrics.insert("scope".to_string(), json!(func.name));

                            findings.push(Finding {
                                id: format!("unused-variable-{}-{}-{}", file_path, i + 1, var_name),
                                kind: "unused_variable".to_string(),
                                severity: Severity::Low,
                                location: FindingLocation {
                                    file_path: file_path.to_string(),
                                    range: Some(Range {
                                        start: Position {
                                            line: (i + 1) as u32,
                                            character: 0,
                                        },
                                        end: Position {
                                            line: (i + 1) as u32,
                                            character: line.len() as u32,
                                        },
                                    }),
                                    symbol: Some(var_name.to_string()),
                                    symbol_kind: Some("variable".to_string()),
                                },
                                metrics: Some(metrics),
                                message: format!(
                                    "Variable '{}' in function '{}' is declared but never used",
                                    var_name, func.name
                                ),
                                suggestions: vec![Suggestion {
                                    action: "remove_variable".to_string(),
                                    description: format!("Remove unused variable '{}'", var_name),
                                    target: None,
                                    estimated_impact: "Reduces code clutter".to_string(),
                                    safety: SafetyLevel::Safe,
                                    confidence: 0.80,
                                    reversible: true,
                                    refactor_call: None,
                                }],
                            });
                        }
                    }
                }
            }
        }
    }

    findings
}

/// Detect unused function parameters
///
/// This function identifies function parameters that are declared but never
/// used within the function body.
///
/// # Algorithm
/// 1. Extract all function definitions from complexity report
/// 2. For each function, parse parameter names from signature
/// 3. Check if each parameter is referenced in the function body
/// 4. Generate findings for unused parameters
///
/// # Heuristics
/// - Uses regex to extract parameter names from function signatures
/// - Checks if parameter appears in function body (simple text search)
/// - May have false positives if parameter name appears in comments
///
/// # Future Enhancements
/// TODO: Use AST-based parameter analysis for accurate detection
/// TODO: Handle destructured parameters and complex parameter patterns
/// TODO: Detect parameters used only in debug/logging statements
///
/// # Parameters
/// - `complexity_report`: Used to get all function definitions
/// - `content`: The raw file content to search for parameter usage
/// - `symbols`: Not used for unused parameters detection
/// - `language`: The language name for language-specific patterns
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A vector of findings for unused parameters, each with:
/// - Location with function line and range
/// - Metrics including parameter name and function name
/// - Suggestion to remove the parameter (requires review)
pub(crate) fn detect_unused_parameters(
    complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    _symbols: &[mill_plugin_api::Symbol],
    language: &str,
    file_path: &str,
    _registry: &dyn mill_handler_api::LanguagePluginRegistry,
    _config: &AnalysisConfig,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    // Language-specific parameter extraction patterns
    static RUST_PARAM_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    static JS_PARAM_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    static PYTHON_PARAM_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    static GO_PARAM_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    static DEFAULT_PARAM_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

    let param_patterns: &Vec<Regex> = match language.to_lowercase().as_str() {
        "rust" => RUST_PARAM_PATTERNS.get_or_init(|| {
            vec![Regex::new(r"\(([^)]+)\)").expect("Invalid regex")]
        }),
        "typescript" | "javascript" => JS_PARAM_PATTERNS.get_or_init(|| {
            vec![Regex::new(r"\(([^)]+)\)").expect("Invalid regex")]
        }),
        "python" => PYTHON_PARAM_PATTERNS.get_or_init(|| {
            vec![Regex::new(r"def\s+\w+\(([^)]+)\)").expect("Invalid regex")]
        }),
        "go" => GO_PARAM_PATTERNS.get_or_init(|| {
            vec![Regex::new(r"func\s+\w+\(([^)]+)\)").expect("Invalid regex")]
        }),
        _ => DEFAULT_PARAM_PATTERNS.get_or_init(|| {
            vec![Regex::new(r"\(([^)]+)\)").expect("Invalid regex")]
        }),
    };

    for func in &complexity_report.functions {
        // Get function signature and body
        if func.line == 0 || func.line > lines.len() {
            continue;
        }

        let func_start = func.line - 1;
        let func_end = (func_start + func.metrics.sloc as usize).min(lines.len());

        // Extract function signature (may span multiple lines)
        let mut signature = String::new();
        let mut found_opening_brace = false;
        #[allow(clippy::needless_range_loop)]
        for i in func_start..func_end {
            signature.push_str(lines[i]);
            if lines[i].contains('{') {
                found_opening_brace = true;
                break;
            }
        }

        if !found_opening_brace {
            continue;
        }

        for pattern in param_patterns {
            if let Some(captures) = pattern.captures(&signature) {
                    if let Some(params_str) = captures.get(1) {
                        let params_str = params_str.as_str();

                        // Skip if no parameters
                        if params_str.trim().is_empty() {
                            break;
                        }

                        // Extract individual parameter names
                        let param_names = extract_parameter_names(params_str, language);

                        // Get function body (exclude signature)
                        let body_start = func_start + signature.lines().count();
                        let body_end = func_end;
                        let mut body = String::new();
                        for i in body_start..body_end {
                            if i < lines.len() {
                                body.push_str(lines[i]);
                                body.push('\n');
                            }
                        }

                        // Check each parameter for usage in body
                        for param_name in param_names {
                            // Skip special parameters
                            if param_name == "self" || param_name == "this" || param_name == "_" {
                                continue;
                            }

                            // Check if parameter is used in function body
                            if !is_parameter_used_in_body(&body, &param_name) {
                                let mut metrics = HashMap::new();
                                metrics.insert("parameter_name".to_string(), json!(param_name));
                                metrics.insert("function_name".to_string(), json!(func.name));

                                findings.push(Finding {
                                    id: format!(
                                        "unused-parameter-{}-{}-{}",
                                        file_path, func.line, param_name
                                    ),
                                    kind: "unused_parameter".to_string(),
                                    severity: Severity::Low,
                                    location: FindingLocation {
                                        file_path: file_path.to_string(),
                                        range: Some(Range {
                                            start: Position {
                                                line: func.line as u32,
                                                character: 0,
                                            },
                                            end: Position {
                                                line: (func.line + signature.lines().count())
                                                    as u32,
                                                character: 0,
                                            },
                                        }),
                                        symbol: Some(func.name.clone()),
                                        symbol_kind: Some("parameter".to_string()),
                                    },
                                    metrics: Some(metrics),
                                    message: format!(
                                        "Parameter '{}' in function '{}' is never used",
                                        param_name, func.name
                                    ),
                                    suggestions: vec![Suggestion {
                                        action: "remove_parameter".to_string(),
                                        description: format!(
                                            "Remove unused parameter '{}'",
                                            param_name
                                        ),
                                        target: None,
                                        estimated_impact: "Simplifies function signature"
                                            .to_string(),
                                        safety: SafetyLevel::RequiresReview,
                                        confidence: 0.75,
                                        reversible: true,
                                        refactor_call: None,
                                    }],
                                });
                            }
                        }
                        break;
                    }
                }
        }
    }

    findings
}

/// Extract parameter names from a parameter list string
///
/// # Parameters
/// - `params_str`: The parameter list string (e.g., "x: i32, y: String")
/// - `language`: The language name for parsing rules
///
/// # Returns
/// A vector of parameter names
fn extract_parameter_names(params_str: &str, language: &str) -> Vec<String> {
    let mut names = Vec::new();

    match language.to_lowercase().as_str() {
        "rust" => {
            // Rust: param: Type or mut param: Type
            for param in params_str.split(',') {
                let param = param.trim();
                if let Some(name) = param.split(':').next() {
                    let name = name.trim().trim_start_matches("mut ").trim();
                    if !name.is_empty() && name != "&" && name != "&mut" {
                        names.push(name.to_string());
                    }
                }
            }
        }
        "typescript" | "javascript" => {
            // TS/JS: param or param: Type or param = default
            for param in params_str.split(',') {
                let param = param.trim();
                // Extract name before : or =
                let name = param
                    .split(':')
                    .next()
                    .unwrap_or(param)
                    .split('=')
                    .next()
                    .unwrap_or("")
                    .trim();
                if !name.is_empty() {
                    names.push(name.to_string());
                }
            }
        }
        "python" => {
            // Python: param or param: Type or param=default
            for param in params_str.split(',') {
                let param = param.trim();
                let name = param
                    .split(':')
                    .next()
                    .unwrap_or(param)
                    .split('=')
                    .next()
                    .unwrap_or("")
                    .trim();
                if !name.is_empty() {
                    names.push(name.to_string());
                }
            }
        }
        "go" => {
            // Go: name Type or name, name Type
            // This is simplified - Go has complex parameter syntax
            for param in params_str.split(',') {
                let parts: Vec<&str> = param.split_whitespace().collect();
                if !parts.is_empty() {
                    names.push(parts[0].to_string());
                }
            }
        }
        _ => {
            // Generic: split by comma and take first word
            for param in params_str.split(',') {
                if let Some(name) = param.split_whitespace().next() {
                    names.push(name.to_string());
                }
            }
        }
    }

    names
}

/// Check if a parameter is used in the function body
///
/// # Parameters
/// - `body`: The function body content
/// - `param_name`: The parameter name to search for
///
/// # Returns
/// `true` if the parameter is used, `false` otherwise
fn is_parameter_used_in_body(body: &str, param_name: &str) -> bool {
    if param_name.is_empty() {
        return false;
    }

    // Remove comments before checking usage to avoid false positives
    let mut body_without_comments = String::with_capacity(body.len());
    for line in body.lines() {
        // Remove line comments (// and #)
        let code_part = if let Some(pos) = line.find("//") {
            &line[..pos]
        } else if let Some(pos) = line.find('#') {
            &line[..pos]
        } else {
            line
        };
        body_without_comments.push_str(code_part);
        body_without_comments.push('\n');
    }

    let content = &body_without_comments;
    let symbol = param_name;
    let mut start_search_at = 0;

    while let Some(relative_pos) = content[start_search_at..].find(symbol) {
        let match_start = start_search_at + relative_pos;
        let match_end = match_start + symbol.len();

        // Check start boundary
        let start_boundary = if match_start == 0 {
            true
        } else {
            // Get char before match_start
            match content[..match_start].chars().next_back() {
                Some(c) => !c.is_alphanumeric() && c != '_',
                None => true,
            }
        };

        // Check end boundary
        let end_boundary = if match_end >= content.len() {
            true
        } else {
            match content[match_end..].chars().next() {
                Some(c) => !c.is_alphanumeric() && c != '_',
                None => true,
            }
        };

        if start_boundary && end_boundary {
            return true;
        }

        start_search_at = match_end;
    }

    false
}
