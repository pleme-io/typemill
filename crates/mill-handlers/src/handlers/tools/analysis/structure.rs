#![allow(dead_code, unused_variables)]

//! Structure analysis handler
//!
//! This module provides detection for code structure-related patterns including:
//! - Symbols: Extract and categorize all symbols by kind
//! - Hierarchy: Analyze class/module hierarchy structure
//! - Interfaces: Find interface/trait definitions
//! - Inheritance: Track inheritance chains and depth
//! - Modules: Analyze module organization patterns
//!
//! Uses the shared analysis engine for orchestration and focuses only on
//! detection logic.

use super::super::{ToolHandler, ToolHandlerContext};
use super::suggestions::{
    ActionableSuggestion, AnalysisContext, EvidenceStrength, Location, RefactoringCandidate,
    Scope, SuggestionGenerator, RefactorType,
};
use anyhow::Result;
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::protocol::analysis_result::{
    Finding, FindingLocation, SafetyLevel, Severity, Suggestion,
};
use mill_foundation::protocol::{ApiError as ServerError, ApiResult as ServerResult};
use mill_plugin_api::{Symbol, SymbolKind};
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::debug;

/// Detect and categorize all symbols in a file
///
/// This function extracts all symbols (functions, classes, interfaces, etc.)
/// and categorizes them by SymbolKind to provide a structural overview.
///
/// # Algorithm
/// 1. Iterate through all symbols from the parsed source
/// 2. Categorize each symbol by its SymbolKind
/// 3. Count symbols per category
/// 4. Calculate visibility breakdown (public/private heuristics)
/// 5. Generate findings with structural metrics
///
/// # Heuristics
/// - Uses SymbolKind enum from language plugin
/// - Visibility detection based on naming conventions (MVP)
/// - For TypeScript: 'export' keyword, for Rust: 'pub' keyword
/// - Full visibility analysis requires AST traversal (TODO)
///
/// # Future Enhancements
/// TODO: Add AST-based visibility analysis
/// TODO: Detect unused symbols (private symbols with no references)
/// TODO: Calculate symbol complexity scores
/// TODO: Group symbols by scope/namespace
///
/// # Parameters
/// - `complexity_report`: Not used for symbol detection
/// - `content`: The raw file content for visibility detection
/// - `symbols`: The parsed symbols from the language plugin
/// - `language`: The language name (e.g., "rust", "typescript")
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A single finding with:
/// - Metrics including total_symbols, symbols_by_kind, visibility_breakdown
/// - Low severity (informational)
/// - No suggestions (structural information only)
pub fn detect_symbols(
    _complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    symbols: &[Symbol],
    language: &str,
    file_path: &str,
    _registry: &crate::LanguagePluginRegistry,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Categorize symbols by kind
    let mut symbols_by_kind: HashMap<String, usize> = HashMap::new();
    for symbol in symbols {
        let kind_name = format!("{:?}", symbol.kind);
        *symbols_by_kind.entry(kind_name).or_insert(0) += 1;
    }

    // Calculate visibility breakdown (heuristic-based)
    let (public_count, private_count) = calculate_visibility(content, symbols, language);

    let total_symbols = symbols.len();

    let mut metrics = HashMap::new();
    metrics.insert("total_symbols".to_string(), json!(total_symbols));
    metrics.insert("symbols_by_kind".to_string(), json!(symbols_by_kind));
    metrics.insert(
        "visibility_breakdown".to_string(),
        json!({
            "public": public_count,
            "private": private_count,
        }),
    );

    let message = format!(
        "Symbol analysis: {} total symbols ({} public, {} private) across {} categories",
        total_symbols,
        public_count,
        private_count,
        symbols_by_kind.len()
    );

    findings.push(Finding {
        id: format!("symbols-{}", file_path),
        kind: "symbols".to_string(),
        severity: Severity::Low, // Informational
        location: FindingLocation {
            file_path: file_path.to_string(),
            range: None, // File-level finding
            symbol: None,
            symbol_kind: Some("module".to_string()),
        },
        metrics: Some(metrics),
        message,
        suggestions: vec![],
    });

    findings
}

/// Analyze class/module hierarchy structure
///
/// This function builds a hierarchy tree showing parent-child relationships
/// between classes and calculates hierarchy depth metrics.
///
/// # Algorithm
/// 1. Extract classes/structs from symbols
/// 2. Build parent-child relationships (MVP: heuristic-based)
/// 3. Calculate hierarchy depth using recursive traversal
/// 4. Identify root classes (no parents) and leaf classes (no children)
/// 5. Generate findings with hierarchy metrics
///
/// # Heuristics
/// - For MVP: Use complexity report and symbol analysis
/// - Parent-child detection via inheritance keywords (extends, implements)
/// - Depth calculation: levels from root to deepest leaf
/// - Deep hierarchies (> 5 levels) indicate fragile design
///
/// # Future Enhancements
/// TODO: Add AST-based inheritance graph construction
/// TODO: Detect diamond inheritance patterns
/// TODO: Calculate hierarchy breadth metrics
/// TODO: Suggest flattening strategies for deep hierarchies
///
/// # Parameters
/// - `complexity_report`: Used for class metrics
/// - `content`: The raw file content for inheritance detection
/// - `symbols`: The parsed symbols for class identification
/// - `language`: The language name for syntax patterns
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A single finding with:
/// - Metrics including max_depth, total_classes, hierarchy_tree
/// - Medium severity if depth > 5
/// - Suggestions to flatten hierarchy if too deep
pub fn detect_hierarchy(
    _complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    symbols: &[Symbol],
    language: &str,
    file_path: &str,
    _registry: &crate::LanguagePluginRegistry,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Extract classes/structs
    let classes: Vec<&Symbol> = symbols
        .iter()
        .filter(|s| {
            matches!(
                s.kind,
                SymbolKind::Class | SymbolKind::Struct | SymbolKind::Interface
            )
        })
        .collect();

    if classes.is_empty() {
        // No hierarchy to analyze
        return findings;
    }

    // Build hierarchy tree (MVP: simplified version)
    let hierarchy_tree = build_hierarchy_tree(content, &classes, language);

    // Calculate hierarchy depth
    let max_depth = calculate_max_hierarchy_depth(&hierarchy_tree);

    // Count root and leaf classes
    let root_classes: Vec<String> = hierarchy_tree
        .iter()
        .filter(|(_, info)| info.parent.is_none())
        .map(|(name, _)| name.clone())
        .collect();

    let leaf_classes: Vec<String> = hierarchy_tree
        .iter()
        .filter(|(_, info)| info.children.is_empty())
        .map(|(name, _)| name.clone())
        .collect();

    let total_classes = classes.len();
    let deep_hierarchy = max_depth > 5;

    let severity = if deep_hierarchy {
        Severity::Medium
    } else {
        Severity::Low
    };

    let mut metrics = HashMap::new();
    metrics.insert("max_depth".to_string(), json!(max_depth));
    metrics.insert("total_classes".to_string(), json!(total_classes));
    metrics.insert("root_classes".to_string(), json!(root_classes));
    metrics.insert("leaf_classes".to_string(), json!(leaf_classes));
    metrics.insert(
        "hierarchy_tree".to_string(),
        json!(serialize_hierarchy(&hierarchy_tree)),
    );

    let message = if deep_hierarchy {
        format!(
            "Deep hierarchy detected: {} levels with {} classes. Consider flattening for maintainability.",
            max_depth, total_classes
        )
    } else {
        format!(
            "Hierarchy analysis: {} levels with {} classes ({} roots, {} leaves)",
            max_depth,
            total_classes,
            root_classes.len(),
            leaf_classes.len()
        )
    };

    let mut finding = Finding {
        id: format!("hierarchy-{}", file_path),
        kind: "hierarchy".to_string(),
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

    if deep_hierarchy {
        let suggestion_generator = SuggestionGenerator::new();
        let context = AnalysisContext {
            file_path: file_path.to_string(),
            has_full_type_info: false,
            has_partial_type_info: false,
            ast_parse_errors: 0,
        };

        if let Ok(candidates) =
            generate_structure_refactoring_candidates(&finding)
        {
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

/// Find and analyze interface/trait definitions
///
/// This function identifies all interface/trait definitions and analyzes
/// their complexity, flagging "fat interfaces" that violate ISP.
///
/// # Algorithm
/// 1. Filter symbols for Interface/Trait kind
/// 2. For each interface, count methods (heuristic-based)
/// 3. Identify "fat interfaces" with > 10 methods
/// 4. Calculate interface complexity metrics
/// 5. Generate findings with ISP violation warnings
///
/// # Heuristics
/// - Interface detection: SymbolKind::Interface or 'interface'/'trait' keyword
/// - Method counting: Regex-based for MVP
/// - Fat interface threshold: > 10 methods (ISP guideline)
/// - For TypeScript: 'interface' keyword, for Rust: 'trait' keyword
///
/// # Future Enhancements
/// TODO: Add AST-based method extraction for accuracy
/// TODO: Detect interface segregation opportunities
/// TODO: Calculate interface cohesion metrics
/// TODO: Suggest interface splitting patterns
///
/// # Parameters
/// - `complexity_report`: Not used for interface detection
/// - `content`: The raw file content for method counting
/// - `symbols`: The parsed symbols for interface identification
/// - `language`: The language name for syntax patterns
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A vector of findings (one per interface), each with:
/// - Metrics including interface_count, methods_per_interface, fat_interfaces
/// - Medium severity if fat interfaces found
/// - Suggestions to split fat interfaces (ISP)
pub fn detect_interfaces(
    _complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    symbols: &[Symbol],
    language: &str,
    file_path: &str,
    _registry: &crate::LanguagePluginRegistry,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Filter for interface/trait symbols
    let interfaces: Vec<&Symbol> = symbols
        .iter()
        .filter(|s| matches!(s.kind, SymbolKind::Interface))
        .collect();

    // Also detect via regex if symbols don't have Interface kind
    let detected_interfaces = if interfaces.is_empty() {
        detect_interfaces_by_pattern(content, language)
    } else {
        Vec::new()
    };

    let all_interfaces_count = interfaces.len() + detected_interfaces.len();

    if all_interfaces_count == 0 {
        // No interfaces to analyze
        return findings;
    }

    // Analyze each interface
    let mut methods_per_interface: HashMap<String, usize> = HashMap::new();
    let mut fat_interfaces: Vec<String> = Vec::new();

    for interface in &interfaces {
        let method_count = count_interface_methods(content, &interface.name, language);
        methods_per_interface.insert(interface.name.clone(), method_count);

        if method_count > 10 {
            fat_interfaces.push(interface.name.clone());
        }
    }

    for (name, line) in &detected_interfaces {
        let method_count = count_interface_methods(content, name, language);
        methods_per_interface.insert(name.clone(), method_count);

        if method_count > 10 {
            fat_interfaces.push(name.clone());
        }
    }

    let has_fat_interfaces = !fat_interfaces.is_empty();
    let severity = if has_fat_interfaces {
        Severity::Medium
    } else {
        Severity::Low
    };

    let mut metrics = HashMap::new();
    metrics.insert("interface_count".to_string(), json!(all_interfaces_count));
    metrics.insert(
        "methods_per_interface".to_string(),
        json!(methods_per_interface),
    );
    metrics.insert("fat_interfaces".to_string(), json!(fat_interfaces));

    let message = if has_fat_interfaces {
        format!(
            "Fat interfaces detected: {} interface(s) with > 10 methods violate Interface Segregation Principle",
            fat_interfaces.len()
        )
    } else {
        format!(
            "Interface analysis: {} interface(s) found, all follow Interface Segregation Principle",
            all_interfaces_count
        )
    };

    let mut finding = Finding {
        id: format!("interfaces-{}", file_path),
        kind: "interfaces".to_string(),
        severity,
        location: FindingLocation {
            file_path: file_path.to_string(),
            range: None,
            symbol: None,
            symbol_kind: Some("interface".to_string()),
        },
        metrics: Some(metrics),
        message,
        suggestions: vec![],
    };

    if has_fat_interfaces {
        let suggestion_generator = SuggestionGenerator::new();
        let context = AnalysisContext {
            file_path: file_path.to_string(),
            has_full_type_info: false,
            has_partial_type_info: false,
            ast_parse_errors: 0,
        };

        if let Ok(candidates) =
            generate_structure_refactoring_candidates(&finding)
        {
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

/// Track and analyze inheritance chains
///
/// This function builds an inheritance graph and detects long inheritance
/// chains that may violate Liskov Substitution Principle.
///
/// # Algorithm
/// 1. Build inheritance graph from symbols and content
/// 2. Calculate inheritance depth for each class
/// 3. Detect long chains (> 4 levels)
/// 4. Identify inheritance patterns (linear vs branching)
/// 5. Generate findings with LSP concerns
///
/// # Heuristics
/// - Inheritance detection: extends/implements keywords
/// - Depth threshold: > 4 levels (fragile base class problem)
/// - For MVP: File-level analysis (not workspace-wide)
/// - Full analysis requires cross-file dependency tracking
///
/// # Future Enhancements
/// TODO: Add workspace-wide inheritance graph
/// TODO: Detect Liskov Substitution violations
/// TODO: Calculate inheritance coupling metrics
/// TODO: Suggest composition-over-inheritance refactoring
///
/// # Parameters
/// - `complexity_report`: Not used for inheritance detection
/// - `content`: The raw file content for inheritance parsing
/// - `symbols`: The parsed symbols for class identification
/// - `language`: The language name for syntax patterns
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A vector of findings with:
/// - Metrics including max_inheritance_depth, classes_by_depth, inheritance_chains
/// - High severity if depth > 4 (fragile base class)
/// - Suggestions to prefer composition over deep inheritance
pub fn detect_inheritance(
    _complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    symbols: &[Symbol],
    language: &str,
    file_path: &str,
    _registry: &crate::LanguagePluginRegistry,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Extract classes
    let classes: Vec<&Symbol> = symbols
        .iter()
        .filter(|s| matches!(s.kind, SymbolKind::Class | SymbolKind::Struct))
        .collect();

    if classes.is_empty() {
        return findings;
    }

    // Build inheritance graph
    let inheritance_graph = build_inheritance_graph(content, &classes, language);

    // Calculate inheritance depth for each class
    let mut classes_by_depth: HashMap<usize, Vec<String>> = HashMap::new();
    let mut max_depth = 0;

    for (class_name, parents) in &inheritance_graph {
        let depth = parents.len();
        classes_by_depth
            .entry(depth)
            .or_default()
            .push(class_name.clone());
        max_depth = max_depth.max(depth);
    }

    // Build inheritance chains
    let inheritance_chains: Vec<Vec<String>> = inheritance_graph
        .iter()
        .filter(|(_, parents)| !parents.is_empty())
        .map(|(class, parents)| {
            let mut chain = parents.clone();
            chain.push(class.clone());
            chain
        })
        .collect();

    let excessive_depth = max_depth > 4;
    let severity = if excessive_depth {
        Severity::High
    } else {
        Severity::Low
    };

    let mut metrics = HashMap::new();
    metrics.insert("max_inheritance_depth".to_string(), json!(max_depth));
    metrics.insert("classes_by_depth".to_string(), json!(classes_by_depth));
    metrics.insert("inheritance_chains".to_string(), json!(inheritance_chains));

    let message = if excessive_depth {
        format!(
            "Deep inheritance detected: {} levels. Long inheritance chains increase coupling and fragility (fragile base class problem).",
            max_depth
        )
    } else {
        format!(
            "Inheritance analysis: {} max depth with {} classes",
            max_depth,
            classes.len()
        )
    };

    let mut finding = Finding {
        id: format!("inheritance-{}", file_path),
        kind: "inheritance".to_string(),
        severity,
        location: FindingLocation {
            file_path: file_path.to_string(),
            range: None,
            symbol: None,
            symbol_kind: Some("class".to_string()),
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

        if let Ok(candidates) =
            generate_structure_refactoring_candidates(&finding)
        {
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

/// Analyze module organization patterns
///
/// This function examines how code is organized into modules/namespaces,
/// detecting "god modules" with too many items.
///
/// # Algorithm
/// 1. Extract module/namespace structure from symbols
/// 2. Count items per module (functions, classes, types)
/// 3. Detect "god modules" (> 50 items)
/// 4. Check for orphaned items (not in any module)
/// 5. Generate findings with organization metrics
///
/// # Heuristics
/// - Module detection: SymbolKind::Module or namespace analysis
/// - God module threshold: > 50 items (SRP violation)
/// - Orphaned items: Top-level symbols not in explicit modules
/// - For MVP: File-level analysis (not cross-file)
///
/// # Future Enhancements
/// TODO: Add workspace-wide module analysis
/// TODO: Calculate module cohesion metrics
/// TODO: Detect circular module dependencies
/// TODO: Suggest module splitting strategies
///
/// # Parameters
/// - `complexity_report`: Used for item count metrics
/// - `content`: The raw file content for module detection
/// - `symbols`: The parsed symbols for categorization
/// - `language`: The language name for module syntax
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A vector of findings with:
/// - Metrics including module_count, items_per_module, god_modules, orphaned_items
/// - Medium severity if god modules or many orphaned items
/// - Suggestions to split large modules or organize orphaned items
pub fn detect_modules(
    complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    symbols: &[Symbol],
    language: &str,
    file_path: &str,
    _registry: &crate::LanguagePluginRegistry,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Extract modules
    let modules: Vec<&Symbol> = symbols
        .iter()
        .filter(|s| matches!(s.kind, SymbolKind::Module))
        .collect();

    // Categorize symbols by module (MVP: simplified)
    let items_per_module = categorize_symbols_by_module(content, symbols, language);

    // Detect god modules (> 50 items)
    let god_modules: Vec<String> = items_per_module
        .iter()
        .filter(|(_, count)| **count > 50)
        .map(|(name, _)| name.clone())
        .collect();

    // Count orphaned items (symbols not in any explicit module)
    let orphaned_items: Vec<String> = symbols
        .iter()
        .filter(|s| !matches!(s.kind, SymbolKind::Module))
        .filter(|s| {
            // Heuristic: Check if symbol name suggests it's at top level
            !modules.iter().any(|m| s.name.starts_with(&m.name))
        })
        .map(|s| s.name.clone())
        .collect();

    let total_functions = complexity_report.total_functions;
    let total_items = symbols.len();

    let has_issues = !god_modules.is_empty() || orphaned_items.len() > 10;
    let severity = if has_issues {
        Severity::Medium
    } else {
        Severity::Low
    };

    let mut metrics = HashMap::new();
    metrics.insert("module_count".to_string(), json!(modules.len()));
    metrics.insert("items_per_module".to_string(), json!(items_per_module));
    metrics.insert("god_modules".to_string(), json!(god_modules));
    metrics.insert(
        "orphaned_items_count".to_string(),
        json!(orphaned_items.len()),
    );
    metrics.insert("total_items".to_string(), json!(total_items));
    metrics.insert("total_functions".to_string(), json!(total_functions));

    let message = if !god_modules.is_empty() {
        format!(
            "God modules detected: {} module(s) with > 50 items violate Single Responsibility Principle",
            god_modules.len()
        )
    } else if orphaned_items.len() > 10 {
        format!(
            "Module organization: {} orphaned items not organized into modules",
            orphaned_items.len()
        )
    } else {
        format!(
            "Module analysis: {} modules organizing {} items ({} functions)",
            modules.len(),
            total_items,
            total_functions
        )
    };

    let mut finding = Finding {
        id: format!("modules-{}", file_path),
        kind: "modules".to_string(),
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

    if has_issues {
        let suggestion_generator = SuggestionGenerator::new();
        let context = AnalysisContext {
            file_path: file_path.to_string(),
            has_full_type_info: false,
            has_partial_type_info: false,
            ast_parse_errors: 0,
        };

        if let Ok(candidates) =
            generate_structure_refactoring_candidates(&finding)
        {
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

/// Calculate visibility breakdown using heuristics
///
/// This function estimates public vs private symbol counts based on
/// language-specific patterns.
///
/// # Parameters
/// - `content`: The file content to search for visibility keywords
/// - `symbols`: The symbols to analyze
/// - `language`: The language for pattern matching
///
/// # Returns
/// A tuple of (public_count, private_count)
fn calculate_visibility(content: &str, symbols: &[Symbol], language: &str) -> (usize, usize) {
    let mut public_count = 0;
    let mut private_count = 0;

    let lines: Vec<&str> = content.lines().collect();

    for symbol in symbols {
        let line_idx = symbol.location.line.saturating_sub(1);
        if line_idx >= lines.len() {
            private_count += 1; // Default to private if line not found
            continue;
        }

        let line = lines[line_idx];

        match language.to_lowercase().as_str() {
            "rust" => {
                if line.contains("pub ") || line.contains("pub(") {
                    public_count += 1;
                } else {
                    private_count += 1;
                }
            }
            "typescript" | "javascript" => {
                if line.contains("export ") {
                    public_count += 1;
                } else {
                    private_count += 1;
                }
            }
            "python" => {
                // Python convention: names starting with _ are private
                if symbol.name.starts_with('_') {
                    private_count += 1;
                } else {
                    public_count += 1;
                }
            }
            _ => {
                // Default: assume public if no clear indicator
                public_count += 1;
            }
        }
    }

    (public_count, private_count)
}

/// Hierarchy node information
#[derive(Debug, Clone)]
struct HierarchyNode {
    parent: Option<String>,
    children: Vec<String>,
}

/// Build hierarchy tree from classes
///
/// This function constructs parent-child relationships between classes.
///
/// # Parameters
/// - `content`: The file content to parse for inheritance
/// - `classes`: The class symbols to analyze
/// - `language`: The language for pattern matching
///
/// # Returns
/// A HashMap mapping class names to their hierarchy information
fn build_hierarchy_tree(
    content: &str,
    classes: &[&Symbol],
    language: &str,
) -> HashMap<String, HierarchyNode> {
    let mut tree: HashMap<String, HierarchyNode> = HashMap::new();

    // Initialize all classes
    for class in classes {
        tree.insert(
            class.name.clone(),
            HierarchyNode {
                parent: None,
                children: Vec::new(),
            },
        );
    }

    // Detect parent-child relationships
    let lines: Vec<&str> = content.lines().collect();

    for class in classes {
        let line_idx = class.location.line.saturating_sub(1);
        if line_idx >= lines.len() {
            continue;
        }

        // Look for inheritance keywords
        let parent_opt = detect_parent_class(lines[line_idx], language);

        if let Some(parent) = parent_opt {
            // Update parent relationship
            if let Some(node) = tree.get_mut(&class.name) {
                node.parent = Some(parent.clone());
            }

            // Update children relationship
            if let Some(parent_node) = tree.get_mut(&parent) {
                parent_node.children.push(class.name.clone());
            }
        }
    }

    tree
}

/// Detect parent class from a class definition line
///
/// # Parameters
/// - `line`: The line containing the class definition
/// - `language`: The language for pattern matching
///
/// # Returns
/// The parent class name if found
fn detect_parent_class(line: &str, language: &str) -> Option<String> {
    match language.to_lowercase().as_str() {
        "typescript" | "javascript" => {
            // class Child extends Parent
            if let Some(pos) = line.find(" extends ") {
                let after_extends = &line[pos + 9..];
                if let Some(parent) = after_extends.split_whitespace().next() {
                    return Some(parent.trim_end_matches('{').trim().to_string());
                }
            }
        }
        "rust" => {
            // Rust uses traits, not inheritance
            // Look for trait implementations: impl Trait for Struct
            if line.contains(" for ") {
                // Not a typical parent-child relationship
                return None;
            }
        }
        "python" => {
            // class Child(Parent):
            if let Some(start) = line.find('(') {
                if let Some(end) = line.find(')') {
                    let parent = &line[start + 1..end];
                    return Some(parent.trim().to_string());
                }
            }
        }
        _ => {}
    }

    None
}

/// Calculate maximum hierarchy depth
///
/// # Parameters
/// - `tree`: The hierarchy tree
///
/// # Returns
/// The maximum depth of the hierarchy
fn calculate_max_hierarchy_depth(tree: &HashMap<String, HierarchyNode>) -> usize {
    let mut max_depth = 0;

    for class_name in tree.keys() {
        let depth = calculate_depth_recursive(class_name, tree, 0);
        max_depth = max_depth.max(depth);
    }

    max_depth
}

/// Calculate depth recursively
fn calculate_depth_recursive(
    class_name: &str,
    tree: &HashMap<String, HierarchyNode>,
    current_depth: usize,
) -> usize {
    if let Some(node) = tree.get(class_name) {
        if node.children.is_empty() {
            return current_depth;
        }

        let mut max_child_depth = current_depth;
        for child in &node.children {
            let child_depth = calculate_depth_recursive(child, tree, current_depth + 1);
            max_child_depth = max_child_depth.max(child_depth);
        }
        max_child_depth
    } else {
        current_depth
    }
}

/// Serialize hierarchy tree for JSON output
fn serialize_hierarchy(tree: &HashMap<String, HierarchyNode>) -> HashMap<String, Value> {
    let mut result = HashMap::new();

    for (class_name, node) in tree {
        result.insert(
            class_name.clone(),
            json!({
                "parent": node.parent,
                "children": node.children,
            }),
        );
    }

    result
}

/// Detect interfaces by pattern matching
///
/// # Parameters
/// - `content`: The file content to search
/// - `language`: The language for pattern matching
///
/// # Returns
/// A vector of (interface_name, line_number) tuples
fn detect_interfaces_by_pattern(content: &str, language: &str) -> Vec<(String, usize)> {
    let mut interfaces = Vec::new();

    let pattern = match language.to_lowercase().as_str() {
        "typescript" | "javascript" => r"interface\s+(\w+)",
        "rust" => r"trait\s+(\w+)",
        "python" => r"class\s+(\w+)\(.*Protocol\)",
        "go" => r"type\s+(\w+)\s+interface",
        _ => return interfaces,
    };

    if let Ok(re) = Regex::new(pattern) {
        for (line_num, line) in content.lines().enumerate() {
            if let Some(captures) = re.captures(line) {
                if let Some(name) = captures.get(1) {
                    interfaces.push((name.as_str().to_string(), line_num + 1));
                }
            }
        }
    }

    interfaces
}

/// Count methods in an interface
///
/// # Parameters
/// - `content`: The file content
/// - `interface_name`: The name of the interface
/// - `language`: The language for pattern matching
///
/// # Returns
/// The number of methods in the interface
fn count_interface_methods(content: &str, interface_name: &str, language: &str) -> usize {
    let lines: Vec<&str> = content.lines().collect();

    // Find the interface definition
    let interface_pattern = match language.to_lowercase().as_str() {
        "typescript" | "javascript" => format!(r"interface\s+{}", interface_name),
        "rust" => format!(r"trait\s+{}", interface_name),
        _ => return 0,
    };

    let Ok(interface_re) = Regex::new(&interface_pattern) else {
        return 0;
    };

    // Find interface start line
    let mut start_line = None;
    for (idx, line) in lines.iter().enumerate() {
        if interface_re.is_match(line) {
            start_line = Some(idx);
            break;
        }
    }

    let Some(start) = start_line else {
        return 0;
    };

    // Count methods within the interface block
    let mut method_count = 0;
    let mut brace_count = 0;
    let mut in_interface = false;

    for line in &lines[start..] {
        for ch in line.chars() {
            if ch == '{' {
                brace_count += 1;
                in_interface = true;
            } else if ch == '}' {
                brace_count -= 1;
                if brace_count == 0 && in_interface {
                    return method_count;
                }
            }
        }

        if in_interface && brace_count > 0 {
            // Count method signatures (lines with '(' and not comments)
            if line.contains('(') && !line.trim().starts_with("//") && !line.trim().starts_with('*')
            {
                method_count += 1;
            }
        }
    }

    method_count
}

/// Build inheritance graph
///
/// # Parameters
/// - `content`: The file content
/// - `classes`: The class symbols
/// - `language`: The language for pattern matching
///
/// # Returns
/// A HashMap mapping class names to their parent chain
fn build_inheritance_graph(
    content: &str,
    classes: &[&Symbol],
    language: &str,
) -> HashMap<String, Vec<String>> {
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();

    let lines: Vec<&str> = content.lines().collect();

    for class in classes {
        let line_idx = class.location.line.saturating_sub(1);
        if line_idx >= lines.len() {
            graph.insert(class.name.clone(), Vec::new());
            continue;
        }

        let line = lines[line_idx];
        let parent_opt = detect_parent_class(line, language);

        if let Some(parent) = parent_opt {
            // For MVP, just store direct parent
            // TODO: Build full transitive parent chain
            graph.insert(class.name.clone(), vec![parent]);
        } else {
            graph.insert(class.name.clone(), Vec::new());
        }
    }

    graph
}

/// Categorize symbols by module
///
/// # Parameters
/// - `content`: The file content
/// - `symbols`: The symbols to categorize
/// - `language`: The language for pattern matching
///
/// # Returns
/// A HashMap mapping module names to item counts
fn categorize_symbols_by_module(
    _content: &str,
    symbols: &[Symbol],
    _language: &str,
) -> HashMap<String, usize> {
    let mut items_per_module: HashMap<String, usize> = HashMap::new();

    // Extract modules
    let modules: Vec<&Symbol> = symbols
        .iter()
        .filter(|s| matches!(s.kind, SymbolKind::Module))
        .collect();

    // For MVP: Simple heuristic based on symbol name prefixes
    for symbol in symbols {
        if matches!(symbol.kind, SymbolKind::Module) {
            continue; // Skip modules themselves
        }

        // Check if symbol belongs to any module (by name prefix)
        let mut assigned = false;
        for module in &modules {
            if symbol.name.starts_with(&module.name) {
                *items_per_module.entry(module.name.clone()).or_insert(0) += 1;
                assigned = true;
                break;
            }
        }

        // If not assigned to any module, count as top-level
        if !assigned {
            *items_per_module
                .entry("(top-level)".to_string())
                .or_insert(0) += 1;
        }
    }

    items_per_module
}

fn generate_structure_refactoring_candidates(
    finding: &Finding,
) -> Result<Vec<RefactoringCandidate>> {
    let mut candidates = Vec::new();
    let location = finding.location.clone();
    let line = location.range.as_ref().map(|r| r.start.line).unwrap_or(0) as usize;

    match finding.kind.as_str() {
        "hierarchy" if finding.severity >= Severity::Medium => {
            // Suggest flattening hierarchy. This is a complex, multi-step refactoring.
        }
        "interfaces" if finding.severity >= Severity::Medium => {
            // Suggest splitting fat interfaces.
        }
        "inheritance" if finding.severity >= Severity::Medium => {
            // Suggest composition over inheritance.
        }
        "modules" if finding.severity >= Severity::Medium => {
            // Suggest splitting god modules.
        }
        _ => {}
    }

    Ok(candidates)
}


// ============================================================================
// Handler Implementation
// ============================================================================

pub struct StructureHandler;

impl StructureHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ToolHandler for StructureHandler {
    fn tool_names(&self) -> &[&str] {
        &["analyze.structure"]
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
            "symbols" | "hierarchy" | "interfaces" | "inheritance" | "modules"
        ) {
            return Err(ServerError::InvalidRequest(format!(
                "Unsupported kind '{}'. Supported: 'symbols', 'hierarchy', 'interfaces', 'inheritance', 'modules'",
                kind
            )));
        }

        debug!(kind = %kind, "Handling analyze.structure request");

        // Dispatch to appropriate analysis function
        match kind {
            "symbols" => {
                super::engine::run_analysis(context, tool_call, "structure", kind, detect_symbols)
                    .await
            }
            "hierarchy" => {
                super::engine::run_analysis(context, tool_call, "structure", kind, detect_hierarchy)
                    .await
            }
            "interfaces" => {
                super::engine::run_analysis(
                    context,
                    tool_call,
                    "structure",
                    kind,
                    detect_interfaces,
                )
                .await
            }
            "inheritance" => {
                super::engine::run_analysis(
                    context,
                    tool_call,
                    "structure",
                    kind,
                    detect_inheritance,
                )
                .await
            }
            "modules" => {
                super::engine::run_analysis(context, tool_call, "structure", kind, detect_modules)
                    .await
            }
            _ => unreachable!("Kind validated earlier"),
        }
    }
}