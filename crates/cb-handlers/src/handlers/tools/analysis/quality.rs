use super::super::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::analysis_result::{
    AnalysisResult, AnalysisScope, Finding, FindingLocation, Position, Range, RefactorCall,
    SafetyLevel, Severity, Suggestion,
};
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
use regex::Regex;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, info};

#[derive(Deserialize, Debug)]
struct QualityOptions {
    #[serde(default)]
    thresholds: Option<QualityThresholds>,
    #[serde(default)]
    severity_filter: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    offset: usize,
    #[serde(default = "default_format")]
    format: String,
    #[serde(default = "default_include_suggestions")]
    include_suggestions: bool,
}

fn default_limit() -> usize {
    1000
}

fn default_format() -> String {
    "detailed".to_string()
}

fn default_include_suggestions() -> bool {
    true
}

#[derive(Deserialize, Debug)]
struct QualityThresholds {
    #[serde(default = "default_cyclomatic")]
    cyclomatic_complexity: u32,
    #[serde(default = "default_cognitive")]
    cognitive_complexity: u32,
    #[serde(default = "default_nesting")]
    nesting_depth: u32,
    #[serde(default = "default_params")]
    parameter_count: u32,
    #[serde(default = "default_function_length")]
    function_length: u32,
}

fn default_cyclomatic() -> u32 {
    15
}
fn default_cognitive() -> u32 {
    10
}
fn default_nesting() -> u32 {
    4
}
fn default_params() -> u32 {
    5
}
fn default_function_length() -> u32 {
    50
}

impl Default for QualityThresholds {
    fn default() -> Self {
        Self {
            cyclomatic_complexity: default_cyclomatic(),
            cognitive_complexity: default_cognitive(),
            nesting_depth: default_nesting(),
            parameter_count: default_params(),
            function_length: default_function_length(),
        }
    }
}

#[derive(Deserialize, Debug)]
struct QualityScopeParam {
    #[serde(rename = "type")]
    scope_type: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
}

pub struct QualityHandler;

impl QualityHandler {
    pub fn new() -> Self {
        Self
    }

    /// Transform ComplexityReport into AnalysisResult
    fn transform_complexity_report(
        &self,
        report: cb_ast::complexity::ComplexityReport,
        thresholds: &QualityThresholds,
        include_suggestions: bool,
        scope: AnalysisScope,
        analysis_time_ms: u64,
    ) -> AnalysisResult {
        let mut result = AnalysisResult::new("quality", "complexity", scope);

        // Set language if available
        result.metadata.language = Some("unknown".to_string());

        // Add thresholds to metadata
        let mut threshold_map = HashMap::new();
        threshold_map.insert(
            "cyclomatic_complexity".to_string(),
            json!(thresholds.cyclomatic_complexity),
        );
        threshold_map.insert(
            "cognitive_complexity".to_string(),
            json!(thresholds.cognitive_complexity),
        );
        threshold_map.insert(
            "nesting_depth".to_string(),
            json!(thresholds.nesting_depth),
        );
        threshold_map.insert(
            "parameter_count".to_string(),
            json!(thresholds.parameter_count),
        );
        threshold_map.insert(
            "function_length".to_string(),
            json!(thresholds.function_length),
        );
        result.metadata.thresholds = Some(threshold_map);

        // Transform each function into a finding
        for func in &report.functions {
            // Only include functions that exceed thresholds
            if func.complexity.cognitive < thresholds.cognitive_complexity
                && func.complexity.cyclomatic < thresholds.cyclomatic_complexity
            {
                continue;
            }

            // Determine severity based on rating
            let severity = match func.rating {
                cb_ast::complexity::ComplexityRating::VeryComplex => Severity::High,
                cb_ast::complexity::ComplexityRating::Complex => Severity::Medium,
                _ => Severity::Low,
            };

            // Build metrics
            let mut metrics = HashMap::new();
            metrics.insert(
                "cyclomatic_complexity".to_string(),
                json!(func.complexity.cyclomatic),
            );
            metrics.insert(
                "cognitive_complexity".to_string(),
                json!(func.complexity.cognitive),
            );
            metrics.insert("nesting_depth".to_string(), json!(func.complexity.max_nesting_depth));
            metrics.insert(
                "parameter_count".to_string(),
                json!(func.metrics.parameters),
            );
            metrics.insert("line_count".to_string(), json!(func.metrics.sloc));

            // Build message
            let message = format!(
                "Function '{}' has high complexity (cyclomatic: {}, cognitive: {}, rating: {})",
                func.name,
                func.complexity.cyclomatic,
                func.complexity.cognitive,
                func.rating.description()
            );

            // Build location
            let location = FindingLocation {
                file_path: report.file_path.clone(),
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
            };

            // Build suggestions if requested
            let mut suggestions = Vec::new();
            if include_suggestions {
                // Suggest extract function for high complexity
                if func.complexity.cognitive > thresholds.cognitive_complexity {
                    suggestions.push(Suggestion {
                        action: "extract_function".to_string(),
                        description: "Extract nested blocks to separate functions to reduce cognitive complexity".to_string(),
                        target: None,
                        estimated_impact: format!(
                            "Could reduce complexity from {} to ~{}",
                            func.complexity.cognitive,
                            func.complexity.cognitive * 2 / 3
                        ),
                        safety: SafetyLevel::RequiresReview,
                        confidence: 0.75,
                        reversible: true,
                        refactor_call: Some(cb_protocol::analysis_result::RefactorCall {
                            command: "extract.plan".to_string(),
                            arguments: json!({
                                "kind": "function",
                                "source": {
                                    "file_path": report.file_path,
                                    "range": {
                                        "start": { "line": func.line, "character": 0 },
                                        "end": { "line": func.line + func.metrics.sloc as usize, "character": 0 }
                                    }
                                }
                            }),
                        }),
                    });
                }

                // Suggest reduce nesting
                if func.complexity.max_nesting_depth > thresholds.nesting_depth {
                    suggestions.push(Suggestion {
                        action: "reduce_nesting".to_string(),
                        description: "Use early returns or guard clauses to reduce nesting depth".to_string(),
                        target: None,
                        estimated_impact: "Improves readability significantly".to_string(),
                        safety: SafetyLevel::RequiresReview,
                        confidence: 0.80,
                        reversible: true,
                        refactor_call: None,
                    });
                }

                // Suggest consolidate parameters
                if func.metrics.parameters > thresholds.parameter_count {
                    suggestions.push(Suggestion {
                        action: "consolidate_parameters".to_string(),
                        description: "Group related parameters into a configuration struct/object".to_string(),
                        target: None,
                        estimated_impact: "Reduces parameter count, improves maintainability".to_string(),
                        safety: SafetyLevel::RequiresReview,
                        confidence: 0.70,
                        reversible: false,
                        refactor_call: None,
                    });
                }
            }

            // Create finding
            let finding = Finding {
                id: format!("complexity-{}-{}", report.file_path, func.line),
                kind: "complexity_hotspot".to_string(),
                severity,
                location,
                metrics: Some(metrics),
                message,
                suggestions,
            };

            result.add_finding(finding);
        }

        // Update summary
        result.summary.files_analyzed = 1;
        result.summary.symbols_analyzed = Some(report.total_functions);
        result.finalize(analysis_time_ms);

        result
    }
}

/// Detect code smells in a file
fn detect_smells(
    complexity_report: &cb_ast::complexity::ComplexityReport,
    content: &str,
    _symbols: &[cb_plugin_api::Symbol],
    language: &str,
    file_path: &str,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // 1. Long methods (from functions in complexity_report)
    for func in &complexity_report.functions {
        if func.metrics.sloc > 50 {
            let severity = if func.metrics.sloc > 100 {
                Severity::High
            } else {
                Severity::Medium
            };

            let mut metrics = HashMap::new();
            metrics.insert("sloc".to_string(), json!(func.metrics.sloc));

            findings.push(Finding {
                id: format!("long-method-{}-{}", file_path, func.line),
                kind: "long_method".to_string(),
                severity,
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
                message: format!(
                    "Function '{}' is too long ({} SLOC, >50 recommended)",
                    func.name, func.metrics.sloc
                ),
                suggestions: vec![Suggestion {
                    action: "extract_function".to_string(),
                    description: "Break this function into smaller, focused functions".to_string(),
                    target: None,
                    estimated_impact: format!(
                        "Reduces function length from {} to ~{} SLOC",
                        func.metrics.sloc,
                        func.metrics.sloc / 2
                    ),
                    safety: SafetyLevel::RequiresReview,
                    confidence: 0.70,
                    reversible: true,
                    refactor_call: Some(RefactorCall {
                        command: "extract.plan".to_string(),
                        arguments: json!({
                            "kind": "function",
                            "source": {
                                "file_path": file_path,
                                "range": {
                                    "start": { "line": func.line, "character": 0 },
                                    "end": { "line": func.line + func.metrics.sloc as usize, "character": 0 }
                                }
                            }
                        }),
                    }),
                }],
            });
        }
    }

    // 2. God classes (>20 methods)
    let classes = cb_ast::complexity::aggregate_class_complexity(
        file_path,
        &complexity_report.functions,
        language,
    );

    for class in classes {
        if class.function_count > 20 {
            let mut metrics = HashMap::new();
            metrics.insert("method_count".to_string(), json!(class.function_count));
            metrics.insert("total_sloc".to_string(), json!(class.total_sloc));
            metrics.insert("avg_complexity".to_string(), json!(class.average_complexity));

            findings.push(Finding {
                id: format!("god-class-{}-{}", file_path, class.line),
                kind: "god_class".to_string(),
                severity: Severity::Medium,
                location: FindingLocation {
                    file_path: file_path.to_string(),
                    range: Some(Range {
                        start: Position {
                            line: class.line as u32,
                            character: 0,
                        },
                        end: Position {
                            line: class.line as u32,
                            character: 0,
                        },
                    }),
                    symbol: Some(class.name.clone()),
                    symbol_kind: Some("class".to_string()),
                },
                metrics: Some(metrics),
                message: format!(
                    "Class/module '{}' has too many methods ({} methods, >20 recommended)",
                    class.name, class.function_count
                ),
                suggestions: vec![Suggestion {
                    action: "split_class".to_string(),
                    description: "Consider splitting this class into smaller, focused classes with single responsibilities".to_string(),
                    target: None,
                    estimated_impact: "Improves maintainability and testability significantly"
                        .to_string(),
                    safety: SafetyLevel::RequiresReview,
                    confidence: 0.65,
                    reversible: false,
                    refactor_call: None,
                }],
            });
        }
    }

    // 3. Magic numbers (copy logic from code.rs:260-302)
    let magic_number_findings = detect_magic_numbers_for_smells(content, file_path, language);
    findings.extend(magic_number_findings);

    // 4. Duplicate code patterns
    // TODO: Implement duplicate code detection using token-based similarity analysis
    // Requires: AST token extraction, sliding window comparison, similarity threshold tuning
    // Estimated effort: ~1 week (non-trivial algorithm)
    // Priority: Medium (valuable but complex)
    // Approach: Consider using tree-sitter or similar for token extraction

    findings
}

/// Helper for magic number detection (adapted from code.rs)
///
/// TODO: Future enhancement - Use AST-level context awareness to filter numbers
/// in string literals and improve detection accuracy. Current implementation uses
/// line-level filtering which is effective for MVP but could be refined using
/// language plugin's Symbol data to distinguish literal vs code contexts.
/// Estimated effort: ~1-2 days. Priority: Low (current approach is effective).
fn detect_magic_numbers_for_smells(
    content: &str,
    file_path: &str,
    language: &str,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    let number_pattern = match language.to_lowercase().as_str() {
        "rust" | "go" | "java" | "typescript" | "javascript" | "python" => {
            Regex::new(r"\b(?:[2-9]|[1-9]\d+)(?:\.\d+)?\b").ok()
        }
        _ => None,
    };

    if let Some(pattern) = number_pattern {
        let mut found_numbers = std::collections::HashMap::new();
        for (i, line) in content.lines().enumerate() {
            // Skip comment lines (basic context filtering for MVP)
            if line.trim().starts_with("//") || line.trim().starts_with('#') {
                continue;
            }
            // TODO: Also filter string literal contexts - requires AST awareness
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
                let severity = if lines.len() > 3 {
                    Severity::Medium
                } else {
                    Severity::Low
                };

                let mut metrics = HashMap::new();
                metrics.insert("number".to_string(), json!(number));
                metrics.insert("occurrences".to_string(), json!(lines.len()));

                findings.push(Finding {
                    id: format!("magic-number-{}-{}", file_path, lines[0]),
                    kind: "magic_number".to_string(),
                    severity,
                    location: FindingLocation {
                        file_path: file_path.to_string(),
                        range: Some(Range {
                            start: Position {
                                line: lines[0] as u32,
                                character: 0,
                            },
                            end: Position {
                                line: lines[0] as u32,
                                character: 0,
                            },
                        }),
                        symbol: None,
                        symbol_kind: None,
                    },
                    metrics: Some(metrics),
                    message: format!("Magic number '{}' appears {} times", number, lines.len()),
                    suggestions: vec![Suggestion {
                        action: "extract_constant".to_string(),
                        description: format!(
                            "Extract '{}' to a named constant for better maintainability",
                            number
                        ),
                        target: None,
                        estimated_impact: "Improves code readability and maintainability"
                            .to_string(),
                        safety: SafetyLevel::Safe,
                        confidence: 0.90,
                        reversible: true,
                        refactor_call: None,
                    }],
                });
            }
        }
    }

    findings
}

/// Analyze readability issues in functions
fn analyze_readability(
    complexity_report: &cb_ast::complexity::ComplexityReport,
    file_path: &str,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    for func in &complexity_report.functions {
        // 1. Deep nesting (>4 levels)
        if func.complexity.max_nesting_depth > 4 {
            let mut metrics = HashMap::new();
            metrics.insert(
                "nesting_depth".to_string(),
                json!(func.complexity.max_nesting_depth),
            );

            findings.push(Finding {
                id: format!("deep-nesting-{}-{}", file_path, func.line),
                kind: "deep_nesting".to_string(),
                severity: if func.complexity.max_nesting_depth > 6 {
                    Severity::High
                } else {
                    Severity::Medium
                },
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
                message: format!(
                    "Function '{}' has deep nesting ({} levels, >4 recommended)",
                    func.name, func.complexity.max_nesting_depth
                ),
                suggestions: vec![Suggestion {
                    action: "reduce_nesting".to_string(),
                    description: "Use early returns or guard clauses to reduce nesting depth"
                        .to_string(),
                    target: None,
                    estimated_impact: "Significantly improves readability".to_string(),
                    safety: SafetyLevel::RequiresReview,
                    confidence: 0.85,
                    reversible: true,
                    refactor_call: None,
                }],
            });
        }

        // 2. Too many parameters (>5)
        if func.metrics.parameters > 5 {
            let mut metrics = HashMap::new();
            metrics.insert("parameter_count".to_string(), json!(func.metrics.parameters));

            findings.push(Finding {
                id: format!("too-many-params-{}-{}", file_path, func.line),
                kind: "too_many_parameters".to_string(),
                severity: if func.metrics.parameters > 7 {
                    Severity::High
                } else {
                    Severity::Medium
                },
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
                message: format!(
                    "Function '{}' has too many parameters ({} params, >5 recommended)",
                    func.name, func.metrics.parameters
                ),
                suggestions: vec![Suggestion {
                    action: "consolidate_parameters".to_string(),
                    description:
                        "Group related parameters into a configuration struct or object".to_string(),
                    target: None,
                    estimated_impact: "Improves function signature readability and maintainability"
                        .to_string(),
                    safety: SafetyLevel::RequiresReview,
                    confidence: 0.75,
                    reversible: false,
                    refactor_call: None,
                }],
            });
        }

        // 3. Long functions (>50 SLOC) - readability perspective
        if func.metrics.sloc > 50 {
            let mut metrics = HashMap::new();
            metrics.insert("sloc".to_string(), json!(func.metrics.sloc));

            findings.push(Finding {
                id: format!("long-function-{}-{}", file_path, func.line),
                kind: "long_function".to_string(),
                severity: if func.metrics.sloc > 100 {
                    Severity::High
                } else {
                    Severity::Medium
                },
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
                message: format!(
                    "Function '{}' is difficult to read due to length ({} SLOC, >50 recommended)",
                    func.name, func.metrics.sloc
                ),
                suggestions: vec![Suggestion {
                    action: "split_function".to_string(),
                    description: "Split into smaller functions with clear responsibilities".to_string(),
                    target: None,
                    estimated_impact: format!(
                        "Reduces cognitive load from {} to ~{} SLOC per function",
                        func.metrics.sloc,
                        func.metrics.sloc / 2
                    ),
                    safety: SafetyLevel::RequiresReview,
                    confidence: 0.70,
                    reversible: true,
                    refactor_call: Some(RefactorCall {
                        command: "extract.plan".to_string(),
                        arguments: json!({
                            "kind": "function",
                            "source": {
                                "file_path": file_path,
                                "range": {
                                    "start": { "line": func.line, "character": 0 },
                                    "end": { "line": func.line + func.metrics.sloc as usize, "character": 0 }
                                }
                            }
                        }),
                    }),
                }],
            });
        }

        // 4. Low comment ratio (<0.1 for functions >20 SLOC)
        if func.metrics.comment_ratio < 0.1 && func.metrics.sloc > 20 {
            let mut metrics = HashMap::new();
            metrics.insert("comment_ratio".to_string(), json!(func.metrics.comment_ratio));
            metrics.insert("sloc".to_string(), json!(func.metrics.sloc));

            findings.push(Finding {
                id: format!("low-comments-{}-{}", file_path, func.line),
                kind: "low_comment_ratio".to_string(),
                severity: Severity::Low,
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
                message: format!(
                    "Function '{}' has insufficient comments ({:.1}% comment ratio for {} SLOC)",
                    func.name,
                    func.metrics.comment_ratio * 100.0,
                    func.metrics.sloc
                ),
                suggestions: vec![Suggestion {
                    action: "add_documentation".to_string(),
                    description:
                        "Add inline comments or documentation to explain complex logic".to_string(),
                    target: None,
                    estimated_impact: "Improves code understanding for future maintainers"
                        .to_string(),
                    safety: SafetyLevel::Safe,
                    confidence: 0.95,
                    reversible: true,
                    refactor_call: None,
                }],
            });
        }
    }

    findings
}

/// Analyze overall maintainability metrics for a file or workspace
fn analyze_maintainability(
    complexity_report: &cb_ast::complexity::ComplexityReport,
    file_path: &str,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Calculate distribution by rating
    let mut rating_counts: HashMap<String, usize> = HashMap::new();
    for func in &complexity_report.functions {
        let rating_str = match func.rating {
            cb_ast::complexity::ComplexityRating::Simple => "simple",
            cb_ast::complexity::ComplexityRating::Moderate => "moderate",
            cb_ast::complexity::ComplexityRating::Complex => "complex",
            cb_ast::complexity::ComplexityRating::VeryComplex => "very_complex",
        };
        *rating_counts.entry(rating_str.to_string()).or_insert(0) += 1;
    }

    let simple = *rating_counts.get("simple").unwrap_or(&0);
    let moderate = *rating_counts.get("moderate").unwrap_or(&0);
    let complex = *rating_counts.get("complex").unwrap_or(&0);
    let very_complex = *rating_counts.get("very_complex").unwrap_or(&0);
    let needs_attention = complex + very_complex;

    // Determine overall severity
    let total_functions = complexity_report.total_functions;
    let attention_ratio = if total_functions > 0 {
        needs_attention as f64 / total_functions as f64
    } else {
        0.0
    };

    let severity = if attention_ratio > 0.3 {
        Severity::High
    } else if attention_ratio > 0.1 {
        Severity::Medium
    } else {
        Severity::Low
    };

    // Build comprehensive metrics
    let mut metrics = HashMap::new();
    metrics.insert(
        "avg_cyclomatic".to_string(),
        json!(complexity_report.average_complexity),
    );
    metrics.insert(
        "avg_cognitive".to_string(),
        json!(complexity_report.average_cognitive_complexity),
    );
    metrics.insert(
        "max_cyclomatic".to_string(),
        json!(complexity_report.max_complexity),
    );
    metrics.insert(
        "max_cognitive".to_string(),
        json!(complexity_report.max_cognitive_complexity),
    );
    metrics.insert("avg_sloc".to_string(), json!(complexity_report.average_sloc));
    metrics.insert("total_sloc".to_string(), json!(complexity_report.total_sloc));
    metrics.insert(
        "total_functions".to_string(),
        json!(complexity_report.total_functions),
    );
    metrics.insert("needs_attention".to_string(), json!(needs_attention));
    metrics.insert("attention_ratio".to_string(), json!(attention_ratio));
    metrics.insert("simple".to_string(), json!(simple));
    metrics.insert("moderate".to_string(), json!(moderate));
    metrics.insert("complex".to_string(), json!(complex));
    metrics.insert("very_complex".to_string(), json!(very_complex));

    // Generate message
    let message = if total_functions == 0 {
        "No functions found to analyze".to_string()
    } else if needs_attention == 0 {
        format!(
            "Excellent maintainability: {} functions analyzed, all have acceptable complexity",
            total_functions
        )
    } else {
        format!(
            "Maintainability needs attention: {} of {} functions ({:.1}%) require refactoring",
            needs_attention,
            total_functions,
            attention_ratio * 100.0
        )
    };

    // Build suggestions for top issues
    let mut suggestions = Vec::new();

    if needs_attention > 0 {
        // Get top 3 most complex functions
        let mut complex_funcs: Vec<_> = complexity_report.functions.iter().collect();
        complex_funcs.sort_by(|a, b| {
            b.complexity
                .cognitive
                .cmp(&a.complexity.cognitive)
                .then_with(|| b.complexity.cyclomatic.cmp(&a.complexity.cyclomatic))
        });

        let top_issues = complex_funcs
            .iter()
            .take(3)
            .filter(|f| {
                matches!(
                    f.rating,
                    cb_ast::complexity::ComplexityRating::Complex
                        | cb_ast::complexity::ComplexityRating::VeryComplex
                )
            })
            .map(|f| {
                format!(
                    "{} (cognitive: {}, cyclomatic: {})",
                    f.name, f.complexity.cognitive, f.complexity.cyclomatic
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        if !top_issues.is_empty() {
            suggestions.push(Suggestion {
                action: "refactor_high_complexity".to_string(),
                description: format!(
                    "Focus refactoring efforts on these high-complexity functions: {}",
                    top_issues
                ),
                target: None,
                estimated_impact: format!(
                    "Could improve overall maintainability by reducing {:.1}% of problem functions",
                    (3.min(needs_attention) as f64 / needs_attention as f64) * 100.0
                ),
                safety: SafetyLevel::RequiresReview,
                confidence: 0.85,
                reversible: true,
                refactor_call: None,
            });
        }
    }

    if complexity_report.average_cognitive_complexity > 10.0 {
        suggestions.push(Suggestion {
            action: "reduce_average_complexity".to_string(),
            description: "Average cognitive complexity is high across the codebase".to_string(),
            target: None,
            estimated_impact: format!(
                "Target: reduce average from {:.1} to <10.0",
                complexity_report.average_cognitive_complexity
            ),
            safety: SafetyLevel::RequiresReview,
            confidence: 0.80,
            reversible: true,
            refactor_call: None,
        });
    }

    findings.push(Finding {
        id: format!("maintainability-summary-{}", file_path),
        kind: "maintainability_summary".to_string(),
        severity,
        location: FindingLocation {
            file_path: file_path.to_string(),
            range: None,
            symbol: None,
            symbol_kind: None,
        },
        metrics: Some(metrics),
        message,
        suggestions,
    });

    findings
}

#[async_trait]
impl ToolHandler for QualityHandler {
    fn tool_names(&self) -> &[&str] {
        &["analyze.quality"]
    }

    fn is_internal(&self) -> bool {
        false // PUBLIC tool
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        let start_time = Instant::now();
        let args = tool_call.arguments.clone().unwrap_or(json!({}));

        // Parse kind (required)
        let kind = args
            .get("kind")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'kind' parameter".into()))?;

        // Support complexity, smells, maintainability, and readability
        if !matches!(kind, "complexity" | "smells" | "maintainability" | "readability") {
            return Err(ServerError::InvalidRequest(format!(
                "Unsupported kind '{}'. Supported: 'complexity', 'smells', 'maintainability', 'readability'",
                kind
            )));
        }

        debug!(kind = %kind, "Handling analyze.quality request");

        // Parse scope
        let scope_param: QualityScopeParam = if let Some(scope_value) = args.get("scope") {
            serde_json::from_value(scope_value.clone())
                .map_err(|e| ServerError::InvalidRequest(format!("Invalid scope: {}", e)))?
        } else {
            QualityScopeParam {
                scope_type: None,
                path: None,
                include: vec![],
                exclude: vec![],
            }
        };

        // Determine file path (for MVP, only support file scope)
        let file_path = scope_param
            .path
            .or_else(|| args.get("file_path").and_then(|v| v.as_str()).map(String::from))
            .ok_or_else(|| {
                ServerError::InvalidRequest(
                    "Missing file path. For MVP, only file-level analysis is supported via scope.path or file_path parameter".into(),
                )
            })?;

        let scope_type = scope_param.scope_type.unwrap_or_else(|| "file".to_string());

        // Parse options
        let options: QualityOptions = args
            .get("options")
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()
            .map_err(|e| ServerError::InvalidRequest(format!("Invalid options: {}", e)))?
            .unwrap_or_else(|| QualityOptions {
                thresholds: None,
                severity_filter: None,
                limit: default_limit(),
                offset: 0,
                format: default_format(),
                include_suggestions: default_include_suggestions(),
            });

        let thresholds = options.thresholds.unwrap_or_default();

        info!(
            file_path = %file_path,
            kind = %kind,
            scope_type = %scope_type,
            "Analyzing code quality"
        );

        // Read file
        let file_path_obj = Path::new(&file_path);
        let extension = file_path_obj
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| {
                ServerError::InvalidRequest(format!("File has no extension: {}", file_path))
            })?;

        let content = context
            .app_state
            .file_service
            .read_file(file_path_obj)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to read file: {}", e)))?;

        // Get language plugin
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

        // Parse file
        let parsed = plugin
            .parse(&content)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to parse file: {}", e)))?;

        let language = plugin.metadata().name;

        // Analyze complexity
        let complexity_report = cb_ast::complexity::analyze_file_complexity(
            &file_path,
            &content,
            &parsed.symbols,
            &language,
        );

        // Build scope for result
        let scope = AnalysisScope {
            scope_type,
            path: file_path.clone(),
            include: scope_param.include,
            exclude: scope_param.exclude,
        };

        // Transform to AnalysisResult based on kind
        let mut result = if kind == "complexity" {
            self.transform_complexity_report(
                complexity_report,
                &thresholds,
                options.include_suggestions,
                scope,
                start_time.elapsed().as_millis() as u64,
            )
        } else if kind == "smells" {
            // Code smell detection
            let mut result = AnalysisResult::new("quality", "smells", scope);

            let findings = detect_smells(
                &complexity_report,
                &content,
                &parsed.symbols,
                &language,
                &file_path,
            );

            for finding in findings {
                result.add_finding(finding);
            }

            result.summary.files_analyzed = 1;
            result.summary.symbols_analyzed = Some(complexity_report.total_functions);
            result.finalize(start_time.elapsed().as_millis() as u64);
            result
        } else if kind == "maintainability" {
            // Maintainability aggregation
            let mut result = AnalysisResult::new("quality", "maintainability", scope);

            let findings = analyze_maintainability(&complexity_report, &file_path);

            for finding in findings {
                result.add_finding(finding);
            }

            result.summary.files_analyzed = 1;
            result.summary.symbols_analyzed = Some(complexity_report.total_functions);
            result.finalize(start_time.elapsed().as_millis() as u64);
            result
        } else if kind == "readability" {
            // Readability analysis
            let mut result = AnalysisResult::new("quality", "readability", scope);

            let findings = analyze_readability(&complexity_report, &file_path);

            for finding in findings {
                result.add_finding(finding);
            }

            result.summary.files_analyzed = 1;
            result.summary.symbols_analyzed = Some(complexity_report.total_functions);
            result.finalize(start_time.elapsed().as_millis() as u64);
            result
        } else {
            unreachable!("Unsupported kind validated earlier");
        };

        // Set language in metadata
        result.metadata.language = Some(language.to_string());

        info!(
            file_path = %file_path,
            findings_count = result.summary.total_findings,
            analysis_time_ms = result.summary.analysis_time_ms,
            "Quality analysis complete"
        );

        // Serialize to JSON
        serde_json::to_value(result)
            .map_err(|e| ServerError::Internal(format!("Failed to serialize result: {}", e)))
    }
}
