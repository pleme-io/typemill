use super::utils::{extract_imported_symbols, is_module_used_in_code, is_symbol_used_in_code};
use crate::AnalysisConfig;
use mill_foundation::protocol::analysis_result::{
    Finding, FindingLocation, Position, Range, SafetyLevel, Severity, Suggestion,
};
use regex::Regex;
use serde_json::json;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Detect unused imports in a file
///
/// This function identifies imports that are declared but never used in the code.
/// It handles language-specific import patterns for Rust, TypeScript/JavaScript,
/// Python, and Go.
///
/// # Algorithm
/// 1. Parse imports using language-specific regex patterns
/// 2. For each import, extract the imported symbols
/// 3. Check if each symbol appears in the code more than once (>1 indicates usage)
/// 4. Generate findings for unused imports with removal suggestions
///
/// # Heuristics
/// - A symbol appearing once is likely the import declaration itself
/// - A symbol appearing >1 times indicates actual usage in the code
/// - This is a conservative heuristic that may have false positives but avoids false negatives
///
/// # Parameters
/// - `complexity_report`: Not used for unused imports detection
/// - `content`: The raw file content to search for imports
/// - `symbols`: Not used for unused imports detection
/// - `language`: The language name (e.g., "rust", "typescript")
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A vector of findings for unused imports, each with:
/// - Location with line number
/// - Metrics including imported symbols
/// - Suggestion to remove the import
pub(crate) fn detect_unused_imports(
    _complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    _symbols: &[mill_plugin_api::Symbol],
    language: &str,
    file_path: &str,
    _registry: &dyn mill_handler_api::LanguagePluginRegistry,
    _config: &AnalysisConfig,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Language-specific import patterns
    // These patterns detect import statements and extract the module path
    static RUST_IMPORT_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    static JS_IMPORT_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    static PYTHON_IMPORT_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    static GO_IMPORT_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    static EMPTY_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

    let import_patterns: &Vec<Regex> = match language.to_lowercase().as_str() {
        "rust" => RUST_IMPORT_PATTERNS
            .get_or_init(|| vec![Regex::new(r"use\s+([\w:]+)").expect("Invalid regex")]),
        "typescript" | "javascript" => JS_IMPORT_PATTERNS.get_or_init(|| {
            vec![Regex::new(
                r#"import\s+(?:\{[^}]*\}|\*\s+as\s+\w+|\w+)\s+from\s+['"]([^'"]+)['"]"#,
            )
            .expect("Invalid regex")]
        }),
        "python" => PYTHON_IMPORT_PATTERNS.get_or_init(|| {
            vec![
                Regex::new(r"from\s+([\w.]+)\s+import").expect("Invalid regex"),
                Regex::new(r"import\s+([\w.]+)").expect("Invalid regex"),
            ]
        }),
        "go" => GO_IMPORT_PATTERNS
            .get_or_init(|| vec![Regex::new(r#"import\s+"([^"]+)""#).expect("Invalid regex")]),
        _ => EMPTY_PATTERNS.get_or_init(Vec::new),
    };

    if import_patterns.is_empty() {
        return findings; // Language not supported
    }

    // LSP uses 0-indexed line numbers
    let lines: Vec<&str> = content.lines().collect();

    for (line_num, line) in lines.iter().enumerate() {
        // Check if this line contains an import
        for pattern in import_patterns {
            if let Some(captures) = pattern.captures(line) {
                // Get the module path from the first capture group
                if let Some(module_path) = captures.get(1) {
                    let module_path_str = module_path.as_str();

                    // Extract symbols from this import
                    let symbols =
                        extract_imported_symbols(&lines, line_num, module_path_str, language);

                    if symbols.is_empty() {
                        // Side-effect import (no symbols) - check if module is used
                        if !is_module_used_in_code(content, module_path_str) {
                            let mut metrics = HashMap::new();
                            metrics.insert("module_path".to_string(), json!(module_path_str));
                            metrics.insert("import_type".to_string(), json!("side_effect"));

                            findings.push(Finding {
                                id: format!("unused-import-{}-{}", file_path, line_num),
                                kind: "unused_import".to_string(),
                                severity: Severity::Low,
                                location: FindingLocation {
                                    file_path: file_path.to_string(),
                                    range: Some(Range {
                                        start: Position {
                                            line: line_num as u32,
                                            character: 0,
                                        },
                                        end: Position {
                                            line: line_num as u32,
                                            character: line.len() as u32,
                                        },
                                    }),
                                    symbol: None,
                                    symbol_kind: Some("import".to_string()),
                                },
                                metrics: Some(metrics),
                                message: format!("Unused side-effect import: {}", module_path_str),
                                suggestions: vec![Suggestion {
                                    action: "remove_import".to_string(),
                                    description: format!(
                                        "Remove unused import '{}'",
                                        module_path_str
                                    ),
                                    target: None,
                                    estimated_impact:
                                        "Reduces unnecessary dependencies and improves build time"
                                            .to_string(),
                                    safety: SafetyLevel::Safe,
                                    confidence: 0.85,
                                    reversible: true,
                                    refactor_call: None,
                                }],
                            });
                        }
                    } else {
                        // Named imports - check each symbol
                        let mut unused_symbols = Vec::new();
                        for symbol in &symbols {
                            if !is_symbol_used_in_code(content, symbol) {
                                unused_symbols.push(symbol.clone());
                            }
                        }

                        if !unused_symbols.is_empty() {
                            let all_unused = unused_symbols.len() == symbols.len();
                            // Both fully unused and partially unused symbols are low priority
                            let severity = Severity::Low;

                            let mut metrics = HashMap::new();
                            metrics.insert("module_path".to_string(), json!(module_path_str));
                            metrics.insert("unused_symbols".to_string(), json!(unused_symbols));
                            metrics.insert("total_symbols".to_string(), json!(symbols.len()));
                            metrics.insert(
                                "import_type".to_string(),
                                json!(if all_unused {
                                    "fully_unused"
                                } else {
                                    "partially_unused"
                                }),
                            );

                            let message = if all_unused {
                                format!(
                                    "Entire import from '{}' is unused: {}",
                                    module_path_str,
                                    unused_symbols.join(", ")
                                )
                            } else {
                                format!(
                                    "Unused symbols from '{}': {}",
                                    module_path_str,
                                    unused_symbols.join(", ")
                                )
                            };

                            let suggestion = if all_unused {
                                Suggestion {
                                    action: "remove_import".to_string(),
                                    description: format!(
                                        "Remove entire import from '{}'",
                                        module_path_str
                                    ),
                                    target: None,
                                    estimated_impact: "Reduces unused dependencies".to_string(),
                                    safety: SafetyLevel::Safe,
                                    confidence: 0.90,
                                    reversible: true,
                                    refactor_call: None,
                                }
                            } else {
                                Suggestion {
                                    action: "remove_unused_symbols".to_string(),
                                    description: format!(
                                        "Remove unused symbols: {}",
                                        unused_symbols.join(", ")
                                    ),
                                    target: None,
                                    estimated_impact: "Cleans up import statement".to_string(),
                                    safety: SafetyLevel::Safe,
                                    confidence: 0.85,
                                    reversible: true,
                                    refactor_call: None,
                                }
                            };

                            findings.push(Finding {
                                id: format!("unused-import-{}-{}", file_path, line_num),
                                kind: "unused_import".to_string(),
                                severity,
                                location: FindingLocation {
                                    file_path: file_path.to_string(),
                                    range: Some(Range {
                                        start: Position {
                                            line: line_num as u32,
                                            character: 0,
                                        },
                                        end: Position {
                                            line: line_num as u32,
                                            character: line.len() as u32,
                                        },
                                    }),
                                    symbol: None,
                                    symbol_kind: Some("import".to_string()),
                                },
                                metrics: Some(metrics),
                                message,
                                suggestions: vec![suggestion],
                            });
                        }
                    }
                }
            }
        }
    }

    findings
}
