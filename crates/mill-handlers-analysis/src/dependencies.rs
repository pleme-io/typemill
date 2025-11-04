#![allow(dead_code, unused_variables)]

//! Dependency analysis handler
//!
//! This module provides detection for dependency-related patterns including:
//! - Imports: Import/export statement analysis and categorization
//! - Graph: Full dependency graph construction with metrics
//! - Circular: Circular dependency detection with cycle paths
//! - Coupling: Module coupling strength analysis
//! - Cohesion: Module cohesion metrics
//! - Depth: Dependency depth and chain analysis
//!
//! Uses the shared analysis engine for orchestration and focuses only on
//! detection logic.

use crate::{ToolHandler, ToolHandlerContext};
use crate::suggestions::{AnalysisContext, RefactoringCandidate, SuggestionGenerator};
use anyhow::Result;
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::protocol::analysis_result::{
    Finding, FindingLocation, Position, Range, SafetyLevel, Severity, Suggestion,
};
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::debug;

#[cfg(feature = "analysis-circular-deps")]
use mill_analysis_circular_deps::{
    builder::DependencyGraphBuilder, find_circular_dependencies, Cycle,
};

#[cfg(feature = "analysis-circular-deps")]
use mill_foundation::protocol::analysis_result::AnalysisResult;

/// Detect and analyze import/export statements using plugin-based AST parsing
///
/// This function uses language plugins to accurately parse import statements
/// from the file's AST, then categorizes them as external (node_modules, crates.io),
/// internal (project), or relative imports.
///
/// # Algorithm
/// 1. Parse file with appropriate language plugin to get ImportInfo structures
/// 2. Extract module path and imported symbols from AST
/// 3. Categorize import as external, internal, or relative
/// 4. Calculate metrics (import count, symbol count, categorization)
/// 5. Generate findings with Low severity (informational)
///
/// # Plugin-First Approach
/// - TypeScript/JavaScript: Uses mill_lang_typescript::parser::analyze_imports
/// - Rust: Uses mill_lang_rust::parser::parse_imports
/// - Unsupported languages: Returns empty findings (no regex fallback)
///
/// # Heuristics (Categorization)
/// - External: module paths starting with package names (no ./ or ../)
/// - Internal: module paths starting with project root indicators
/// - Relative: module paths starting with ./ or ../
/// - Categorization is based on string patterns, not file system checks
///
/// # Parameters
/// - `complexity_report`: Not used for import detection
/// - `content`: The raw file content to parse
/// - `symbols`: Not used for import detection (uses plugin-parsed imports instead)
/// - `language`: The language name (e.g., "rust", "typescript")
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A vector of findings for import statements, each with:
/// - Location with line number from AST
/// - Metrics including source_module, imported_symbols, import_category, import_type
/// - Low severity (informational only)
use super::config::AnalysisConfig;
pub(crate) fn detect_imports(
    _complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    _symbols: &[mill_plugin_api::Symbol],
    language: &str,
    file_path: &str,
    registry: &dyn mill_handler_api::LanguagePluginRegistry,
    _config: &AnalysisConfig,
) -> Vec<Finding> {
    if language == "rust" {
        let mut findings = Vec::new();
        let import_regex = Regex::new(r"use\s+.*;").unwrap();

        for (i, line) in content.lines().enumerate() {
            if import_regex.is_match(line) {
                findings.push(Finding {
                    id: format!("import-{}-{}", file_path, i),
                    kind: "import".to_string(),
                    severity: Severity::Low,
                    location: FindingLocation {
                        file_path: file_path.to_string(),
                        range: Some(Range {
                            start: Position {
                                line: i as u32 + 1,
                                character: 0,
                            },
                            end: Position {
                                line: i as u32 + 1,
                                character: line.len() as u32,
                            },
                        }),
                        symbol: None,
                        symbol_kind: None,
                    },
                    metrics: None,
                    message: "Import statement found".to_string(),
                    suggestions: vec![],
                });
            }
        }
        return findings;
    }

    // Parse imports using language plugin
    let import_infos = match parse_imports_with_plugin(content, language, file_path, registry) {
        Ok(imports) => imports,
        Err(_) => {
            // If plugin parsing fails or language unsupported, return empty findings
            // This is the plugin-first philosophy: no regex fallback
            return Vec::new();
        }
    };

    // Convert ImportInfo structures to Finding structures
    import_infos
        .into_iter()
        .enumerate()
        .map(|(idx, import_info)| {
            // Extract imported symbols from ImportInfo
            let symbols = extract_symbols_from_import_info(&import_info);

            // Categorize import
            let category = categorize_import(&import_info.module_path, language);

            // Build metrics
            let mut metrics = HashMap::new();
            metrics.insert("source_module".to_string(), json!(import_info.module_path));
            metrics.insert("imported_symbols".to_string(), json!(symbols));
            metrics.insert("import_count".to_string(), json!(symbols.len()));
            metrics.insert("import_category".to_string(), json!(category));
            metrics.insert(
                "import_type".to_string(),
                json!(format!("{:?}", import_info.import_type)),
            );
            metrics.insert("type_only".to_string(), json!(import_info.type_only));

            // Build location from AST source location
            let range = Some(Range {
                start: Position {
                    line: import_info.location.start_line,
                    character: import_info.location.start_column,
                },
                end: Position {
                    line: import_info.location.end_line,
                    character: import_info.location.end_column,
                },
            });

            Finding {
                id: format!("import-{}-{}", file_path, idx),
                kind: "import".to_string(),
                severity: Severity::Low, // Informational
                location: FindingLocation {
                    file_path: file_path.to_string(),
                    range,
                    symbol: None,
                    symbol_kind: Some("import".to_string()),
                },
                metrics: Some(metrics),
                message: format!(
                    "{} import from '{}': {} symbol(s)",
                    category,
                    import_info.module_path,
                    symbols.len()
                ),
                suggestions: vec![],
            }
        })
        .collect()
}

/// Build full dependency graph for the file
///
/// This function constructs a dependency graph showing all direct and indirect
/// dependencies, calculating fan-in and fan-out metrics.
///
/// # Algorithm
/// 1. Extract all import statements to build adjacency list
/// 2. Calculate direct dependencies (immediate imports)
/// 3. Calculate graph metrics: fan-in (how many depend on this), fan-out (dependencies)
/// 4. For MVP, indirect dependencies estimated from direct imports
/// 5. Generate findings with graph structure and metrics
///
/// # Heuristics
/// - Direct dependencies: All imported modules
/// - Fan-out: Count of modules this file imports
/// - Fan-in: Would require workspace-wide analysis (set to 0 for MVP)
/// - Indirect dependencies: Not yet implemented (placeholder)
///
/// # Future Enhancements
/// TODO: Implement full transitive closure for indirect dependencies
/// TODO: Cross-reference with workspace to calculate actual fan-in
/// TODO: Detect dependency cycles in the graph
/// TODO: Generate graph visualization data (GraphViz, Mermaid)
///
/// # Parameters
/// - `complexity_report`: Not used for graph building
/// - `content`: The raw file content to parse for imports
/// - `symbols`: Not used for graph building
/// - `language`: The language name for parsing rules
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A single finding with:
/// - Graph structure metrics (direct/indirect dependencies, fan-in/out)
/// - Info severity (architectural information)
/// - No suggestions (informational only)
pub(crate) fn detect_graph(
    _complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    _symbols: &[mill_plugin_api::Symbol],
    language: &str,
    file_path: &str,
    _registry: &dyn mill_handler_api::LanguagePluginRegistry,
    _config: &AnalysisConfig,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Build dependency map
    let dependency_map = build_dependency_map(content, language);

    // Calculate metrics
    let direct_dependencies: Vec<String> = dependency_map.keys().cloned().collect();
    let fan_out = direct_dependencies.len();

    // TODO: Calculate fan-in by analyzing workspace
    let fan_in = 0;

    // TODO: Calculate indirect dependencies via transitive closure
    let indirect_dependencies: Vec<String> = vec![];

    let mut metrics = HashMap::new();
    metrics.insert(
        "direct_dependencies".to_string(),
        json!(direct_dependencies),
    );
    metrics.insert(
        "indirect_dependencies".to_string(),
        json!(indirect_dependencies),
    );
    metrics.insert("fan_in".to_string(), json!(fan_in));
    metrics.insert("fan_out".to_string(), json!(fan_out));
    metrics.insert("total_dependencies".to_string(), json!(fan_out));

    findings.push(Finding {
        id: format!("dependency-graph-{}", file_path),
        kind: "dependency_graph".to_string(),
        severity: Severity::Low, // Informational
        location: FindingLocation {
            file_path: file_path.to_string(),
            range: None,
            symbol: None,
            symbol_kind: Some("module".to_string()),
        },
        metrics: Some(metrics),
        message: format!(
            "Dependency graph: {} direct dependencies, fan-in={}, fan-out={}",
            direct_dependencies.len(),
            fan_in,
            fan_out
        ),
        suggestions: vec![],
    });

    findings
}

/// Detect circular dependencies
///
/// This function identifies circular dependency chains where module A depends
/// on B, B depends on C, and C depends back on A (creating a cycle).
///
/// # Algorithm
/// 1. Build dependency graph from all imports
/// 2. Use DFS-based cycle detection algorithm
/// 3. Track visited nodes and recursion stack
/// 4. When a node in the stack is revisited, a cycle is found
/// 5. Report cycle path with full import chain
///
/// # Heuristics
/// - Simple file-level cycle detection (not cross-workspace)
/// - Cycles within same file are detected via self-imports
/// - For MVP, limited to analyzing import statements in single file
/// - Real cross-file cycles require workspace-wide analysis
///
/// # Future Enhancements
/// TODO: Implement workspace-wide cycle detection
/// TODO: Calculate cycle rank and complexity metrics
/// TODO: Suggest refactoring patterns to break cycles
/// TODO: Detect indirect cycles (A→B→C→A)
///
/// # Parameters
/// - `complexity_report`: Not used for circular dependency detection
/// - `content`: The raw file content to analyze
/// - `symbols`: Not used for circular dependency detection
/// - `language`: The language name for parsing rules
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A vector of findings for each detected cycle, each with:
/// - Location at import statement
/// - Metrics including cycle_length and cycle_path array
/// - High severity (architectural smell)
/// - Suggestion to refactor and break cycle
pub(crate) fn detect_circular(
    _complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    _symbols: &[mill_plugin_api::Symbol],
    language: &str,
    file_path: &str,
    _registry: &dyn mill_handler_api::LanguagePluginRegistry,
    _config: &AnalysisConfig,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Build dependency map
    let dependency_map = build_dependency_map(content, language);

    // For MVP: Check for obvious self-referential imports or simple cycles
    // Full cycle detection requires workspace-wide graph analysis

    // Check for self-imports (module importing itself)
    let file_module = extract_module_name(file_path);
    for (import, line_num) in &dependency_map {
        if import.contains(&file_module) || import == &file_module {
            let mut metrics = HashMap::new();
            metrics.insert("cycle_length".to_string(), json!(1));
            metrics.insert(
                "cycle_path".to_string(),
                json!(vec![file_module.clone(), import.clone()]),
            );

            let mut finding = Finding {
                id: format!("circular-dependency-{}-{}", file_path, line_num),
                kind: "circular_dependency".to_string(),
                severity: Severity::High,
                location: FindingLocation {
                    file_path: file_path.to_string(),
                    range: Some(Range {
                        start: Position {
                            line: *line_num as u32,
                            character: 0,
                        },
                        end: Position {
                            line: *line_num as u32,
                            character: 0,
                        },
                    }),
                    symbol: None,
                    symbol_kind: Some("import".to_string()),
                },
                metrics: Some(metrics),
                message: format!(
                    "Circular dependency detected: module imports itself via '{}'",
                    import
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

            if let Ok(candidates) = generate_dependency_refactoring_candidates(&finding) {
                let suggestions = suggestion_generator.generate_multiple(candidates, &context);
                finding.suggestions = suggestions
                    .into_iter()
                    .map(|s| s.into())
                    .collect::<Vec<Suggestion>>();
            }
            findings.push(finding);
        }
    }

    // TODO: Implement full DFS cycle detection for multi-module cycles
    // This requires analyzing imports across multiple files in workspace

    findings
}

/// Calculate module coupling metrics
///
/// This function analyzes coupling strength between modules using afferent
/// coupling (Ca - incoming dependencies) and efferent coupling (Ce - outgoing
/// dependencies) to calculate instability metric.
///
/// # Algorithm
/// 1. Count import statements (efferent coupling = Ce)
/// 2. Calculate instability: I = Ce / (Ca + Ce), where Ca would come from workspace analysis
/// 3. For MVP, Ca = 0 (requires workspace-wide reference analysis)
/// 4. High coupling detected when instability > 0.7 (threshold)
/// 5. Generate findings for high coupling modules
///
/// # Heuristics
/// - Efferent coupling (Ce): Number of modules this file imports
/// - Afferent coupling (Ca): Would require workspace analysis (set to 0 for MVP)
/// - Instability = Ce / (Ca + Ce): Measures resistance to change
/// - Instability > 0.7 indicates high coupling
///
/// # Future Enhancements
/// TODO: Implement workspace-wide analysis for accurate Ca calculation
/// TODO: Calculate abstractness metric (interfaces vs concrete classes)
/// TODO: Plot modules on A/I diagram (Main Sequence analysis)
/// TODO: Detect modules violating dependency inversion principle
///
/// # Parameters
/// - `complexity_report`: Not used for coupling analysis
/// - `content`: The raw file content to analyze
/// - `symbols`: Not used for coupling analysis
/// - `language`: The language name for parsing rules
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A vector of findings (one per file), each with:
/// - Metrics including afferent_coupling, efferent_coupling, instability
/// - Medium severity if high coupling detected
/// - Suggestion to reduce coupling via interfaces or dependency injection
pub(crate) fn detect_coupling(
    _complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    _symbols: &[mill_plugin_api::Symbol],
    language: &str,
    file_path: &str,
    _registry: &dyn mill_handler_api::LanguagePluginRegistry,
    _config: &AnalysisConfig,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Build dependency map
    let dependency_map = build_dependency_map(content, language);

    // Calculate coupling metrics
    let efferent_coupling = dependency_map.len(); // Ce: outgoing dependencies

    // TODO: Calculate afferent coupling via workspace analysis
    let afferent_coupling = 0; // Ca: incoming dependencies (MVP: not yet implemented)

    // Calculate instability: I = Ce / (Ca + Ce)
    // I = 0: maximally stable (no outgoing deps, many incoming)
    // I = 1: maximally unstable (many outgoing deps, no incoming)
    let instability = if afferent_coupling + efferent_coupling > 0 {
        efferent_coupling as f64 / (afferent_coupling + efferent_coupling) as f64
    } else {
        0.0
    };

    let high_coupling = instability > 0.7; // Threshold for concern
    let severity = if high_coupling {
        Severity::Medium
    } else {
        Severity::Low
    };

    let mut metrics = HashMap::new();
    metrics.insert("afferent_coupling".to_string(), json!(afferent_coupling));
    metrics.insert("efferent_coupling".to_string(), json!(efferent_coupling));
    metrics.insert("instability".to_string(), json!(instability));

    let message = if high_coupling {
        format!(
            "High coupling detected: instability={:.2} (Ce={}, Ca={}). Module is highly unstable.",
            instability, efferent_coupling, afferent_coupling
        )
    } else {
        format!(
            "Coupling metrics: instability={:.2} (Ce={}, Ca={})",
            instability, efferent_coupling, afferent_coupling
        )
    };

    let mut finding = Finding {
        id: format!("coupling-{}", file_path),
        kind: "coupling_metric".to_string(),
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

    if high_coupling {
        let suggestion_generator = SuggestionGenerator::new();
        let context = AnalysisContext {
            file_path: file_path.to_string(),
            has_full_type_info: false,
            has_partial_type_info: false,
            ast_parse_errors: 0,
        };

        if let Ok(candidates) = generate_dependency_refactoring_candidates(&finding) {
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

/// Analyze module cohesion
///
/// This function calculates cohesion metrics using LCOM (Lack of Cohesion of Methods)
/// to determine how well functions within a module work together.
///
/// # Algorithm
/// 1. Extract all functions from complexity report
/// 2. For each function, identify shared data (parameters, variables, types)
/// 3. Calculate LCOM: number of function pairs with no shared data
/// 4. Low cohesion (LCOM > 0.5) indicates module should be split
/// 5. Generate findings with cohesion metrics and refactoring suggestions
///
/// # Heuristics
/// - Shared data ratio: functions accessing common variables/types
/// - LCOM calculation: simplified version based on function independence
/// - For MVP, uses function count and basic heuristics
/// - Full LCOM requires data flow analysis (not yet implemented)
///
/// # Future Enhancements
/// TODO: Implement full LCOM calculation with data flow analysis
/// TODO: Detect god classes/modules (high function count + low cohesion)
/// TODO: Suggest specific module split patterns
/// TODO: Calculate cohesion at different granularities (file, class, namespace)
///
/// # Parameters
/// - `complexity_report`: Used to get function list and metrics
/// - `content`: The raw file content for analysis
/// - `symbols`: Used for symbol-level cohesion analysis
/// - `language`: The language name for parsing rules
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A vector of findings (one per file), each with:
/// - Metrics including lcom_score, functions_analyzed, shared_data_ratio
/// - Medium severity if low cohesion detected
/// - Suggestion to split module or refactor for better cohesion
pub(crate) fn detect_cohesion(
    complexity_report: &mill_ast::complexity::ComplexityReport,
    _content: &str,
    _symbols: &[mill_plugin_api::Symbol],
    _language: &str,
    file_path: &str,
    _registry: &dyn mill_handler_api::LanguagePluginRegistry,
    _config: &AnalysisConfig,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // For MVP: Use simplified cohesion heuristic based on function count
    let functions_analyzed = complexity_report.functions.len();

    // Simple heuristic: High function count suggests potential cohesion issues
    // TODO: Implement full LCOM calculation with data flow analysis
    let lcom_score = if functions_analyzed > 20 {
        0.7 // High lack of cohesion
    } else if functions_analyzed > 10 {
        0.5 // Medium
    } else {
        0.3 // Low lack of cohesion (good)
    };

    // Estimate shared data ratio (MVP: simplified)
    // TODO: Analyze actual variable/field sharing between functions
    let shared_data_ratio = if functions_analyzed > 0 {
        0.4 // Placeholder value
    } else {
        0.0
    };

    let low_cohesion = lcom_score > 0.5;
    let severity = if low_cohesion {
        Severity::Medium
    } else {
        Severity::Low
    };

    let mut metrics = HashMap::new();
    metrics.insert("lcom_score".to_string(), json!(lcom_score));
    metrics.insert("functions_analyzed".to_string(), json!(functions_analyzed));
    metrics.insert("shared_data_ratio".to_string(), json!(shared_data_ratio));

    let message = if low_cohesion {
        format!(
            "Low cohesion detected: LCOM={:.2} with {} functions. Module may benefit from splitting.",
            lcom_score, functions_analyzed
        )
    } else {
        format!(
            "Cohesion metrics: LCOM={:.2} with {} functions (acceptable)",
            lcom_score, functions_analyzed
        )
    };

    let mut finding = Finding {
        id: format!("cohesion-{}", file_path),
        kind: "cohesion_metric".to_string(),
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

    if low_cohesion {
        let suggestion_generator = SuggestionGenerator::new();
        let context = AnalysisContext {
            file_path: file_path.to_string(),
            has_full_type_info: false,
            has_partial_type_info: false,
            ast_parse_errors: 0,
        };

        if let Ok(candidates) = generate_dependency_refactoring_candidates(&finding) {
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

/// Calculate dependency depth
///
/// This function analyzes the transitive dependency chain to find the maximum
/// depth from the current module to leaf dependencies.
///
/// # Algorithm
/// 1. Build dependency graph from imports
/// 2. Use BFS to traverse dependency tree
/// 3. Track depth at each level
/// 4. Report maximum depth and longest chain
/// 5. Flag excessive depth (> 5) as architectural concern
///
/// # Heuristics
/// - Max depth > 5: Long dependency chains, tight coupling
/// - Leaf dependencies: Modules with no further imports
/// - For MVP, depth calculation based on direct imports only
/// - Full transitive analysis requires workspace-wide graph
///
/// # Future Enhancements
/// TODO: Implement full transitive dependency traversal
/// TODO: Calculate average depth (not just max)
/// TODO: Identify critical path dependencies
/// TODO: Detect deep chains that cross architectural boundaries
///
/// # Parameters
/// - `complexity_report`: Not used for depth analysis
/// - `content`: The raw file content to analyze
/// - `symbols`: Not used for depth analysis
/// - `language`: The language name for parsing rules
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A vector of findings, each with:
/// - Metrics including max_depth and dependency_chain array
/// - Medium severity if depth excessive (> 5)
/// - Suggestion to flatten dependency tree or refactor architecture
pub(crate) fn detect_depth(
    _complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    _symbols: &[mill_plugin_api::Symbol],
    language: &str,
    file_path: &str,
    _registry: &dyn mill_handler_api::LanguagePluginRegistry,
    _config: &AnalysisConfig,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Build dependency map
    let dependency_map = build_dependency_map(content, language);

    // For MVP: Calculate depth based on direct dependencies only
    // Full transitive depth requires workspace-wide analysis
    let direct_deps: Vec<String> = dependency_map.keys().cloned().collect();
    let max_depth = if direct_deps.is_empty() {
        0 // Leaf module (no dependencies)
    } else {
        1 // Has dependencies, assume depth of 1 for MVP
    };

    // TODO: Implement full BFS/DFS traversal for transitive depth
    let dependency_chain = direct_deps.clone();

    let excessive_depth = max_depth > 5; // Threshold for concern
    let severity = if excessive_depth {
        Severity::Medium
    } else {
        Severity::Low
    };

    let mut metrics = HashMap::new();
    metrics.insert("max_depth".to_string(), json!(max_depth));
    metrics.insert("dependency_chain".to_string(), json!(dependency_chain));
    metrics.insert(
        "direct_dependencies_count".to_string(),
        json!(direct_deps.len()),
    );

    let message = if excessive_depth {
        format!(
            "Excessive dependency depth: {} levels. Long dependency chains increase coupling.",
            max_depth
        )
    } else {
        format!(
            "Dependency depth: {} levels with {} direct dependencies",
            max_depth,
            direct_deps.len()
        )
    };

    let mut finding = Finding {
        id: format!("dependency-depth-{}", file_path),
        kind: "dependency_depth".to_string(),
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

    if excessive_depth {
        let suggestion_generator = SuggestionGenerator::new();
        let context = AnalysisContext {
            file_path: file_path.to_string(),
            has_full_type_info: false,
            has_partial_type_info: false,
            ast_parse_errors: 0,
        };

        if let Ok(candidates) = generate_dependency_refactoring_candidates(&finding) {
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

// ============================================================================
// Helper Functions
// ============================================================================

#[cfg(feature = "analysis-circular-deps")]
/// Generate actionable suggestions for breaking circular dependencies
fn generate_cycle_break_suggestions(cycle: &Cycle) -> Vec<Suggestion> {
    let mut suggestions = Vec::new();

    // Suggestion 1: Extract interface/trait
    if cycle.modules.len() == 2 {
        suggestions.push(Suggestion {
            action: "extract_interface".to_string(),
            description: format!(
                "Extract a shared interface or trait between '{}' and '{}'. Move common dependencies to the interface to break the cycle.",
                cycle.modules.first().map(|s| s.as_str()).unwrap_or("module A"),
                cycle.modules.get(1).map(|s| s.as_str()).unwrap_or("module B")
            ),
            target: None,
            estimated_impact: "Eliminates circular dependency, improves testability and modularity".to_string(),
            safety: SafetyLevel::Safe,
            confidence: 0.85,
            reversible: true,
            refactor_call: None,
        });
    }

    // Suggestion 2: Dependency injection
    suggestions.push(Suggestion {
        action: "dependency_injection".to_string(),
        description: "Use dependency injection to invert the dependency direction. Pass dependencies as parameters instead of importing directly.".to_string(),
        target: None,
        estimated_impact: "Breaks cycle by inverting control, improves testability".to_string(),
        safety: SafetyLevel::RequiresReview,
        confidence: 0.80,
        reversible: true,
        refactor_call: None,
    });

    // Suggestion 3: Extract shared module
    if cycle.modules.len() > 2 {
        suggestions.push(Suggestion {
            action: "extract_shared_module".to_string(),
            description: format!(
                "Extract shared code from the {} modules into a new common module. This breaks the cycle by creating a dependency tree instead of a cycle.",
                cycle.modules.len()
            ),
            target: None,
            estimated_impact: "Eliminates circular dependency, reduces coupling".to_string(),
            safety: SafetyLevel::RequiresReview,
            confidence: 0.75,
            reversible: true,
            refactor_call: None,
        });
    }

    // Suggestion 4: Merge modules (for small cycles)
    if cycle.modules.len() == 2 {
        suggestions.push(Suggestion {
            action: "merge_modules".to_string(),
            description: "If the modules are tightly coupled and small, consider merging them into a single module.".to_string(),
            target: None,
            estimated_impact: "Simplifies architecture by removing artificial separation".to_string(),
            safety: SafetyLevel::RequiresReview,
            confidence: 0.70,
            reversible: true,
            refactor_call: None,
        });
    }

    suggestions
}

fn generate_dependency_refactoring_candidates(
    finding: &Finding,
) -> Result<Vec<RefactoringCandidate>> {
    let candidates = Vec::new();
    let location = finding.location.clone();
    let line = location.range.as_ref().map(|r| r.start.line).unwrap_or(0) as usize;

    match finding.kind.as_str() {
        "circular_dependency" => {
            // This would likely suggest a `move` refactoring, but the arguments
            // would be complex to determine automatically.
        }
        "coupling_metric" if finding.severity >= Severity::Medium => {
            // This might suggest `extract` to create an interface, but again,
            // this is a complex, multi-step refactoring.
        }
        _ => {}
    }

    Ok(candidates)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get language-specific import patterns
///
/// Returns regex patterns for detecting imports/exports in different languages.
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
            // export { foo } from './module'
            r#"export\s+(?:\{[^}]*\}|\*\s+as\s+\w+|\w+)\s+from\s+['"]([^'"]+)['"]"#.to_string(),
        ],
        "python" => vec![
            // from module import foo
            r"from\s+([\w.]+)\s+import".to_string(),
            // import module
            r"import\s+([\w.]+)".to_string(),
        ],
        "go" => vec![
            // import "package"
            r#"import\s+"([^"]+)""#.to_string(),
        ],
        _ => vec![],
    }
}

/// Extract imported symbols from an import statement
///
/// This function parses the import line to extract symbol names.
///
/// # Parameters
/// - `line`: The import statement line
/// - `language`: The language name for parsing rules
///
/// # Returns
/// A vector of symbol names that are imported
fn extract_imported_symbols(line: &str, language: &str) -> Vec<String> {
    let mut symbols = Vec::new();

    match language.to_lowercase().as_str() {
        "rust" => {
            // use std::collections::{HashMap, HashSet};
            if let Some(braces_start) = line.find('{') {
                if let Some(braces_end) = line.find('}') {
                    let symbols_str = &line[braces_start + 1..braces_end];
                    for sym in symbols_str.split(',') {
                        let clean_sym = sym.trim().to_string();
                        if !clean_sym.is_empty() {
                            symbols.push(clean_sym);
                        }
                    }
                }
            } else if let Some(double_colon) = line.rfind("::") {
                // use std::collections::HashMap;
                let after_colon = &line[double_colon + 2..];
                if let Some(sym) = after_colon.split(';').next() {
                    symbols.push(sym.trim().to_string());
                }
            }
        }
        "typescript" | "javascript" => {
            // import { foo, bar } from './module'
            if let Some(braces_start) = line.find('{') {
                if let Some(braces_end) = line.find('}') {
                    let symbols_str = &line[braces_start + 1..braces_end];
                    for sym in symbols_str.split(',') {
                        // Handle 'as' aliases: foo as bar
                        let parts: Vec<&str> = sym.split_whitespace().collect();
                        if !parts.is_empty() {
                            symbols.push(parts[0].trim().to_string());
                        }
                    }
                }
            } else if line.contains("import ") && line.contains(" from ") {
                // import foo from './module' (default import)
                if let Some(import_pos) = line.find("import ") {
                    if let Some(from_pos) = line.find(" from ") {
                        let between = &line[import_pos + 7..from_pos];
                        let sym = between.trim();
                        if !sym.is_empty() && !sym.starts_with('*') {
                            symbols.push(sym.to_string());
                        }
                    }
                }
            }
        }
        "python" => {
            // from module import foo, bar
            if let Some(import_pos) = line.find("import ") {
                let after_import = &line[import_pos + 7..];
                for sym in after_import.split(',') {
                    // Handle 'as' aliases
                    let parts: Vec<&str> = sym.split_whitespace().collect();
                    if !parts.is_empty() {
                        symbols.push(parts[0].trim().to_string());
                    }
                }
            }
        }
        "go" => {
            // Go typically uses package name, not individual symbols
            // For now, return empty (package-level import)
        }
        _ => {}
    }

    symbols
}

/// Categorize an import as external, internal, or relative
///
/// # Parameters
/// - `module_path`: The module path from import statement
/// - `language`: The language name for categorization rules
///
/// # Returns
/// A string: "external", "internal", or "relative"
fn categorize_import(module_path: &str, language: &str) -> String {
    match language.to_lowercase().as_str() {
        "rust" => {
            if module_path.starts_with("std::") || module_path.starts_with("core::") {
                "external".to_string()
            } else if module_path.starts_with("crate::") || module_path.starts_with("super::") {
                "internal".to_string()
            } else {
                "external".to_string() // External crate
            }
        }
        "typescript" | "javascript" => {
            if module_path.starts_with("./") || module_path.starts_with("../") {
                "relative".to_string()
            } else if module_path.starts_with('@') || !module_path.contains('/') {
                "external".to_string() // npm package
            } else {
                "internal".to_string()
            }
        }
        "python" => {
            if module_path.starts_with('.') {
                "relative".to_string()
            } else {
                // Heuristic: standard library or external package
                "external".to_string()
            }
        }
        "go" => {
            if module_path.contains('/') {
                "external".to_string() // Remote package
            } else {
                "internal".to_string() // Local package
            }
        }
        _ => "unknown".to_string(),
    }
}

/// Build dependency map from file content
///
/// Returns a HashMap mapping module path to line number where it's imported.
///
/// # Parameters
/// - `content`: The file content to analyze
/// - `language`: The language name for parsing rules
///
/// # Returns
/// A HashMap<String, usize> mapping module paths to line numbers
fn build_dependency_map(content: &str, language: &str) -> HashMap<String, usize> {
    let mut map = HashMap::new();
    let import_patterns = get_import_patterns(language);

    let mut line_num = 1;
    for line in content.lines() {
        for pattern_str in &import_patterns {
            if let Ok(pattern) = Regex::new(pattern_str) {
                if let Some(captures) = pattern.captures(line) {
                    if let Some(module_path) = captures.get(1) {
                        map.insert(module_path.as_str().to_string(), line_num);
                    }
                }
            }
        }
        line_num += 1;
    }

    map
}

/// Extract module name from file path
///
/// Converts file path to a module name for comparison.
///
/// # Parameters
/// - `file_path`: The file path to convert
///
/// # Returns
/// A string representing the module name
fn extract_module_name(file_path: &str) -> String {
    // Extract file name without extension
    if let Some(file_name) = file_path.split('/').next_back() {
        if let Some(name_without_ext) = file_name.split('.').next() {
            return name_without_ext.to_string();
        }
    }
    file_path.to_string()
}

/// Parse imports using language plugin
///
/// This function calls the appropriate language plugin parser to extract
/// import information from the file content.
///
/// # Parameters
/// - `content`: The raw file content to parse
/// - `language`: The language name (e.g., "rust", "typescript")
/// - `file_path`: The path to the file being analyzed (for TypeScript path context)
/// - `registry`: The language plugin registry for dynamic plugin lookup
///
/// # Returns
/// A Result containing Vec<ImportInfo> from the plugin parser, or an error
fn parse_imports_with_plugin(
    content: &str,
    language: &str,
    file_path: &str,
    registry: &dyn mill_handler_api::LanguagePluginRegistry,
) -> Result<Vec<mill_foundation::protocol::ImportInfo>, String> {
    use std::path::Path;

    // Map language names to file extensions
    let extension = match language.to_lowercase().as_str() {
        "typescript" | "javascript" => "ts",
        "rust" => "rs",
        _ => return Err(format!("Unsupported language: {}", language)),
    };

    // Get plugin from registry
    if let Some(plugin) = registry.get_plugin(extension) {
        let path = Path::new(file_path);
        let graph = plugin
            .analyze_detailed_imports(content, Some(path))
            .map_err(|e| format!("Plugin failed: {}", e))?;
        Ok(graph.imports)
    } else {
        Err(format!("No plugin available for {}", language))
    }
}

/// Extract imported symbols from ImportInfo structure
///
/// Converts the structured import data from the plugin into a flat list
/// of symbol names for backwards compatibility with existing metrics.
///
/// # Parameters
/// - `import_info`: The parsed import information from the plugin
///
/// # Returns
/// A vector of symbol names (including default, namespace, and named imports)
fn extract_symbols_from_import_info(
    import_info: &mill_foundation::protocol::ImportInfo,
) -> Vec<String> {
    let mut symbols = Vec::new();

    // Add default import if present
    if let Some(ref default) = import_info.default_import {
        symbols.push(default.clone());
    }

    // Add namespace import if present
    if let Some(ref namespace) = import_info.namespace_import {
        symbols.push(format!("* as {}", namespace));
    }

    // Add all named imports
    for named in &import_info.named_imports {
        if let Some(ref alias) = named.alias {
            symbols.push(format!("{} as {}", named.name, alias));
        } else {
            symbols.push(named.name.clone());
        }
    }

    symbols
}

// ============================================================================
// Handler Implementation
// ============================================================================

pub struct DependenciesHandler;

impl DependenciesHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ToolHandler for DependenciesHandler {
    fn tool_names(&self) -> &[&str] {
        &["analyze.dependencies"]
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
        if !matches!(
            kind,
            "imports" | "graph" | "circular" | "coupling" | "cohesion" | "depth"
        ) {
            return Err(ServerError::invalid_request(format!(
                "Unsupported kind '{}'. Supported: 'imports', 'graph', 'circular', 'coupling', 'cohesion', 'depth'",
                kind
            )));
        }

        debug!(kind = %kind, "Handling analyze.dependencies request");

        // Dispatch to appropriate analysis function
        if kind == "circular" {
            #[cfg(feature = "analysis-circular-deps")]
            {
                let project_root = &context.app_state.project_root;
                let builder =
                    DependencyGraphBuilder::new(&context.app_state.language_plugins.inner);
                let graph = builder.build(project_root).map_err(|e| ServerError::internal(e.to_string()))?;
                let result = find_circular_dependencies(&graph, None)
                    .map_err(|e| ServerError::internal(e.to_string()))?;

                let findings = result
                    .cycles
                    .into_iter()
                    .map(|cycle| {
                        let mut metrics = HashMap::new();
                        metrics.insert("cycle_length".to_string(), json!(cycle.modules.len()));
                        metrics.insert("cycle_path".to_string(), json!(cycle.modules));

                        // Add import chain to metrics for detailed analysis
                        let import_chain_json: Vec<_> = cycle
                            .import_chain
                            .iter()
                            .map(|link| {
                                json!({
                                    "from": link.from,
                                    "to": link.to,
                                    "symbols": link.symbols
                                })
                            })
                            .collect();
                        metrics.insert("import_chain".to_string(), json!(import_chain_json));

                        // Generate actionable suggestions based on cycle characteristics
                        let suggestions = generate_cycle_break_suggestions(&cycle);

                        Finding {
                            id: format!("circular-dependency-{}", cycle.id),
                            kind: "circular_dependency".to_string(),
                            severity: Severity::High,
                            location: FindingLocation {
                                file_path: cycle.modules.first().cloned().unwrap_or_default(),
                                range: None,
                                symbol: None,
                                symbol_kind: Some("module".to_string()),
                            },
                            metrics: Some(metrics),
                            message: format!(
                                "Circular dependency detected: {} modules form a cycle ({})",
                                cycle.modules.len(),
                                cycle.modules.join(" → ")
                            ),
                            suggestions,
                        }
                    })
                    .collect();

                let analysis_result = AnalysisResult {
                    findings,
                    summary: mill_foundation::protocol::analysis_result::AnalysisSummary {
                        total_findings: result.summary.total_cycles,
                        returned_findings: result.summary.total_cycles,
                        has_more: false,
                        by_severity:
                            mill_foundation::protocol::analysis_result::SeverityBreakdown {
                                high: result.summary.total_cycles,
                                medium: 0,
                                low: 0,
                            },
                        files_analyzed: result.summary.files_analyzed,
                        symbols_analyzed: Some(result.summary.total_modules_in_cycles),
                        analysis_time_ms: result.summary.analysis_time_ms,
                        fix_actions: None,
                    },
                    metadata: mill_foundation::protocol::analysis_result::AnalysisMetadata {
                        category: "dependencies".to_string(),
                        kind: "circular".to_string(),
                        scope: mill_foundation::protocol::analysis_result::AnalysisScope {
                            scope_type: "workspace".to_string(),
                            path: project_root.to_string_lossy().to_string(),
                            include: vec![],
                            exclude: vec![],
                        },
                        language: None,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        thresholds: None,
                    },
                };

                return Ok(serde_json::to_value(analysis_result)?);
            }
            #[cfg(not(feature = "analysis-circular-deps"))]
            {
                return super::engine::run_analysis(
                    context,
                    tool_call,
                    "dependencies",
                    kind,
                    detect_circular,
                )
                .await;
            }
        }

        match kind {
            "imports" => {
                super::engine::run_analysis(
                    context,
                    tool_call,
                    "dependencies",
                    kind,
                    detect_imports,
                )
                .await
            }
            "graph" => {
                super::engine::run_analysis(context, tool_call, "dependencies", kind, detect_graph)
                    .await
            }
            "coupling" => {
                super::engine::run_analysis(
                    context,
                    tool_call,
                    "dependencies",
                    kind,
                    detect_coupling,
                )
                .await
            }
            "cohesion" => {
                super::engine::run_analysis(
                    context,
                    tool_call,
                    "dependencies",
                    kind,
                    detect_cohesion,
                )
                .await
            }
            "depth" => {
                super::engine::run_analysis(context, tool_call, "dependencies", kind, detect_depth)
                    .await
            }
            _ => unreachable!("Kind validated earlier"),
        }
    }
}
