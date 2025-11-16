//! Java-specific refactoring operations
//!
//! Provides AST-based refactoring capabilities including:
//! - Extract function
//! - Extract variable
//! - Inline variable

use mill_foundation::protocol::{EditPlan, EditType, TextEdit};
use mill_lang_common::find_literal_occurrences;
use mill_lang_common::is_escaped;
use mill_lang_common::is_valid_code_literal_location;
use mill_lang_common::refactoring::CodeRange as CommonCodeRange;
use mill_lang_common::refactoring::edit_plan_builder::EditPlanBuilder;
use mill_lang_common::{ExtractConstantAnalysis, LineExtractor};
use mill_plugin_api::{PluginApiError, PluginResult};
use serde::{Deserialize, Serialize};
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

/// Generate edit plan for extract function refactoring
pub(crate) fn plan_extract_function(
    source: &str,
    range: &CodeRange,
    function_name: &str,
    file_path: &str,
) -> PluginResult<EditPlan> {
    let mut parser = Parser::new();
    parser
        .set_language(&get_language())
        .map_err(|e| PluginApiError::parse(format!("Failed to load Java grammar: {}", e)))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| PluginApiError::parse("Failed to parse Java source".to_string()))?;
    let root = tree.root_node();

    let start_point = Point::new(range.start_line as usize - 1, range.start_col as usize);
    let end_point = Point::new(range.end_line as usize - 1, range.end_col as usize);

    let selected_node = find_smallest_node_containing_range(root, start_point, end_point)
        .ok_or_else(|| {
            PluginApiError::invalid_input("Could not find a node for the selection.".to_string())
        })?;

    let selected_text = selected_node
        .utf8_text(source.as_bytes())
        .map_err(|e| PluginApiError::parse(format!("Invalid UTF-8 in source: {}", e)))?
        .to_string();

    let enclosing_method =
        find_ancestor_of_kind(selected_node, "method_declaration").ok_or_else(|| {
            PluginApiError::invalid_input("Selection is not inside a method.".to_string())
        })?;

    let indent = LineExtractor::get_indentation_str(source, enclosing_method.start_position().row as u32);
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

    Ok(EditPlanBuilder::new(file_path, "extract_function")
        .with_edits(vec![insert_edit, replace_edit])
        .with_syntax_validation("Verify syntax after extraction")
        .with_intent_args(serde_json::json!({ "function_name": function_name }))
        .with_complexity(3)
        .with_impact_area("function_extraction")
        .build())
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
) -> PluginResult<EditPlan> {
    let mut parser = Parser::new();
    parser
        .set_language(&get_language())
        .map_err(|e| PluginApiError::parse(format!("Failed to load Java grammar: {}", e)))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| PluginApiError::parse("Failed to parse Java source".to_string()))?;
    let root = tree.root_node();

    let start_point = Point::new(start_line as usize - 1, start_col as usize);
    let end_point = Point::new(end_line as usize - 1, end_col as usize);

    let selected_node = find_smallest_node_containing_range(root, start_point, end_point)
        .ok_or_else(|| {
            PluginApiError::invalid_input("Could not find a node for the selection.".to_string())
        })?;

    let expression_text = selected_node
        .utf8_text(source.as_bytes())
        .map_err(|e| PluginApiError::parse(format!("Invalid UTF-8 in source: {}", e)))?
        .to_string();

    let insertion_node = find_ancestor_of_kind(selected_node, "statement")
        .or_else(|| find_ancestor_of_kind(selected_node, "local_variable_declaration"))
        .ok_or_else(|| {
            PluginApiError::invalid_input("Could not find statement to insert before.".to_string())
        })?;

    let indent = LineExtractor::get_indentation_str(source, insertion_node.start_position().row as u32);
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

    Ok(EditPlanBuilder::new(file_path, "extract_variable")
        .with_edits(vec![insert_edit, replace_edit])
        .with_syntax_validation("Verify syntax after extraction")
        .with_intent_args(serde_json::json!({
            "expression": expression_text,
            "variable_name": var_name,
        }))
        .with_complexity(2)
        .with_impact_area("variable_extraction")
        .build())
}

/// Generate edit plan for inline variable refactoring
pub(crate) fn plan_inline_variable(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    file_path: &str,
) -> PluginResult<EditPlan> {
    let mut parser = Parser::new();
    parser
        .set_language(&get_language())
        .map_err(|e| PluginApiError::parse(format!("Failed to load Java grammar: {}", e)))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| PluginApiError::parse("Failed to parse Java source".to_string()))?;
    let root = tree.root_node();
    let point = Point::new(variable_line as usize - 1, variable_col as usize);

    let var_ident_node = find_node_at_point(root, point).ok_or_else(|| {
        PluginApiError::invalid_input("Could not find variable at specified location.".to_string())
    })?;

    let (var_name, var_value, declaration_node) = extract_java_var_info(var_ident_node, source)?;

    let scope_node =
        find_ancestor_of_kind(declaration_node, "method_declaration").ok_or_else(|| {
            PluginApiError::invalid_input("Variable is not inside a method.".to_string())
        })?;

    let mut edits = Vec::new();
    let query_str = format!(r#"((identifier) @ref (#eq? @ref "{}"))"#, var_name);
    let query = Query::new(&get_language(), &query_str)
        .map_err(|e| PluginApiError::internal(e.to_string()))?;
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
            .map_err(|e| PluginApiError::parse(format!("Invalid UTF-8 in source: {}", e)))?
            .to_string(),
        new_text: String::new(),
        priority: 100,
        description: format!("Remove declaration of '{}'", var_name),
    });

    Ok(EditPlanBuilder::new(file_path, "inline_variable")
        .with_edits(edits)
        .with_syntax_validation("Verify syntax is valid")
        .with_intent_args(serde_json::json!({ "variable_name": var_name }))
        .with_complexity(4)
        .with_impact_area("variable_inlining")
        .build())
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

/// Finds the AST node at a specific point in Java source code.
///
/// # Arguments
/// * `node` - The root node to search within
/// * `point` - The source code position (line, column) to search for
///
/// # Returns
/// * `Some(Node)` - The smallest named node containing the point
/// * `None` - If no node exists at the specified point
fn find_node_at_point<'a>(node: Node<'a>, point: Point) -> Option<Node<'a>> {
    node.named_descendant_for_point_range(point, point)
}

/// Finds the nearest ancestor node of a specific kind in the Java AST.
///
/// Traverses up the AST tree to find the first ancestor matching the specified node kind.
///
/// # Arguments
/// * `node` - The starting node to search from
/// * `kind` - The AST node kind to search for (e.g., "method_declaration", "class_declaration")
///
/// # Returns
/// * `Some(Node)` - The first ancestor matching the specified kind
/// * `None` - If no matching ancestor is found
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
) -> PluginResult<(String, String, Node<'a>)> {
    let declaration_node =
        find_ancestor_of_kind(node, "local_variable_declaration").ok_or_else(|| {
            PluginApiError::invalid_input("Not a local variable declaration".to_string())
        })?;

    let declarator = declaration_node
        .children_by_field_name("declarator", &mut declaration_node.walk())
        .find(|d| {
            d.range().start_byte <= node.range().start_byte
                && d.range().end_byte >= node.range().end_byte
        })
        .ok_or_else(|| {
            PluginApiError::invalid_input("Could not find variable declarator".to_string())
        })?;

    let name_node = declarator
        .child_by_field_name("name")
        .ok_or_else(|| PluginApiError::invalid_input("Could not find variable name".to_string()))?;
    let value_node = declarator
        .child_by_field_name("value")
        .ok_or_else(|| PluginApiError::invalid_input("Could not find variable value".to_string()))?;

    let name = name_node
        .utf8_text(source.as_bytes())
        .map_err(|e| PluginApiError::parse(format!("Invalid UTF-8 in variable name: {}", e)))?
        .to_string();
    let value = value_node
        .utf8_text(source.as_bytes())
        .map_err(|e| PluginApiError::parse(format!("Invalid UTF-8 in variable value: {}", e)))?
        .to_string();

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
) -> PluginResult<ExtractConstantAnalysis> {
    let lines: Vec<&str> = source.lines().collect();

    // Get the line at cursor position
    let line_text = lines
        .get(line as usize)
        .ok_or_else(|| PluginApiError::invalid_input("Invalid line number".to_string()))?;

    // Find the literal at the cursor position
    let found_literal = find_java_literal_at_position(line_text, character as usize)
        .ok_or_else(|| {
            PluginApiError::invalid_input("No literal found at the specified location".to_string())
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
) -> PluginResult<EditPlan> {
    let analysis = analyze_extract_constant(source, line, character, file_path)?;

    if !analysis.is_valid_literal {
        return Err(PluginApiError::invalid_input(format!(
            "Cannot extract constant: {}",
            analysis.blocking_reasons.join(", ")
        )));
    }

    use mill_lang_common::ExtractConstantEditPlanBuilder;

    // Capture Java-specific information for the declaration
    let java_type = infer_java_type(&analysis.literal_value);
    let indent = LineExtractor::get_indentation_str(source, analysis.insertion_point.start_line);

    ExtractConstantEditPlanBuilder::new(analysis, name.to_string(), file_path.to_string())
        .with_declaration_format(|name, value| {
            format!(
                "{}private static final {} {} = {};\n",
                indent, java_type, name, value
            )
        })
        .map_err(|e| PluginApiError::invalid_input(e))
}

/// Finds a Java literal at a given position in a line of code.
///
/// This function attempts to identify any Java literal (numeric, string, boolean, or null)
/// at the cursor position by trying each literal type in sequence.
///
/// # Arguments
/// * `line_text` - The complete line of code
/// * `col` - Zero-based character position within the line
///
/// # Returns
/// * `Some((literal, range))` - The literal string and its position range
/// * `None` - If no literal is found at the cursor position
///
/// # Supported Literals
/// Tries in order: numeric → string → keyword (boolean/null)
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

/// Finds a numeric literal at a cursor position in Java code.
///
/// This function identifies Java numeric literals including integers, floats, hexadecimal
/// numbers, and numbers with type suffixes (L, f, F, d, D). Handles negative numbers and
/// underscores in numeric literals (Java 7+).
///
/// # Arguments
/// * `line_text` - The complete line of code
/// * `col` - Zero-based character position within the line
///
/// # Returns
/// * `Some((literal, range))` - The numeric literal string and its position range
/// * `None` - If no valid numeric literal is found at the cursor position
///
/// # Supported Formats
/// - Decimal: `42`, `-100`, `123_456`
/// - Hexadecimal: `0xFF`, `0x1A2B`
/// - Float/Double: `3.14`, `2.5f`, `1.0d`
/// - With suffixes: `100L`, `2.5f`, `1.0d`
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

/// Finds a string literal at a cursor position in Java code.
///
/// This function identifies string literals in Java, handling escaped characters properly.
/// Searches backwards for an opening quote and forwards for the closing quote, accounting
/// for escaped quotes within the string.
///
/// # Arguments
/// * `line_text` - The complete line of code
/// * `col` - Zero-based character position within the line
///
/// # Returns
/// * `Some((literal, range))` - The complete string literal (with quotes) and its range
/// * `None` - If no valid string literal is found at the cursor position
///
/// # Example
/// ```rust
/// let result = find_java_string_literal("String s = \"Hello\";", 11);
/// assert_eq!(result.unwrap().0, "\"Hello\"");
/// ```
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

// is_valid_java_literal_location is now provided by mill_lang_common::is_valid_code_literal_location
fn is_valid_java_literal_location(line: &str, pos: usize, len: usize) -> bool {
    is_valid_code_literal_location(line, pos, len)
}

/// Finds the appropriate insertion point for a constant declaration in Java code
fn find_java_insertion_point_for_constant(source: &str) -> PluginResult<CodeRange> {
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

/// Infers the appropriate Java type for a constant based on its literal value.
///
/// This function analyzes a literal string and determines the most appropriate Java type
/// for declaring a constant, considering literal format, prefixes, and suffixes.
///
/// # Arguments
/// * `literal_value` - The string representation of the literal value
///
/// # Returns
/// A static string representing the inferred Java type:
/// - `"boolean"` - for true/false
/// - `"String"` - for quoted strings
/// - `"int"` - for plain integers or hex without suffix
/// - `"long"` - for literals with L/l suffix
/// - `"float"` - for literals with f/F suffix
/// - `"double"` - for decimals with d/D suffix or decimal point
///
/// # Examples
/// ```rust
/// assert_eq!(infer_java_type("42"), "int");
/// assert_eq!(infer_java_type("100L"), "long");
/// assert_eq!(infer_java_type("3.14"), "double");
/// assert_eq!(infer_java_type("2.5f"), "float");
/// assert_eq!(infer_java_type("\"Hello\""), "String");
/// assert_eq!(infer_java_type("true"), "boolean");
/// ```
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
    use mill_lang_common::is_screaming_snake_case;

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
