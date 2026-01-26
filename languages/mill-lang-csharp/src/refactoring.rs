//! C#-specific refactoring operations
//!
//! Provides AST-based refactoring capabilities including:
//! - Extract Method (function)
//! - Extract Variable
//! - Inline Variable

use mill_foundation::protocol::{EditPlan, EditType, TextEdit};
use mill_lang_common::{
    is_escaped, is_valid_code_literal_location,
    refactoring::{edit_plan_builder::EditPlanBuilder, find_literal_occurrences},
    CodeRange, ExtractConstantAnalysis, ExtractConstantEditPlanBuilder, LineExtractor,
};
use mill_plugin_api::{PluginApiError, PluginResult};
use tree_sitter::{Node, Parser, Point, Query, QueryCursor, StreamingIterator};

/// Get the C# language for tree-sitter
fn get_language() -> tree_sitter::Language {
    tree_sitter_c_sharp::LANGUAGE.into()
}

/// Extracts selected code into a new C# method.
///
/// This refactoring operation takes a range of code and creates a new private method
/// containing that code, replacing the original selection with a call to the new method.
/// The new method is inserted immediately after the enclosing method.
///
/// # Arguments
/// * `source` - The complete C# source code
/// * `range` - The code range specifying the selection to extract
/// * `function_name` - The name for the new method (called function_name for consistency)
/// * `file_path` - Path to the file being refactored
///
/// # Returns
/// * `Ok(EditPlan)` - The refactoring plan with two edits: method creation and call replacement
/// * `Err(PluginApiError)` - If the selection is invalid or not inside a method
///
/// # Examples
/// ```rust
/// let source = r#"
/// class Program {
///     static void Main(string[] args) {
///         Console.WriteLine("Hello");
///     }
/// }"#;
/// let range = CodeRange { start_line: 5, start_col: 8, end_line: 5, end_col: 41 };
/// let plan = plan_extract_function(source, &range, "Greet", "Program.cs")?;
/// assert_eq!(plan.edits.len(), 2); // insert new method + replace with call
/// ```
pub fn plan_extract_function(
    source: &str,
    range: &CodeRange,
    function_name: &str, // In C#, this is a method name
    file_path: &str,
) -> PluginResult<EditPlan> {
    let mut parser = Parser::new();
    parser
        .set_language(&get_language())
        .map_err(|e| PluginApiError::parse(format!("Failed to load C# grammar: {}", e)))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| PluginApiError::parse("Failed to parse C# source".to_string()))?;
    let root = tree.root_node();

    let start_point = Point::new(range.start_line as usize, range.start_col as usize);
    let end_point = Point::new(range.end_line as usize, range.end_col as usize);

    let start_node = find_node_at_point(root, start_point).ok_or_else(|| {
        PluginApiError::invalid_input(
            "Could not find a node at the start of the selection.".to_string(),
        )
    })?;
    let end_node = find_node_at_point(root, end_point).ok_or_else(|| {
        PluginApiError::invalid_input(
            "Could not find a node at the end of the selection.".to_string(),
        )
    })?;

    let selected_text = &source[start_node.start_byte()..end_node.end_byte()];

    let enclosing_method =
        find_ancestor_of_kind(start_node, "method_declaration").ok_or_else(|| {
            PluginApiError::invalid_input("Selection is not inside a method.".to_string())
        })?;

    let indent =
        LineExtractor::get_indentation_str(source, enclosing_method.start_position().row as u32);
    let method_indent = format!("{}    ", indent);

    let new_method_text = format!(
        "\n\n{}private void {}()\n{}{{\n{}{}\n{}}}\n",
        indent,
        function_name,
        indent,
        method_indent,
        selected_text.trim(),
        indent
    );

    let insert_edit = TextEdit {
        file_path: Some(file_path.to_string()),
        edit_type: EditType::Insert,
        location: CodeRange::new(
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
        file_path: Some(file_path.to_string()),
        edit_type: EditType::Replace,
        location: (*range).into(),
        original_text: selected_text.to_string(),
        new_text: format!("{}();", function_name),
        priority: 90,
        description: format!("Replace selection with call to '{}'", function_name),
    };

    Ok(EditPlanBuilder::new(file_path, "extract_method")
        .with_edits(vec![insert_edit, replace_edit])
        .with_syntax_validation("Verify syntax after extraction")
        .with_intent_args(serde_json::json!({ "function_name": function_name }))
        .with_complexity(3)
        .with_impact_area("function_extraction")
        .build())
}

/// Extracts an expression into a new C# variable.
///
/// This refactoring operation identifies an expression in C# code and extracts it into
/// a new variable declaration using `var`, replacing the original expression with the
/// variable name. The variable is declared before the statement containing the expression.
///
/// # Arguments
/// * `source` - The complete C# source code
/// * `start_line` - Zero-based starting line of the expression
/// * `start_col` - Zero-based starting column of the expression
/// * `end_line` - Zero-based ending line of the expression
/// * `end_col` - Zero-based ending column of the expression
/// * `variable_name` - Optional name for the variable (defaults to "extracted")
/// * `file_path` - Path to the file being refactored
///
/// # Returns
/// * `Ok(EditPlan)` - The refactoring plan with two edits: variable declaration and replacement
/// * `Err(PluginApiError)` - If the selection is invalid or cannot be extracted
///
/// # Examples
/// ```rust
/// let source = r#"
/// class Program {
///     static void Main(string[] args) {
///         var x = 10 + 20;
///     }
/// }"#;
/// let plan = plan_extract_variable(source, 5, 16, 5, 23, Some("sum".to_string()), "Program.cs")?;
/// assert_eq!(plan.edits.len(), 2); // variable declaration + replacement
/// ```
pub fn plan_extract_variable(
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
        .map_err(|e| PluginApiError::parse(format!("Failed to load C# grammar: {}", e)))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| PluginApiError::parse("Failed to parse C# source".to_string()))?;
    let root = tree.root_node();

    let start_point = Point::new(start_line as usize, start_col as usize);
    let end_point = Point::new(end_line as usize, end_col as usize);

    let selected_node = find_smallest_node_containing_range(root, start_point, end_point)
        .ok_or_else(|| {
            PluginApiError::invalid_input("Could not find a node for the selection.".to_string())
        })?;

    let expression_text = selected_node
        .utf8_text(source.as_bytes())
        .map_err(|e| PluginApiError::parse(format!("Invalid UTF-8 in source: {}", e)))?
        .to_string();

    let insertion_statement = find_ancestor_of_kind(selected_node, "local_declaration_statement")
        .or_else(|| find_ancestor_of_kind(selected_node, "expression_statement"))
        .or_else(|| find_ancestor_of_kind(selected_node, "return_statement"))
        .or_else(|| find_ancestor_of_kind(selected_node, "assignment_expression"))
        .or_else(|| find_ancestor_of_kind(selected_node, "argument"))
        .ok_or_else(|| {
            PluginApiError::invalid_input(
                "Could not find an appropriate statement to insert the variable before."
                    .to_string(),
            )
        })?;

    let indent =
        LineExtractor::get_indentation_str(source, insertion_statement.start_position().row as u32);
    let var_name = variable_name.unwrap_or_else(|| "extracted".to_string());

    let declaration_text = format!("var {} = {};", var_name, expression_text);

    let insert_edit = TextEdit {
        file_path: Some(file_path.to_string()),
        edit_type: EditType::Insert,
        location: CodeRange::new(
            insertion_statement.start_position().row as u32,
            0,
            insertion_statement.start_position().row as u32,
            0,
        )
        .into(),
        original_text: String::new(),
        new_text: format!("{}{}\n", indent, declaration_text),
        priority: 100,
        description: format!("Declare new variable '{}'", var_name),
    };

    let replace_edit = TextEdit {
        file_path: Some(file_path.to_string()),
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

/// Inlines a C# variable by replacing all references with its initializer value.
///
/// This refactoring operation finds a variable declaration, replaces all references to that
/// variable with its initializer expression, and removes the variable declaration. The operation
/// scopes the search to the enclosing method to avoid unintended replacements.
///
/// # Arguments
/// * `source` - The complete C# source code
/// * `variable_line` - Zero-based line number where the variable is declared
/// * `variable_col` - Zero-based column offset within the line
/// * `file_path` - Path to the file being refactored
///
/// # Returns
/// * `Ok(EditPlan)` - The edit plan with edits to replace references and delete the declaration
/// * `Err(PluginApiError)` - If the variable is not found or not inside a method
///
/// # Examples
/// ```rust
/// let source = r#"
/// class Program {
///     static void Main(string[] args) {
///         var greeting = "Hello";
///         Console.WriteLine(greeting);
///     }
/// }"#;
/// let plan = plan_inline_variable(source, 5, 12, "Program.cs")?;
/// assert!(plan.edits.len() >= 2); // replacements + declaration removal
/// ```
pub fn plan_inline_variable(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    file_path: &str,
) -> PluginResult<EditPlan> {
    let mut parser = Parser::new();
    parser
        .set_language(&get_language())
        .map_err(|e| PluginApiError::parse(format!("Failed to load C# grammar: {}", e)))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| PluginApiError::parse("Failed to parse C# source".to_string()))?;
    let root = tree.root_node();
    let point = Point::new(variable_line as usize, variable_col as usize);

    let var_ident_node = find_node_at_point(root, point).ok_or_else(|| {
        PluginApiError::invalid_input("Could not find variable at specified location.".to_string())
    })?;

    let (var_name, var_value, declaration_node) = extract_csharp_var_info(var_ident_node, source)?;

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
                        file_path: Some(file_path.to_string()),
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
        file_path: Some(file_path.to_string()),
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
    start: Point,
    end: Point,
) -> Option<Node<'a>> {
    // Start from root and descend to the smallest node that contains the range
    let mut current = node;

    'outer: loop {
        // Check if any child fully contains the range
        let mut cursor = current.walk();
        for child in current.children(&mut cursor) {
            if child.start_position() <= start && child.end_position() >= end {
                // This child contains the range, descend into it
                current = child;
                continue 'outer;
            }
        }
        // No child fully contains the range, so current is the smallest node
        break;
    }

    if current.start_position() <= start && current.end_position() >= end {
        Some(current)
    } else {
        None
    }
}

/// Finds the AST node at a specific point in C# source code.
///
/// # Arguments
/// * `node` - The root node to search within
/// * `point` - The source code position (line, column) to search for
///
/// # Returns
/// * `Some(Node)` - The smallest named node containing the point
/// * `None` - If no node exists at the specified point
fn find_node_at_point<'a>(node: Node<'a>, point: Point) -> Option<Node<'a>> {
    find_smallest_node_containing_range(node, point, point)
}

/// Finds the nearest ancestor node of a specific kind in the C# AST.
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
    let mut current = node;
    while let Some(parent) = current.parent() {
        if parent.kind() == kind {
            return Some(parent);
        }
        current = parent;
    }
    None
}

fn node_to_location(node: Node) -> CodeRange {
    let range = node.range();
    CodeRange::new(
        range.start_point.row as u32,
        range.start_point.column as u32,
        range.end_point.row as u32,
        range.end_point.column as u32,
    )
}

fn extract_csharp_var_info<'a>(
    node: Node<'a>,
    source: &str,
) -> PluginResult<(String, String, Node<'a>)> {
    let declaration_statement = find_ancestor_of_kind(node, "local_declaration_statement")
        .ok_or_else(|| {
            PluginApiError::invalid_input(format!(
                "Not a local variable declaration. Node kind: {}",
                node.kind()
            ))
        })?;

    // Get variable_declaration child directly (not via field name)
    let mut cursor = declaration_statement.walk();
    let var_declaration = declaration_statement
        .children(&mut cursor)
        .find(|n| n.kind() == "variable_declaration")
        .ok_or_else(|| {
            let child_kinds: Vec<_> = declaration_statement
                .children(&mut declaration_statement.walk())
                .map(|n| n.kind())
                .collect();
            PluginApiError::invalid_input(format!(
                "Invalid declaration statement: missing variable_declaration. Children: {:?}",
                child_kinds
            ))
        })?;

    // Get variable_declarator from variable_declaration
    let mut cursor_decl = var_declaration.walk();
    let declarator = var_declaration
        .children(&mut cursor_decl)
        .find(|n| n.kind() == "variable_declarator")
        .ok_or_else(|| {
            PluginApiError::invalid_input(
                "Invalid declaration: missing variable_declarator".to_string(),
            )
        })?;

    // Get the identifier (variable name) from declarator
    let mut cursor_name = declarator.walk();
    let name_node = declarator
        .children(&mut cursor_name)
        .find(|n| n.kind() == "identifier")
        .ok_or_else(|| PluginApiError::invalid_input("Could not find variable name".to_string()))?;

    // Get the value - in newer tree-sitter-c-sharp, the value is a direct child
    // (no equals_value_clause wrapper)
    let mut cursor_value = declarator.walk();
    let value_node = declarator
        .children(&mut cursor_value)
        .find(|n| n.kind() != "identifier" && n.kind() != "=")
        .ok_or_else(|| {
            PluginApiError::invalid_input("Could not find variable initializer value".to_string())
        })?;

    let name = name_node
        .utf8_text(source.as_bytes())
        .map_err(|e| PluginApiError::parse(format!("Invalid UTF-8 in source: {}", e)))?
        .to_string();
    let value = value_node
        .utf8_text(source.as_bytes())
        .map_err(|e| PluginApiError::parse(format!("Invalid UTF-8 in source: {}", e)))?
        .to_string();

    Ok((name, value, declaration_statement))
}

// ============================================================================
// Extract Constant Refactoring
// ============================================================================

/// Analyzes source code to extract information about a literal value at a cursor position.
///
/// This analysis function identifies literals in C# source code and gathers information for
/// constant extraction. It analyzes:
/// - The literal value at the specified cursor position (number, string, boolean, or null)
/// - All occurrences of that literal throughout the file
/// - A suitable insertion point for the constant declaration (class level)
/// - Whether extraction is valid and any blocking reasons
///
/// # Arguments
/// * `source` - The C# source code
/// * `line` - Zero-based line number where the cursor is positioned
/// * `character` - Zero-based character offset within the line
/// * `file_path` - Path to the file (used for error reporting)
///
/// # Returns
/// * `Ok(ExtractConstantAnalysis)` - Analysis result with literal value, occurrence ranges,
///   validation status, and insertion point
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
    let found_literal =
        find_csharp_literal_at_position(line_text, character as usize).ok_or_else(|| {
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
    let occurrence_ranges =
        find_literal_occurrences(source, &literal_value, is_valid_csharp_literal_location);

    // Insertion point: at class level (after opening brace of class)
    let insertion_point = find_csharp_insertion_point_for_constant(source)?;

    Ok(ExtractConstantAnalysis {
        literal_value,
        occurrence_ranges,
        is_valid_literal,
        blocking_reasons,
        insertion_point,
    })
}

/// Infer the C# type from a literal value
fn infer_csharp_type(literal: &str) -> &'static str {
    // Check for boolean
    if literal == "true" || literal == "false" {
        return "bool";
    }

    // Check for null
    if literal == "null" {
        return "object";
    }

    // Check for string literals
    if literal.starts_with('"') || literal.starts_with('\'') {
        return "string";
    }

    // Check for hexadecimal
    if literal.starts_with("0x") || literal.starts_with("0X") {
        return "int";
    }

    // Check for decimal suffix
    if literal.ends_with('m') || literal.ends_with('M') {
        return "decimal";
    }

    // Check for float suffix
    if literal.ends_with('f') || literal.ends_with('F') {
        return "float";
    }

    // Check for double suffix or contains decimal point
    if literal.ends_with('d') || literal.ends_with('D') || literal.contains('.') {
        return "double";
    }

    // Check for long suffix
    if literal.ends_with('L') || literal.ends_with('l') {
        return "long";
    }

    // Default to int for plain integers
    "int"
}

/// Extracts a literal value to a named constant in C# code.
///
/// This refactoring operation replaces all occurrences of a literal (number, string, boolean, or null)
/// with a named constant declaration at the class level, improving code maintainability by
/// eliminating magic values and making it easier to update values globally.
///
/// # Arguments
/// * `source` - The C# source code
/// * `line` - Zero-based line number where the cursor is positioned
/// * `character` - Zero-based character offset within the line
/// * `name` - The constant name (must be SCREAMING_SNAKE_CASE)
/// * `file_path` - Path to the file being refactored
///
/// # Returns
/// * `Ok(EditPlan)` - The edit plan with constant declaration inserted at class level and all
///   literal occurrences replaced with the constant name
/// * `Err(RefactoringError)` - If the cursor is not on a literal, the name is invalid, or parsing fails
pub fn plan_extract_constant(
    source: &str,
    line: u32,
    character: u32,
    name: &str,
    file_path: &str,
) -> PluginResult<EditPlan> {
    let analysis = analyze_extract_constant(source, line, character, file_path)?;

    // C# needs type inference and indentation
    let csharp_type = infer_csharp_type(&analysis.literal_value);
    let indent = LineExtractor::get_indentation_str(source, analysis.insertion_point.start_line);
    let const_indent = format!("{}    ", indent); // Add one level of indentation

    ExtractConstantEditPlanBuilder::new(analysis, name.to_string(), file_path.to_string())
        .with_declaration_format(|name, value| {
            format!(
                "{}private const {} {} = {};\n",
                const_indent, csharp_type, name, value
            )
        })
        .map_err(PluginApiError::invalid_input)
}

/// Finds a C# literal at a given position in a line of code.
///
/// This function attempts to identify any C# literal (numeric, string, or keyword)
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
fn find_csharp_literal_at_position(line_text: &str, col: usize) -> Option<(String, CodeRange)> {
    // Try to find different kinds of literals at the cursor position

    // Check for numeric literal (including negative numbers)
    if let Some((literal, range)) = find_csharp_numeric_literal(line_text, col) {
        return Some((literal, range));
    }

    // Check for string literal (quoted with single/double quote support)
    if let Some((literal, range)) = find_csharp_string_literal(line_text, col) {
        return Some((literal, range));
    }

    // Check for boolean (true/false) or null (C# lowercase keywords)
    if let Some((literal, range)) = find_csharp_keyword_literal(line_text, col) {
        return Some((literal, range));
    }

    None
}

/// Finds a numeric literal at a cursor position in C# code.
///
/// This function identifies C# numeric literals including integers, floats, hexadecimal
/// numbers, and numbers with type suffixes (L, f, F, d, D, m, M). Handles negative numbers
/// and underscores in numeric literals (C# 7.0+).
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
/// - Float: `3.14f`, `2.5F`
/// - Double: `1.0`, `2.5d`, `3.14D`
/// - Decimal (C#): `100.0m`, `99.99M`
/// - Long: `100L`, `1000l`
fn find_csharp_numeric_literal(line_text: &str, col: usize) -> Option<(String, CodeRange)> {
    if col >= line_text.len() {
        return None;
    }

    let chars: Vec<char> = line_text.chars().collect();
    if col >= chars.len() {
        return None;
    }

    // Check for hexadecimal literal (0x or 0X prefix)
    let is_hex = col >= 2
        && (chars[col - 1] == 'x' || chars[col - 1] == 'X')
        && chars[col - 2] == '0'
        || col >= 1 && (chars[col] == 'x' || chars[col] == 'X') && col > 0 && chars[col - 1] == '0'
        || col > 0
            && chars[col - 1].is_ascii_hexdigit()
            && col >= 2
            && (chars[col - 2] == 'x' || chars[col - 2] == 'X');

    // If we're in a hex literal, find its boundaries
    if is_hex {
        // Find the start (should be '0x' or '0X')
        let mut start = col;
        while start > 0 {
            if chars[start] == '0'
                && start + 1 < chars.len()
                && (chars[start + 1] == 'x' || chars[start + 1] == 'X')
            {
                break;
            }
            if !chars[start].is_ascii_hexdigit()
                && chars[start] != 'x'
                && chars[start] != 'X'
                && chars[start] != '_'
            {
                start += 1;
                break;
            }
            start -= 1;
        }

        // Find the end
        let mut end = col;
        let mut found_x = false;
        for (i, &ch) in chars.iter().enumerate().skip(start) {
            if ch == 'x' || ch == 'X' {
                found_x = true;
                end = i + 1;
            } else if found_x && (ch.is_ascii_hexdigit() || ch == '_' || ch == 'L' || ch == 'l') {
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
    for (i, &c) in chars.iter().enumerate().skip(col) {
        if c.is_ascii_digit()
            || c == '.'
            || c == '_'
            || c == 'f'
            || c == 'F'
            || c == 'L'
            || c == 'l'
            || c == 'd'
            || c == 'D'
            || c == 'm'
            || c == 'M'
        {
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

/// Finds a string literal at a cursor position in C# code.
fn find_csharp_string_literal(line_text: &str, col: usize) -> Option<(String, CodeRange)> {
    if col >= line_text.len() {
        return None;
    }

    // Look for opening quote before cursor, skipping escaped quotes
    let chars: Vec<char> = line_text.chars().collect();
    let mut opening_quote_pos = None;

    for i in (0..col).rev() {
        if i < chars.len() && (chars[i] == '"' || chars[i] == '\'') && !is_escaped(line_text, i) {
            opening_quote_pos = Some((i, chars[i]));
            break;
        }
    }

    if let Some((start, quote)) = opening_quote_pos {
        // Find closing quote after cursor, skipping escaped quotes
        for (j, &ch) in chars.iter().enumerate().skip(col) {
            if ch == quote && !is_escaped(line_text, j) {
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

/// Finds a C# keyword literal (true, false, or null) at a cursor position.
fn find_csharp_keyword_literal(line_text: &str, col: usize) -> Option<(String, CodeRange)> {
    let keywords = ["true", "false", "null"];

    for keyword in &keywords {
        // Try to match keyword at or near cursor
        for start in col.saturating_sub(keyword.len())
            ..=col.min(line_text.len().saturating_sub(keyword.len()))
        {
            if start + keyword.len() <= line_text.len()
                && &line_text[start..start + keyword.len()] == *keyword
            {
                // Check word boundaries
                let before_ok = start == 0
                    || !line_text[..start].ends_with(|c: char| c.is_alphanumeric() || c == '_');
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

    None
}

// is_valid_csharp_literal_location is now provided by mill_lang_common::is_valid_code_literal_location
fn is_valid_csharp_literal_location(line: &str, pos: usize, len: usize) -> bool {
    is_valid_code_literal_location(line, pos, len)
}

/// Finds the appropriate insertion point for a constant declaration in C# code.
///
/// The insertion point is at class level, after the opening brace of the class.
fn find_csharp_insertion_point_for_constant(source: &str) -> PluginResult<CodeRange> {
    let lines: Vec<&str> = source.lines().collect();
    let mut in_class = false;

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let line_idx = idx as u32;

        // Look for class declaration
        if trimmed.starts_with("class ")
            || trimmed.starts_with("public class ")
            || trimmed.starts_with("private class ")
            || trimmed.starts_with("internal class ")
            || trimmed.starts_with("protected class ")
        {
            in_class = true;
        }

        // Find the opening brace of the class
        if in_class && trimmed.contains('{') {
            // Insert after the opening brace line
            return Ok(CodeRange {
                start_line: line_idx + 1,
                start_col: 0,
                end_line: line_idx + 1,
                end_col: 0,
            });
        }
    }

    // Default: insert at the beginning of the file
    Ok(CodeRange {
        start_line: 0,
        start_col: 0,
        end_line: 0,
        end_col: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mill_lang_common::{count_unescaped_quotes, is_screaming_snake_case};

    #[test]
    fn test_extract_csharp_variable() {
        let source = r#"
class Program
{
    static void Main(string[] args)
    {
        var x = 10 + 20;
        Console.WriteLine(x);
    }
}"#;
        // Line and column are 0-indexed.
        // "10 + 20" is on line 5.
        // `var x = 10 + 20;`
        // The expression `10 + 20` starts at column 16.
        let plan = plan_extract_variable(source, 5, 16, 5, 23, Some("sum".to_string()), "test.cs")
            .unwrap();
        assert_eq!(plan.edits.len(), 2);
        let insert_edit = plan
            .edits
            .iter()
            .find(|e| e.edit_type == EditType::Insert)
            .unwrap();
        assert_eq!(insert_edit.new_text, "        var sum = 10 + 20;\n");
        let replace_edit = plan
            .edits
            .iter()
            .find(|e| e.edit_type == EditType::Replace)
            .unwrap();
        assert_eq!(replace_edit.new_text, "sum");
    }

    #[test]
    fn test_extract_csharp_method() {
        let source = r#"
class Program
{
    static void Main(string[] args)
    {
        Console.WriteLine("Hello, World!");
    }
}"#;
        let range = CodeRange {
            start_line: 5,
            start_col: 8,
            end_line: 5,
            end_col: 41,
        };
        let plan = plan_extract_function(source, &range, "Greet", "test.cs").unwrap();
        assert_eq!(plan.edits.len(), 2);
        let insert_edit = &plan.edits[0];
        assert!(insert_edit.new_text.contains("private void Greet()"));
        let replace_edit = &plan.edits[1];
        assert_eq!(replace_edit.new_text, "Greet();");
    }

    #[test]
    fn test_inline_csharp_variable() {
        let source = r#"
class Program
{
    static void Main(string[] args)
    {
        var greeting = "Hello";
        Console.WriteLine(greeting);
    }
}"#;
        // "greeting" identifier on line 5 starts at column 12 (0-indexed)
        let plan = plan_inline_variable(source, 5, 12, "test.cs").unwrap();
        assert_eq!(plan.edits.len(), 2);

        let inline_edit = plan
            .edits
            .iter()
            .find(|e| e.new_text.contains("Hello"))
            .unwrap();
        assert_eq!(inline_edit.new_text, r#""Hello""#);

        let delete_edit = plan
            .edits
            .iter()
            .find(|e| e.edit_type == EditType::Delete)
            .unwrap();
        assert!(delete_edit.edit_type == EditType::Delete);
    }

    // ========================================================================
    // Extract Constant Tests
    // ========================================================================

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
    fn test_find_csharp_literal_occurrences() {
        let source = "var x = 42;\nvar y = 42;\nvar z = 100;";
        let occurrences = find_literal_occurrences(source, "42", is_valid_csharp_literal_location);
        assert_eq!(occurrences.len(), 2);
        assert_eq!(occurrences[0].start_line, 0);
        assert_eq!(occurrences[1].start_line, 1);
    }

    #[test]
    fn test_plan_extract_constant_valid_number() {
        let source = r#"
class Program
{
    static void Main(string[] args)
    {
        var x = 42;
        var y = 42;
    }
}"#;
        let result = plan_extract_constant(source, 5, 16, "ANSWER", "test.cs");
        assert!(
            result.is_ok(),
            "Should extract numeric literal successfully: {:?}",
            result.err()
        );
        let plan = result.unwrap();
        assert_eq!(plan.edits.len(), 3); // 1 insert + 2 replacements

        // Check constant declaration
        let insert_edit = plan
            .edits
            .iter()
            .find(|e| e.edit_type == EditType::Insert)
            .unwrap();
        assert!(insert_edit
            .new_text
            .contains("private const int ANSWER = 42;"));
    }

    #[test]
    fn test_plan_extract_constant_string() {
        let source = r#"
class Program
{
    static void Main(string[] args)
    {
        var msg = "hello";
        var greeting = "hello";
    }
}"#;
        let result = plan_extract_constant(source, 5, 20, "GREETING_MSG", "test.cs");
        assert!(
            result.is_ok(),
            "Should extract string literal: {:?}",
            result.err()
        );
        let plan = result.unwrap();
        assert!(plan.edits.len() >= 2); // At least 1 insert + replacements
    }

    #[test]
    fn test_plan_extract_constant_boolean() {
        let source = r#"
class Program
{
    static void Main(string[] args)
    {
        var debug = true;
        var verbose = true;
    }
}"#;
        let result = plan_extract_constant(source, 5, 20, "DEBUG_MODE", "test.cs");
        assert!(
            result.is_ok(),
            "Should extract boolean literal: {:?}",
            result.err()
        );
        let plan = result.unwrap();
        assert!(plan.edits.len() >= 2);
    }

    #[test]
    fn test_plan_extract_constant_invalid_name() {
        let source = r#"
class Program
{
    static void Main(string[] args)
    {
        var x = 42;
    }
}"#;
        let result = plan_extract_constant(source, 5, 16, "answer", "test.cs");
        assert!(result.is_err(), "Should reject lowercase name");

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("SCREAMING_SNAKE_CASE"));
    }

    // ========================================================================
    // New Edge Case Tests for Extract Constant
    // ========================================================================

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
    fn test_find_csharp_string_literal_with_escaped_quotes() {
        let line = r#"string msg = "He said \"hello\"";"#;
        // Position 22 is inside the string literal, after the opening quote
        let result = find_csharp_string_literal(line, 22);
        assert!(result.is_some(), "Should find string with escaped quotes");
        let (literal, _) = result.unwrap();
        // Expected: opening quote + He said + escaped quote + hello + escaped quote + closing quote
        assert_eq!(literal, r#""He said \"hello\"""#);
    }

    #[test]
    fn test_find_csharp_string_literal_multiple_escapes() {
        let line = r#"string path = "C:\\Users\\Admin\\file.txt";"#;
        // Position 20 is inside the string literal
        let result = find_csharp_string_literal(line, 20);
        assert!(
            result.is_some(),
            "Should find string with escaped backslashes"
        );
        let (literal, _) = result.unwrap();
        assert_eq!(literal, r#""C:\\Users\\Admin\\file.txt""#);
    }

    #[test]
    fn test_find_csharp_numeric_literal_hex() {
        let line = "int color = 0xFF00AA;";
        // Position 15 is inside the hex literal
        let result = find_csharp_numeric_literal(line, 15);
        assert!(result.is_some(), "Should find hex literal");
        let (literal, range) = result.unwrap();
        assert_eq!(literal, "0xFF00AA");
        assert_eq!(range.start_col, 12);
        assert_eq!(range.end_col, 20);
    }

    #[test]
    fn test_find_csharp_numeric_literal_hex_lowercase() {
        let line = "int mask = 0xdeadbeef;";
        // Position 14 is inside the hex literal
        let result = find_csharp_numeric_literal(line, 14);
        assert!(result.is_some(), "Should find lowercase hex literal");
        let (literal, _) = result.unwrap();
        assert_eq!(literal, "0xdeadbeef");
    }

    #[test]
    fn test_find_csharp_numeric_literal_hex_long() {
        let line = "long value = 0xFFFFFFFFL;";
        // Position 16 is inside the hex literal
        let result = find_csharp_numeric_literal(line, 16);
        assert!(result.is_some(), "Should find hex long literal");
        let (literal, _) = result.unwrap();
        assert_eq!(literal, "0xFFFFFFFFL");
    }

    #[test]
    fn test_find_csharp_numeric_literal_negative() {
        let line = "int temp = -42;";
        // Position 12 is inside the numeric literal
        let result = find_csharp_numeric_literal(line, 12);
        assert!(result.is_some(), "Should find negative literal");
        let (literal, _) = result.unwrap();
        assert_eq!(literal, "-42");
    }

    #[test]
    fn test_find_csharp_numeric_literal_decimal() {
        let line = "decimal price = 19.99m;";
        // Position 18 is inside the decimal literal
        let result = find_csharp_numeric_literal(line, 18);
        assert!(result.is_some(), "Should find decimal literal with suffix");
        let (literal, _) = result.unwrap();
        assert_eq!(literal, "19.99m");
    }

    #[test]
    fn test_is_valid_csharp_literal_location_block_comment() {
        let line = "int x = /* 42 */ 100;";
        // Position 11 is inside the block comment (on the '4')
        assert!(
            !is_valid_csharp_literal_location(line, 11, 2),
            "Should detect position inside block comment"
        );
        // Position 17 is after the block comment (on the '1')
        assert!(
            is_valid_csharp_literal_location(line, 17, 3),
            "Should allow position after block comment"
        );
    }

    #[test]
    fn test_is_valid_csharp_literal_location_escaped_quotes() {
        let line = r#"string s = "test \"quote\" test"; int x = 42;"#;
        // Position inside the string should be invalid
        assert!(
            !is_valid_csharp_literal_location(line, 20, 1),
            "Should detect position inside string with escaped quotes"
        );
        // Position after the string should be valid
        assert!(
            is_valid_csharp_literal_location(line, 45, 2),
            "Should allow position after string with escaped quotes"
        );
    }

    #[test]
    fn test_count_unescaped_quotes_empty() {
        assert_eq!(count_unescaped_quotes("", '"'), 0);
        assert_eq!(count_unescaped_quotes("", '\''), 0);
    }

    #[test]
    fn test_count_unescaped_quotes_regular() {
        assert_eq!(count_unescaped_quotes("\"hello\"", '"'), 2);
        assert_eq!(count_unescaped_quotes("'hello'", '\''), 2);
        assert_eq!(count_unescaped_quotes("int x = \"hello\"", '"'), 2);
    }

    #[test]
    fn test_count_unescaped_quotes_escaped() {
        // Escaped quote in double-quoted string
        assert_eq!(count_unescaped_quotes(r#""He said \"hi\"""#, '"'), 2);
        // Escaped quote in single-quoted string
        assert_eq!(count_unescaped_quotes(r"'It\'s fine'", '\''), 2);
    }

    #[test]
    fn test_count_unescaped_quotes_escaped_backslash() {
        // Double backslash doesn't escape the quote
        assert_eq!(count_unescaped_quotes(r#""path\\to\\file""#, '"'), 2);
        // Triple backslash escapes the quote
        assert_eq!(count_unescaped_quotes(r#""test\\\""#, '"'), 1);
    }

    #[test]
    fn test_count_unescaped_quotes_mixed() {
        // Single quotes inside double-quoted string
        assert_eq!(count_unescaped_quotes("\"It's fine\"", '"'), 2);
        assert_eq!(count_unescaped_quotes("\"It's fine\"", '\''), 1);
    }

    #[test]
    fn test_plan_extract_constant_hex_literal() {
        let source = r#"
class Colors
{
    public void SetColor()
    {
        int red = 0xFF0000;
        int green = 0x00FF00;
    }
}"#;
        // Column 18 is inside the hex literal 0xFF0000
        let result = plan_extract_constant(source, 5, 18, "COLOR_RED", "Colors.cs");
        assert!(
            result.is_ok(),
            "Should extract hex literal: {:?}",
            result.err()
        );

        let plan = result.unwrap();
        // Check that we have the expected edits
        assert!(
            plan.edits.len() >= 2,
            "Should have insertion and replacement edits"
        );
    }

    #[test]
    fn test_plan_extract_constant_with_escaped_quotes() {
        let source = r#"
class Program
{
    static void Main(string[] args)
    {
        string msg = "He said \"hello\"";
        string greeting = "He said \"hello\"";
    }
}"#;
        // Column 24 is inside the first string literal
        let result = plan_extract_constant(source, 5, 24, "GREETING_MSG", "test.cs");
        assert!(
            result.is_ok(),
            "Should extract string with escaped quotes: {:?}",
            result.err()
        );

        let plan = result.unwrap();
        // Should find both occurrences
        assert_eq!(
            plan.edits.len(),
            3,
            "Should have 1 insert + 2 replace edits"
        );
    }

    #[test]
    fn test_find_csharp_literal_occurrences_escaped_quotes() {
        // Should not match literal inside string with escaped quotes
        let source = r#"const int TAX_RATE = 8;
string msg = "Rate is \"8\"";
int value = 8;"#;
        let occurrences = find_literal_occurrences(source, "8", is_valid_csharp_literal_location);
        // Should find 2 occurrences (lines 0 and 2), but not the one inside the string
        assert_eq!(
            occurrences.len(),
            2,
            "Should find exactly 2 valid occurrences"
        );
        assert_eq!(occurrences[0].start_line, 0);
        assert_eq!(occurrences[1].start_line, 2);
    }

    #[test]
    fn test_plan_extract_constant_negative_number() {
        let source = r#"
class Program
{
    static void Main(string[] args)
    {
        int temp = -42;
        int cold = -42;
    }
}"#;
        // Column 20 is inside the negative literal (on the '4')
        let result = plan_extract_constant(source, 5, 20, "FREEZING_TEMP", "test.cs");
        assert!(
            result.is_ok(),
            "Should extract negative number: {:?}",
            result.err()
        );

        let plan = result.unwrap();
        assert_eq!(
            plan.edits.len(),
            3,
            "Should have 1 insert + 2 replace edits"
        );
    }

    #[test]
    fn test_plan_extract_constant_decimal_with_suffix() {
        let source = r#"
class Program
{
    static void Main(string[] args)
    {
        decimal price = 19.99m;
        decimal tax = 19.99m;
    }
}"#;
        // Column 24 is inside the decimal literal
        let result = plan_extract_constant(source, 5, 24, "DEFAULT_PRICE", "test.cs");
        assert!(
            result.is_ok(),
            "Should extract decimal with suffix: {:?}",
            result.err()
        );

        let plan = result.unwrap();
        assert_eq!(
            plan.edits.len(),
            3,
            "Should have 1 insert + 2 replace edits"
        );
    }
}
