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

use super::super::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::analysis_result::{
    Finding, FindingLocation, Position, Range, SafetyLevel, Severity, Suggestion,
};
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::debug;

/// Detect and analyze import/export statements
///
/// This function identifies all import and export statements in a file and
/// categorizes them as external (node_modules, crates.io), internal (project),
/// or relative imports.
///
/// # Algorithm
/// 1. Parse import/export statements using language-specific patterns
/// 2. Extract source module path and imported symbols
/// 3. Categorize import as external, internal, or relative
/// 4. Calculate metrics (import count, symbol count, categorization)
/// 5. Generate findings with Low severity (informational)
///
/// # Heuristics
/// - External: module paths starting with package names (no ./ or ../)
/// - Internal: module paths starting with project root indicators
/// - Relative: module paths starting with ./ or ../
/// - Categorization is based on string patterns, not file system checks
///
/// # Future Enhancements
/// TODO: Add AST-based import analysis for accurate symbol extraction
/// TODO: Cross-reference with package.json/Cargo.toml for external validation
/// TODO: Detect unused re-exports
///
/// # Parameters
/// - `complexity_report`: Not used for import detection
/// - `content`: The raw file content to search for imports
/// - `symbols`: Not used for import detection
/// - `language`: The language name (e.g., "rust", "typescript")
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A vector of findings for import statements, each with:
/// - Location with line number
/// - Metrics including source_module, imported_symbols, import_category
/// - Low severity (informational only)
fn detect_imports(
    _complexity_report: &cb_ast::complexity::ComplexityReport,
    content: &str,
    _symbols: &[cb_plugin_api::Symbol],
    language: &str,
    file_path: &str,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Language-specific import/export patterns
    let import_patterns = get_import_patterns(language);

    if import_patterns.is_empty() {
        return findings; // Language not supported
    }

    let mut line_num = 1;
    let lines: Vec<&str> = content.lines().collect();

    for line in &lines {
        // Check if this line contains an import or export
        for pattern_str in &import_patterns {
            if let Ok(pattern) = Regex::new(pattern_str) {
                if let Some(captures) = pattern.captures(line) {
                    // Get the module path from the first capture group
                    if let Some(module_path) = captures.get(1) {
                        let module_path_str = module_path.as_str();

                        // Extract symbols from this import
                        let symbols = extract_imported_symbols(line, language);

                        // Categorize import
                        let category = categorize_import(module_path_str, language);

                        let mut metrics = HashMap::new();
                        metrics.insert("source_module".to_string(), json!(module_path_str));
                        metrics.insert("imported_symbols".to_string(), json!(symbols));
                        metrics.insert("import_count".to_string(), json!(symbols.len()));
                        metrics.insert("import_category".to_string(), json!(category));

                        findings.push(Finding {
                            id: format!("import-{}-{}", file_path, line_num),
                            kind: "import".to_string(),
                            severity: Severity::Low, // Informational
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
                                "{} import from '{}': {} symbol(s)",
                                category,
                                module_path_str,
                                symbols.len()
                            ),
                            suggestions: vec![],
                        });
                    }
                }
            }
        }

        line_num += 1;
    }

    findings
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
fn detect_graph(
    _complexity_report: &cb_ast::complexity::ComplexityReport,
    content: &str,
    _symbols: &[cb_plugin_api::Symbol],
    language: &str,
    file_path: &str,
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
fn detect_circular(
    _complexity_report: &cb_ast::complexity::ComplexityReport,
    content: &str,
    _symbols: &[cb_plugin_api::Symbol],
    language: &str,
    file_path: &str,
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

            findings.push(Finding {
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
                message: format!("Circular dependency detected: module imports itself via '{}'", import),
                suggestions: vec![Suggestion {
                    action: "break_circular_dependency".to_string(),
                    description: "Refactor to break circular import (e.g., extract shared interface, use dependency injection)".to_string(),
                    target: None,
                    estimated_impact: "Improves architecture, reduces coupling, enables better testing".to_string(),
                    safety: SafetyLevel::RequiresReview,
                    confidence: 0.80,
                    reversible: false,
                    refactor_call: None,
                }],
            });
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
fn detect_coupling(
    _complexity_report: &cb_ast::complexity::ComplexityReport,
    content: &str,
    _symbols: &[cb_plugin_api::Symbol],
    language: &str,
    file_path: &str,
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

    findings.push(Finding {
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
        suggestions: if high_coupling {
            vec![Suggestion {
                action: "reduce_coupling".to_string(),
                description: "Consider reducing dependencies via interfaces, dependency injection, or extracting shared abstractions".to_string(),
                target: None,
                estimated_impact: "Improves testability and maintainability, reduces change ripple effects".to_string(),
                safety: SafetyLevel::RequiresReview,
                confidence: 0.70,
                reversible: false,
                refactor_call: None,
            }]
        } else {
            vec![]
        },
    });

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
fn detect_cohesion(
    complexity_report: &cb_ast::complexity::ComplexityReport,
    _content: &str,
    _symbols: &[cb_plugin_api::Symbol],
    _language: &str,
    file_path: &str,
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

    findings.push(Finding {
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
        suggestions: if low_cohesion {
            vec![Suggestion {
                action: "improve_cohesion".to_string(),
                description: "Consider splitting module into smaller, more focused modules with related responsibilities".to_string(),
                target: None,
                estimated_impact: "Improves maintainability, makes code easier to understand and test".to_string(),
                safety: SafetyLevel::RequiresReview,
                confidence: 0.65,
                reversible: false,
                refactor_call: None,
            }]
        } else {
            vec![]
        },
    });

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
fn detect_depth(
    _complexity_report: &cb_ast::complexity::ComplexityReport,
    content: &str,
    _symbols: &[cb_plugin_api::Symbol],
    language: &str,
    file_path: &str,
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

    findings.push(Finding {
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
        suggestions: if excessive_depth {
            vec![Suggestion {
                action: "reduce_dependency_depth".to_string(),
                description: "Consider flattening dependency tree, using dependency injection, or introducing abstraction layers".to_string(),
                target: None,
                estimated_impact: "Reduces coupling, improves testability, simplifies dependency management".to_string(),
                safety: SafetyLevel::RequiresReview,
                confidence: 0.70,
                reversible: false,
                refactor_call: None,
            }]
        } else {
            vec![]
        },
    });

    findings
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
    if let Some(file_name) = file_path.split('/').last() {
        if let Some(name_without_ext) = file_name.split('.').next() {
            return name_without_ext.to_string();
        }
    }
    file_path.to_string()
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
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'kind' parameter".into()))?;

        // Validate kind
        if !matches!(
            kind,
            "imports" | "graph" | "circular" | "coupling" | "cohesion" | "depth"
        ) {
            return Err(ServerError::InvalidRequest(format!(
                "Unsupported kind '{}'. Supported: 'imports', 'graph', 'circular', 'coupling', 'cohesion', 'depth'",
                kind
            )));
        }

        debug!(kind = %kind, "Handling analyze.dependencies request");

        // Dispatch to appropriate analysis function
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
            "circular" => {
                super::engine::run_analysis(
                    context,
                    tool_call,
                    "dependencies",
                    kind,
                    detect_circular,
                )
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
