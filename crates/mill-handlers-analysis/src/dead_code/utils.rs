use crate::suggestions::{
    self, EvidenceStrength, Location, RefactorType, RefactoringCandidate, Scope,
};
use mill_foundation::protocol::analysis_result::{Finding, SafetyLevel};
use mill_plugin_api::ParsedSource;
use serde_json::json;

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
pub(crate) fn is_symbol_used_in_code(content: &str, symbol: &str) -> bool {
    let mut occurrences = 0;
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
            occurrences += 1;
            if occurrences > 1 {
                return true;
            }
        }

        start_search_at = match_end;
    }

    false
}

/// Extract imported symbols from an import statement
///
/// This function looks for the actual import statement in the source code
/// and extracts the symbols being imported. It avoids using dynamic regexes
/// for performance.
///
/// # Parameters
/// - `lines`: All lines in the file
/// - `start_line`: The line number where the import was detected
/// - `module_path`: The module path to look for
/// - `language`: The language name for pattern matching
///
/// # Returns
/// A vector of symbol names that are imported
pub(crate) fn extract_imported_symbols(
    lines: &[&str],
    start_line: usize,
    module_path: &str,
    language: &str,
) -> Vec<String> {
    let mut symbols = Vec::new();

    // Collect the full import statement (handling multi-line)
    let mut statement = String::new();
    let mut i = start_line;
    let mut statement_end = false;

    // Safety check
    if i >= lines.len() {
        return symbols;
    }

    // Heuristic for statement termination
    // Rust/JS/TS: ends with ; (usually) or just check a reasonable number of lines
    let max_lines = 20; // Don't scan too far
    let mut lines_scanned = 0;

    while i < lines.len() && lines_scanned < max_lines {
        let line = lines[i].trim();
        statement.push_str(line);
        statement.push(' '); // Add space to avoid merging words

        // Check for termination
        match language.to_lowercase().as_str() {
            "rust" => {
                if line.ends_with(';') {
                    statement_end = true;
                }
            }
            "typescript" | "javascript" => {
                if line.ends_with(';')
                    || (line.contains("from") && (line.ends_with('\'') || line.ends_with('"')))
                {
                    // This is a rough heuristic, but should cover most cases
                    statement_end = true;
                }
            }
            _ => {}
        }

        if statement_end {
            break;
        }

        // If we found the module path and braces are closed, we might be done
        // (For languages without explicit terminators like Python)
        if statement.contains(module_path) {
            match language.to_lowercase().as_str() {
                "python" => {
                    // Python imports typically end at newline unless backslash used
                    // But we are concatenating lines.
                    // If the current line doesn't end with \, we assume end.
                    if !line.ends_with('\\') {
                        break;
                    }
                }
                _ => {}
            }
        }

        i += 1;
        lines_scanned += 1;
    }

    let statement = statement.trim();

    match language.to_lowercase().as_str() {
        "rust" => {
            // use module::{a, b}; or use module::Item;
            // Check if this statement actually matches the module path we are looking for
            // (It should, because detect_unused_imports found it here)

            if let Some(brace_start) = statement.find('{') {
                if let Some(brace_end) = statement.rfind('}') {
                    let inside = &statement[brace_start + 1..brace_end];
                    for part in inside.split(',') {
                        let part = part.trim();
                        // Handle `use ... as ...`
                        if let Some(alias_pos) = part.find(" as ") {
                            let alias = part[alias_pos + 4..].trim();
                            if !alias.is_empty() {
                                symbols.push(alias.to_string());
                            }
                        } else if !part.is_empty() {
                            symbols.push(part.to_string());
                        }
                    }
                }
            } else if !statement.contains('{') {
                // Case 1: `use std::collections::HashMap;` -> module_path = `std::collections::HashMap`.
                // Symbol is `HashMap`.
                if let Some(last_colon) = module_path.rfind("::") {
                    let symbol = &module_path[last_colon + 2..];
                    symbols.push(symbol.to_string());
                } else {
                    // `use HashMap;` -> symbol is HashMap
                    symbols.push(module_path.to_string());
                }
            }
        }
        "typescript" | "javascript" => {
            // import { a, b } from 'module';
            // import A from 'module';
            // import * as A from 'module';

            // Extract part before 'from'
            if let Some(from_pos) = statement.find("from") {
                let before_from = statement[..from_pos].trim();
                let after_import = if let Some(import_pos) = before_from.find("import") {
                    before_from[import_pos + 6..].trim()
                } else {
                    before_from // Should include import
                };

                // Handle braces
                if let Some(brace_start) = after_import.find('{') {
                    if let Some(brace_end) = after_import.rfind('}') {
                        // Named imports
                        let inside = &after_import[brace_start + 1..brace_end];
                        for part in inside.split(',') {
                            let part = part.trim();
                            // Handle `as`
                            if let Some(as_pos) = part.find(" as ") {
                                let alias = part[as_pos + 4..].trim();
                                symbols.push(alias.to_string());
                            } else if !part.is_empty() {
                                symbols.push(part.to_string());
                            }
                        }

                        // There might be default import too: `import A, { B } from ...`
                        if brace_start > 0 {
                            let default_part = &after_import[..brace_start];
                            if let Some(comma) = default_part.find(',') {
                                let default = default_part[..comma].trim();
                                if !default.is_empty() {
                                    symbols.push(default.to_string());
                                }
                            }
                        }
                    }
                } else {
                    // Default or namespace import
                    // import A from ...
                    // import * as A from ...
                    if let Some(stripped) = after_import.strip_prefix("* as ") {
                        let alias = stripped.trim();
                        symbols.push(alias.to_string());
                    } else {
                        symbols.push(after_import.to_string());
                    }
                }
            }
        }
        "python" => {
            // from module import a, b
            if statement.starts_with("from") {
                if let Some(import_pos) = statement.find(" import ") {
                    let after_import = &statement[import_pos + 8..];
                    for part in after_import.split(',') {
                        let part = part.trim();
                        // Handle `as`? Python: `import a as b`
                        if let Some(as_pos) = part.find(" as ") {
                            let alias = part[as_pos + 4..].trim();
                            symbols.push(alias.to_string());
                        } else if !part.is_empty() {
                            // Strip comments? #
                            let clean_part = part.split('#').next().unwrap_or(part).trim();
                            if !clean_part.is_empty() {
                                symbols.push(clean_part.to_string());
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }

    symbols
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
pub(crate) fn is_module_used_in_code(content: &str, module_path: &str) -> bool {
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

pub(crate) fn to_protocol_safety_level(level: suggestions::SafetyLevel) -> SafetyLevel {
    match level {
        suggestions::SafetyLevel::Safe => SafetyLevel::Safe,
        suggestions::SafetyLevel::RequiresReview => SafetyLevel::RequiresReview,
        suggestions::SafetyLevel::Experimental => SafetyLevel::Experimental,
    }
}

pub(crate) fn generate_dead_code_refactoring_candidates(
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
