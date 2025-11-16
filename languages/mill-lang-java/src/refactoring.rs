//! Java-specific refactoring operations
//!
//! Provides AST-based refactoring capabilities including:
//! - Extract function
//! - Extract variable
//! - Inline variable

use mill_foundation::protocol::{
    EditPlan, EditPlanMetadata, EditType, TextEdit, ValidationRule, ValidationType,
};
use mill_lang_common::find_literal_occurrences;
use mill_lang_common::is_escaped;
use mill_lang_common::is_screaming_snake_case;
use mill_lang_common::refactoring::CodeRange as CommonCodeRange;
use mill_lang_common::ExtractConstantAnalysis;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tree_sitter::{Node, Parser, Point, Query, QueryCursor, StreamingIterator};

fn get_language() -> tree_sitter::Language {
    // The tree-sitter-java grammar is compiled via build.rs and linked
    // This extern function is provided by the compiled C code
    use tree_sitter::ffi::TSLanguage;
    extern "C" {
        fn tree_sitter_java() -> *const TSLanguage;
    }
    unsafe { tree_sitter::Language::from_raw(tree_sitter_java()) }
}

/// Code range for refactoring operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

/// Error type for refactoring operations
#[derive(Debug, thiserror::Error)]
pub enum RefactoringError {
    #[error("Analysis error: {0}")]
    Analysis(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Query error: {0}")]
    Query(String),
}

pub type RefactoringResult<T> = Result<T, RefactoringError>;

/// Generate edit plan for extract function refactoring
pub(crate) fn plan_extract_function(
    source: &str,
    range: &CodeRange,
    function_name: &str,
    file_path: &str,
) -> RefactoringResult<EditPlan> {
    let mut parser = Parser::new();
    parser
        .set_language(&get_language())
        .map_err(|e| RefactoringError::Parse(format!("Failed to load Java grammar: {}", e)))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| RefactoringError::Parse("Failed to parse Java source".to_string()))?;
    let root = tree.root_node();

    let start_point = Point::new(range.start_line as usize - 1, range.start_col as usize);
    let end_point = Point::new(range.end_line as usize - 1, range.end_col as usize);

    let selected_node = find_smallest_node_containing_range(root, start_point, end_point)
        .ok_or_else(|| {
            RefactoringError::Analysis("Could not find a node for the selection.".to_string())
        })?;

    let selected_text = selected_node
        .utf8_text(source.as_bytes())
        .unwrap()
        .to_string();

    let enclosing_method =
        find_ancestor_of_kind(selected_node, "method_declaration").ok_or_else(|| {
            RefactoringError::Analysis("Selection is not inside a method.".to_string())
        })?;

    let indent = get_indentation(source, enclosing_method.start_position().row);
    let method_indent = format!("{}    ", indent);

    let new_method_text = format!(
        "\n\n{}private void {}() {{\n{}{}\n{}}}\n",
        indent,
        function_name,
        method_indent,
        selected_text.trim(),
        indent
    );

    let insert_edit = TextEdit {
        file_path: None,
        edit_type: EditType::Insert,
        location: CommonCodeRange::new(
            enclosing_method.end_position().row as u32,
            0,
            enclosing_method.end_position().row as u32,
            0,
        )
        .into(),
        original_text: String::new(),
        new_text: new_method_text,
        priority: 100,
        description: format!("Create new method '{}'", function_name),
    };

    let replace_edit = TextEdit {
        file_path: None,
        edit_type: EditType::Replace,
        location: node_to_location(selected_node).into(),
        original_text: selected_text,
        new_text: format!("{}();", function_name),
        priority: 90,
        description: format!("Replace selection with call to '{}'", function_name),
    };

    Ok(EditPlan {
        source_file: file_path.to_string(),
        edits: vec![insert_edit, replace_edit],
        dependency_updates: vec![],
        validations: vec![ValidationRule {
            rule_type: ValidationType::SyntaxCheck,
            description: "Verify syntax after extraction".to_string(),
            parameters: HashMap::new(),
        }],
        metadata: EditPlanMetadata {
            intent_name: "extract_function".to_string(),
            intent_arguments: serde_json::json!({ "function_name": function_name }),
            created_at: chrono::Utc::now(),
            complexity: 3,
            impact_areas: vec!["function_extraction".to_string()],
            consolidation: None,
        },
    })
}

/// Generate edit plan for extract variable refactoring
pub(crate) fn plan_extract_variable(
    source: &str,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    variable_name: Option<String>,
    file_path: &str,
) -> RefactoringResult<EditPlan> {
    let mut parser = Parser::new();
    parser
        .set_language(&get_language())
        .map_err(|e| RefactoringError::Parse(format!("Failed to load Java grammar: {}", e)))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| RefactoringError::Parse("Failed to parse Java source".to_string()))?;
    let root = tree.root_node();

    let start_point = Point::new(start_line as usize - 1, start_col as usize);
    let end_point = Point::new(end_line as usize - 1, end_col as usize);

    let selected_node = find_smallest_node_containing_range(root, start_point, end_point)
        .ok_or_else(|| {
            RefactoringError::Analysis("Could not find a node for the selection.".to_string())
        })?;

    let expression_text = selected_node
        .utf8_text(source.as_bytes())
        .unwrap()
        .to_string();

    let insertion_node = find_ancestor_of_kind(selected_node, "statement")
        .or_else(|| find_ancestor_of_kind(selected_node, "local_variable_declaration"))
        .ok_or_else(|| {
            RefactoringError::Analysis("Could not find statement to insert before.".to_string())
        })?;

    let indent = get_indentation(source, insertion_node.start_position().row);
    let var_name = variable_name.unwrap_or_else(|| "extracted".to_string());

    let var_type = if expression_text.starts_with('"') {
        "String"
    } else if expression_text.parse::<i32>().is_ok() {
        "int"
    } else {
        "var"
    };

    let declaration_text = format!("{} {} = {};\n", var_type, var_name, expression_text);

    let insert_edit = TextEdit {
        file_path: None,
        edit_type: EditType::Insert,
        location: CommonCodeRange::new(
            insertion_node.start_position().row as u32 + 1,
            0,
            insertion_node.start_position().row as u32 + 1,
            0,
        )
        .into(),
        original_text: String::new(),
        new_text: format!("{}{}", indent, declaration_text),
        priority: 100,
        description: format!("Declare new variable '{}'", var_name),
    };

    let replace_edit = TextEdit {
        file_path: None,
        edit_type: EditType::Replace,
        location: node_to_location(selected_node).into(),
        original_text: expression_text.clone(),
        new_text: var_name.clone(),
        priority: 90,
        description: format!("Replace expression with '{}'", var_name),
    };

    Ok(EditPlan {
        source_file: file_path.to_string(),
        edits: vec![insert_edit, replace_edit],
        dependency_updates: vec![],
        validations: vec![ValidationRule {
            rule_type: ValidationType::SyntaxCheck,
            description: "Verify syntax after extraction".to_string(),
            parameters: HashMap::new(),
        }],
        metadata: EditPlanMetadata {
            intent_name: "extract_variable".to_string(),
            intent_arguments: serde_json::json!({
                "expression": expression_text,
                "variable_name": var_name,
            }),
            created_at: chrono::Utc::now(),
            complexity: 2,
            impact_areas: vec!["variable_extraction".to_string()],
            consolidation: None,
        },
    })
}

/// Generate edit plan for inline variable refactoring
pub(crate) fn plan_inline_variable(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    file_path: &str,
) -> RefactoringResult<EditPlan> {
    let mut parser = Parser::new();
    parser
        .set_language(&get_language())
        .map_err(|e| RefactoringError::Parse(format!("Failed to load Java grammar: {}", e)))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| RefactoringError::Parse("Failed to parse Java source".to_string()))?;
    let root = tree.root_node();
    let point = Point::new(variable_line as usize - 1, variable_col as usize);

    let var_ident_node = find_node_at_point(root, point).ok_or_else(|| {
        RefactoringError::Analysis("Could not find variable at specified location.".to_string())
    })?;

    let (var_name, var_value, declaration_node) = extract_java_var_info(var_ident_node, source)?;

    let scope_node =
        find_ancestor_of_kind(declaration_node, "method_declaration").ok_or_else(|| {
            RefactoringError::Analysis("Variable is not inside a method.".to_string())
        })?;

    let mut edits = Vec::new();
    let query_str = format!(r#"((identifier) @ref (#eq? @ref "{}"))"#, var_name);
    let query = Query::new(&get_language(), &query_str)
        .map_err(|e| RefactoringError::Query(e.to_string()))?;
    let mut cursor = QueryCursor::new();

    cursor
        .matches(&query, scope_node, source.as_bytes())
        .for_each(|match_| {
            for capture in match_.captures {
                let reference_node = capture.node;
                if reference_node.id() != var_ident_node.id() {
                    edits.push(TextEdit {
                        file_path: None,
                        edit_type: EditType::Replace,
                        location: node_to_location(reference_node).into(),
                        original_text: var_name.clone(),
                        new_text: var_value.clone(),
                        priority: 90,
                        description: format!("Inline variable '{}'", var_name),
                    });
                }
            }
        });

    edits.push(TextEdit {
        file_path: None,
        edit_type: EditType::Delete,
        location: node_to_location(declaration_node).into(),
        original_text: declaration_node
            .utf8_text(source.as_bytes())
            .unwrap()
            .to_string(),
        new_text: String::new(),
        priority: 100,
        description: format!("Remove declaration of '{}'", var_name),
    });

    Ok(EditPlan {
        source_file: file_path.to_string(),
        edits,
        dependency_updates: vec![],
        validations: vec![ValidationRule {
            rule_type: ValidationType::SyntaxCheck,
            description: "Verify syntax is valid".to_string(),
            parameters: HashMap::new(),
        }],
        metadata: EditPlanMetadata {
            intent_name: "inline_variable".to_string(),
            intent_arguments: serde_json::json!({ "variable_name": var_name }),
            created_at: chrono::Utc::now(),
            complexity: 4,
            impact_areas: vec!["variable_inlining".to_string()],
            consolidation: None,
        },
    })
}

// Helper functions
fn find_smallest_node_containing_range<'a>(
    node: Node<'a>,
    start_point: Point,
    end_point: Point,
) -> Option<Node<'a>> {
    if node.start_position() > start_point || node.end_position() < end_point {
        return None;
    }

    for child in node.children(&mut node.walk()) {
        if let Some(containing_child) =
            find_smallest_node_containing_range(child, start_point, end_point)
        {
            return Some(containing_child);
        }
    }

    Some(node)
}

fn find_node_at_point<'a>(node: Node<'a>, point: Point) -> Option<Node<'a>> {
    node.named_descendant_for_point_range(point, point)
}

fn find_ancestor_of_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut current = Some(node);
    while let Some(current_node) = current {
        if current_node.kind() == kind {
            return Some(current_node);
        }
        current = current_node.parent();
    }
    None
}

fn get_indentation(source: &str, line: usize) -> String {
    source
        .lines()
        .nth(line)
        .map(|l| l.chars().take_while(|c| c.is_whitespace()).collect())
        .unwrap_or_default()
}

fn node_to_location(node: Node) -> CommonCodeRange {
    let range = node.range();
    CommonCodeRange::new(
        range.start_point.row as u32 + 1,
        range.start_point.column as u32,
        range.end_point.row as u32 + 1,
        range.end_point.column as u32,
    )
}

fn extract_java_var_info<'a>(
    node: Node<'a>,
    source: &str,
) -> RefactoringResult<(String, String, Node<'a>)> {
    let declaration_node =
        find_ancestor_of_kind(node, "local_variable_declaration").ok_or_else(|| {
            RefactoringError::Analysis("Not a local variable declaration".to_string())
        })?;

    let declarator = declaration_node
        .children_by_field_name("declarator", &mut declaration_node.walk())
        .find(|d| {
            d.range().start_byte <= node.range().start_byte
                && d.range().end_byte >= node.range().end_byte
        })
        .ok_or_else(|| {
            RefactoringError::Analysis("Could not find variable declarator".to_string())
        })?;

    let name_node = declarator
        .child_by_field_name("name")
        .ok_or_else(|| RefactoringError::Analysis("Could not find variable name".to_string()))?;
    let value_node = declarator
        .child_by_field_name("value")
        .ok_or_else(|| RefactoringError::Analysis("Could not find variable value".to_string()))?;

    let name = name_node.utf8_text(source.as_bytes()).unwrap().to_string();
    let value = value_node.utf8_text(source.as_bytes()).unwrap().to_string();

    Ok((name, value, declaration_node))
}

/// Analyzes source code to extract information about a literal value at a cursor position.
///
/// This analysis function identifies literals in Java source code and gathers information for
/// constant extraction. It analyzes:
/// - The literal value at the specified cursor position (number, string, boolean, or null)
/// - All occurrences of that literal throughout the file
/// - A suitable insertion point for the constant declaration (class level)
/// - Whether extraction is valid and any blocking reasons
///
/// # Arguments
/// * `source` - The Java source code
/// * `line` - Zero-based line number where the cursor is positioned
/// * `character` - Zero-based character offset within the line
/// * `file_path` - Path to the file (used for error reporting)
///
/// # Returns
/// * `Ok(ExtractConstantAnalysis)` - Analysis result with literal value, occurrence ranges,
///                                     validation status, and insertion point
/// * `Err(RefactoringError)` - If no literal is found at the cursor position
pub(crate) fn analyze_extract_constant(
    source: &str,
    line: u32,
    character: u32,
    _file_path: &str,
) -> RefactoringResult<ExtractConstantAnalysis> {
    let lines: Vec<&str> = source.lines().collect();

    // Get the line at cursor position
    let line_text = lines
        .get(line as usize)
        .ok_or_else(|| RefactoringError::Analysis("Invalid line number".to_string()))?;

    // Find the literal at the cursor position
    let found_literal = find_java_literal_at_position(line_text, character as usize)
        .ok_or_else(|| {
            RefactoringError::Analysis("No literal found at the specified location".to_string())
        })?;

    let literal_value = found_literal.0;
    let is_valid_literal = !literal_value.is_empty();
    let blocking_reasons = if !is_valid_literal {
        vec!["Could not extract literal at cursor position".to_string()]
    } else {
        vec![]
    };

    // Find all occurrences of this literal value in the source
    let occurrence_ranges = find_literal_occurrences(source, &literal_value, is_valid_java_literal_location);

    // Insertion point: after class declaration, at class level
    let insertion_point = find_java_insertion_point_for_constant(source)?;

    Ok(ExtractConstantAnalysis {
        literal_value,
        occurrence_ranges,
        is_valid_literal,
        blocking_reasons,
        insertion_point: CommonCodeRange {
            start_line: insertion_point.start_line,
            start_col: insertion_point.start_col,
            end_line: insertion_point.end_line,
            end_col: insertion_point.end_col,
        },
    })
}

/// Extracts a literal value to a named constant in Java code.
///
/// This refactoring operation replaces all occurrences of a literal (number, string, boolean, or null)
/// with a named constant declaration at the class level, improving code maintainability.
///
/// # Arguments
/// * `source` - The Java source code
/// * `line` - Zero-based line number where the cursor is positioned
/// * `character` - Zero-based character offset within the line
/// * `name` - The constant name (must be SCREAMING_SNAKE_CASE)
/// * `file_path` - Path to the file being refactored
///
/// # Returns
/// * `Ok(EditPlan)` - The edit plan with constant declaration inserted at class level
/// * `Err(RefactoringError)` - If the cursor is not on a literal or the name is invalid
pub(crate) fn plan_extract_constant(
    source: &str,
    line: u32,
    character: u32,
    name: &str,
    file_path: &str,
) -> RefactoringResult<EditPlan> {
    let analysis = analyze_extract_constant(source, line, character, file_path)?;

    if !analysis.is_valid_literal {
        return Err(RefactoringError::Analysis(format!(
            "Cannot extract constant: {}",
            analysis.blocking_reasons.join(", ")
        )));
    }

    // Validate that the name is in SCREAMING_SNAKE_CASE format
    if !is_screaming_snake_case(name) {
        return Err(RefactoringError::Analysis(format!(
            "Constant name '{}' must be in SCREAMING_SNAKE_CASE format. Valid examples: TAX_RATE, MAX_VALUE, API_KEY, DB_TIMEOUT_MS. Requirements: only uppercase letters (A-Z), digits (0-9), and underscores; must contain at least one uppercase letter; cannot start or end with underscore.",
            name
        )));
    }

    let mut edits = Vec::new();

    // Determine constant type based on literal value
    let java_type = infer_java_type(&analysis.literal_value);

    // Generate the constant declaration (Java style: private static final)
    let indent = get_indentation(source, analysis.insertion_point.start_line as usize);
    let declaration = format!(
        "{}private static final {} {} = {};\n",
        indent, java_type, name, analysis.literal_value
    );

    edits.push(TextEdit {
        file_path: None,
        edit_type: EditType::Insert,
        location: CommonCodeRange::new(
            analysis.insertion_point.start_line + 1,
            analysis.insertion_point.start_col,
            analysis.insertion_point.end_line + 1,
            analysis.insertion_point.end_col,
        )
        .into(),
        original_text: String::new(),
        new_text: declaration,
        priority: 100,
        description: format!(
            "Extract '{}' into constant '{}'",
            analysis.literal_value, name
        ),
    });

    // Replace all occurrences of the literal with the constant name
    for (idx, occurrence_range) in analysis.occurrence_ranges.iter().enumerate() {
        let priority = 90_u32.saturating_sub(idx as u32);
        edits.push(TextEdit {
            file_path: None,
            edit_type: EditType::Replace,
            location: CommonCodeRange::new(
                occurrence_range.start_line + 1,
                occurrence_range.start_col,
                occurrence_range.end_line + 1,
                occurrence_range.end_col,
            )
            .into(),
            original_text: analysis.literal_value.clone(),
            new_text: name.to_string(),
            priority,
            description: format!(
                "Replace occurrence {} of literal with constant '{}'",
                idx + 1,
                name
            ),
        });
    }

    Ok(EditPlan {
        source_file: file_path.to_string(),
        edits,
        dependency_updates: Vec::new(),
        validations: vec![ValidationRule {
            rule_type: ValidationType::SyntaxCheck,
            description: "Verify Java syntax is valid after constant extraction".to_string(),
            parameters: HashMap::new(),
        }],
        metadata: EditPlanMetadata {
            intent_name: "extract_constant".to_string(),
            intent_arguments: serde_json::json!({
                "literal": analysis.literal_value,
                "constantName": name,
                "occurrences": analysis.occurrence_ranges.len(),
            }),
            created_at: chrono::Utc::now(),
            complexity: (analysis.occurrence_ranges.len().min(10)) as u8,
            impact_areas: vec!["constant_extraction".to_string()],
            consolidation: None,
        },
    })
}

/// Finds a Java literal at a given position in a line of code
fn find_java_literal_at_position(line_text: &str, col: usize) -> Option<(String, CodeRange)> {
    // Try numeric literal first
    if let Some((literal, range)) = find_java_numeric_literal(line_text, col) {
        return Some((literal, range));
    }

    // Try string literal
    if let Some((literal, range)) = find_java_string_literal(line_text, col) {
        return Some((literal, range));
    }

    // Try boolean/null keywords
    if let Some((literal, range)) = find_java_keyword_literal(line_text, col) {
        return Some((literal, range));
    }

    None
}

/// Finds a numeric literal at a cursor position in Java code
fn find_java_numeric_literal(line_text: &str, col: usize) -> Option<(String, CodeRange)> {
    if col >= line_text.len() {
        return None;
    }

    let chars: Vec<char> = line_text.chars().collect();
    if col >= chars.len() {
        return None;
    }

    // Check for hexadecimal literal (0x or 0X prefix)
    let is_hex = col >= 2 && chars[col - 1] == 'x' || chars[col - 1] == 'X' && chars[col - 2] == '0'
        || col >= 1 && chars[col] == 'x' || chars[col] == 'X' && col > 0 && chars[col - 1] == '0'
        || col > 0 && chars[col - 1].is_ascii_hexdigit() && col >= 2 && (chars[col - 2] == 'x' || chars[col - 2] == 'X');

    // If we're in a hex literal, find its boundaries
    if is_hex {
        // Find the start (should be '0x' or '0X')
        let mut start = col;
        while start > 0 {
            if chars[start] == '0' && start + 1 < chars.len() && (chars[start + 1] == 'x' || chars[start + 1] == 'X') {
                break;
            }
            if !chars[start].is_ascii_hexdigit() && chars[start] != 'x' && chars[start] != 'X' && chars[start] != '_' {
                start += 1;
                break;
            }
            start -= 1;
        }

        // Find the end
        let mut end = col;
        let mut found_x = false;
        for i in start..chars.len() {
            if chars[i] == 'x' || chars[i] == 'X' {
                found_x = true;
                end = i + 1;
            } else if found_x && (chars[i].is_ascii_hexdigit() || chars[i] == '_' || chars[i] == 'L' || chars[i] == 'l') {
                end = i + 1;
            } else if found_x {
                break;
            } else {
                end = i + 1;
            }
        }

        if start < end && end <= chars.len() {
            let text: String = chars[start..end].iter().collect();
            if text.starts_with("0x") || text.starts_with("0X") {
                return Some((
                    text,
                    CodeRange {
                        start_line: 0,
                        start_col: start as u32,
                        end_line: 0,
                        end_col: end as u32,
                    },
                ));
            }
        }
    }

    // Handle decimal literals
    // Find the start of the number (handle negative sign)
    let start = if col > 0 && chars[col - 1] == '-' {
        col.saturating_sub(1)
    } else {
        let mut s = col;
        while s > 0 {
            s -= 1;
            if !chars[s].is_ascii_digit() && chars[s] != '.' && chars[s] != '_' {
                s += 1;
                break;
            }
        }
        s
    };

    // Adjust start if we found a leading minus sign
    let actual_start = if start > 0 && chars[start - 1] == '-' {
        start - 1
    } else {
        start
    };

    // Find the end of the number
    let mut end = col;
    for i in col..chars.len() {
        let c = chars[i];
        if c.is_ascii_digit() || c == '.' || c == '_' || c == 'f' || c == 'F' || c == 'L' || c == 'l' || c == 'd' || c == 'D' {
            end = i + 1;
        } else {
            break;
        }
    }

    if actual_start < end && end <= chars.len() {
        let text: String = chars[actual_start..end].iter().collect();
        // Validate: must contain at least one digit
        if text.chars().any(|c| c.is_ascii_digit()) {
            return Some((
                text,
                CodeRange {
                    start_line: 0,
                    start_col: actual_start as u32,
                    end_line: 0,
                    end_col: end as u32,
                },
            ));
        }
    }

    None
}

/// Finds a string literal at a cursor position in Java code
fn find_java_string_literal(line_text: &str, col: usize) -> Option<(String, CodeRange)> {
    if col >= line_text.len() {
        return None;
    }

    // Look for opening quote before cursor
    let chars: Vec<char> = line_text.chars().collect();
    let mut opening_quote_pos = None;

    for i in (0..col).rev() {
        if i < chars.len() && chars[i] == '"' && !is_escaped(line_text, i) {
            opening_quote_pos = Some(i);
            break;
        }
    }

    if let Some(start) = opening_quote_pos {
        // Find closing quote after cursor, skipping escaped quotes
        for j in col..chars.len() {
            if chars[j] == '"' && !is_escaped(line_text, j) {
                let end = j + 1;
                let literal = line_text[start..end].to_string();
                return Some((
                    literal,
                    CodeRange {
                        start_line: 0,
                        start_col: start as u32,
                        end_line: 0,
                        end_col: end as u32,
                    },
                ));
            }
        }
    }

    None
}

/// Finds a Java keyword literal (true, false, or null) at a cursor position
fn find_java_keyword_literal(line_text: &str, col: usize) -> Option<(String, CodeRange)> {
    let keywords = ["true", "false", "null"];

    for keyword in &keywords {
        // Try to match keyword at or near cursor
        for start in col
            .saturating_sub(keyword.len())
            ..=col.min(line_text.len().saturating_sub(keyword.len()))
        {
            if start + keyword.len() <= line_text.len() {
                if &line_text[start..start + keyword.len()] == *keyword {
                    // Check word boundaries
                    let before_ok = start == 0
                        || !line_text[..start]
                            .ends_with(|c: char| c.is_alphanumeric() || c == '_');
                    let after_ok = start + keyword.len() == line_text.len()
                        || !line_text[start + keyword.len()..]
                            .starts_with(|c: char| c.is_alphanumeric() || c == '_');

                    if before_ok && after_ok {
                        return Some((
                            keyword.to_string(),
                            CodeRange {
                                start_line: 0,
                                start_col: start as u32,
                                end_line: 0,
                                end_col: (start + keyword.len()) as u32,
                            },
                        ));
                    }
                }
            }
        }
    }

    None
}

/// Validates whether a position in source code is a valid location for a literal
fn is_valid_java_literal_location(line: &str, pos: usize, _len: usize) -> bool {
    // Count non-escaped quotes before position to determine if we're inside a string literal
    let before = &line[..pos];
    let mut double_quotes = 0;
    for (i, ch) in before.char_indices() {
        if ch == '"' && !is_escaped(before, i) {
            double_quotes += 1;
        }
    }

    // If odd number of quotes appear before the position, we're inside a string literal
    if double_quotes % 2 == 1 {
        return false;
    }

    // Check for single-line comment marker (//). Anything after it is a comment.
    if let Some(comment_pos) = line.find("//") {
        if pos > comment_pos {
            return false;
        }
    }

    // Check for block comment markers (/* ... */)
    // This is a simplified check - doesn't handle multi-line block comments
    // but catches single-line block comments like /* comment */ code
    if let Some(block_start) = line.find("/*") {
        if pos > block_start {
            // Check if we're before the closing */
            if let Some(block_end) = line[block_start..].find("*/") {
                let actual_block_end = block_start + block_end + 2; // +2 for */
                if pos < actual_block_end {
                    return false;
                }
            } else {
                // Block comment opened but not closed on this line - assume we're in it
                return false;
            }
        }
    }

    true
}

/// Finds the appropriate insertion point for a constant declaration in Java code
fn find_java_insertion_point_for_constant(source: &str) -> RefactoringResult<CodeRange> {
    let lines: Vec<&str> = source.lines().collect();
    let mut insertion_line = 0;
    let mut found_class = false;

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let line_idx = idx as u32;

        // Look for class declaration
        if trimmed.contains("class ") && !found_class {
            found_class = true;
            // Look for opening brace
            if trimmed.contains('{') {
                insertion_line = line_idx + 1;
                break;
            }
        } else if found_class && trimmed.contains('{') {
            insertion_line = line_idx + 1;
            break;
        }
    }

    Ok(CodeRange {
        start_line: insertion_line,
        start_col: 0,
        end_line: insertion_line,
        end_col: 0,
    })
}

/// Infer Java type from literal value
fn infer_java_type(literal_value: &str) -> &'static str {
    if literal_value.starts_with('"') {
        "String"
    } else if literal_value == "true" || literal_value == "false" {
        "boolean"
    } else if literal_value == "null" {
        "Object"
    } else if literal_value.starts_with("0x") || literal_value.starts_with("0X") {
        // Hexadecimal literal
        if literal_value.ends_with('L') || literal_value.ends_with('l') {
            "long"
        } else {
            "int"
        }
    } else if literal_value.contains('.') || literal_value.ends_with('f') || literal_value.ends_with('F') {
        if literal_value.ends_with('f') || literal_value.ends_with('F') {
            "float"
        } else {
            "double"
        }
    } else if literal_value.ends_with('L') || literal_value.ends_with('l') {
        "long"
    } else {
        "int"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_screaming_snake_case() {
        assert!(is_screaming_snake_case("TAX_RATE"));
        assert!(is_screaming_snake_case("MAX_VALUE"));
        assert!(is_screaming_snake_case("A"));
        assert!(is_screaming_snake_case("PI"));

        assert!(!is_screaming_snake_case(""));
        assert!(!is_screaming_snake_case("_TAX_RATE")); // starts with underscore
        assert!(!is_screaming_snake_case("TAX_RATE_")); // ends with underscore
        assert!(!is_screaming_snake_case("tax_rate")); // lowercase
        assert!(!is_screaming_snake_case("TaxRate")); // camelCase
        assert!(!is_screaming_snake_case("tax-rate")); // kebab-case
    }

    #[test]
    fn test_plan_extract_constant_valid_number() {
        let source = r#"
public class Main {
    public void method() {
        int x = 42;
        int y = 42;
    }
}
"#;
        let result = plan_extract_constant(source, 3, 16, "ANSWER", "Main.java");
        assert!(
            result.is_ok(),
            "Should extract numeric literal successfully: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_plan_extract_constant_string() {
        let source = r#"
public class Main {
    public void method() {
        String msg = "hello";
        String greeting = "hello";
    }
}
"#;
        // Column 22 is inside the string literal "hello"
        let result = plan_extract_constant(source, 3, 22, "GREETING_MSG", "Main.java");
        assert!(
            result.is_ok(),
            "Should extract string literal: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_plan_extract_constant_boolean() {
        let source = r#"
public class Main {
    public void method() {
        boolean debug = true;
        boolean verbose = true;
    }
}
"#;
        let result = plan_extract_constant(source, 3, 24, "DEBUG_MODE", "Main.java");
        assert!(result.is_ok(), "Should extract boolean literal");
    }

    #[test]
    fn test_plan_extract_constant_invalid_name() {
        let source = r#"
public class Main {
    public void method() {
        int x = 42;
    }
}
"#;
        let result = plan_extract_constant(source, 3, 16, "answer", "Main.java");
        assert!(result.is_err(), "Should reject lowercase name");
    }

    #[test]
    fn test_is_escaped() {
        assert!(!is_escaped("hello", 0));
        assert!(!is_escaped("hello", 2));
        assert!(is_escaped(r#"\"hello"#, 1)); // \" - quote is escaped
        assert!(is_escaped(r#"\\"#, 1)); // \\ - second backslash IS escaped by the first
        assert!(!is_escaped(r#"\\\"#, 2)); // \\\ - third backslash is NOT escaped (two backslashes before it)
        assert!(is_escaped(r#"\\\\"#, 3)); // \\\\ - fourth backslash IS escaped by the third
    }

    #[test]
    fn test_find_java_string_literal_with_escaped_quotes() {
        let line = r#"String msg = "He said \"hello\"";"#;
        // Position 20 is inside the string literal, after the opening quote
        let result = find_java_string_literal(line, 20);
        assert!(result.is_some(), "Should find string with escaped quotes");
        let (literal, _) = result.unwrap();
        // Expected: opening quote + He said + escaped quote + hello + escaped quote + closing quote
        assert_eq!(literal, "\"He said \\\"hello\\\"\"");
    }

    #[test]
    fn test_find_java_string_literal_multiple_escapes() {
        let line = r#"String path = "C:\\Users\\Admin\\file.txt";"#;
        // Position 20 is inside the string literal
        let result = find_java_string_literal(line, 20);
        assert!(result.is_some(), "Should find string with escaped backslashes");
        let (literal, _) = result.unwrap();
        assert_eq!(literal, r#""C:\\Users\\Admin\\file.txt""#);
    }

    #[test]
    fn test_find_java_numeric_literal_hex() {
        let line = "int color = 0xFF00AA;";
        // Position 15 is inside the hex literal
        let result = find_java_numeric_literal(line, 15);
        assert!(result.is_some(), "Should find hex literal");
        let (literal, range) = result.unwrap();
        assert_eq!(literal, "0xFF00AA");
        assert_eq!(range.start_col, 12);
        assert_eq!(range.end_col, 20);
    }

    #[test]
    fn test_find_java_numeric_literal_hex_lowercase() {
        let line = "int mask = 0xdeadbeef;";
        // Position 14 is inside the hex literal
        let result = find_java_numeric_literal(line, 14);
        assert!(result.is_some(), "Should find lowercase hex literal");
        let (literal, _) = result.unwrap();
        assert_eq!(literal, "0xdeadbeef");
    }

    #[test]
    fn test_find_java_numeric_literal_hex_long() {
        let line = "long value = 0xFFFFFFFFL;";
        // Position 16 is inside the hex literal
        let result = find_java_numeric_literal(line, 16);
        assert!(result.is_some(), "Should find hex long literal");
        let (literal, _) = result.unwrap();
        assert_eq!(literal, "0xFFFFFFFFL");
    }

    #[test]
    fn test_is_valid_java_literal_location_block_comment() {
        let line = "int x = /* 42 */ 100;";
        // Position 11 is inside the block comment (on the '4')
        assert!(
            !is_valid_java_literal_location(line, 11, 2),
            "Should detect position inside block comment"
        );
        // Position 17 is after the block comment (on the '1')
        assert!(
            is_valid_java_literal_location(line, 17, 3),
            "Should allow position after block comment"
        );
    }

    #[test]
    fn test_is_valid_java_literal_location_escaped_quotes() {
        let line = r#"String s = "test \"quote\" test"; int x = 42;"#;
        // Position inside the string should be invalid
        assert!(
            !is_valid_java_literal_location(line, 20, 1),
            "Should detect position inside string with escaped quotes"
        );
        // Position after the string should be valid
        assert!(
            is_valid_java_literal_location(line, 45, 2),
            "Should allow position after string with escaped quotes"
        );
    }

    #[test]
    fn test_plan_extract_constant_hex_literal() {
        let source = r#"
public class Colors {
    public void setColor() {
        int red = 0xFF0000;
        int green = 0x00FF00;
    }
}
"#;
        // Column 18 is inside the hex literal 0xFF0000
        let result = plan_extract_constant(source, 3, 18, "COLOR_RED", "Colors.java");
        assert!(
            result.is_ok(),
            "Should extract hex literal: {:?}",
            result.err()
        );

        let plan = result.unwrap();
        // Check that the constant type is inferred correctly
        let declaration_edit = plan.edits.iter().find(|e| e.edit_type == EditType::Insert);
        assert!(declaration_edit.is_some(), "Should have insertion edit");
        assert!(
            declaration_edit.unwrap().new_text.contains("int COLOR_RED"),
            "Should declare as int type for hex literal"
        );
    }

    #[test]
    fn test_plan_extract_constant_inner_class() {
        let source = r#"
public class Outer {
    public class Inner {
        public void method() {
            int timeout = 5000;
        }
    }
}
"#;
        // Column 26 is inside the numeric literal 5000
        let result = plan_extract_constant(source, 4, 26, "TIMEOUT_MS", "Outer.java");
        assert!(
            result.is_ok(),
            "Should handle extract constant in inner class: {:?}",
            result.err()
        );

        // The insertion point should still be at the class level
        // (We use simple heuristic that finds first class)
        let plan = result.unwrap();
        assert!(plan.edits.len() >= 2, "Should have insertion and replacement edits");
    }

    #[test]
    fn test_infer_java_type_hex() {
        assert_eq!(infer_java_type("0xFF"), "int");
        assert_eq!(infer_java_type("0xDEADBEEF"), "int");
        assert_eq!(infer_java_type("0xFFFFFFFFL"), "long");
        assert_eq!(infer_java_type("0xabc123"), "int");
    }
}

// Refactoring tests: Core operations (extract/inline) tested in other languages (C++/Python)
// Kept: Java-specific refactoring tests would go here (if any)
