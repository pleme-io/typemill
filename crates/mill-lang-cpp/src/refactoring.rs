//! Refactoring operations for C++ code
//!
//! Provides AST-based refactoring capabilities including:
//! - Extract function (simple cases - no templates, no complex captures)
//! - Extract variable
//! - Inline variable
//!
//! Note: This implementation handles common C++ refactoring scenarios.
//! Complex cases involving templates, macros, or advanced C++ features
//! may require manual intervention or LSP-based refactoring (clangd).

use async_trait::async_trait;
use mill_foundation::protocol::{
    EditPlan, EditPlanMetadata, EditType, TextEdit, ValidationRule, ValidationType,
};
use mill_lang_common::refactoring::CodeRange as CommonCodeRange;
use mill_plugin_api::{PluginError, PluginResult, RefactoringProvider};
use std::collections::HashMap;
use tree_sitter::{Node, Parser, Point};
use crate::ast_parser::get_cpp_language;

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
            .map_err(|e| PluginError::invalid_input(format!("Inline variable failed: {}", e)))
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
            .map_err(|e| PluginError::invalid_input(format!("Extract function failed: {}", e)))
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
        .map_err(|e| PluginError::invalid_input(format!("Extract variable failed: {}", e)))
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

/// Generate edit plan for C++ extract function refactoring
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

    let selected_node = find_smallest_node_containing_range(root, start_point, end_point)
        .ok_or_else(|| "Could not find a node for the selection".to_string())?;

    let selected_text = selected_node
        .utf8_text(source.as_bytes())
        .map_err(|e| format!("Failed to get selected text: {}", e))?
        .to_string();

    // Find the enclosing function
    let enclosing_function = find_ancestor_of_kind(selected_node, "function_definition")
        .ok_or_else(|| "Selection is not inside a function".to_string())?;

    // Get indentation
    let indent = get_indentation(source, enclosing_function.start_position().row);
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

/// Generate edit plan for C++ extract variable refactoring
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

    let start_point = Point::new(start_line as usize - 1, start_col as usize);
    let end_point = Point::new(end_line as usize - 1, end_col as usize);

    let selected_node = find_smallest_node_containing_range(root, start_point, end_point)
        .ok_or_else(|| "Could not find a node for the selection".to_string())?;

    let expression_text = selected_node
        .utf8_text(source.as_bytes())
        .map_err(|e| format!("Failed to get expression text: {}", e))?
        .to_string();

    // Find a statement to insert before
    let insertion_node = find_ancestor_of_kind(selected_node, "expression_statement")
        .or_else(|| find_ancestor_of_kind(selected_node, "declaration"))
        .ok_or_else(|| "Could not find statement to insert before".to_string())?;

    let indent = get_indentation(source, insertion_node.start_position().row);
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

/// Generate edit plan for C++ inline variable refactoring
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

    #[test]
    fn test_extract_cpp_function() {
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
    fn test_extract_cpp_variable() {
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
    fn test_inline_cpp_variable() {
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
}
