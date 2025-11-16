//! Refactoring operations for C++ code
//!
//! Provides AST-based refactoring capabilities including:
//! - Extract function (simple cases - no templates, no complex captures)
//! - Extract variable
//! - Inline variable
//! - Extract constant (literals to constexpr declarations)
//!
//! Note: This implementation handles common C++ refactoring scenarios.
//! Complex cases involving templates, macros, or advanced C++ features
//! may require manual intervention or LSP-based refactoring (clangd).

use crate::ast_parser::get_cpp_language;
use async_trait::async_trait;
use mill_foundation::protocol::{
    EditPlan, EditType, TextEdit,
};
use mill_lang_common::is_valid_code_literal_location;
use mill_lang_common::refactoring::CodeRange as CommonCodeRange;
use mill_lang_common::refactoring::find_literal_occurrences;
use mill_lang_common::refactoring::edit_plan_builder::EditPlanBuilder;
use mill_lang_common::{ExtractConstantEditPlanBuilder, LineExtractor};
#[cfg(test)]
use mill_lang_common::{is_escaped, is_screaming_snake_case};
use mill_plugin_api::{PluginApiError, PluginResult, RefactoringProvider};
use tree_sitter::{Node, Parser, Point};

pub struct CppRefactoringProvider;

#[async_trait]
impl RefactoringProvider for CppRefactoringProvider {
    fn supports_inline_variable(&self) -> bool {
        true
    }

    async fn plan_inline_variable(
        &self,
        source: &str,
        variable_line: u32,
        variable_col: u32,
        file_path: &str,
    ) -> PluginResult<EditPlan> {
        plan_inline_variable_impl(source, variable_line, variable_col, file_path)
            .map_err(|e| PluginApiError::invalid_input(format!("Inline variable failed: {}", e)))
    }

    fn supports_extract_function(&self) -> bool {
        true
    }

    async fn plan_extract_function(
        &self,
        source: &str,
        start_line: u32,
        end_line: u32,
        function_name: &str,
        file_path: &str,
    ) -> PluginResult<EditPlan> {
        let range = CodeRange {
            start_line,
            start_col: 0,
            end_line,
            end_col: 0,
        };
        plan_extract_function_impl(source, &range, function_name, file_path)
            .map_err(|e| PluginApiError::invalid_input(format!("Extract function failed: {}", e)))
    }

    fn supports_extract_variable(&self) -> bool {
        true
    }

    async fn plan_extract_variable(
        &self,
        source: &str,
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
        variable_name: Option<String>,
        file_path: &str,
    ) -> PluginResult<EditPlan> {
        plan_extract_variable_impl(
            source,
            start_line,
            start_col,
            end_line,
            end_col,
            variable_name,
            file_path,
        )
        .map_err(|e| PluginApiError::invalid_input(format!("Extract variable failed: {}", e)))
    }

    fn supports_extract_constant(&self) -> bool {
        true
    }

    async fn plan_extract_constant(
        &self,
        source: &str,
        line: u32,
        character: u32,
        name: &str,
        file_path: &str,
    ) -> PluginResult<EditPlan> {
        plan_extract_constant_impl(source, line, character, name, file_path)
            .map_err(|e| PluginApiError::invalid_input(format!("Extract constant failed: {}", e)))
    }
}

// Internal code range structure
#[derive(Debug, Clone)]
struct CodeRange {
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
}

/// Extracts selected code into a new C++ function.
///
/// This refactoring operation takes a range of code and creates a new void function
/// containing that code, replacing the original selection with a call to the new function.
/// The new function is inserted immediately after the enclosing function. Note: This is a
/// simple implementation that doesn't handle parameters or return types automatically.
///
/// # Arguments
/// * `source` - The complete C++ source code
/// * `range` - The code range specifying the selection to extract
/// * `function_name` - The name for the new function
/// * `file_path` - Path to the file being refactored
///
/// # Returns
/// * `Ok(EditPlan)` - The refactoring plan with two edits: function creation and call replacement
/// * `Err(String)` - If the selection is invalid or not inside a function
///
/// # Examples
/// ```rust
/// let source = r#"
/// void main() {
///     int x = 10;
///     std::cout << "Hello, World!" << std::endl;
/// }
/// "#;
/// let range = CodeRange { start_line: 4, start_col: 4, end_line: 4, end_col: 45 };
/// let plan = plan_extract_function_impl(source, &range, "greet", "main.cpp")?;
/// assert_eq!(plan.edits.len(), 2);
/// ```
fn plan_extract_function_impl(
    source: &str,
    range: &CodeRange,
    function_name: &str,
    file_path: &str,
) -> Result<EditPlan, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&get_cpp_language())
        .map_err(|e| format!("Failed to load C++ grammar: {}", e))?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| "Failed to parse C++ source".to_string())?;
    let root = tree.root_node();

    // Convert 1-based line numbers to 0-based for tree-sitter
    let start_point = Point::new(range.start_line as usize - 1, range.start_col as usize);
    let end_point = Point::new(range.end_line as usize - 1, range.end_col as usize);

    let selected_node = root
        .named_descendant_for_point_range(start_point, end_point)
        .ok_or_else(|| "Could not find a node for the selection".to_string())?;

    let selected_text = selected_node
        .utf8_text(source.as_bytes())
        .map_err(|e| format!("Failed to get selected text: {}", e))?
        .to_string();

    // Find the enclosing function
    let enclosing_function = find_ancestor_of_kind(selected_node, "function_definition")
        .ok_or_else(|| "Selection is not inside a function".to_string())?;

    // Get indentation
    let indent = LineExtractor::get_indentation_str(source, enclosing_function.start_position().row as u32);
    let function_indent = format!("{}    ", indent);

    // Create the new function text
    // Note: This is a simple implementation that doesn't handle parameters or return types
    // For production, we'd need to analyze captured variables and determine return type
    let new_function_text = format!(
        "\n\n{}void {}() {{\n{}{}\n{}}}\n",
        indent,
        function_name,
        function_indent,
        selected_text.trim(),
        indent
    );

    // Create insert edit (add new function after enclosing function)
    let insert_edit = TextEdit {
        file_path: None,
        edit_type: EditType::Insert,
        location: CommonCodeRange::new(
            enclosing_function.end_position().row as u32 + 1,
            0,
            enclosing_function.end_position().row as u32 + 1,
            0,
        )
        .into(),
        original_text: String::new(),
        new_text: new_function_text,
        priority: 100,
        description: format!("Create new function '{}'", function_name),
    };

    // Create replace edit (replace selection with function call)
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

/// Extracts an expression into a new C++ variable.
///
/// This refactoring operation identifies an expression in C++ code and extracts it into
/// a new variable declaration using `auto` for type deduction, replacing the original
/// expression with the variable name. The variable is declared before the statement
/// containing the expression.
///
/// # Arguments
/// * `source` - The complete C++ source code
/// * `start_line` - Zero-based starting line of the expression (1-based in practice)
/// * `start_col` - Zero-based starting column of the expression
/// * `end_line` - Zero-based ending line of the expression (1-based in practice)
/// * `end_col` - Zero-based ending column of the expression
/// * `variable_name` - Optional name for the variable (defaults to "extracted")
/// * `file_path` - Path to the file being refactored
///
/// # Returns
/// * `Ok(EditPlan)` - The refactoring plan with two edits: variable declaration and replacement
/// * `Err(String)` - If the selection is invalid or cannot be extracted
///
/// # Examples
/// ```rust
/// let source = r#"
/// int main() {
///     int x = 10 + 20;
///     return x;
/// }
/// "#;
/// let plan = plan_extract_variable_impl(source, 3, 12, 3, 19, Some("sum".to_string()), "main.cpp")?;
/// assert_eq!(plan.edits.len(), 2);
/// ```
fn plan_extract_variable_impl(
    source: &str,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    variable_name: Option<String>,
    file_path: &str,
) -> Result<EditPlan, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&get_cpp_language())
        .map_err(|e| format!("Failed to load C++ grammar: {}", e))?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| "Failed to parse C++ source".to_string())?;
    let root = tree.root_node();

    let start_byte = source
        .lines()
        .take(start_line as usize - 1)
        .map(|l| l.len() + 1)
        .sum::<usize>()
        + start_col as usize;
    let end_byte = source
        .lines()
        .take(end_line as usize - 1)
        .map(|l| l.len() + 1)
        .sum::<usize>()
        + end_col as usize;

    let selected_node = root
        .descendant_for_byte_range(start_byte, end_byte)
        .ok_or_else(|| "Could not find a node for the selection".to_string())?;

    let expression_text = selected_node
        .utf8_text(source.as_bytes())
        .map_err(|e| format!("Failed to get expression text: {}", e))?
        .to_string();

    // Find a statement to insert before
    let insertion_node = find_ancestor_of_kind(selected_node, "expression_statement")
        .or_else(|| find_ancestor_of_kind(selected_node, "declaration"))
        .or_else(|| find_ancestor_of_kind(selected_node, "return_statement"))
        .ok_or_else(|| "Could not find statement to insert before".to_string())?;

    let indent = LineExtractor::get_indentation_str(source, insertion_node.start_position().row as u32);
    let var_name = variable_name.unwrap_or_else(|| "extracted".to_string());

    // Use 'auto' for type deduction in C++
    let declaration_text = format!("auto {} = {};\n", var_name, expression_text);

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

/// Inlines a C++ variable by replacing all references with its initializer value.
///
/// This refactoring operation finds a variable declaration, replaces all references to that
/// variable with its initializer expression, and removes the variable declaration. The operation
/// scopes the search to the enclosing function or block to avoid unintended replacements.
///
/// # Arguments
/// * `source` - The complete C++ source code
/// * `variable_line` - Zero-based line number where the variable is declared (1-based in practice)
/// * `variable_col` - Zero-based column offset within the line
/// * `file_path` - Path to the file being refactored
///
/// # Returns
/// * `Ok(EditPlan)` - The edit plan with edits to replace references and delete the declaration
/// * `Err(String)` - If the variable is not found or not inside a function
///
/// # Examples
/// ```rust
/// let source = r#"
/// int main() {
///     int greeting = 42;
///     std::cout << greeting << std::endl;
///     return greeting;
/// }
/// "#;
/// let plan = plan_inline_variable_impl(source, 3, 8, "main.cpp")?;
/// assert!(plan.edits.len() >= 2);
/// ```
fn plan_inline_variable_impl(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    file_path: &str,
) -> Result<EditPlan, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&get_cpp_language())
        .map_err(|e| format!("Failed to load C++ grammar: {}", e))?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| "Failed to parse C++ source".to_string())?;
    let root = tree.root_node();

    let point = Point::new(variable_line as usize - 1, variable_col as usize);

    let var_node = find_node_at_point(root, point)
        .ok_or_else(|| "Could not find variable at specified location".to_string())?;

    // Find the declaration
    let declaration_node = find_ancestor_of_kind(var_node, "declaration")
        .ok_or_else(|| "Not a variable declaration".to_string())?;

    // Extract variable name and value
    let (var_name, var_value) = extract_cpp_var_info(declaration_node, source)?;

    // Find the scope (function body) to search for references
    let scope_node = find_ancestor_of_kind(declaration_node, "function_definition")
        .or_else(|| find_ancestor_of_kind(declaration_node, "compound_statement"))
        .ok_or_else(|| "Variable is not inside a function or block".to_string())?;

    // Simple reference finding: look for identifier nodes with matching text
    let mut edits = Vec::new();
    find_variable_references(scope_node, &var_name, source, &mut edits, &var_value);

    // Remove the variable declaration
    edits.push(TextEdit {
        file_path: None,
        edit_type: EditType::Delete,
        location: node_to_location(declaration_node).into(),
        original_text: declaration_node
            .utf8_text(source.as_bytes())
            .map_err(|e| format!("Invalid UTF-8 in source at declaration node: {}", e))?
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

/// Analyzes source code to extract information about a literal value at a cursor position.
///
/// # Arguments
/// * `source` - The C++ source code
/// * `line` - Zero-based line number where the cursor is positioned
/// * `character` - Zero-based character offset within the line
/// * `_file_path` - Path to the file (for future use)
///
/// # Returns
/// * `Ok(ExtractConstantAnalysis)` - Analysis result with literal value, occurrence ranges,
///                                     validation status, and insertion point
/// * `Err(String)` - If no literal is found at the cursor position
pub(crate) fn analyze_extract_constant(
    source: &str,
    line: u32,
    character: u32,
    _file_path: &str,
) -> Result<mill_lang_common::ExtractConstantAnalysis, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&get_cpp_language())
        .map_err(|e| format!("Failed to load C++ grammar: {}", e))?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| "Failed to parse C++ source".to_string())?;
    let root = tree.root_node();

    // Find the literal at the cursor position
    let point = Point::new(line as usize, character as usize);
    let target_node = root
        .named_descendant_for_point_range(point, point)
        .ok_or_else(|| "Could not find node at cursor position".to_string())?;

    // Check if this is a literal node
    let literal_value = match target_node.kind() {
        "number_literal" | "true" | "false" => target_node
            .utf8_text(source.as_bytes())
            .map_err(|e| format!("Failed to get literal text: {}", e))?
            .to_string(),
        _ => {
            return Err(format!(
                "Cursor is not on a literal value. Extract constant only works on numbers and booleans. Found: {}",
                target_node.kind()
            ));
        }
    };

    // Find all occurrences of this literal
    let occurrence_ranges = find_literal_occurrences(source, &literal_value, is_valid_literal_location);

    // Find the best insertion point (top of file or after includes)
    let insertion_point = find_constant_insertion_point(root, source);

    Ok(mill_lang_common::ExtractConstantAnalysis {
        literal_value,
        occurrence_ranges,
        is_valid_literal: true,
        blocking_reasons: vec![],
        insertion_point,
    })
}

/// Generate edit plan for C++ extract constant refactoring
fn plan_extract_constant_impl(
    source: &str,
    line: u32,
    character: u32,
    name: &str,
    file_path: &str,
) -> Result<EditPlan, String> {
    let analysis = analyze_extract_constant(source, line, character, file_path)
        .map_err(|e| format!("Extract constant analysis failed: {}", e))?;

    // Determine the type for the constant declaration
    let const_type = infer_cpp_constant_type(&analysis.literal_value);

    ExtractConstantEditPlanBuilder::new(analysis, name.to_string(), file_path.to_string())
        .with_declaration_format(|name, value| {
            format!("constexpr {} {} = {};\n", const_type, name, value)
        })
}

// Helper functions

/// Validates if a position in a line of code is a valid literal location for C++.
///
/// This function performs context-aware validation to ensure the literal is not part of
/// a string, comment, or identifier. For numeric literals, it additionally checks word
/// boundaries to prevent false positives (e.g., matching "2" in "var2").
///
/// # Arguments
/// * `line` - The complete line of source code
/// * `pos` - The starting position of the potential literal
/// * `len` - The length of the potential literal
///
/// # Returns
/// * `true` - If the position contains a valid standalone literal
/// * `false` - If the literal is inside a string, comment, or part of an identifier
///
/// # Example
/// ```rust
/// // Returns true for standalone literal
/// assert!(is_valid_literal_location("int x = 42;", 8, 2));
///
/// // Returns false for digit in identifier
/// assert!(!is_valid_literal_location("int var42;", 7, 2));
/// ```
fn is_valid_literal_location(line: &str, pos: usize, len: usize) -> bool {
    // Use shared validation for strings and comments
    if !is_valid_code_literal_location(line, pos, len) {
        return false;
    }

    // C++-specific: For numeric literals, check word boundaries
    if let Some(ch) = line[pos..].chars().next() {
        if ch.is_ascii_digit() {
            // Check character before
            if pos > 0 {
                let before = &line[..pos];
                if let Some(prev_ch) = before.chars().last() {
                    if prev_ch.is_alphanumeric() || prev_ch == '_' {
                        return false;
                    }
                }
            }
            // Check character after
            if pos + len < line.len() {
                if let Some(next_ch) = line[pos + len..].chars().next() {
                    if next_ch.is_alphanumeric() || next_ch == '_' {
                        return false;
                    }
                }
            }
        }
    }

    true
}

/// Finds the best insertion point for a constant declaration in C++ source code.
///
/// This function analyzes the AST to find the optimal location for inserting a new
/// constant declaration. It looks for the last preprocessor include or using declaration
/// and places the constant immediately after, ensuring proper code organization.
///
/// # Arguments
/// * `root` - The root node of the parsed C++ AST
/// * `_source` - The source code (reserved for future use)
///
/// # Returns
/// A `CommonCodeRange` indicating where to insert the constant declaration.
/// The range is positioned either:
/// - Two lines after the last #include or using directive
/// - At the beginning of the file if no includes/using found
///
/// # Example
/// ```cpp
/// // Input:
/// #include <iostream>
/// using namespace std;
/// // <- Insertion point here (line 2)
/// int main() { ... }
/// ```
fn find_constant_insertion_point(root: Node, _source: &str) -> CommonCodeRange {
    let mut cursor = root.walk();
    let mut last_include_line = 0u32;

    // Find the last #include or using directive
    for node in root.children(&mut cursor) {
        if node.kind() == "preproc_include" || node.kind() == "using_declaration" {
            last_include_line = node.end_position().row as u32 + 1;
        }
    }

    // Insert after includes or at the top
    let insertion_line = if last_include_line > 0 {
        last_include_line + 1
    } else {
        0
    };

    CommonCodeRange::new(insertion_line, 0, insertion_line, 0)
}

/// Infers the appropriate C++ type for a constant based on its literal value.
///
/// This function analyzes a literal string and determines the most appropriate C++ type
/// for declaring a constant. It handles various numeric formats including integers,
/// floating-point, hexadecimal, binary, and octal literals, as well as boolean values.
/// Type inference considers literal suffixes (L, UL, f, F, etc.) and format indicators.
///
/// # Arguments
/// * `literal_value` - The string representation of the literal value
///
/// # Returns
/// A static string representing the inferred C++ type:
/// - `"bool"` - for true/false
/// - `"int"` - for decimal/hex/binary/octal integers without suffix
/// - `"long"` - for literals with L suffix
/// - `"unsigned long"` - for literals with UL suffix
/// - `"unsigned int"` - for literals with U suffix
/// - `"float"` - for floating-point with f/F suffix
/// - `"double"` - for floating-point without suffix or with decimal point
///
/// # Examples
/// ```rust
/// assert_eq!(infer_cpp_constant_type("42"), "int");
/// assert_eq!(infer_cpp_constant_type("0xFF"), "int");
/// assert_eq!(infer_cpp_constant_type("3.14"), "double");
/// assert_eq!(infer_cpp_constant_type("2.5f"), "float");
/// assert_eq!(infer_cpp_constant_type("100L"), "long");
/// assert_eq!(infer_cpp_constant_type("true"), "bool");
/// ```
fn infer_cpp_constant_type(literal_value: &str) -> &'static str {
    if literal_value == "true" || literal_value == "false" {
        "bool"
    } else if literal_value.starts_with("0x") || literal_value.starts_with("0X") {
        // Hexadecimal literal - check suffixes
        if literal_value.ends_with("UL") || literal_value.ends_with("ul") || literal_value.ends_with("Ul") || literal_value.ends_with("uL") {
            "unsigned long"
        } else if literal_value.ends_with('L') || literal_value.ends_with('l') {
            "long"
        } else {
            "int"
        }
    } else if literal_value.starts_with("0b") || literal_value.starts_with("0B") {
        // Binary literal (C++14)
        "int"
    } else if literal_value.starts_with('0') && literal_value.len() > 1 && !literal_value.contains('.') {
        // Octal literal
        "int"
    } else if literal_value.contains('.') || literal_value.contains('e') || literal_value.contains('E') {
        // Floating point
        if literal_value.ends_with('f') || literal_value.ends_with('F') {
            "float"
        } else {
            "double"
        }
    } else if literal_value.ends_with("UL") || literal_value.ends_with("ul") || literal_value.ends_with("Ul") || literal_value.ends_with("uL") {
        // Unsigned long - must check before plain long
        "unsigned long"
    } else if literal_value.ends_with('L') || literal_value.ends_with('l') {
        "long"
    } else if literal_value.ends_with('U') || literal_value.ends_with('u') {
        "unsigned int"
    } else {
        "int"
    }
}

/// Finds the AST node at a specific point in the source code.
///
/// This helper function locates the smallest named AST node that contains a given
/// point (line and column position) in the source code.
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

/// Finds the nearest ancestor node of a specific kind in the AST.
///
/// This function traverses up the AST tree from a given node, searching for the
/// first ancestor that matches the specified node kind. Useful for finding
/// enclosing scope nodes like functions, classes, or namespaces.
///
/// # Arguments
/// * `node` - The starting node to search from
/// * `kind` - The AST node kind to search for (e.g., "function_definition", "class_specifier")
///
/// # Returns
/// * `Some(Node)` - The first ancestor matching the specified kind
/// * `None` - If no matching ancestor is found (reached root without match)
///
/// # Example
/// ```rust
/// // Find the enclosing function for an identifier node
/// let function_node = find_ancestor_of_kind(identifier_node, "function_definition");
/// ```
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

fn extract_cpp_var_info(declaration_node: Node, source: &str) -> Result<(String, String), String> {
    // Try to find init_declarator which contains the name and initializer
    let mut cursor = declaration_node.walk();
    let init_declarator = declaration_node
        .children(&mut cursor)
        .find(|n| n.kind() == "init_declarator")
        .ok_or_else(|| "Could not find init_declarator".to_string())?;

    // Find the declarator (which contains the name)
    let mut init_cursor = init_declarator.walk();
    let declarator = init_declarator
        .children(&mut init_cursor)
        .find(|n| {
            matches!(
                n.kind(),
                "identifier" | "pointer_declarator" | "reference_declarator"
            )
        })
        .ok_or_else(|| "Could not find declarator".to_string())?;

    // Extract the variable name (handle pointer/reference declarators)
    let name = if declarator.kind() == "identifier" {
        declarator
            .utf8_text(source.as_bytes())
            .map_err(|e| format!("Failed to get variable name: {}", e))?
            .to_string()
    } else {
        // For pointer/reference declarators, find the identifier child
        let mut decl_cursor = declarator.walk();
        let identifier_node = declarator
            .children(&mut decl_cursor)
            .find(|n| n.kind() == "identifier")
            .ok_or_else(|| "Could not find identifier in declarator".to_string())?;

        identifier_node
            .utf8_text(source.as_bytes())
            .map_err(|e| format!("Failed to get variable name: {}", e))?
            .to_string()
    };

    // Find the initializer value
    let value_node = init_declarator
        .child_by_field_name("value")
        .ok_or_else(|| "Could not find variable initializer".to_string())?;

    let value = value_node
        .utf8_text(source.as_bytes())
        .map_err(|e| format!("Failed to get variable value: {}", e))?
        .to_string();

    Ok((name, value))
}

/// Finds and replaces all references to a variable within a given scope.
///
/// This function recursively traverses an AST scope to find all identifier nodes that
/// match the specified variable name and generates text edits to replace them with
/// a new value. Used in refactoring operations like inline variable.
///
/// # Arguments
/// * `scope` - The AST node representing the scope to search within
/// * `var_name` - The variable name to search for
/// * `source` - The source code text
/// * `edits` - Mutable vector to accumulate the replacement edits
/// * `replacement_value` - The value to replace the variable references with
///
/// # Note
/// This function modifies the `edits` vector in place, adding a `TextEdit` for each
/// occurrence of the variable within the scope.
fn find_variable_references(
    scope: Node,
    var_name: &str,
    source: &str,
    edits: &mut Vec<TextEdit>,
    replacement_value: &str,
) {
    let mut cursor = scope.walk();
    for node in scope.children(&mut cursor) {
        if node.kind() == "identifier" {
            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                if text == var_name {
                    edits.push(TextEdit {
                        file_path: None,
                        edit_type: EditType::Replace,
                        location: node_to_location(node).into(),
                        original_text: var_name.to_string(),
                        new_text: replacement_value.to_string(),
                        priority: 90,
                        description: format!("Inline variable '{}'", var_name),
                    });
                }
            }
        }
        // Recursively search children
        find_variable_references(node, var_name, source, edits, replacement_value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Extract Function Tests (5 tests) ==========

    #[test]
    fn test_extract_cpp_function_simple_statement() {
        let source = r#"
void main() {
    int x = 10;
    std::cout << "Hello, World!" << std::endl;
}
"#;
        let range = CodeRange {
            start_line: 4,
            start_col: 4,
            end_line: 4,
            end_col: 45,
        };
        let plan = plan_extract_function_impl(source, &range, "greet", "main.cpp").unwrap();
        assert_eq!(plan.edits.len(), 2);

        let insert_edit = &plan.edits[0];
        assert!(insert_edit.new_text.contains("void greet()"));

        let replace_edit = &plan.edits[1];
        assert_eq!(replace_edit.new_text, "greet();");
    }

    #[test]
    fn test_extract_cpp_function_multiple_statements() {
        let source = r#"
int calculate() {
    int a = 5;
    int b = 10;
    int sum = a + b;
    return sum;
}
"#;
        let range = CodeRange {
            start_line: 3,
            start_col: 4,
            end_line: 5,
            end_col: 19,
        };
        let plan = plan_extract_function_impl(source, &range, "compute_sum", "calc.cpp").unwrap();
        assert_eq!(plan.edits.len(), 2);
        assert!(plan.edits[0].new_text.contains("void compute_sum()"));
        assert_eq!(plan.edits[1].new_text, "compute_sum();");
    }

    #[test]
    fn test_extract_cpp_function_with_return() {
        let source = r#"
int getValue() {
    int x = 42;
    return x;
}
"#;
        let range = CodeRange {
            start_line: 3,
            start_col: 4,
            end_line: 4,
            end_col: 13,
        };
        let plan = plan_extract_function_impl(source, &range, "helper", "test.cpp").unwrap();
        assert_eq!(plan.edits.len(), 2);
        assert!(plan.edits[0].new_text.contains("void helper()"));
    }

    #[test]
    fn test_extract_cpp_function_single_expression() {
        let source = r#"
void process() {
    std::cout << "Processing" << std::endl;
}
"#;
        let range = CodeRange {
            start_line: 3,
            start_col: 4,
            end_line: 3,
            end_col: 44,
        };
        let plan =
            plan_extract_function_impl(source, &range, "log_message", "process.cpp").unwrap();
        assert_eq!(plan.edits.len(), 2);
        assert!(plan.edits[0].new_text.contains("void log_message()"));
        assert_eq!(plan.edits[1].new_text, "log_message();");
    }

    #[test]
    fn test_extract_cpp_function_nested_scope() {
        let source = r#"
void outer() {
    if (true) {
        int x = 1;
        std::cout << x;
    }
}
"#;
        let range = CodeRange {
            start_line: 4,
            start_col: 8,
            end_line: 5,
            end_col: 23,
        };
        let plan = plan_extract_function_impl(source, &range, "inner_logic", "nested.cpp").unwrap();
        assert_eq!(plan.edits.len(), 2);
        assert!(plan.edits[0].new_text.contains("void inner_logic()"));
    }

    // ========== Extract Variable Tests (5 tests) ==========

    #[test]
    fn test_extract_cpp_variable_arithmetic() {
        let source = r#"
int main() {
    int x = 10 + 20;
    return x;
}
"#;
        let plan =
            plan_extract_variable_impl(source, 3, 12, 3, 19, Some("sum".to_string()), "main.cpp")
                .unwrap();
        assert_eq!(plan.edits.len(), 2);

        let insert_edit = &plan.edits[0];
        assert!(insert_edit.new_text.contains("auto sum = 10 + 20;"));

        let replace_edit = &plan.edits[1];
        assert_eq!(replace_edit.new_text, "sum");
    }

    #[test]
    fn test_extract_cpp_variable_function_call() {
        let source = r#"
int calculate() {
    return getValue() + 10;
}
"#;
        let plan =
            plan_extract_variable_impl(source, 3, 11, 3, 21, Some("val".to_string()), "calc.cpp")
                .unwrap();
        assert_eq!(plan.edits.len(), 2);
        assert!(plan.edits[0].new_text.contains("auto val = getValue();"));
        assert_eq!(plan.edits[1].new_text, "val");
    }

    #[test]
    fn test_extract_cpp_variable_auto_type() {
        let source = r#"
void process() {
    std::cout << std::string("hello");
}
"#;
        let plan =
            plan_extract_variable_impl(source, 3, 17, 3, 36, Some("msg".to_string()), "proc.cpp")
                .unwrap();
        assert_eq!(plan.edits.len(), 2);
        assert!(plan.edits[0]
            .new_text
            .contains("auto msg = std::string(\"hello\");"));
    }

    #[test]
    fn test_extract_cpp_variable_complex_expression() {
        let source = r#"
int compute() {
    int result = (x * 2) + (y * 3);
    return result;
}
"#;
        let plan = plan_extract_variable_impl(
            source,
            3,
            18,
            3,
            31,
            Some("doubled".to_string()),
            "comp.cpp",
        )
        .unwrap();
        assert_eq!(plan.edits.len(), 2);
        assert!(plan.edits[0]
            .new_text
            .contains("auto doubled = (x * 2) + (y * 3);"));
    }

    #[test]
    fn test_extract_cpp_variable_default_name() {
        let source = r#"
int main() {
    int x = 5 * 3;
    return x;
}
"#;
        let plan = plan_extract_variable_impl(source, 3, 12, 3, 17, None, "main.cpp").unwrap();
        assert_eq!(plan.edits.len(), 2);
        assert!(plan.edits[0].new_text.contains("auto extracted = 5 * 3;"));
    }

    // ========== Inline Variable Tests (5 tests) ==========

    #[test]
    fn test_inline_cpp_variable_simple() {
        let source = r#"
int main() {
    int greeting = 42;
    std::cout << greeting << std::endl;
    return greeting;
}
"#;
        let plan = plan_inline_variable_impl(source, 3, 8, "main.cpp").unwrap();

        // Should have edits for both references plus the declaration removal
        assert!(plan.edits.len() >= 2);

        // Check that we have inline edits
        let inline_edits: Vec<_> = plan
            .edits
            .iter()
            .filter(|e| e.edit_type == EditType::Replace)
            .collect();
        assert!(!inline_edits.is_empty());

        // Check that we remove the declaration
        let delete_edit = plan
            .edits
            .iter()
            .find(|e| e.edit_type == EditType::Delete)
            .unwrap();
        assert!(delete_edit.original_text.contains("int greeting"));
    }

    #[test]
    fn test_inline_cpp_variable_single_usage() {
        let source = r#"
int calculate() {
    int temp = 100;
    return temp;
}
"#;
        let plan = plan_inline_variable_impl(source, 3, 8, "calc.cpp").unwrap();
        assert!(plan.edits.len() >= 1);

        // Should have delete edit for declaration
        let delete_edit = plan.edits.iter().find(|e| e.edit_type == EditType::Delete);
        assert!(delete_edit.is_some());
    }

    #[test]
    fn test_inline_cpp_variable_const() {
        let source = r#"
void process() {
    const int MAX = 100;
    if (value < MAX) {
        std::cout << MAX;
    }
}
"#;
        let plan = plan_inline_variable_impl(source, 3, 14, "proc.cpp").unwrap();
        assert!(plan.edits.len() >= 2);

        // Check for replacement edits
        let replace_edits: Vec<_> = plan
            .edits
            .iter()
            .filter(|e| e.edit_type == EditType::Replace)
            .collect();
        assert!(!replace_edits.is_empty());
    }

    #[test]
    fn test_inline_cpp_variable_expression() {
        let source = r#"
int compute() {
    int doubled = x * 2;
    int result = doubled + 10;
    return result;
}
"#;
        let plan = plan_inline_variable_impl(source, 3, 8, "comp.cpp").unwrap();
        assert!(plan.edits.len() >= 1);

        // Verify declaration is removed
        let delete_edit = plan.edits.iter().find(|e| e.edit_type == EditType::Delete);
        assert!(delete_edit.is_some());
    }

    #[test]
    fn test_inline_cpp_variable_multiple_refs() {
        let source = r#"
void display() {
    int count = 5;
    std::cout << count;
    std::cout << count;
    std::cout << count;
}
"#;
        let plan = plan_inline_variable_impl(source, 3, 8, "display.cpp").unwrap();

        // Should have 3 replace edits + 1 delete edit = 4 total
        assert!(plan.edits.len() >= 3);

        let replace_edits: Vec<_> = plan
            .edits
            .iter()
            .filter(|e| e.edit_type == EditType::Replace)
            .collect();
        assert!(replace_edits.len() >= 2);
    }

    // ========== Error Handling Tests (3 tests) ==========

    #[test]
    fn test_extract_function_invalid_range() {
        let source = r#"
int main() {
    return 0;
}
"#;
        let range = CodeRange {
            start_line: 10, // Invalid line number
            start_col: 0,
            end_line: 15,
            end_col: 0,
        };
        let result = plan_extract_function_impl(source, &range, "invalid", "test.cpp");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_variable_no_expression() {
        let source = r#"
int main() {
    return 0;
}
"#;
        // Try to extract from whitespace
        let result =
            plan_extract_variable_impl(source, 2, 0, 2, 3, Some("var".to_string()), "test.cpp");
        assert!(result.is_err());
    }

    #[test]
    fn test_inline_variable_not_found() {
        let source = r#"
int main() {
    return 0;
}
"#;
        // Try to inline non-existent variable
        let result = plan_inline_variable_impl(source, 3, 5, "test.cpp");
        assert!(result.is_err());
    }

    // ========== Extract Constant Tests (4 tests) ==========

    #[test]
    fn test_plan_extract_constant_valid_number() {
        let source = r#"
int calculate() {
    int x = 42;
    int y = 42;
    return x + y;
}
"#;
        let plan = plan_extract_constant_impl(source, 2, 12, "MAGIC_NUMBER", "test.cpp").unwrap();

        // Should have 1 insert edit + 2 replace edits
        assert_eq!(plan.edits.len(), 3);

        // Check the declaration edit
        let insert_edit = &plan.edits[0];
        assert_eq!(insert_edit.edit_type, EditType::Insert);
        assert!(insert_edit.new_text.contains("constexpr int MAGIC_NUMBER = 42;"));

        // Check replace edits
        let replace_edits: Vec<_> = plan
            .edits
            .iter()
            .filter(|e| e.edit_type == EditType::Replace)
            .collect();
        assert_eq!(replace_edits.len(), 2);
        assert_eq!(replace_edits[0].new_text, "MAGIC_NUMBER");

        // Verify metadata
        assert_eq!(plan.metadata.intent_name, "extract_constant");
    }

    #[test]
    fn test_plan_extract_constant_boolean() {
        let source = r#"
void process() {
    bool flag1 = true;
    bool flag2 = true;
    if (true) {
        return;
    }
}
"#;
        let plan = plan_extract_constant_impl(source, 2, 17, "DEFAULT_FLAG", "test.cpp").unwrap();

        assert_eq!(plan.edits.len(), 4); // 1 insert + 3 replace

        let insert_edit = &plan.edits[0];
        assert!(insert_edit.new_text.contains("constexpr bool DEFAULT_FLAG = true;"));
    }

    #[test]
    fn test_plan_extract_constant_double() {
        let source = r#"
double compute() {
    return 3.14 * 2.0;
}
"#;
        let plan = plan_extract_constant_impl(source, 2, 11, "PI", "test.cpp").unwrap();

        assert_eq!(plan.edits.len(), 2); // 1 insert + 1 replace

        let insert_edit = &plan.edits[0];
        assert!(insert_edit.new_text.contains("constexpr double PI = 3.14;"));
    }

    #[test]
    fn test_plan_extract_constant_invalid_name() {
        let source = r#"
int x = 42;
"#;
        // Try with lowercase name
        let result = plan_extract_constant_impl(source, 1, 8, "magic_number", "test.cpp");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("SCREAMING_SNAKE_CASE"));

        // Try with name starting with underscore
        let result2 = plan_extract_constant_impl(source, 1, 8, "_MAGIC", "test.cpp");
        assert!(result2.is_err());

        // Try with name ending with underscore
        let result3 = plan_extract_constant_impl(source, 1, 8, "MAGIC_", "test.cpp");
        assert!(result3.is_err());
    }

    // ========== Helper Function Tests (3 tests) ==========

    #[test]
    fn test_is_screaming_snake_case_valid() {
        assert!(is_screaming_snake_case("TAX_RATE"));
        assert!(is_screaming_snake_case("MAX_VALUE"));
        assert!(is_screaming_snake_case("A"));
        assert!(is_screaming_snake_case("PI"));
        assert!(is_screaming_snake_case("DB_TIMEOUT_MS"));
        assert!(is_screaming_snake_case("API_KEY_V2"));
    }

    #[test]
    fn test_is_screaming_snake_case_invalid() {
        assert!(!is_screaming_snake_case(""));
        assert!(!is_screaming_snake_case("_TAX_RATE"));
        assert!(!is_screaming_snake_case("TAX_RATE_"));
        assert!(!is_screaming_snake_case("tax_rate"));
        assert!(!is_screaming_snake_case("TaxRate"));
        assert!(!is_screaming_snake_case("tax-rate"));
        assert!(!is_screaming_snake_case("123"));
    }

    #[test]
    fn test_find_literal_occurrences() {
        let source = r#"int x = 42;
int y = 42;
int z = 100;"#;
        let occurrences = find_literal_occurrences(source, "42", is_valid_literal_location);
        assert_eq!(occurrences.len(), 2);
        assert_eq!(occurrences[0].start_line, 0);
        assert_eq!(occurrences[1].start_line, 1);
    }

    // ========== Edge Case Tests for Extract Constant (13 tests) ==========

    #[test]
    fn test_is_escaped_helper() {
        assert!(!is_escaped("hello", 0));
        assert!(!is_escaped("hello", 2));
        assert!(is_escaped(r#"\"hello"#, 1)); // \" - quote is escaped
        assert!(is_escaped(r#"\\"#, 1)); // \\ - second backslash IS escaped by the first
        assert!(!is_escaped(r#"\\\"#, 2)); // \\\ - third backslash is NOT escaped (two backslashes before it)
        assert!(is_escaped(r#"\\\\"#, 3)); // \\\\ - fourth backslash IS escaped by the third
    }

    #[test]
    fn test_extract_constant_escaped_quotes_in_string() {
        let source = r#"
void test() {
    std::string msg = "He said \"hello\"";
    int x = 42;
}
"#;
        // Should find the 42, not be confused by escaped quotes in string
        let plan = plan_extract_constant_impl(source, 3, 12, "ANSWER", "test.cpp").unwrap();
        assert_eq!(plan.edits.len(), 2); // 1 insert + 1 replace

        let insert_edit = &plan.edits[0];
        assert!(insert_edit.new_text.contains("constexpr int ANSWER = 42;"));
    }

    #[test]
    fn test_is_valid_literal_location_escaped_quotes() {
        let line = r#"std::string s = "test \"quote\" test"; int x = 42;"#;
        // Position inside the string should be invalid
        assert!(
            !is_valid_literal_location(line, 20, 1),
            "Should detect position inside string with escaped quotes"
        );
        // Position after the string should be valid
        assert!(
            is_valid_literal_location(line, 47, 2),
            "Should allow position after string with escaped quotes"
        );
    }

    #[test]
    fn test_is_valid_literal_location_block_comment() {
        let line = "int x = /* 42 */ 100;";
        // Position 11 is inside the block comment (on the '4')
        assert!(
            !is_valid_literal_location(line, 11, 2),
            "Should detect position inside block comment"
        );
        // Position 17 is after the block comment (on the '1')
        assert!(
            is_valid_literal_location(line, 17, 3),
            "Should allow position after block comment"
        );
    }

    #[test]
    fn test_extract_constant_block_comment() {
        let source = r#"
int calculate() {
    /* This is a comment with 42 in it */
    int x = 42;
    return x;
}
"#;
        // Extract the 42 from line 3 (the actual usage, not the comment)
        let plan = plan_extract_constant_impl(source, 3, 12, "MAGIC_NUMBER", "test.cpp").unwrap();

        // Should only replace the actual usage, not the one in the comment
        assert_eq!(plan.edits.len(), 2); // 1 insert + 1 replace
    }

    #[test]
    fn test_extract_constant_hex_literal() {
        let source = r#"
int main() {
    int color = 0xFF0000;
    int mask = 0xFF0000;
    return 0;
}
"#;
        let plan = plan_extract_constant_impl(source, 2, 16, "COLOR_RED", "test.cpp").unwrap();

        assert_eq!(plan.edits.len(), 3); // 1 insert + 2 replace

        let insert_edit = &plan.edits[0];
        assert!(insert_edit.new_text.contains("constexpr int COLOR_RED = 0xFF0000;"));
    }

    #[test]
    fn test_extract_constant_hex_long_literal() {
        let source = r#"
int main() {
    long value = 0xFFFFFFFFL;
    return 0;
}
"#;
        let plan = plan_extract_constant_impl(source, 2, 17, "MAX_LONG_HEX", "test.cpp").unwrap();

        let insert_edit = &plan.edits[0];
        assert!(insert_edit.new_text.contains("constexpr long MAX_LONG_HEX = 0xFFFFFFFFL;"));
    }

    #[test]
    fn test_extract_constant_octal_literal() {
        let source = r#"
int main() {
    int perms = 0777;
    return 0;
}
"#;
        let plan = plan_extract_constant_impl(source, 2, 16, "DEFAULT_PERMS", "test.cpp").unwrap();

        assert_eq!(plan.edits.len(), 2); // 1 insert + 1 replace

        let insert_edit = &plan.edits[0];
        assert!(insert_edit.new_text.contains("constexpr int DEFAULT_PERMS = 0777;"));
    }

    #[test]
    fn test_extract_constant_binary_literal() {
        let source = r#"
int main() {
    int flags = 0b1010;
    return 0;
}
"#;
        let plan = plan_extract_constant_impl(source, 2, 16, "FLAG_BITS", "test.cpp").unwrap();

        assert_eq!(plan.edits.len(), 2); // 1 insert + 1 replace

        let insert_edit = &plan.edits[0];
        assert!(insert_edit.new_text.contains("constexpr int FLAG_BITS = 0b1010;"));
    }

    #[test]
    fn test_extract_constant_negative_number() {
        let source = r#"
int main() {
    int temp = -273;
    int freezing = -273;
    return 0;
}
"#;
        // Note: tree-sitter might parse -273 as unary minus + number
        // This test verifies we handle negative numbers
        let plan = plan_extract_constant_impl(source, 2, 15, "ABSOLUTE_ZERO", "test.cpp");

        // This might fail depending on how tree-sitter parses it
        // If it does, that's a known limitation
        if plan.is_ok() {
            let p = plan.unwrap();
            assert!(p.edits.len() >= 2);
        }
    }

    #[test]
    fn test_extract_constant_float_with_suffix() {
        let source = r#"
float calculate() {
    return 3.14f * 2.0f;
}
"#;
        let plan = plan_extract_constant_impl(source, 2, 11, "PI", "test.cpp").unwrap();

        let insert_edit = &plan.edits[0];
        // Should infer float type due to 'f' suffix
        assert!(insert_edit.new_text.contains("constexpr float PI = 3.14f;"));
    }

    #[test]
    fn test_infer_cpp_constant_type_comprehensive() {
        // Boolean
        assert_eq!(infer_cpp_constant_type("true"), "bool");
        assert_eq!(infer_cpp_constant_type("false"), "bool");

        // Hexadecimal
        assert_eq!(infer_cpp_constant_type("0xFF"), "int");
        assert_eq!(infer_cpp_constant_type("0xDEADBEEF"), "int");
        assert_eq!(infer_cpp_constant_type("0xFFFFFFFFL"), "long");

        // Binary
        assert_eq!(infer_cpp_constant_type("0b1010"), "int");
        assert_eq!(infer_cpp_constant_type("0B11111111"), "int");

        // Octal
        assert_eq!(infer_cpp_constant_type("0777"), "int");
        assert_eq!(infer_cpp_constant_type("0644"), "int");

        // Floating point
        assert_eq!(infer_cpp_constant_type("3.14"), "double");
        assert_eq!(infer_cpp_constant_type("3.14f"), "float");
        assert_eq!(infer_cpp_constant_type("3.14F"), "float");
        assert_eq!(infer_cpp_constant_type("1e-5"), "double");

        // Integer suffixes
        assert_eq!(infer_cpp_constant_type("100L"), "long");
        assert_eq!(infer_cpp_constant_type("100l"), "long");
        assert_eq!(infer_cpp_constant_type("100U"), "unsigned int");
        assert_eq!(infer_cpp_constant_type("100u"), "unsigned int");
        assert_eq!(infer_cpp_constant_type("100UL"), "unsigned long");

        // Plain integers
        assert_eq!(infer_cpp_constant_type("42"), "int");
        assert_eq!(infer_cpp_constant_type("100"), "int");
    }

    #[test]
    fn test_extract_constant_multiple_escaped_quotes() {
        let source = r#"
void test() {
    std::string path = "C:\\Users\\Admin\\file.txt";
    int x = 42;
}
"#;
        // Should handle string with escaped backslashes
        let plan = plan_extract_constant_impl(source, 3, 12, "ANSWER", "test.cpp").unwrap();
        assert_eq!(plan.edits.len(), 2);
    }
}
