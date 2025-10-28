//! Swift-specific refactoring operations
//!
//! Provides AST-based refactoring capabilities including:
//! - Extract function
//! - Extract variable
//! - Inline variable

use cb_lang_common::CodeRange as CommonCodeRange;
use cb_protocol::{
    EditPlan, EditPlanMetadata, EditType, TextEdit, ValidationRule, ValidationType,
};
use serde::{Deserialize, Serialize};
use tree_sitter::{Node, Parser, Point, Query, QueryCursor};
use std::collections::HashMap;

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
pub fn plan_extract_function(
    source: &str,
    range: &CodeRange,
    function_name: &str,
    file_path: &str,
) -> RefactoringResult<EditPlan> {
    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_swift::language())
        .map_err(|e| RefactoringError::Parse(format!("Failed to load Swift grammar: {}", e)))?;
    let tree = parser.parse(source, None).ok_or_else(|| RefactoringError::Parse("Failed to parse Swift source".to_string()))?;
    let root = tree.root_node();

    let start_point = Point::new(range.start_line as usize - 1, range.start_col as usize);
    let end_point = Point::new(range.end_line as usize - 1, range.end_col as usize);

    let selected_node = find_smallest_node_containing_range(root, start_point, end_point)
        .ok_or_else(|| RefactoringError::Analysis("Could not find a node for the selection.".to_string()))?;

    let selected_text = selected_node.utf8_text(source.as_bytes()).unwrap().to_string();

    let enclosing_scope = find_ancestor_of_kind(selected_node, "function_declaration")
        .or_else(|| find_ancestor_of_kind(selected_node, "class_declaration"))
        .or_else(|| find_ancestor_of_kind(selected_node, "struct_declaration"))
        .ok_or_else(|| RefactoringError::Analysis("Selection is not inside a valid scope (class, struct, func).".to_string()))?;

    let indent = get_indentation(source, enclosing_scope.start_position().row);
    let func_indent = format!("{}    ", indent);

    let new_function_text = format!("\n\n{}private func {}() {{\n{}{}\n{}}}\n", indent, function_name, func_indent, selected_text.trim(), indent);

    let insert_edit = TextEdit {
        file_path: None,
        edit_type: EditType::Insert,
        location: CommonCodeRange::new(enclosing_scope.end_position().row as u32, 0, enclosing_scope.end_position().row as u32, 0).into(),
        original_text: String::new(),
        new_text: new_function_text,
        priority: 100,
        description: format!("Create new function '{}'", function_name),
    };

    let replace_edit = TextEdit {
        file_path: None,
        edit_type: EditType::Replace,
        location: node_to_location(selected_node).into(),
        original_text: selected_text.to_string(),
        new_text: format!("{}()", function_name),
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
        },
    })
}

/// Generate edit plan for extract variable refactoring
pub fn plan_extract_variable(
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
        .set_language(tree_sitter_swift::language())
        .map_err(|e| RefactoringError::Parse(format!("Failed to load Swift grammar: {}", e)))?;
    let tree = parser.parse(source, None).ok_or_else(|| RefactoringError::Parse("Failed to parse Swift source".to_string()))?;
    let root = tree.root_node();

    let start_point = Point::new(start_line as usize - 1, start_col as usize);
    let end_point = Point::new(end_line as usize - 1, end_col as usize);

    let selected_node = find_smallest_node_containing_range(root, start_point, end_point)
        .ok_or_else(|| RefactoringError::Analysis("Could not find a node for the selection.".to_string()))?;

    let expression_text = selected_node.utf8_text(source.as_bytes()).unwrap().to_string();

    let insertion_node = find_ancestor_of_kind(selected_node, "property_declaration")
        .or_else(|| find_ancestor_of_kind(selected_node, "call_expression"))
        .ok_or_else(|| RefactoringError::Analysis("Could not find statement to insert before.".to_string()))?;

    let indent = get_indentation(source, insertion_node.start_position().row);
    let var_name = variable_name.unwrap_or_else(|| "extracted".to_string());

    let declaration_text = format!("let {} = {}", var_name, expression_text);

    let insert_edit = TextEdit {
        file_path: None,
        edit_type: EditType::Insert,
        location: CommonCodeRange::new(insertion_node.start_position().row as u32 + 1, 0, insertion_node.start_position().row as u32 + 1, 0).into(),
        original_text: String::new(),
        new_text: format!("{}{}\n", indent, declaration_text),
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
        },
    })
}

/// Generate edit plan for inline variable refactoring
pub fn plan_inline_variable(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    file_path: &str,
) -> RefactoringResult<EditPlan> {
    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_swift::language())
        .map_err(|e| RefactoringError::Parse(format!("Failed to load Swift grammar: {}", e)))?;
    let tree = parser.parse(source, None).ok_or_else(|| RefactoringError::Parse("Failed to parse Swift source".to_string()))?;
    let root = tree.root_node();
    let point = Point::new(variable_line as usize - 1, variable_col as usize);

    let var_ident_node = find_node_at_point(root, point)
        .ok_or_else(|| RefactoringError::Analysis("Could not find variable at specified location.".to_string()))?;

    let (var_name, var_value, declaration_node) = extract_swift_var_info(var_ident_node, source)?;

    let scope_node = find_ancestor_of_kind(declaration_node, "function_declaration")
        .or_else(|| find_ancestor_of_kind(declaration_node, "class_declaration"))
        .or_else(|| find_ancestor_of_kind(declaration_node, "struct_declaration"))
        .ok_or_else(|| RefactoringError::Analysis("Variable is not inside a valid scope.".to_string()))?;

    let mut edits = Vec::new();
    let query_str = format!(r#"((simple_identifier) @ref (#eq? @ref "{}"))"#, var_name);
    let query = Query::new(tree_sitter_swift::language(), &query_str).map_err(|e| RefactoringError::Query(e.to_string()))?;
    let mut cursor = QueryCursor::new();

    for match_ in cursor.matches(&query, scope_node, source.as_bytes()) {
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
    }

    edits.push(TextEdit {
        file_path: None,
        edit_type: EditType::Delete,
        location: node_to_location(declaration_node).into(),
        original_text: declaration_node.utf8_text(source.as_bytes()).unwrap().to_string(),
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
        if let Some(containing_child) = find_smallest_node_containing_range(child, start_point, end_point) {
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
    source.lines().nth(line).map(|l| l.chars().take_while(|c| c.is_whitespace()).collect()).unwrap_or_default()
}

fn node_to_location(node: Node) -> CommonCodeRange {
    let range = node.range();
    CommonCodeRange::new(range.start_point.row as u32 + 1, range.start_point.column as u32, range.end_point.row as u32 + 1, range.end_point.column as u32)
}

fn extract_swift_var_info<'a>(node: Node<'a>, source: &str) -> RefactoringResult<(String, String, Node<'a>)> {
    let declaration_node = find_ancestor_of_kind(node, "property_declaration")
        .ok_or_else(|| RefactoringError::Analysis("Not a property declaration".to_string()))?;

    let mut name_node: Option<Node> = None;
    let mut value_node: Option<Node> = None;

    let mut cursor = declaration_node.walk();
    let mut found_equals = false;
    for child in declaration_node.children(&mut cursor) {
        if child.kind() == "pattern" {
            name_node = child.child(0);
        } else if child.kind() == "=" {
            found_equals = true;
        } else if found_equals && value_node.is_none() {
            value_node = Some(child);
        }
    }

    let final_name_node = name_node.ok_or_else(|| RefactoringError::Analysis("Could not find variable name node".to_string()))?;
    let final_value_node = value_node.ok_or_else(|| RefactoringError::Analysis("Could not find variable value node".to_string()))?;

    let name = final_name_node.utf8_text(source.as_bytes()).unwrap().to_string();
    let value = final_value_node.utf8_text(source.as_bytes()).unwrap().to_string();

    Ok((name, value, declaration_node))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_swift_variable() {
        let source = r#"func myFunc() {
    let x = 10 + 20
    print(x)
}"#;
        let plan = plan_extract_variable(source, 2, 12, 2, 19, Some("sum".to_string()), "test.swift").unwrap();
        assert_eq!(plan.edits.len(), 2);
        let insert_edit = plan.edits.iter().find(|e| e.edit_type == EditType::Insert).unwrap();
        assert_eq!(insert_edit.new_text, "    let sum = 10 + 20\n");
        let replace_edit = plan.edits.iter().find(|e| e.edit_type == EditType::Replace).unwrap();
        assert_eq!(replace_edit.new_text, "sum");
    }

    #[test]
    fn test_extract_swift_function() {
        let source = r#"func myFunc() {
    print("Hello, World!")
}"#;
        let range = CodeRange { start_line: 2, start_col: 4, end_line: 2, end_col: 27 };
        let plan = plan_extract_function(source, &range, "greet", "test.swift").unwrap();
        assert_eq!(plan.edits.len(), 2);
        let insert_edit = plan.edits.iter().find(|e| e.edit_type == EditType::Insert).unwrap();
        assert!(insert_edit.new_text.contains("private func greet()"));
        let replace_edit = plan.edits.iter().find(|e| e.edit_type == EditType::Replace).unwrap();
        assert_eq!(replace_edit.new_text, "greet()");
    }

    #[test]
    fn test_inline_swift_variable() {
        let source = r#"func myFunc() {
    let greeting = "Hello"
    print(greeting)
}"#;
        let plan = plan_inline_variable(source, 2, 8, "test.swift").unwrap();
        assert_eq!(plan.edits.len(), 2);

        let inline_edit = plan.edits.iter().find(|e| e.edit_type == EditType::Replace).unwrap();
        assert_eq!(inline_edit.new_text, r#""Hello""#);

        let delete_edit = plan.edits.iter().find(|e| e.edit_type == EditType::Delete).unwrap();
        assert!(delete_edit.edit_type == EditType::Delete);
    }
}