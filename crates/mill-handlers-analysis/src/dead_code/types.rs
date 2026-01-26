use super::utils::is_symbol_used_in_code;
use crate::AnalysisConfig;
use mill_foundation::protocol::analysis_result::{
    Finding, FindingLocation, Position, Range, SafetyLevel, Severity, Suggestion,
};
use regex::Regex;
use serde_json::json;
use std::collections::HashMap;

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
