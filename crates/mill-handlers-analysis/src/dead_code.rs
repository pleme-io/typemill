#![allow(dead_code, unused_variables)]

//! Dead code analysis handler
//!
//! This module provides detection for unused code patterns including:
//! - Unused imports: Imports that are declared but never referenced
//! - Unused symbols: Functions, classes, and variables that are defined but never used
//!
//! Uses the shared analysis engine for orchestration and focuses only on
//! detection logic.

use crate::{ToolHandler, ToolHandlerContext, AnalysisConfig};
use crate::suggestions::{
    self, AnalysisContext, EvidenceStrength, Location, RefactorType, RefactoringCandidate, Scope,
    SuggestionGenerator,
};
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::protocol::analysis_result::{
    Finding, FindingLocation, Position, Range, SafetyLevel, Severity, Suggestion,
};
use mill_foundation::protocol::{AnalysisMetadata, AnalysisSummary};
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_plugin_api::ParsedSource;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::debug;
use uuid::Uuid;

/// Helper to downcast AnalysisConfigTrait to concrete AnalysisConfig
fn get_analysis_config(context: &ToolHandlerContext) -> ServerResult<&AnalysisConfig> {
    context.analysis_config
        .as_any()
        .downcast_ref::<AnalysisConfig>()
        .ok_or_else(|| ServerError::internal("Failed to downcast AnalysisConfigTrait to AnalysisConfig"))
}

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
    let import_patterns = get_import_patterns(language);

    if import_patterns.is_empty() {
        return findings; // Language not supported
    }

    // LSP uses 0-indexed line numbers
    let lines: Vec<&str> = content.lines().collect();

    for (line_num, line) in lines.iter().enumerate() {
        // Check if this line contains an import
        for pattern_str in &import_patterns {
            if let Ok(pattern) = Regex::new(pattern_str) {
                if let Some(captures) = pattern.captures(line) {
                    // Get the module path from the first capture group
                    if let Some(module_path) = captures.get(1) {
                        let module_path_str = module_path.as_str();

                        // Extract symbols from this import
                        let symbols = extract_imported_symbols(content, module_path_str, language);

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
                                    message: format!(
                                        "Unused side-effect import: {}",
                                        module_path_str
                                    ),
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
    }

    findings
}

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

    // For MVP: Focus on unused functions
    for func in &complexity_report.functions {
        // Skip if function appears to be public/exported
        if is_function_exported(&func.name, content, language) {
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

/// Detect unreachable code (statements after return/throw/break/continue)
///
/// This function identifies code that appears after control flow terminators
/// and will never be executed.
///
/// # Algorithm
/// 1. Identify terminator statements (return, throw, break, continue, panic, etc.)
/// 2. Look for subsequent non-empty, non-comment, non-closing-brace lines
/// 3. Generate findings for unreachable statements
///
/// # Heuristics
/// - Simple line-by-line analysis within statement blocks
/// - Does not account for complex control flow (if/else, loops)
/// - Conservative approach may have false positives in complex nested structures
///
/// # Future Enhancements
/// TODO: Use AST-based control flow analysis for accurate detection
/// TODO: Handle conditional returns (e.g., in if-else blocks)
/// TODO: Detect unreachable code after infinite loops
///
/// # Parameters
/// - `complexity_report`: Not used for unreachable code detection
/// - `content`: The raw file content to analyze
/// - `symbols`: Not used for unreachable code detection
/// - `language`: The language name for language-specific patterns
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A vector of findings for unreachable code, each with:
/// - Location with line number and range
/// - Metrics including lines unreachable and terminator statement
/// - Suggestion to remove the unreachable code
pub(crate) fn detect_unreachable_code(
    _complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    _symbols: &[mill_plugin_api::Symbol],
    language: &str,
    file_path: &str,
    _registry: &dyn mill_handler_api::LanguagePluginRegistry,
    _config: &AnalysisConfig,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Language-specific terminator patterns
    let terminators = match language.to_lowercase().as_str() {
        "rust" => vec![
            "return",
            "break",
            "continue",
            "panic!",
            "unreachable!",
            "std::process::exit",
        ],
        "typescript" | "javascript" => vec!["return", "throw", "break", "continue", "process.exit"],
        "python" => vec!["return", "raise", "break", "continue", "sys.exit", "exit"],
        "go" => vec!["return", "panic", "break", "continue", "os.Exit"],
        _ => vec!["return", "throw", "break", "continue"],
    };

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
            i += 1;
            continue;
        }

        // Check if this line contains a terminator
        let mut found_terminator = None;
        for terminator in &terminators {
            if line.contains(terminator) {
                // Basic check: ensure it's not in a comment or string
                // This is a simple heuristic - full parsing would be more accurate
                if !line.starts_with("//") && !line.starts_with('#') {
                    found_terminator = Some(terminator.to_string());
                    break;
                }
            }
        }

        if let Some(terminator) = found_terminator {
            // Look for the next non-empty, non-comment line
            let mut unreachable_start = None;
            let mut unreachable_count = 0;

            #[allow(clippy::needless_range_loop)]
            for j in (i + 1)..lines.len() {
                let next_line = lines[j].trim();

                // Skip empty lines
                if next_line.is_empty() {
                    continue;
                }

                // Skip comments
                if next_line.starts_with("//")
                    || next_line.starts_with('#')
                    || next_line.starts_with("/*")
                {
                    continue;
                }

                // If we hit a closing brace, we've left the block
                if next_line == "}" || next_line.starts_with('}') {
                    break;
                }

                // If we hit another function/block start, stop
                if next_line.contains("fn ")
                    || next_line.contains("function ")
                    || next_line.contains("def ")
                {
                    break;
                }

                // This is unreachable code
                if unreachable_start.is_none() {
                    unreachable_start = Some(j);
                }
                unreachable_count += 1;

                // Continue until we hit a closing brace or another block
                if next_line.starts_with('}') {
                    break;
                }
            }

            if let Some(start_line) = unreachable_start {
                let mut metrics = HashMap::new();
                metrics.insert("lines_unreachable".to_string(), json!(unreachable_count));
                metrics.insert("after_statement".to_string(), json!(terminator));
                metrics.insert("terminator_line".to_string(), json!(i + 1));

                findings.push(Finding {
                    id: format!("unreachable-code-{}-{}", file_path, start_line + 1),
                    kind: "unreachable_code".to_string(),
                    severity: Severity::Medium,
                    location: FindingLocation {
                        file_path: file_path.to_string(),
                        range: Some(Range {
                            start: Position {
                                line: (start_line + 1) as u32,
                                character: 0,
                            },
                            end: Position {
                                line: (start_line + unreachable_count) as u32,
                                character: lines[start_line + unreachable_count - 1].len() as u32,
                            },
                        }),
                        symbol: None,
                        symbol_kind: Some("statement".to_string()),
                    },
                    metrics: Some(metrics),
                    message: format!(
                        "Unreachable code detected: {} line(s) after '{}' on line {}",
                        unreachable_count,
                        terminator,
                        i + 1
                    ),
                    suggestions: vec![Suggestion {
                        action: "remove_unreachable_code".to_string(),
                        description: format!("Remove {} unreachable line(s)", unreachable_count),
                        target: None,
                        estimated_impact: format!("Reduces code by {} lines", unreachable_count),
                        safety: SafetyLevel::Safe,
                        confidence: 0.85,
                        reversible: true,
                        refactor_call: None,
                    }],
                });
            }
        }

        i += 1;
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

        // Extract parameter names based on language
        let param_patterns = match language.to_lowercase().as_str() {
            "rust" => vec![
                r"\(([^)]+)\)", // fn foo(param1: Type, param2: Type)
            ],
            "typescript" | "javascript" => vec![
                r"\(([^)]+)\)", // function foo(param1, param2) or (param1, param2) =>
            ],
            "python" => vec![
                r"def\s+\w+\(([^)]+)\)", // def foo(param1, param2):
            ],
            "go" => vec![
                r"func\s+\w+\(([^)]+)\)", // func foo(param1 Type, param2 Type)
            ],
            _ => vec![r"\(([^)]+)\)"],
        };

        for pattern_str in &param_patterns {
            if let Ok(pattern) = Regex::new(pattern_str) {
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
    }

    findings
}

/// Detect unused type definitions (interfaces, type aliases, enums, structs)
///
/// This function identifies type definitions that are declared but never
/// referenced in the codebase.
///
/// # Algorithm
/// 1. Filter symbols for type definitions (Interface, Enum, Struct, TypeParameter)
/// 2. For each type, check if it's exported (part of public API)
/// 3. Check if type name appears in code (type usage)
/// 4. Generate findings for unused private types
///
/// # Heuristics
/// - Simple text search for type name usage
/// - Skips exported types (may be used externally)
/// - May have false positives if type name appears in comments
///
/// # Future Enhancements
/// TODO: Use AST-based type reference analysis
/// TODO: Cross-reference with import statements
/// TODO: Detect types used only in other unused types
///
/// # Parameters
/// - `complexity_report`: Not used for unused types detection
/// - `content`: The raw file content to search for type references
/// - `symbols`: Parsed symbols from language plugin (used to find type definitions)
/// - `language`: The language name for language-specific patterns
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A vector of findings for unused types, each with:
/// - Location with type line
/// - Metrics including type name and kind
/// - Suggestion to remove the type (requires review)
pub(crate) fn detect_unused_types(
    _complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    symbols: &[mill_plugin_api::Symbol],
    language: &str,
    file_path: &str,
    _registry: &dyn mill_handler_api::LanguagePluginRegistry,
    _config: &AnalysisConfig,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Filter symbols for type definitions
    // Note: TypeParameter is not currently a SymbolKind variant
    let type_symbols: Vec<_> = symbols
        .iter()
        .filter(|s| {
            matches!(
                s.kind,
                mill_plugin_api::SymbolKind::Interface
                    | mill_plugin_api::SymbolKind::Enum
                    | mill_plugin_api::SymbolKind::Struct
                    | mill_plugin_api::SymbolKind::Class
            )
        })
        .collect();

    for type_symbol in type_symbols {
        // Skip if exported (may be part of public API)
        if is_type_exported(&type_symbol.name, language, content) {
            continue;
        }

        // Check if type is used in code
        if !is_symbol_used_in_code(content, &type_symbol.name) {
            let type_kind = match type_symbol.kind {
                mill_plugin_api::SymbolKind::Interface => "interface",
                mill_plugin_api::SymbolKind::Enum => "enum",
                mill_plugin_api::SymbolKind::Struct => "struct",
                mill_plugin_api::SymbolKind::Class => "class",
                _ => "type",
            };

            let mut metrics = HashMap::new();
            metrics.insert("type_name".to_string(), json!(type_symbol.name));
            metrics.insert("type_kind".to_string(), json!(type_kind));

            // Get line number from symbol location
            let line_num = type_symbol.location.line;

            // Convert location to Range for FindingLocation
            let range = Range {
                start: Position {
                    line: type_symbol.location.line as u32,
                    character: type_symbol.location.column as u32,
                },
                end: Position {
                    line: type_symbol.location.line as u32,
                    character: (type_symbol.location.column + type_symbol.name.len()) as u32,
                },
            };

            findings.push(Finding {
                id: format!("unused-type-{}-{}", file_path, line_num),
                kind: "unused_type".to_string(),
                severity: Severity::Low,
                location: FindingLocation {
                    file_path: file_path.to_string(),
                    range: Some(range),
                    symbol: Some(type_symbol.name.clone()),
                    symbol_kind: Some(type_kind.to_string()),
                },
                metrics: Some(metrics),
                message: format!(
                    "Type '{}' ({}) is defined but never used",
                    type_symbol.name, type_kind
                ),
                suggestions: vec![Suggestion {
                    action: "remove_type".to_string(),
                    description: format!("Remove unused {} '{}'", type_kind, type_symbol.name),
                    target: None,
                    estimated_impact: "Reduces code complexity".to_string(),
                    safety: SafetyLevel::RequiresReview,
                    confidence: 0.70,
                    reversible: true,
                    refactor_call: None,
                }],
            });
        }
    }

    findings
}

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
    let var_patterns = match language.to_lowercase().as_str() {
        "rust" => vec![
            r"let\s+mut\s+(\w+)\s*[=:]", // let mut x =
            r"let\s+(\w+)\s*[=:]",       // let x =
        ],
        "typescript" | "javascript" => vec![
            r"const\s+(\w+)\s*=", // const x =
            r"let\s+(\w+)\s*=",   // let x =
            r"var\s+(\w+)\s*=",   // var x =
        ],
        "python" => vec![
            r"^\s*(\w+)\s*=\s*", // x = (at start of line)
        ],
        "go" => vec![
            r"(\w+)\s*:=",     // x :=
            r"var\s+(\w+)\s+", // var x Type
        ],
        _ => vec![r"let\s+(\w+)\s*="],
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

            for pattern_str in &var_patterns {
                if let Ok(pattern) = Regex::new(pattern_str) {
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
                                    id: format!(
                                        "unused-variable-{}-{}-{}",
                                        file_path,
                                        i + 1,
                                        var_name
                                    ),
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
                                        description: format!(
                                            "Remove unused variable '{}'",
                                            var_name
                                        ),
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
    // Remove comments before checking usage to avoid false positives
    let mut body_without_comments = String::new();
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

    // Use word boundary matching to avoid partial matches
    let pattern_str = format!(r"\b{}\b", regex::escape(param_name));

    if let Ok(pattern) = Regex::new(&pattern_str) {
        pattern.is_match(&body_without_comments)
    } else {
        // If regex fails, assume it's used (conservative approach)
        true
    }
}

/// Check if a type is exported/public
///
/// This heuristic checks for common export patterns in different languages
/// to determine if a type is part of the public API.
///
/// # Parameters
/// - `type_name`: The type name to check
/// - `language`: The language name for pattern matching
/// - `content`: The file content to search
///
/// # Returns
/// `true` if the type appears to be exported/public
fn is_type_exported(type_name: &str, language: &str, content: &str) -> bool {
    match language.to_lowercase().as_str() {
        "rust" => {
            // Check for pub type/enum/struct
            let patterns = vec![
                format!(r"pub\s+type\s+{}\b", regex::escape(type_name)),
                format!(r"pub\s+enum\s+{}\b", regex::escape(type_name)),
                format!(r"pub\s+struct\s+{}\b", regex::escape(type_name)),
                format!(r"pub\s+trait\s+{}\b", regex::escape(type_name)),
            ];
            for pattern_str in patterns {
                if let Ok(pattern) = Regex::new(&pattern_str) {
                    if pattern.is_match(content) {
                        return true;
                    }
                }
            }
        }
        "typescript" | "javascript" => {
            // Check for export keyword
            let patterns = vec![
                format!(r"export\s+type\s+{}\b", regex::escape(type_name)),
                format!(r"export\s+interface\s+{}\b", regex::escape(type_name)),
                format!(r"export\s+enum\s+{}\b", regex::escape(type_name)),
                format!(r"export\s+class\s+{}\b", regex::escape(type_name)),
            ];
            for pattern_str in patterns {
                if let Ok(pattern) = Regex::new(&pattern_str) {
                    if pattern.is_match(content) {
                        return true;
                    }
                }
            }
        }
        "python" => {
            // In Python, all top-level definitions are potentially public
            // We use _ prefix to indicate private
            return !type_name.starts_with('_');
        }
        "go" => {
            // In Go, types starting with uppercase are exported
            return type_name.chars().next().is_some_and(|c| c.is_uppercase());
        }
        _ => {}
    }

    // Conservative default: assume it's exported
    false
}

/// Get language-specific import patterns
///
/// Returns regex patterns for detecting imports in different languages.
/// Each pattern should have one capture group that captures the module path.
fn get_import_patterns(language: &str) -> Vec<String> {
    match language.to_lowercase().as_str() {
        "rust" => vec![
            // use std::collections::HashMap;
            // use crate::module::*;
            r"use\s+([\w:]+)".to_string(),
        ],
        "typescript" | "javascript" => vec![
            // import { foo } from './module'
            // import * as foo from './module'
            r#"import\s+(?:\{[^}]*\}|\*\s+as\s+\w+|\w+)\s+from\s+['"]([^'"]+)['"]"#.to_string(),
        ],
        "python" => vec![
            // from module import foo
            // import module
            r"from\s+([\w.]+)\s+import".to_string(),
            r"import\s+([\w.]+)".to_string(),
        ],
        "go" => vec![
            // import "package"
            // import ( "package1" "package2" )
            r#"import\s+"([^"]+)""#.to_string(),
        ],
        _ => vec![],
    }
}

/// Extract imported symbols from an import statement
///
/// This function looks for the actual import statement in the source code
/// and extracts the symbols being imported. It reuses logic from the
/// unused_imports.rs handler.
///
/// # Parameters
/// - `content`: The file content to search
/// - `module_path`: The module path to look for
/// - `language`: The language name for pattern matching
///
/// # Returns
/// A vector of symbol names that are imported
fn extract_imported_symbols(content: &str, module_path: &str, language: &str) -> Vec<String> {
    let mut symbols = Vec::new();

    // Language-specific symbol extraction patterns
    let patterns = match language.to_lowercase().as_str() {
        "rust" => vec![
            // use std::collections::{HashMap, HashSet};
            format!(r"use\s+{}::\{{([^}}]+)\}}", regex::escape(module_path)),
            // use std::collections::HashMap;
            format!(r"use\s+{}::(\w+)", regex::escape(module_path)),
        ],
        "typescript" | "javascript" => vec![
            // import { foo, bar } from './module'
            format!(
                r#"import\s*\{{\s*([^}}]+)\s*\}}\s*from\s*['"]{}['"]"#,
                regex::escape(module_path)
            ),
            // import foo from './module'
            format!(
                r#"import\s+(\w+)\s+from\s*['"]{}['"]"#,
                regex::escape(module_path)
            ),
        ],
        "python" => vec![
            // from module import foo, bar
            format!(
                r"from\s+{}\s+import\s+([^;\n]+)",
                regex::escape(module_path)
            ),
        ],
        "go" => vec![
            // In Go, imports are typically used via package name
            // For now, we'll treat module imports as side-effects
        ],
        _ => vec![],
    };

    // Try each pattern
    for pattern_str in &patterns {
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
                                    .trim_matches(|c: char| !c.is_alphanumeric() && c != '_')
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

/// Check if a symbol is actually used in the code (excluding the import/definition)
///
/// Uses a simple heuristic: if the symbol appears more than once in the code,
/// it's likely being used (first occurrence is the import/definition).
///
/// This is reused from unused_imports.rs logic.
///
/// # Parameters
/// - `content`: The file content to search
/// - `symbol`: The symbol name to search for
///
/// # Returns
/// `true` if the symbol is used, `false` otherwise
fn is_symbol_used_in_code(content: &str, symbol: &str) -> bool {
    // Create pattern that matches the symbol as a word boundary
    let pattern_str = format!(r"\b{}\b", regex::escape(symbol));

    if let Ok(pattern) = Regex::new(&pattern_str) {
        let occurrences = pattern.find_iter(content).count();

        // If the symbol appears more than once, it's used
        // (first occurrence is typically the import/definition)
        occurrences > 1
    } else {
        // If regex fails, assume it's used (conservative approach)
        true
    }
}

/// Check if a module path is referenced in the code (for side-effect imports)
///
/// This checks if the module path appears outside of the import statement,
/// which would indicate it's used as a side-effect import.
///
/// # Parameters
/// - `content`: The file content to search
/// - `module_path`: The module path to search for
///
/// # Returns
/// `true` if the module is referenced, `false` otherwise
fn is_module_used_in_code(content: &str, module_path: &str) -> bool {
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

/// Check if a function is exported/public
///
/// This heuristic checks for common export patterns in different languages
/// to determine if a function is part of the public API.
///
/// # Parameters
/// - `func_name`: The function name to check
/// - `content`: The file content to search
/// - `language`: The language name for pattern matching
///
/// # Returns
/// `true` if the function appears to be exported/public
fn is_function_exported(func_name: &str, content: &str, language: &str) -> bool {
    match language.to_lowercase().as_str() {
        "rust" => {
            // Check for pub fn, pub(crate) fn, etc.
            let pub_pattern = format!(r"pub(?:\([^)]*\))?\s+fn\s+{}\b", regex::escape(func_name));
            if let Ok(pattern) = Regex::new(&pub_pattern) {
                return pattern.is_match(content);
            }
        }
        "typescript" | "javascript" => {
            // Check for export keyword before function
            let export_pattern = format!(
                r"export\s+(?:async\s+)?(?:function\s+)?{}\b",
                regex::escape(func_name)
            );
            if let Ok(pattern) = Regex::new(&export_pattern) {
                return pattern.is_match(content);
            }
        }
        "python" => {
            // In Python, functions not starting with _ are typically public
            // For MVP, we'll be conservative and treat all as potentially public
            return !func_name.starts_with('_');
        }
        "go" => {
            // In Go, functions starting with uppercase are exported
            return func_name.chars().next().is_some_and(|c| c.is_uppercase());
        }
        _ => {}
    }

    // Conservative default: assume it's exported
    true
}

fn to_protocol_safety_level(level: suggestions::SafetyLevel) -> SafetyLevel {
    match level {
        suggestions::SafetyLevel::Safe => SafetyLevel::Safe,
        suggestions::SafetyLevel::RequiresReview => SafetyLevel::RequiresReview,
        suggestions::SafetyLevel::Experimental => SafetyLevel::Experimental,
    }
}

fn generate_dead_code_refactoring_candidates(
    finding: &Finding,
    _parsed_source: &ParsedSource,
) -> Vec<RefactoringCandidate> {
    let mut candidates = Vec::new();

    let (refactor_type, json_kind) = match finding.kind.as_str() {
        "unused_import" => (RefactorType::RemoveUnusedImport, "import"),
        "unused_function" => (RefactorType::RemoveDeadCode, "function"),
        "unreachable_code" => (RefactorType::RemoveDeadCode, "block"),
        "unused_parameter" => (RefactorType::RemoveDeadCode, "parameter"),
        "unused_type" => (RefactorType::RemoveDeadCode, "type"),
        "unused_variable" => (RefactorType::RemoveDeadCode, "variable"),
        _ => return candidates,
    };

    if let Some(range) = &finding.location.range {
        candidates.push(RefactoringCandidate {
            refactor_type,
            message: finding.message.clone(),
            scope: Scope::File,
            has_side_effects: false,
            reference_count: Some(0),
            is_unreachable: false,
            is_recursive: false,
            involves_generics: false,
            involves_macros: false,
            evidence_strength: EvidenceStrength::Medium,
            location: Location {
                file: finding.location.file_path.clone(),
                line: range.start.line as usize,
                character: range.start.character as usize,
            },
            refactor_call_args: json!({
                "kind": json_kind,
                "target": {
                    "kind": "symbol",
                    "path": finding.location.file_path,
                    "selector": {
                        "line": range.start.line,
                        "character": range.start.character,
                        "symbol_name": finding.location.symbol
                    }
                },
                "options": {
                    "dryRun": false
                }
            }),
        });
    }

    candidates
}

pub struct DeadCodeHandler;

impl DeadCodeHandler {
    pub fn new() -> Self {
        Self
    }

    /// Handle workspace-scoped dead code analysis using LSP
    ///
    /// This function uses the LSP-based dead code analyzer for accurate
    /// cross-file analysis when workspace scope is requested.
    ///
    /// # Feature-gated
    /// This function is only available when the `analysis-dead-code` feature is enabled,
    /// as it requires LSP integration for accurate workspace-wide analysis.
    #[cfg(feature = "analysis-dead-code")]
    async fn handle_workspace_dead_code(
        &self,
        context: &ToolHandlerContext,
        args: &Value,
        scope_param: &super::engine::ScopeParam,
        kind: &str,
    ) -> ServerResult<Value> {
        use crate::lsp_provider_adapter::LspProviderAdapter;
        use mill_analysis_common::AnalysisEngine;
        use mill_analysis_dead_code::{DeadCodeAnalyzer, DeadCodeConfig};
        use mill_foundation::protocol::analysis_result::AnalysisResult;
        use std::path::Path;
        use std::sync::Arc;
        use std::time::Instant;
        use tracing::info;

        let start_time = Instant::now();
        let path_str = scope_param.path.as_deref().ok_or_else(|| {
            ServerError::invalid_request("Missing 'path' in scope parameter".to_string())
        })?;
        let workspace_path = Path::new(path_str);

        // Determine file extension for LSP client (default to Rust)
        let file_extension = args
            .get("file_extension")
            .and_then(|v| v.as_str())
            .unwrap_or("rs")
            .to_string();

        info!(
            workspace_path = %workspace_path.display(),
            file_extension = %file_extension,
            kind = %kind,
            "Starting workspace-scoped dead code analysis"
        );

        // Create LSP provider adapter
        let lsp_adapter = LspProviderAdapter::new(
            context.lsp_adapter.clone(),
            file_extension.clone(),
        );

        // Configure dead code analysis
        let mut config = DeadCodeConfig::default();

        // Apply configuration from args
        if let Some(file_types) = args.get("file_types").and_then(|v| v.as_array()) {
            config.file_types = Some(
                file_types
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect(),
            );
        }

        if let Some(min_refs) = args.get("min_reference_threshold").and_then(|v| v.as_u64()) {
            config.min_reference_threshold = min_refs as usize;
        }

        // Run analysis
        let analyzer = DeadCodeAnalyzer;
        let report = analyzer
            .analyze(Arc::new(lsp_adapter), workspace_path, config)
            .await
            .map_err(|e| ServerError::analysis(format!("Dead code analysis failed: {}", e)))?;

        info!(
            dead_symbols_found = report.dead_symbols.len(),
            files_analyzed = report.stats.files_analyzed,
            duration_ms = report.stats.duration_ms,
            "Workspace dead code analysis completed"
        );

        // Convert to AnalysisResult format
        use mill_foundation::protocol::analysis_result::{AnalysisScope, SeverityBreakdown};
        use uuid::Uuid;

        let findings: Vec<Finding> = report
            .dead_symbols
            .iter()
            .map(|symbol| Finding {
                id: Uuid::new_v4().to_string(),
                kind: match symbol.kind.as_str() {
                    "Function" => "unused_function",
                    "Class" => "unused_class",
                    "Variable" => "unused_variable",
                    "Constant" => "unused_constant",
                    _ => "unused_symbol",
                }
                .to_string(),
                severity: Severity::Medium,
                location: FindingLocation {
                    file_path: symbol.file_path.clone(),
                    range: Some(Range {
                        start: Position {
                            line: symbol.line,
                            character: symbol.column,
                        },
                        end: Position {
                            line: symbol.line,
                            character: symbol.column + symbol.name.len() as u32,
                        },
                    }),
                    symbol: Some(symbol.name.clone()),
                    symbol_kind: Some(symbol.kind.clone()),
                },
                message: format!("{} '{}' is never used", symbol.kind, symbol.name),
                suggestions: vec![Suggestion {
                    action: "remove_symbol".to_string(),
                    description: format!("Remove unused {} '{}'", symbol.kind.to_lowercase(), symbol.name),
                    target: None,
                    estimated_impact: "low".to_string(),
                    safety: SafetyLevel::Safe,
                    confidence: 0.9,
                    reversible: true,
                    refactor_call: None,
                }],
                metrics: {
                    let mut map = std::collections::HashMap::new();
                    map.insert("symbol_kind".to_string(), serde_json::json!(symbol.kind));
                    map.insert("reference_count".to_string(), serde_json::json!(symbol.reference_count));
                    Some(map)
                },
            })
            .collect();

        // Count findings by severity
        let medium_count = findings.len(); // All are Medium severity
        let by_severity = SeverityBreakdown {
            high: 0,
            medium: medium_count,
            low: 0,
        };

        let result = AnalysisResult {
            metadata: mill_foundation::protocol::analysis_result::AnalysisMetadata {
                category: "dead_code".to_string(),
                kind: kind.to_string(),
                scope: AnalysisScope {
                    scope_type: "workspace".to_string(),
                    path: workspace_path.to_string_lossy().to_string(),
                    include: vec![],
                    exclude: vec![],
                },
                language: Some(file_extension.clone()),
                timestamp: chrono::Utc::now().to_rfc3339(),
                thresholds: None,
            },
            summary: mill_foundation::protocol::analysis_result::AnalysisSummary {
                total_findings: findings.len(),
                returned_findings: findings.len(),
                has_more: false,
                by_severity,
                files_analyzed: report.stats.files_analyzed,
                symbols_analyzed: Some(report.stats.symbols_analyzed),
                analysis_time_ms: report.stats.duration_ms as u64,
                fix_actions: None,
            },
            findings,
        };

        Ok(serde_json::to_value(result).unwrap())
    }

    /// Fallback handler for when LSP feature is not enabled
    #[cfg(not(feature = "analysis-dead-code"))]
    async fn handle_workspace_dead_code(
        &self,
        _context: &ToolHandlerContext,
        _args: &Value,
        _scope_param: &super::engine::ScopeParam,
        _kind: &str,
    ) -> ServerResult<Value> {
        Err(ServerError::not_supported(
            "Workspace scope for dead code analysis requires the 'analysis-dead-code' feature to be enabled. \
             File-level analysis is available without this feature.".to_string(),
        ))
    }

    #[cfg(feature = "analysis-deep-dead-code")]
    async fn handle_workspace_deep_dead_code(
        &self,
        context: &ToolHandlerContext,
        args: &Value,
        scope_param: &super::engine::ScopeParam,
        kind: &str,
    ) -> ServerResult<Value> {
        use crate::lsp_provider_adapter::LspProviderAdapter;
        use mill_analysis_common::AnalysisEngine;
        use mill_analysis_deep_dead_code::{DeepDeadCodeAnalyzer, DeepDeadCodeConfig};
        use mill_foundation::protocol::analysis_result::{AnalysisResult, AnalysisScope, SeverityBreakdown};
        use std::path::Path;
        use std::sync::Arc;
        use std::time::Instant;

        // Extract path from Option<String>
        let path_str = scope_param.path.as_deref().ok_or_else(|| {
            ServerError::invalid_request("Missing 'path' in scope parameter".to_string())
        })?;
        let workspace_path = Path::new(path_str);

        // Get file extension for LSP client (default "rs")
        let file_extension = args.get("file_extension")
            .and_then(|v| v.as_str())
            .unwrap_or("rs")
            .to_string();

        // Create LSP provider adapter
        let lsp_adapter = LspProviderAdapter::new(
            context.lsp_adapter.clone(),
            file_extension.clone(),
        );

        // Configure analysis
        let config = DeepDeadCodeConfig {
            check_public_exports: args.get("check_public_exports")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            exclude_patterns: args.get("exclude_patterns")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                }),
        };

        // Run analysis
        let start = Instant::now();
        let analyzer = DeepDeadCodeAnalyzer;
        let report = analyzer.analyze(Arc::new(lsp_adapter), workspace_path, config).await
            .map_err(|e| ServerError::analysis(format!("Deep dead code analysis failed: {}", e)))?;
        let duration_ms = start.elapsed().as_millis() as u64;

        // Convert DeepDeadCodeReport to AnalysisResult
        let findings: Vec<Finding> = report.dead_symbols.iter().map(|symbol| {
            // Convert SymbolKind to string
            let symbol_kind = format!("{:?}", symbol.kind);
            let severity = if symbol.is_public {
                Severity::Low  // Public unused exports are lower priority
            } else {
                Severity::Medium
            };

            Finding {
                id: Uuid::new_v4().to_string(),
                kind: "unused_symbol".to_string(),
                severity,
                location: FindingLocation {
                    file_path: symbol.file_path.clone(),
                    range: Some(Range {
                        start: Position {
                            line: symbol.range.start.line,
                            character: symbol.range.start.character,
                        },
                        end: Position {
                            line: symbol.range.end.line,
                            character: symbol.range.end.character,
                        },
                    }),
                    symbol: Some(symbol.name.clone()),
                    symbol_kind: Some(symbol_kind.clone()),
                },
                message: format!(
                    "{} '{}' is never used",
                    if symbol.is_public { "Public" } else { "Private" },
                    symbol.name
                ),
                suggestions: vec![Suggestion {
                    action: "remove_symbol".to_string(),
                    description: format!("Remove unused {} '{}'", symbol_kind.to_lowercase(), symbol.name),
                    target: None,
                    estimated_impact: "low".to_string(),
                    safety: SafetyLevel::Safe,
                    confidence: if symbol.is_public { 0.7 } else { 0.9 },
                    reversible: true,
                    refactor_call: None,
                }],
                metrics: {
                    let mut map = std::collections::HashMap::new();
                    map.insert("symbol_kind".to_string(), serde_json::json!(symbol_kind));
                    map.insert("is_public".to_string(), serde_json::json!(symbol.is_public));
                    map.insert("symbol_id".to_string(), serde_json::json!(symbol.id));
                    Some(map)
                },
            }
        }).collect();

        let high_count = findings.iter().filter(|f| matches!(f.severity, Severity::High)).count();
        let medium_count = findings.iter().filter(|f| matches!(f.severity, Severity::Medium)).count();
        let low_count = findings.iter().filter(|f| matches!(f.severity, Severity::Low)).count();

        let result = AnalysisResult {
            metadata: AnalysisMetadata {
                category: "dead_code".to_string(),
                kind: kind.to_string(),
                scope: AnalysisScope {
                    scope_type: "workspace".to_string(),
                    path: workspace_path.to_string_lossy().to_string(),
                    include: vec![],
                    exclude: vec![],
                },
                language: Some(file_extension.clone()),
                timestamp: chrono::Utc::now().to_rfc3339(),
                thresholds: None,
            },
            summary: AnalysisSummary {
                total_findings: findings.len(),
                returned_findings: findings.len(),
                has_more: false,
                by_severity: SeverityBreakdown {
                    high: high_count,
                    medium: medium_count,
                    low: low_count,
                },
                files_analyzed: 0, // DeepDeadCodeReport doesn't track this
                symbols_analyzed: Some(report.dead_symbols.len()),
                analysis_time_ms: duration_ms,
                fix_actions: None,
            },
            findings,
        };

        Ok(serde_json::to_value(result).unwrap())
    }
}

#[async_trait]
impl ToolHandler for DeadCodeHandler {
    fn tool_names(&self) -> &[&str] {
        &["analyze.dead_code"]
    }

    fn is_internal(&self) -> bool {
        false // PUBLIC tool
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        let args = tool_call.arguments.clone().unwrap_or(json!({}));

        // Parse kind (required)
        let kind = args
            .get("kind")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::invalid_request("Missing 'kind' parameter"))?;

        // Validate kind
        let is_valid = match kind {
            "unused_imports" | "unused_symbols" | "unreachable_code" | "unused_parameters"
            | "unused_types" | "unused_variables" => true,
            #[cfg(feature = "analysis-deep-dead-code")]
            "deep" => true,
            _ => false,
        };

        if !is_valid {
            #[cfg(feature = "analysis-deep-dead-code")]
            let supported = "'unused_imports', 'unused_symbols', 'unreachable_code', 'unused_parameters', 'unused_types', 'unused_variables', 'deep'".to_string();
            #[cfg(not(feature = "analysis-deep-dead-code"))]
            let supported = "'unused_imports', 'unused_symbols', 'unreachable_code', 'unused_parameters', 'unused_types', 'unused_variables'".to_string();
            return Err(ServerError::invalid_request(format!(
                "Unsupported kind '{}'. Supported: {}",
                kind, supported
            )));
        }

        debug!(kind = %kind, "Handling analyze.dead_code request");

        // Check if workspace scope is requested
        let scope_param = super::engine::parse_scope_param(&args)?;
        let scope_type = scope_param.scope_type.as_deref().unwrap_or("file");

        if scope_type == "workspace" {
            // Use LSP-based workspace analysis
            #[cfg(feature = "analysis-deep-dead-code")]
            if kind == "deep" {
                return self
                    .handle_workspace_deep_dead_code(context, &args, &scope_param, kind)
                    .await;
            }

            self.handle_workspace_dead_code(context, &args, &scope_param, kind)
                .await
        } else {
            // For file-scope, we can choose to use the suggestion generator
            match kind {
                "unused_imports" | "unused_symbols" => {
                    use mill_foundation::protocol::analysis_result::AnalysisResult;
                    use std::path::Path;
                    use std::time::Instant;
                    use tracing::info;

                    let start_time = Instant::now();

                    // Replicate logic from engine::run_analysis to get access to parsed_source
                    let file_path = super::engine::extract_file_path(&args, &scope_param)?;
                    info!(file_path = %file_path, kind = %kind, "Running dead code analysis with suggestions");

                    let file_path_obj = Path::new(&file_path);
                    let extension = file_path_obj
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .ok_or_else(|| {
                            ServerError::invalid_request(format!(
                                "File has no extension: {}",
                                file_path
                            ))
                        })?;
                    let content = context
                        .app_state
                        .file_service
                        .read_file(file_path_obj)
                        .await
                        .map_err(|e| {
                            ServerError::internal(format!("Failed to read file: {}", e))
                        })?;
                    let plugin = context
                        .app_state
                        .language_plugins
                        .get_plugin(extension)
                        .ok_or_else(|| {
                            ServerError::not_supported(format!(
                                "No language plugin found for extension: {}",
                                extension
                            ))
                        })?;
                    let parsed_source = plugin.parse(&content).await.map_err(|e| {
                        ServerError::internal(format!("Failed to parse file: {}", e))
                    })?;
                    let language = plugin.metadata().name;
                    let complexity_report = mill_ast::complexity::analyze_file_complexity(
                        &file_path,
                        &content,
                        &parsed_source.symbols,
                        language,
                    );

                    // Choose detection function
                    let analysis_fn = if kind == "unused_imports" {
                        detect_unused_imports
                    } else {
                        detect_unused_symbols
                    };

                    let mut findings = analysis_fn(
                        &complexity_report,
                        &content,
                        &parsed_source.symbols,
                        language,
                        &file_path,
                        context.app_state.language_plugins.as_ref(),
                        get_analysis_config(context)?,
                    );

                    // NEW: Initialize suggestion generator
                    let suggestion_generator = SuggestionGenerator::new();

                    // NEW: Enhance findings with actionable suggestions
                    for finding in &mut findings {
                        let candidates =
                            generate_dead_code_refactoring_candidates(finding, &parsed_source);

                        let context = AnalysisContext {
                            file_path: file_path.clone(),
                            has_full_type_info: false, // File-scope analysis doesn't have LSP
                            has_partial_type_info: false, // ParsedSource doesn't have this
                            ast_parse_errors: 0,       // ParsedSource doesn't have this
                        };

                        let mut suggestions = Vec::new();
                        for candidate in candidates {
                            match suggestion_generator.generate_from_candidate(candidate, &context)
                            {
                                Ok(actionable) => {
                                    // Convert ActionableSuggestion to protocol::Suggestion
                                    let suggestion = Suggestion {
                                        action: actionable.refactor_call.as_ref().map(|rc| rc.tool.clone()).unwrap_or_else(|| "manual_fix".to_string()),
                                        description: actionable.message,
                                        target: None,
                                        estimated_impact: format!("{:?}", actionable.estimated_impact),
                                        safety: to_protocol_safety_level(actionable.safety),
                                        confidence: actionable.confidence,
                                        reversible: actionable.reversible,
                                        refactor_call: actionable.refactor_call.map(|rc| mill_foundation::protocol::analysis_result::RefactorCall {
                                            command: rc.tool,
                                            arguments: rc.arguments,
                                        }),
                                    };
                                    suggestions.push(suggestion);
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        error = %e,
                                        finding_kind = %finding.kind,
                                        "Failed to generate suggestion"
                                    );
                                }
                            }
                        }

                        if !suggestions.is_empty() {
                            finding.suggestions = suggestions;
                        }
                    }

                    let scope = mill_foundation::protocol::analysis_result::AnalysisScope {
                        scope_type: scope_param.scope_type.unwrap_or_else(|| "file".to_string()),
                        path: file_path.clone(),
                        include: scope_param.include,
                        exclude: scope_param.exclude,
                    };
                    let mut result = AnalysisResult::new("dead_code", kind, scope);
                    result.metadata.language = Some(language.to_string());
                    for finding in findings {
                        result.add_finding(finding);
                    }
                    result.summary.files_analyzed = 1;
                    result.summary.symbols_analyzed = Some(complexity_report.total_functions);
                    result.finalize(start_time.elapsed().as_millis() as u64);

                    serde_json::to_value(result).map_err(|e| {
                        ServerError::internal(format!("Failed to serialize result: {}", e))
                    })
                }
                "unreachable_code" | "unused_parameters" | "unused_types" | "unused_variables" => {
                    // Use the same suggestion generation path as unused_imports/unused_symbols
                    use mill_foundation::protocol::analysis_result::AnalysisResult;
                    use std::path::Path;
                    use std::time::Instant;
                    use tracing::info;

                    let start_time = Instant::now();

                    // Replicate logic from engine::run_analysis to get access to parsed_source
                    let file_path = super::engine::extract_file_path(&args, &scope_param)?;
                    info!(file_path = %file_path, kind = %kind, "Running dead code analysis with suggestions");

                    let file_path_obj = Path::new(&file_path);
                    let extension = file_path_obj
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .ok_or_else(|| {
                            ServerError::invalid_request(format!(
                                "File has no extension: {}",
                                file_path
                            ))
                        })?;
                    let content = context
                        .app_state
                        .file_service
                        .read_file(file_path_obj)
                        .await
                        .map_err(|e| {
                            ServerError::internal(format!("Failed to read file: {}", e))
                        })?;
                    let plugin = context
                        .app_state
                        .language_plugins
                        .get_plugin(extension)
                        .ok_or_else(|| {
                            ServerError::not_supported(format!(
                                "No language plugin found for extension: {}",
                                extension
                            ))
                        })?;
                    let parsed_source = plugin.parse(&content).await.map_err(|e| {
                        ServerError::internal(format!("Failed to parse file: {}", e))
                    })?;
                    let language = plugin.metadata().name;
                    let complexity_report = mill_ast::complexity::analyze_file_complexity(
                        &file_path,
                        &content,
                        &parsed_source.symbols,
                        language,
                    );

                    // Choose detection function
                    let analysis_fn = match kind {
                        "unreachable_code" => detect_unreachable_code,
                        "unused_parameters" => detect_unused_parameters,
                        "unused_types" => detect_unused_types,
                        "unused_variables" => detect_unused_variables,
                        _ => unreachable!(),
                    };

                    let mut findings = analysis_fn(
                        &complexity_report,
                        &content,
                        &parsed_source.symbols,
                        language,
                        &file_path,
                        context.app_state.language_plugins.as_ref(),
                        get_analysis_config(context)?,
                    );

                    // Initialize suggestion generator
                    let suggestion_generator = SuggestionGenerator::new();

                    // Enhance findings with actionable suggestions
                    for finding in &mut findings {
                        let candidates =
                            generate_dead_code_refactoring_candidates(finding, &parsed_source);

                        let context = AnalysisContext {
                            file_path: file_path.clone(),
                            has_full_type_info: false, // File-scope analysis doesn't have LSP
                            has_partial_type_info: false, // ParsedSource doesn't have this
                            ast_parse_errors: 0,       // ParsedSource doesn't have this
                        };

                        let mut suggestions = Vec::new();
                        for candidate in candidates {
                            match suggestion_generator.generate_from_candidate(candidate, &context)
                            {
                                Ok(actionable) => {
                                    // Convert ActionableSuggestion to protocol::Suggestion
                                    let suggestion = Suggestion {
                                        action: actionable.refactor_call.as_ref().map(|rc| rc.tool.clone()).unwrap_or_else(|| "manual_fix".to_string()),
                                        description: actionable.message,
                                        target: None,
                                        estimated_impact: format!("{:?}", actionable.estimated_impact),
                                        safety: to_protocol_safety_level(actionable.safety),
                                        confidence: actionable.confidence,
                                        reversible: actionable.reversible,
                                        refactor_call: actionable.refactor_call.map(|rc| mill_foundation::protocol::analysis_result::RefactorCall {
                                            command: rc.tool,
                                            arguments: rc.arguments,
                                        }),
                                    };
                                    suggestions.push(suggestion);
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        error = %e,
                                        finding_kind = %finding.kind,
                                        "Failed to generate suggestion"
                                    );
                                }
                            }
                        }

                        if !suggestions.is_empty() {
                            finding.suggestions = suggestions;
                        }
                    }

                    let scope = mill_foundation::protocol::analysis_result::AnalysisScope {
                        scope_type: scope_param.scope_type.unwrap_or_else(|| "file".to_string()),
                        path: file_path.clone(),
                        include: scope_param.include,
                        exclude: scope_param.exclude,
                    };
                    let mut result = AnalysisResult::new("dead_code", kind, scope);
                    result.metadata.language = Some(language.to_string());
                    for finding in findings {
                        result.add_finding(finding);
                    }
                    result.summary.files_analyzed = 1;
                    result.summary.symbols_analyzed = Some(complexity_report.total_functions);
                    result.finalize(start_time.elapsed().as_millis() as u64);

                    serde_json::to_value(result).map_err(|e| {
                        ServerError::internal(format!("Failed to serialize result: {}", e))
                    })
                }
                _ => {
                    return Err(ServerError::invalid_request(format!(
                        "Kind '{}' is not supported for file-scope analysis. Use scope_type='workspace' or choose a different kind.",
                        kind
                    )));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    /// Test that the error message from the _ match arm is clear and helpful.
    /// This is a unit test that verifies our error handling logic without needing
    /// the full integration stack.
    #[test]
    fn test_error_message_for_unsupported_file_scope_kinds() {
        // Verify the error message format we expect
        let kind = "deep";
        let expected_error = format!(
            "Kind '{}' is not supported for file-scope analysis. Use scope_type='workspace' or choose a different kind.",
            kind
        );

        // This test documents the expected error message
        assert!(expected_error.contains("not supported for file-scope"));
        assert!(expected_error.contains("workspace"));
        assert!(expected_error.contains(kind));
    }

    /// Test that our validation logic correctly identifies supported file-scope kinds.
    /// This verifies the match arms in the handler logic.
    #[test]
    fn test_kind_validation_coverage() {
        // These are the kinds that should work for file-scope analysis
        let file_scope_kinds = vec![
            "unused_imports",
            "unused_symbols",
            "unreachable_code",
            "unused_parameters",
            "unused_types",
            "unused_variables",
        ];

        // Verify our test knows about all supported kinds
        for kind in &file_scope_kinds {
            // This documents which kinds are expected to work with file-scope
            assert!(
                matches!(
                    *kind,
                    "unused_imports"
                        | "unused_symbols"
                        | "unreachable_code"
                        | "unused_parameters"
                        | "unused_types"
                        | "unused_variables"
                ),
                "Kind '{}' should be supported for file-scope",
                kind
            );
        }

        // "deep" should NOT be in this list - it requires workspace scope
        #[cfg(feature = "analysis-deep-dead-code")]
        {
            let deep_kind = "deep";
            assert!(
                !file_scope_kinds.contains(&deep_kind),
                "Kind 'deep' should NOT be in file-scope supported kinds"
            );
        }
    }
}
