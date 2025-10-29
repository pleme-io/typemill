#![allow(dead_code, unused_variables)]

//! Documentation analysis handler
//!
//! This module provides detection for documentation-related patterns including:
//! - Coverage: Documentation coverage analysis
//! - Quality: Documentation quality assessment
//! - Style: Documentation style consistency
//! - Examples: Code example presence
//! - Todos: TODO/FIXME tracking
//!
//! Uses the shared analysis engine for orchestration and focuses only on
//! detection logic.

use super::super::{ToolHandler, ToolHandlerContext};
use super::suggestions::{AnalysisContext, RefactoringCandidate, SuggestionGenerator};
use anyhow::Result;
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::protocol::analysis_result::{
    Finding, FindingLocation, Position, Range, SafetyLevel, Severity, Suggestion,
};
use mill_foundation::protocol::{ApiError as ServerError, ApiResult as ServerResult};
use mill_plugin_api::{Symbol, SymbolKind};
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::debug;

/// Analyze documentation coverage
///
/// This function calculates documentation coverage by counting documented vs
/// undocumented symbols, with special focus on public API documentation.
///
/// # Algorithm
/// 1. Iterate through all symbols from the parsed source
/// 2. Count total symbols that should be documented (functions, classes, etc.)
/// 3. Check each symbol for doc comments using language-specific patterns
/// 4. Identify undocumented public symbols (high priority)
/// 5. Calculate coverage percentage
/// 6. Generate findings with coverage metrics
///
/// # Heuristics
/// - Doc comment detection: Language-specific patterns (///, /** */, """, etc.)
/// - Public symbol detection: Visibility keywords (pub, export)
/// - Coverage threshold: <50% High, <70% Medium, else Low
/// - For MVP: Regex-based doc comment detection
///
/// # Future Enhancements
/// TODO: Add AST-based doc comment extraction for accuracy
/// TODO: Support workspace-wide documentation coverage
/// TODO: Detect documentation staleness (outdated docs)
/// TODO: Generate documentation coverage reports (HTML/JSON)
///
/// # Parameters
/// - `complexity_report`: Used for function metrics
/// - `content`: The raw file content for doc comment detection
/// - `symbols`: The parsed symbols to analyze
/// - `language`: The language name for doc patterns
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A single finding with:
/// - Metrics including total_symbols, documented_symbols, coverage_percentage, undocumented_public
/// - Severity: High if coverage < 50%, Medium if < 70%, Low otherwise
/// - Suggestions to add documentation to undocumented public symbols
use super::config::AnalysisConfig;
pub fn detect_coverage(
    _complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    symbols: &[Symbol],
    language: &str,
    file_path: &str,
    _registry: &crate::LanguagePluginRegistry,
    _config: &AnalysisConfig,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Filter symbols that should be documented (functions, classes, structs, etc.)
    let documentable_symbols: Vec<&Symbol> = symbols
        .iter()
        .filter(|s| {
            matches!(
                s.kind,
                SymbolKind::Function
                    | SymbolKind::Method
                    | SymbolKind::Class
                    | SymbolKind::Struct
                    | SymbolKind::Interface
                    | SymbolKind::Enum
                    | SymbolKind::Module
            )
        })
        .collect();

    if documentable_symbols.is_empty() {
        // No symbols to analyze
        return findings;
    }

    let lines: Vec<&str> = content.lines().collect();

    // Count documented and undocumented symbols
    let mut documented_count = 0;
    let mut undocumented_public: Vec<String> = Vec::new();

    for symbol in &documentable_symbols {
        let is_public = is_symbol_public(symbol, &lines, language);
        let has_doc = has_doc_comment(symbol, &lines, language);

        if has_doc {
            documented_count += 1;
        } else if is_public {
            undocumented_public.push(symbol.name.clone());
        }
    }

    let total_symbols = documentable_symbols.len();
    let coverage_percentage = if total_symbols > 0 {
        (documented_count as f64 / total_symbols as f64) * 100.0
    } else {
        100.0
    };

    // Determine severity
    let severity = if coverage_percentage < 50.0 {
        Severity::High
    } else if coverage_percentage < 70.0 {
        Severity::Medium
    } else {
        Severity::Low
    };

    let mut metrics = HashMap::new();
    metrics.insert("total_symbols".to_string(), json!(total_symbols));
    metrics.insert("documented_count".to_string(), json!(documented_count));
    metrics.insert(
        "coverage_percentage".to_string(),
        json!(coverage_percentage),
    );
    metrics.insert(
        "undocumented_count".to_string(),
        json!(undocumented_public.len()),
    );
    metrics.insert(
        "undocumented_public".to_string(),
        json!(undocumented_public),
    );

    let message = format!(
        "Documentation coverage: {:.1}% ({}/{} symbols documented, {} undocumented public symbols)",
        coverage_percentage,
        documented_count,
        total_symbols,
        undocumented_public.len()
    );

    let mut finding = Finding {
        id: format!("doc-coverage-{}", file_path),
        kind: "coverage".to_string(),
        severity,
        location: FindingLocation {
            file_path: file_path.to_string(),
            range: None, // File-level finding
            symbol: None,
            symbol_kind: Some("module".to_string()),
        },
        metrics: Some(metrics),
        message,
        suggestions: vec![],
    };

    if !undocumented_public.is_empty() {
        let suggestion_generator = SuggestionGenerator::new();
        let context = AnalysisContext {
            file_path: file_path.to_string(),
            has_full_type_info: false,
            has_partial_type_info: false,
            ast_parse_errors: 0,
        };

        if let Ok(candidates) = generate_documentation_refactoring_candidates(&finding) {
            let suggestions = suggestion_generator.generate_multiple(candidates, &context);
            finding.suggestions = suggestions
                .into_iter()
                .map(|s| s.into())
                .collect::<Vec<Suggestion>>();
        }
    }

    findings.push(finding);

    findings
}

/// Assess documentation quality
///
/// This function analyzes the quality of existing documentation by checking
/// for meaningful descriptions, parameter documentation, return documentation,
/// and examples in complex functions.
///
/// # Algorithm
/// 1. Iterate through documented symbols
/// 2. Extract doc comment content for each symbol
/// 3. Check for meaningful descriptions (> 10 chars, not just type info)
/// 4. Detect missing parameter documentation
/// 5. Detect missing return value documentation
/// 6. Check for examples in complex functions (complexity > 10)
/// 7. Generate findings with quality metrics
///
/// # Heuristics
/// - Meaningful description: > 10 characters, not just type name
/// - Parameter docs: @param, :param:, or "# Arguments" patterns
/// - Return docs: @returns, :returns:, or "# Returns" patterns
/// - Complex functions: cyclomatic complexity > 10 from complexity_report
/// - For MVP: Regex-based pattern matching
///
/// # Future Enhancements
/// TODO: Add AST-based parameter extraction for accuracy
/// TODO: Detect documentation-code consistency (param names match)
/// TODO: Check for examples that actually compile/run
/// TODO: Measure documentation readability (Flesch-Kincaid score)
///
/// # Parameters
/// - `complexity_report`: Used to identify complex functions
/// - `content`: The raw file content for doc extraction
/// - `symbols`: The parsed symbols to analyze
/// - `language`: The language name for doc patterns
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A vector of findings (one per symbol with quality issues), each with:
/// - Metrics including symbols_with_quality_docs, missing_param_docs, missing_return_docs, missing_examples
/// - Severity: Medium if quality issues found
/// - Suggestions to add parameter docs, return docs, examples
pub fn detect_quality(
    complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    symbols: &[Symbol],
    language: &str,
    file_path: &str,
    _registry: &crate::LanguagePluginRegistry,
    _config: &AnalysisConfig,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    let lines: Vec<&str> = content.lines().collect();

    // Filter documented symbols
    let documented_symbols: Vec<&Symbol> = symbols
        .iter()
        .filter(|s| {
            matches!(
                s.kind,
                SymbolKind::Function | SymbolKind::Method | SymbolKind::Class | SymbolKind::Struct
            )
        })
        .filter(|s| has_doc_comment(s, &lines, language))
        .collect();

    if documented_symbols.is_empty() {
        return findings;
    }

    let mut symbols_with_quality_docs = 0;
    let mut missing_param_docs = 0;
    let mut missing_return_docs = 0;
    let mut missing_examples = 0;

    // Build complexity map for quick lookup
    let complexity_map: HashMap<String, u32> = complexity_report
        .functions
        .iter()
        .map(|f| (f.name.clone(), f.complexity.cyclomatic))
        .collect();

    for symbol in documented_symbols {
        let doc_comment = extract_doc_comment(symbol, &lines, language);

        if doc_comment.is_empty() {
            continue;
        }

        let mut has_quality_issue = false;
        let mut issues = Vec::new();

        // Check 1: Meaningful description
        let has_meaningful_desc =
            doc_comment.len() > 10 && !is_trivial_doc(&doc_comment, &symbol.name);
        if !has_meaningful_desc {
            has_quality_issue = true;
            issues.push("trivial_description");
        } else {
            symbols_with_quality_docs += 1;
        }

        // Check 2: Parameter documentation (for functions/methods)
        if matches!(symbol.kind, SymbolKind::Function | SymbolKind::Method) {
            let has_param_docs = has_parameter_docs(&doc_comment, language);
            if !has_param_docs {
                has_quality_issue = true;
                missing_param_docs += 1;
                issues.push("missing_param_docs");
            }
        }

        // Check 3: Return documentation (for functions/methods)
        if matches!(symbol.kind, SymbolKind::Function | SymbolKind::Method) {
            let has_return_docs = has_return_docs(&doc_comment, language);
            if !has_return_docs {
                has_quality_issue = true;
                missing_return_docs += 1;
                issues.push("missing_return_docs");
            }
        }

        // Check 4: Examples in complex functions
        if let Some(&complexity) = complexity_map.get(&symbol.name) {
            if complexity > 10 {
                let has_examples = has_code_examples(&doc_comment, language);
                if !has_examples {
                    has_quality_issue = true;
                    missing_examples += 1;
                    issues.push("missing_examples");
                }
            }
        }

        // Generate finding if quality issues detected
        if has_quality_issue {
            let mut metrics = HashMap::new();
            metrics.insert("issues".to_string(), json!(issues));
            metrics.insert("doc_length".to_string(), json!(doc_comment.len()));

            let mut finding = Finding {
                id: format!("doc-quality-{}-{}", file_path, symbol.location.line),
                kind: "quality".to_string(),
                severity: Severity::Medium,
                location: FindingLocation {
                    file_path: file_path.to_string(),
                    range: Some(Range {
                        start: Position {
                            line: symbol.location.line as u32,
                            character: symbol.location.column as u32,
                        },
                        end: Position {
                            line: symbol.location.line as u32,
                            character: symbol.location.column as u32,
                        },
                    }),
                    symbol: Some(symbol.name.clone()),
                    symbol_kind: Some(format!("{:?}", symbol.kind).to_lowercase()),
                },
                metrics: Some(metrics),
                message: format!(
                    "Documentation quality issues in '{}': {}",
                    symbol.name,
                    issues.join(", ")
                ),
                suggestions: vec![],
            };

            let suggestion_generator = SuggestionGenerator::new();
            let context = AnalysisContext {
                file_path: file_path.to_string(),
                has_full_type_info: false,
                has_partial_type_info: false,
                ast_parse_errors: 0,
            };

            if let Ok(candidates) = generate_documentation_refactoring_candidates(&finding) {
                let suggestions = suggestion_generator.generate_multiple(candidates, &context);
                finding.suggestions = suggestions
                    .into_iter()
                    .map(|s| s.into())
                    .collect::<Vec<Suggestion>>();
            }

            findings.push(finding);
        }
    }

    // Add summary finding if multiple issues
    if !findings.is_empty() {
        let mut summary_metrics = HashMap::new();
        summary_metrics.insert(
            "symbols_with_quality_docs".to_string(),
            json!(symbols_with_quality_docs),
        );
        summary_metrics.insert("missing_param_docs".to_string(), json!(missing_param_docs));
        summary_metrics.insert(
            "missing_return_docs".to_string(),
            json!(missing_return_docs),
        );
        summary_metrics.insert("missing_examples".to_string(), json!(missing_examples));
        summary_metrics.insert("total_issues".to_string(), json!(findings.len()));

        findings.insert(
            0,
            Finding {
                id: format!("doc-quality-summary-{}", file_path),
                kind: "quality_summary".to_string(),
                severity: Severity::Medium,
                location: FindingLocation {
                    file_path: file_path.to_string(),
                    range: None,
                    symbol: None,
                    symbol_kind: Some("module".to_string()),
                },
                metrics: Some(summary_metrics),
                message: format!(
                    "Documentation quality analysis: {} symbols with issues (param docs: {}, return docs: {}, examples: {})",
                    findings.len() - 1,
                    missing_param_docs,
                    missing_return_docs,
                    missing_examples
                ),
                suggestions: vec![],
            },
        );
    }

    findings
}

/// Check documentation style consistency
///
/// This function detects inconsistent documentation styles across the codebase,
/// including mixed comment styles, capitalization, punctuation, and doc tags.
///
/// # Algorithm
/// 1. Extract all doc comments from the file
/// 2. Detect mixed comment styles (/** */ vs ///, # vs """)
/// 3. Check for consistent capitalization (first letter)
/// 4. Check for consistent punctuation (ending with period)
/// 5. Detect missing @param/@returns tags (JSDoc style)
/// 6. Count style violations by category
/// 7. Generate findings with style metrics
///
/// # Heuristics
/// - Comment style detection: Regex patterns for each language
/// - Capitalization: First character of doc description
/// - Punctuation: Last character of doc description
/// - JSDoc tags: @param, @returns, @throws patterns
/// - For MVP: Line-by-line pattern matching
///
/// # Future Enhancements
/// TODO: Add configurable style rules (ESLint/TSDoc style)
/// TODO: Support auto-fixing style violations
/// TODO: Detect inconsistent terminology usage
/// TODO: Check for documentation template compliance
///
/// # Parameters
/// - `complexity_report`: Not used for style detection
/// - `content`: The raw file content for style analysis
/// - `symbols`: The parsed symbols for context
/// - `language`: The language name for style rules
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A single finding with:
/// - Metrics including style_violations, mixed_styles, capitalization_issues, punctuation_issues
/// - Severity: Low (style is informational)
/// - Suggestions to use consistent doc comment style
pub fn detect_style(
    _complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    symbols: &[Symbol],
    language: &str,
    file_path: &str,
    _registry: &crate::LanguagePluginRegistry,
    _config: &AnalysisConfig,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    let lines: Vec<&str> = content.lines().collect();

    // Extract all doc comments with their line numbers
    let doc_comments = extract_all_doc_comments(content, language);

    if doc_comments.is_empty() {
        return findings;
    }

    // Detect mixed comment styles from original content (before stripping prefixes)
    let comment_styles = detect_comment_styles_from_content(content, language);
    let mixed_styles = comment_styles.len() > 1;

    // Check capitalization and punctuation consistency
    let mut capitalization_issues = 0;
    let mut punctuation_issues = 0;
    let mut first_letter_uppercase = 0;
    let mut ends_with_period = 0;

    for (_, comment) in &doc_comments {
        let trimmed = comment.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Check first letter capitalization
        if let Some(first_char) = trimmed.chars().next() {
            if first_char.is_alphabetic() {
                if first_char.is_uppercase() {
                    first_letter_uppercase += 1;
                } else {
                    capitalization_issues += 1;
                }
            }
        }

        // Check ending punctuation
        if let Some(last_char) = trimmed.chars().last() {
            if last_char == '.' {
                ends_with_period += 1;
            } else if last_char.is_alphabetic() {
                punctuation_issues += 1;
            }
        }
    }

    let total_violations =
        if mixed_styles { 1 } else { 0 } + capitalization_issues + punctuation_issues;

    if total_violations == 0 {
        // No style issues
        return findings;
    }

    let mut metrics = HashMap::new();
    metrics.insert("style_violations".to_string(), json!(total_violations));
    metrics.insert("mixed_styles".to_string(), json!(mixed_styles));
    metrics.insert("comment_styles_found".to_string(), json!(comment_styles));
    metrics.insert(
        "capitalization_issues".to_string(),
        json!(capitalization_issues),
    );
    metrics.insert("punctuation_issues".to_string(), json!(punctuation_issues));
    metrics.insert("total_comments".to_string(), json!(doc_comments.len()));

    let message = format!(
        "Documentation style inconsistencies: {} violations (mixed styles: {}, capitalization: {}, punctuation: {})",
        total_violations,
        if mixed_styles { "yes" } else { "no" },
        capitalization_issues,
        punctuation_issues
    );

    let mut finding = Finding {
        id: format!("doc-style-{}", file_path),
        kind: "style".to_string(),
        severity: Severity::Low, // Style is informational
        location: FindingLocation {
            file_path: file_path.to_string(),
            range: None,
            symbol: None,
            symbol_kind: Some("module".to_string()),
        },
        metrics: Some(metrics),
        message,
        suggestions: vec![],
    };

    let suggestion_generator = SuggestionGenerator::new();
    let context = AnalysisContext {
        file_path: file_path.to_string(),
        has_full_type_info: false,
        has_partial_type_info: false,
        ast_parse_errors: 0,
    };

    if let Ok(candidates) = generate_documentation_refactoring_candidates(&finding) {
        let suggestions = suggestion_generator.generate_multiple(candidates, &context);
        finding.suggestions = suggestions
            .into_iter()
            .map(|s| s.into())
            .collect::<Vec<Suggestion>>();
    }

    findings.push(finding);

    findings
}

/// Find code examples in documentation
///
/// This function searches for code blocks in doc comments and calculates
/// example coverage percentage, flagging complex functions without examples.
///
/// # Algorithm
/// 1. Extract all doc comments from the file
/// 2. Search for code blocks in each doc comment (```code``` or indented)
/// 3. Count functions with examples
/// 4. Calculate example coverage percentage
/// 5. Flag complex functions without examples (complexity > 10)
/// 6. Generate findings with example metrics
///
/// # Heuristics
/// - Code block detection: ```language``` or 4-space indented blocks
/// - Complex function threshold: cyclomatic complexity > 10
/// - Example coverage: functions_with_examples / total_functions
/// - For MVP: Regex-based code block detection
///
/// # Future Enhancements
/// TODO: Validate that code examples are syntactically correct
/// TODO: Run code examples as tests (doctest style)
/// TODO: Detect outdated examples (code changes but examples don't)
/// TODO: Suggest generating examples from unit tests
///
/// # Parameters
/// - `complexity_report`: Used to identify complex functions
/// - `content`: The raw file content for example detection
/// - `symbols`: The parsed symbols for function identification
/// - `language`: The language name for code block syntax
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A single finding with:
/// - Metrics including functions_with_examples, example_coverage_percentage, complex_without_examples
/// - Severity: Medium if complex functions lack examples
/// - Suggestions to add examples to complex functions
pub fn detect_examples(
    complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    symbols: &[Symbol],
    language: &str,
    file_path: &str,
    _registry: &crate::LanguagePluginRegistry,
    _config: &AnalysisConfig,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    let lines: Vec<&str> = content.lines().collect();

    // Filter functions/methods
    let functions: Vec<&Symbol> = symbols
        .iter()
        .filter(|s| matches!(s.kind, SymbolKind::Function | SymbolKind::Method))
        .collect();

    if functions.is_empty() {
        return findings;
    }

    // Build complexity map
    let complexity_map: HashMap<String, u32> = complexity_report
        .functions
        .iter()
        .map(|f| (f.name.clone(), f.complexity.cyclomatic))
        .collect();

    let mut functions_with_examples = 0;
    let mut complex_without_examples: Vec<String> = Vec::new();

    for function in &functions {
        if !has_doc_comment(function, &lines, language) {
            continue;
        }

        let doc_comment = extract_doc_comment(function, &lines, language);
        let has_example = has_code_examples(&doc_comment, language);

        if has_example {
            functions_with_examples += 1;
        } else {
            // Check if function is complex
            if let Some(&complexity) = complexity_map.get(&function.name) {
                if complexity > 10 {
                    complex_without_examples.push(function.name.clone());
                }
            }
        }
    }

    let total_functions = functions.len();
    let example_coverage_percentage = if total_functions > 0 {
        (functions_with_examples as f64 / total_functions as f64) * 100.0
    } else {
        0.0
    };

    let severity = if !complex_without_examples.is_empty() {
        Severity::Medium
    } else {
        Severity::Low
    };

    let mut metrics = HashMap::new();
    metrics.insert(
        "functions_with_examples".to_string(),
        json!(functions_with_examples),
    );
    metrics.insert(
        "example_coverage_percentage".to_string(),
        json!(example_coverage_percentage),
    );
    metrics.insert(
        "complex_without_examples_count".to_string(),
        json!(complex_without_examples.len()),
    );
    metrics.insert(
        "complex_without_examples".to_string(),
        json!(complex_without_examples),
    );
    metrics.insert("total_functions".to_string(), json!(total_functions));

    let message = if !complex_without_examples.is_empty() {
        format!(
            "Code examples coverage: {:.1}% ({}/{} functions), {} complex functions lack examples",
            example_coverage_percentage,
            functions_with_examples,
            total_functions,
            complex_without_examples.len()
        )
    } else {
        format!(
            "Code examples coverage: {:.1}% ({}/{} functions documented with examples)",
            example_coverage_percentage, functions_with_examples, total_functions
        )
    };

    let mut finding = Finding {
        id: format!("doc-examples-{}", file_path),
        kind: "examples".to_string(),
        severity,
        location: FindingLocation {
            file_path: file_path.to_string(),
            range: None,
            symbol: None,
            symbol_kind: Some("module".to_string()),
        },
        metrics: Some(metrics),
        message,
        suggestions: vec![],
    };

    if !complex_without_examples.is_empty() {
        let suggestion_generator = SuggestionGenerator::new();
        let context = AnalysisContext {
            file_path: file_path.to_string(),
            has_full_type_info: false,
            has_partial_type_info: false,
            ast_parse_errors: 0,
        };

        if let Ok(candidates) = generate_documentation_refactoring_candidates(&finding) {
            let suggestions = suggestion_generator.generate_multiple(candidates, &context);
            finding.suggestions = suggestions
                .into_iter()
                .map(|s| s.into())
                .collect::<Vec<Suggestion>>();
        }
    }

    findings.push(finding);

    findings
}

fn generate_documentation_refactoring_candidates(
    finding: &Finding,
) -> Result<Vec<RefactoringCandidate>> {
    let candidates = Vec::new();
    let location = finding.location.clone();
    let line = location.range.as_ref().map(|r| r.start.line).unwrap_or(0) as usize;

    match finding.kind.as_str() {
        "coverage" if finding.severity >= Severity::Medium => {
            // Suggest adding documentation, but this would require a new tool.
        }
        "quality" if finding.severity >= Severity::Medium => {
            // Suggest improving documentation.
        }
        "style" if finding.severity >= Severity::Low => {
            // Suggest fixing style.
        }
        "examples" if finding.severity >= Severity::Medium => {
            // Suggest adding examples.
        }
        "todos" if finding.severity >= Severity::Medium => {
            // Suggest creating issues.
        }
        _ => {}
    }

    Ok(candidates)
}

/// Track TODO/FIXME comments
///
/// This function searches for TODO, FIXME, HACK, XXX, and NOTE patterns
/// in comments, categorizes them by severity, and tracks their locations.
///
/// # Algorithm
/// 1. Search for TODO/FIXME/HACK/XXX/NOTE patterns in all comments
/// 2. Extract TODO text and location
/// 3. Categorize by severity (FIXME > TODO > NOTE)
/// 4. Count by category
/// 5. Identify oldest TODOs if dated
/// 6. Generate findings with TODO metrics
///
/// # Heuristics
/// - TODO patterns: TODO, FIXME, HACK, XXX, NOTE (case-insensitive)
/// - Severity: FIXME/HACK (High), TODO/XXX (Medium), NOTE (Low)
/// - Date detection: YYYY-MM-DD, @author patterns
/// - For MVP: Regex-based pattern matching
///
/// # Future Enhancements
/// TODO: Link TODOs to issue tracker (GitHub, Jira)
/// TODO: Detect stale TODOs (> 6 months old)
/// TODO: Group TODOs by author/owner
/// TODO: Generate TODO reports for sprint planning
///
/// # Parameters
/// - `complexity_report`: Not used for TODO detection
/// - `content`: The raw file content for TODO extraction
/// - `symbols`: Not used for TODO detection
/// - `language`: The language name for comment syntax
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A vector of findings (one per TODO category), each with:
/// - Metrics including total_todos, todos_by_category, oldest_todo (if dated)
/// - Severity: High if FIXMEs found, Medium if many TODOs (> 10), Low otherwise
/// - Suggestions to address FIXMEs, create issues for TODOs
pub fn detect_todos(
    _complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    _symbols: &[Symbol],
    language: &str,
    file_path: &str,
    _registry: &crate::LanguagePluginRegistry,
    _config: &AnalysisConfig,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Define TODO patterns with severity
    let patterns = vec![
        ("FIXME", Severity::High),
        ("HACK", Severity::High),
        ("XXX", Severity::Medium),
        ("TODO", Severity::Medium),
        ("NOTE", Severity::Low),
    ];

    let mut todos_by_category: HashMap<String, Vec<TodoItem>> = HashMap::new();
    let comment_pattern = get_comment_pattern(language);

    for (line_num, line) in content.lines().enumerate() {
        // Check if line is a comment
        if !is_comment_line(line, &comment_pattern) {
            continue;
        }

        // Search for TODO patterns
        for (pattern, severity) in &patterns {
            if let Some(todo_text) = extract_todo_text(line, pattern) {
                let item = TodoItem {
                    pattern: pattern.to_string(),
                    text: todo_text,
                    line: line_num + 1,
                    severity: *severity,
                };

                todos_by_category
                    .entry(pattern.to_string())
                    .or_default()
                    .push(item);
            }
        }
    }

    if todos_by_category.is_empty() {
        // No TODOs found
        return findings;
    }

    // Calculate total TODOs and determine overall severity
    let total_todos: usize = todos_by_category.values().map(|v| v.len()).sum();
    let has_fixmes =
        todos_by_category.contains_key("FIXME") || todos_by_category.contains_key("HACK");
    let many_todos = total_todos > 10;

    let overall_severity = if has_fixmes {
        Severity::High
    } else if many_todos {
        Severity::Medium
    } else {
        Severity::Low
    };

    // Build metrics
    let mut metrics = HashMap::new();
    metrics.insert("total_todos".to_string(), json!(total_todos));

    let mut category_counts: HashMap<String, usize> = HashMap::new();
    for (category, items) in &todos_by_category {
        category_counts.insert(category.clone(), items.len());
    }
    metrics.insert("todos_by_category".to_string(), json!(category_counts));

    // Build detailed todo list
    let mut all_todos = Vec::new();
    for (category, items) in &todos_by_category {
        for item in items {
            all_todos.push(json!({
                "category": category,
                "text": item.text,
                "line": item.line,
                "severity": format!("{:?}", item.severity),
            }));
        }
    }
    metrics.insert("todos".to_string(), json!(all_todos));

    let message = if has_fixmes {
        format!(
            "Critical TODOs found: {} total (FIXME/HACK: {}, TODO: {}, NOTE: {})",
            total_todos,
            todos_by_category.get("FIXME").map_or(0, |v| v.len())
                + todos_by_category.get("HACK").map_or(0, |v| v.len()),
            todos_by_category.get("TODO").map_or(0, |v| v.len())
                + todos_by_category.get("XXX").map_or(0, |v| v.len()),
            todos_by_category.get("NOTE").map_or(0, |v| v.len())
        )
    } else if many_todos {
        format!(
            "Many TODOs found: {} total across {} categories",
            total_todos,
            todos_by_category.len()
        )
    } else {
        format!(
            "TODO tracking: {} items found across {} categories",
            total_todos,
            todos_by_category.len()
        )
    };

    // Build suggestions
    let mut suggestions = Vec::new();
    if has_fixmes {
        let fixme_count = todos_by_category.get("FIXME").map_or(0, |v| v.len())
            + todos_by_category.get("HACK").map_or(0, |v| v.len());
        suggestions.push(Suggestion {
            action: "address_fixmes".to_string(),
            description: format!(
                "Address {} critical FIXME/HACK items that indicate broken or hacky code",
                fixme_count
            ),
            target: None,
            estimated_impact: "Fixes technical debt and improves code quality".to_string(),
            safety: SafetyLevel::RequiresReview,
            confidence: 0.90,
            reversible: false,
            refactor_call: None,
        });
    }

    if many_todos {
        suggestions.push(Suggestion {
            action: "create_issues".to_string(),
            description: format!(
                "Create issue tracker tickets for {} TODOs to ensure they're not forgotten",
                total_todos
            ),
            target: None,
            estimated_impact: "Improves project management and accountability".to_string(),
            safety: SafetyLevel::Safe,
            confidence: 0.85,
            reversible: true,
            refactor_call: None,
        });
    }

    let mut finding = Finding {
        id: format!("doc-todos-{}", file_path),
        kind: "todos".to_string(),
        severity: overall_severity,
        location: FindingLocation {
            file_path: file_path.to_string(),
            range: None,
            symbol: None,
            symbol_kind: Some("module".to_string()),
        },
        metrics: Some(metrics),
        message,
        suggestions,
    };

    let suggestion_generator = SuggestionGenerator::new();
    let context = AnalysisContext {
        file_path: file_path.to_string(),
        has_full_type_info: false,
        has_partial_type_info: false,
        ast_parse_errors: 0,
    };

    if let Ok(candidates) = generate_documentation_refactoring_candidates(&finding) {
        let suggestions = suggestion_generator.generate_multiple(candidates, &context);
        finding.suggestions = suggestions
            .into_iter()
            .map(|s| s.into())
            .collect::<Vec<Suggestion>>();
    }

    findings.push(finding);

    findings
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Helper struct for TODO items
#[derive(Debug, Clone)]
struct TodoItem {
    pattern: String,
    text: String,
    line: usize,
    severity: Severity,
}

/// Check if a symbol is public
///
/// # Parameters
/// - `symbol`: The symbol to check
/// - `lines`: The file content lines
/// - `language`: The language for visibility patterns
///
/// # Returns
/// True if the symbol appears to be public
fn is_symbol_public(symbol: &Symbol, lines: &[&str], language: &str) -> bool {
    let line_idx = symbol.location.line.saturating_sub(1);
    if line_idx >= lines.len() {
        return false;
    }

    let line = lines[line_idx];

    match language.to_lowercase().as_str() {
        "rust" => line.contains("pub ") || line.contains("pub("),
        "typescript" | "javascript" => line.contains("export "),
        "python" => !symbol.name.starts_with('_'),
        "go" => {
            // In Go, uppercase first letter means public
            symbol.name.chars().next().is_some_and(|c| c.is_uppercase())
        }
        _ => true, // Default to public if unsure
    }
}

/// Check if a symbol has doc comment
///
/// # Parameters
/// - `symbol`: The symbol to check
/// - `lines`: The file content lines
/// - `language`: The language for doc patterns
///
/// # Returns
/// True if the symbol has a doc comment
fn has_doc_comment(symbol: &Symbol, lines: &[&str], language: &str) -> bool {
    let line_idx = symbol.location.line.saturating_sub(1);
    if line_idx == 0 || line_idx >= lines.len() {
        return false;
    }

    // Check lines above the symbol for doc comments
    let check_lines = 5.min(line_idx);
    let start_idx = line_idx.saturating_sub(check_lines);

    let patterns = get_doc_comment_patterns(language);

    #[allow(clippy::needless_range_loop)]
    for i in start_idx..line_idx {
        let line = lines[i].trim();
        for pattern in &patterns {
            if line.starts_with(pattern) {
                return true;
            }
        }
    }

    false
}

/// Extract doc comment for a symbol
///
/// # Parameters
/// - `symbol`: The symbol to extract doc for
/// - `lines`: The file content lines
/// - `language`: The language for doc patterns
///
/// # Returns
/// The extracted doc comment text
fn extract_doc_comment(symbol: &Symbol, lines: &[&str], language: &str) -> String {
    let line_idx = symbol.location.line.saturating_sub(1);
    if line_idx == 0 || line_idx >= lines.len() {
        return String::new();
    }

    let check_lines = 20.min(line_idx);
    let start_idx = line_idx.saturating_sub(check_lines);

    let patterns = get_doc_comment_patterns(language);
    let mut doc_lines = Vec::new();
    let mut in_doc_block = false;

    #[allow(clippy::needless_range_loop)]
    for i in start_idx..line_idx {
        let line = lines[i].trim();

        // Check for doc comment start
        for pattern in &patterns {
            if line.starts_with(pattern) {
                in_doc_block = true;
                // Remove comment markers
                let cleaned = line
                    .trim_start_matches(pattern)
                    .trim_start_matches('*')
                    .trim();
                if !cleaned.is_empty() {
                    doc_lines.push(cleaned.to_string());
                }
                break;
            }
        }

        // Check for block end
        if in_doc_block
            && (line.is_empty()
                || (!line.starts_with("///")
                    && !line.starts_with("//!")
                    && !line.starts_with("*")
                    && !line.starts_with("#")))
        {
            break;
        }
    }

    doc_lines.join(" ")
}

/// Get language-specific doc comment patterns
///
/// # Parameters
/// - `language`: The language name
///
/// # Returns
/// A vector of doc comment prefix patterns
fn get_doc_comment_patterns(language: &str) -> Vec<&'static str> {
    match language.to_lowercase().as_str() {
        "rust" => vec!["///", "//!", "/**"],
        "typescript" | "javascript" => vec!["/**", "/*"],
        "python" => vec!["\"\"\"", "'''"],
        "go" => vec!["//"],
        _ => vec!["//", "/**"],
    }
}

/// Check if doc comment is trivial
///
/// # Parameters
/// - `doc`: The doc comment text
/// - `symbol_name`: The symbol name
///
/// # Returns
/// True if the doc is trivial (just repeats symbol name or type)
fn is_trivial_doc(doc: &str, symbol_name: &str) -> bool {
    let doc_lower = doc.to_lowercase();
    let name_lower = symbol_name.to_lowercase();

    // Trivial if doc is just the symbol name
    if doc_lower.contains(&name_lower) && doc.len() < symbol_name.len() + 15 {
        return true;
    }

    // Trivial if doc is just a type name
    let trivial_patterns = ["function", "method", "class", "struct", "type", "interface"];
    for pattern in &trivial_patterns {
        if doc_lower.starts_with(pattern) && doc.len() < 20 {
            return true;
        }
    }

    false
}

/// Check if doc has parameter documentation
///
/// # Parameters
/// - `doc`: The doc comment text
/// - `language`: The language for doc tag patterns
///
/// # Returns
/// True if parameter documentation found
fn has_parameter_docs(doc: &str, language: &str) -> bool {
    let patterns = match language.to_lowercase().as_str() {
        "rust" => vec!["# Arguments", "# Parameters"],
        "typescript" | "javascript" => vec!["@param", "@parameter"],
        "python" => vec![":param", "Args:"],
        "go" => vec!["// ", "//"],
        _ => vec!["@param", "param:"],
    };

    patterns.iter().any(|p| doc.contains(p))
}

/// Check if doc has return documentation
///
/// # Parameters
/// - `doc`: The doc comment text
/// - `language`: The language for doc tag patterns
///
/// # Returns
/// True if return documentation found
fn has_return_docs(doc: &str, language: &str) -> bool {
    let patterns = match language.to_lowercase().as_str() {
        "rust" => vec!["# Returns"],
        "typescript" | "javascript" => vec!["@returns", "@return"],
        "python" => vec![":return", "Returns:"],
        "go" => vec!["returns", "Returns"],
        _ => vec!["@returns", "returns:"],
    };

    patterns.iter().any(|p| doc.contains(p))
}

/// Check if doc has code examples
///
/// # Parameters
/// - `doc`: The doc comment text
/// - `language`: The language for code block syntax
///
/// # Returns
/// True if code examples found
fn has_code_examples(doc: &str, _language: &str) -> bool {
    // Check for fenced code blocks
    if doc.contains("```") {
        return true;
    }

    // Check for indented code blocks (4 spaces)
    for line in doc.lines() {
        if line.starts_with("    ") || line.starts_with("\t") {
            return true;
        }
    }

    // Check for common example markers
    let example_markers = ["# Example", "Example:", "@example", "## Example"];
    example_markers.iter().any(|m| doc.contains(m))
}

/// Build quality suggestions based on issues
///
/// # Parameters
/// - `issues`: The list of quality issue types
///
/// # Returns
/// A vector of suggestions
fn build_quality_suggestions(issues: &[&str]) -> Vec<Suggestion> {
    let mut suggestions = Vec::new();

    if issues.contains(&"trivial_description") {
        suggestions.push(Suggestion {
            action: "improve_description".to_string(),
            description: "Add a meaningful description explaining the purpose and behavior"
                .to_string(),
            target: None,
            estimated_impact: "Improves understanding for maintainers".to_string(),
            safety: SafetyLevel::Safe,
            confidence: 0.95,
            reversible: true,
            refactor_call: None,
        });
    }

    if issues.contains(&"missing_param_docs") {
        suggestions.push(Suggestion {
            action: "add_parameter_docs".to_string(),
            description: "Document all function parameters with their types and purposes"
                .to_string(),
            target: None,
            estimated_impact: "Clarifies API usage and expected inputs".to_string(),
            safety: SafetyLevel::Safe,
            confidence: 0.90,
            reversible: true,
            refactor_call: None,
        });
    }

    if issues.contains(&"missing_return_docs") {
        suggestions.push(Suggestion {
            action: "add_return_docs".to_string(),
            description: "Document return value type and meaning".to_string(),
            target: None,
            estimated_impact: "Clarifies function output and behavior".to_string(),
            safety: SafetyLevel::Safe,
            confidence: 0.90,
            reversible: true,
            refactor_call: None,
        });
    }

    if issues.contains(&"missing_examples") {
        suggestions.push(Suggestion {
            action: "add_code_example".to_string(),
            description: "Add code example demonstrating usage for this complex function"
                .to_string(),
            target: None,
            estimated_impact: "Significantly improves understanding for API users".to_string(),
            safety: SafetyLevel::Safe,
            confidence: 0.85,
            reversible: true,
            refactor_call: None,
        });
    }

    suggestions
}

/// Extract all doc comments from content
///
/// # Parameters
/// - `content`: The file content
/// - `language`: The language for doc patterns
///
/// # Returns
/// A vector of (line_number, comment_text) tuples
fn extract_all_doc_comments(content: &str, language: &str) -> Vec<(usize, String)> {
    let mut comments = Vec::new();
    let patterns = get_doc_comment_patterns(language);

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        for pattern in &patterns {
            if trimmed.starts_with(pattern) {
                let comment_text = trimmed
                    .trim_start_matches(pattern)
                    .trim_start_matches('*')
                    .trim();
                if !comment_text.is_empty() {
                    comments.push((line_num + 1, comment_text.to_string()));
                }
                break;
            }
        }
    }

    comments
}

/// Detect comment styles used in doc comments from original content
///
/// # Parameters
/// - `content`: The original source code content
/// - `language`: The language for style detection
///
/// # Returns
/// A vector of distinct comment styles found
fn detect_comment_styles_from_content(content: &str, language: &str) -> Vec<String> {
    let mut styles = std::collections::HashSet::new();

    match language.to_lowercase().as_str() {
        "rust" => {
            // Check for ///,  //!, /** */ styles
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("///") {
                    styles.insert("///".to_string());
                } else if trimmed.starts_with("//!") {
                    styles.insert("//!".to_string());
                } else if trimmed.starts_with("/**")
                    || trimmed.starts_with("*") && line.contains("/**")
                {
                    styles.insert("/**".to_string());
                }
            }
        }
        "python" => {
            // Check for """ vs ''' styles
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("\"\"\"") || trimmed.contains("\"\"\"") {
                    styles.insert("\"\"\"".to_string());
                } else if trimmed.starts_with("'''") || trimmed.contains("'''") {
                    styles.insert("'''".to_string());
                }
            }
        }
        "typescript" | "javascript" => {
            // Check for /** */ vs /// styles
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("/**")
                    || (trimmed.starts_with("*") && content.contains("/**"))
                {
                    styles.insert("/**".to_string());
                } else if trimmed.starts_with("///") {
                    styles.insert("///".to_string());
                }
            }
        }
        _ => {
            styles.insert("mixed".to_string());
        }
    }

    styles.into_iter().collect()
}

/// Detect comment styles used in doc comments (DEPRECATED - use detect_comment_styles_from_content)
///
/// # Parameters
/// - `doc_comments`: The extracted doc comments
/// - `language`: The language for style detection
///
/// # Returns
/// A vector of distinct comment styles found
#[allow(dead_code)]
fn detect_comment_styles(doc_comments: &[(usize, String)], language: &str) -> Vec<String> {
    let mut styles = std::collections::HashSet::new();

    match language.to_lowercase().as_str() {
        "rust" => {
            // Check for ///,  //!, /** */ styles
            for (_, comment) in doc_comments {
                if comment.starts_with("///") {
                    styles.insert("///".to_string());
                } else if comment.starts_with("//!") {
                    styles.insert("//!".to_string());
                } else if comment.starts_with("/**") {
                    styles.insert("/**".to_string());
                }
            }
        }
        "python" => {
            // Check for """ vs ''' styles
            for (_, comment) in doc_comments {
                if comment.starts_with("\"\"\"") {
                    styles.insert("\"\"\"".to_string());
                } else if comment.starts_with("'''") {
                    styles.insert("'''".to_string());
                }
            }
        }
        "typescript" | "javascript" => {
            // Check for /** */ vs // styles
            for (_, comment) in doc_comments {
                if comment.starts_with("/**") {
                    styles.insert("/**".to_string());
                } else if comment.starts_with("//") {
                    styles.insert("//".to_string());
                }
            }
        }
        _ => {
            styles.insert("mixed".to_string());
        }
    }

    styles.into_iter().collect()
}

/// Get comment pattern regex for language
///
/// # Parameters
/// - `language`: The language name
///
/// # Returns
/// A regex pattern for comment detection
fn get_comment_pattern(language: &str) -> String {
    match language.to_lowercase().as_str() {
        "rust" => r"^\s*(///|//!|/\*\*|//)".to_string(),
        "typescript" | "javascript" => r"^\s*(/\*\*|/\*|//)".to_string(),
        "python" => r#"^\s*(#|"""|\'\'\'))"#.to_string(),
        "go" => r"^\s*(//)".to_string(),
        _ => r"^\s*(//)".to_string(),
    }
}

/// Check if line is a comment
///
/// # Parameters
/// - `line`: The line to check
/// - `pattern`: The comment pattern regex
///
/// # Returns
/// True if the line is a comment
fn is_comment_line(line: &str, pattern: &str) -> bool {
    if let Ok(re) = Regex::new(pattern) {
        re.is_match(line)
    } else {
        false
    }
}

/// Extract TODO text from a line
///
/// # Parameters
/// - `line`: The line containing the TODO
/// - `pattern`: The TODO pattern (TODO, FIXME, etc.)
///
/// # Returns
/// The extracted TODO text if found
fn extract_todo_text(line: &str, pattern: &str) -> Option<String> {
    if let Some(pos) = line.to_uppercase().find(pattern) {
        let after_pattern = &line[pos + pattern.len()..];
        let text = after_pattern
            .trim_start_matches(':')
            .trim_start_matches('(')
            .trim_start_matches('[')
            .trim();
        Some(text.to_string())
    } else {
        None
    }
}

// ============================================================================
// Handler Implementation
// ============================================================================

pub struct DocumentationHandler;

impl DocumentationHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ToolHandler for DocumentationHandler {
    fn tool_names(&self) -> &[&str] {
        &["analyze.documentation"]
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
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'kind' parameter".into()))?;

        // Validate kind
        if !matches!(
            kind,
            "coverage" | "quality" | "style" | "examples" | "todos"
        ) {
            return Err(ServerError::InvalidRequest(format!(
                "Unsupported kind '{}'. Supported: 'coverage', 'quality', 'style', 'examples', 'todos'",
                kind
            )));
        }

        debug!(kind = %kind, "Handling analyze.documentation request");

        // Dispatch to appropriate analysis function
        match kind {
            "coverage" => {
                super::engine::run_analysis(
                    context,
                    tool_call,
                    "documentation",
                    kind,
                    detect_coverage,
                )
                .await
            }
            "quality" => {
                super::engine::run_analysis(
                    context,
                    tool_call,
                    "documentation",
                    kind,
                    detect_quality,
                )
                .await
            }
            "style" => {
                super::engine::run_analysis(context, tool_call, "documentation", kind, detect_style)
                    .await
            }
            "examples" => {
                super::engine::run_analysis(
                    context,
                    tool_call,
                    "documentation",
                    kind,
                    detect_examples,
                )
                .await
            }
            "todos" => {
                super::engine::run_analysis(context, tool_call, "documentation", kind, detect_todos)
                    .await
            }
            _ => unreachable!("Kind validated earlier"),
        }
    }
}
