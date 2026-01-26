use crate::AnalysisConfig;
use mill_foundation::protocol::analysis_result::{
    Finding, FindingLocation, Position, Range, SafetyLevel, Severity, Suggestion,
};
use serde_json::json;
use std::collections::HashMap;

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
