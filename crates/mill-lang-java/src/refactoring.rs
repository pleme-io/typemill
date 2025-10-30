//! Java-specific refactoring operations
//!
//! Provides AST-based refactoring capabilities including:
//! - Extract function
//! - Extract variable
//! - Inline variable

use mill_lang_common::refactoring::CodeRange as CommonCodeRange;
use mill_foundation::protocol::{
    EditPlan, EditPlanMetadata, EditType, TextEdit, ValidationRule, ValidationType,
};
use serde::{Deserialize, Serialize};
use tree_sitter::{Node, Parser, Point, Query, QueryCursor, StreamingIterator};
use std::collections::HashMap;

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
pub fn plan_extract_function(
    source: &str,
    range: &CodeRange,
    function_name: &str,
    file_path: &str,
) -> RefactoringResult<EditPlan> {
    let mut parser = Parser::new();
    parser
        .set_language(&get_language())
        .map_err(|e| RefactoringError::Parse(format!("Failed to load Java grammar: {}", e)))?;
    let tree = parser.parse(source, None).ok_or_else(|| RefactoringError::Parse("Failed to parse Java source".to_string()))?;
    let root = tree.root_node();

    let start_point = Point::new(range.start_line as usize - 1, range.start_col as usize);
    let end_point = Point::new(range.end_line as usize - 1, range.end_col as usize);

    let selected_node = find_smallest_node_containing_range(root, start_point, end_point)
        .ok_or_else(|| RefactoringError::Analysis("Could not find a node for the selection.".to_string()))?;

    let selected_text = selected_node.utf8_text(source.as_bytes()).unwrap().to_string();

    let enclosing_method = find_ancestor_of_kind(selected_node, "method_declaration")
        .ok_or_else(|| RefactoringError::Analysis("Selection is not inside a method.".to_string()))?;

    let indent = get_indentation(source, enclosing_method.start_position().row);
    let method_indent = format!("{}    ", indent);

    let new_method_text = format!("\n\n{}private void {}() {{\n{}{}\n{}}}\n", indent, function_name, method_indent, selected_text.trim(), indent);

    let insert_edit = TextEdit {
        file_path: None,
        edit_type: EditType::Insert,
        location: CommonCodeRange::new(enclosing_method.end_position().row as u32, 0, enclosing_method.end_position().row as u32, 0).into(),
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
        .set_language(&get_language())
        .map_err(|e| RefactoringError::Parse(format!("Failed to load Java grammar: {}", e)))?;
    let tree = parser.parse(source, None).ok_or_else(|| RefactoringError::Parse("Failed to parse Java source".to_string()))?;
    let root = tree.root_node();

    let start_point = Point::new(start_line as usize - 1, start_col as usize);
    let end_point = Point::new(end_line as usize - 1, end_col as usize);

    let selected_node = find_smallest_node_containing_range(root, start_point, end_point)
        .ok_or_else(|| RefactoringError::Analysis("Could not find a node for the selection.".to_string()))?;

    let expression_text = selected_node.utf8_text(source.as_bytes()).unwrap().to_string();

    let insertion_node = find_ancestor_of_kind(selected_node, "statement")
        .or_else(|| find_ancestor_of_kind(selected_node, "local_variable_declaration"))
        .ok_or_else(|| RefactoringError::Analysis("Could not find statement to insert before.".to_string()))?;

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
        location: CommonCodeRange::new(insertion_node.start_position().row as u32 + 1, 0, insertion_node.start_position().row as u32 + 1, 0).into(),
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
pub fn plan_inline_variable(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    file_path: &str,
) -> RefactoringResult<EditPlan> {
    let mut parser = Parser::new();
    parser
        .set_language(&get_language())
        .map_err(|e| RefactoringError::Parse(format!("Failed to load Java grammar: {}", e)))?;
    let tree = parser.parse(source, None).ok_or_else(|| RefactoringError::Parse("Failed to parse Java source".to_string()))?;
    let root = tree.root_node();
    let point = Point::new(variable_line as usize - 1, variable_col as usize);

    let var_ident_node = find_node_at_point(root, point)
        .ok_or_else(|| RefactoringError::Analysis("Could not find variable at specified location.".to_string()))?;

    let (var_name, var_value, declaration_node) = extract_java_var_info(var_ident_node, source)?;

    let scope_node = find_ancestor_of_kind(declaration_node, "method_declaration")
        .ok_or_else(|| RefactoringError::Analysis("Variable is not inside a method.".to_string()))?;

    let mut edits = Vec::new();
    let query_str = format!(r#"((identifier) @ref (#eq? @ref "{}"))"#, var_name);
    let query = Query::new(&get_language(), &query_str).map_err(|e| RefactoringError::Query(e.to_string()))?;
    let mut cursor = QueryCursor::new();

    cursor.matches(&query, scope_node, source.as_bytes()).for_each(|match_| {
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

fn extract_java_var_info<'a>(node: Node<'a>, source: &str) -> RefactoringResult<(String, String, Node<'a>)> {
    let declaration_node = find_ancestor_of_kind(node, "local_variable_declaration")
        .ok_or_else(|| RefactoringError::Analysis("Not a local variable declaration".to_string()))?;

    let declarator = declaration_node
        .children_by_field_name("declarator", &mut declaration_node.walk())
        .find(|d| d.range().start_byte <= node.range().start_byte && d.range().end_byte >= node.range().end_byte)
        .ok_or_else(|| RefactoringError::Analysis("Could not find variable declarator".to_string()))?;

    let name_node = declarator.child_by_field_name("name").ok_or_else(|| RefactoringError::Analysis("Could not find variable name".to_string()))?;
    let value_node = declarator.child_by_field_name("value").ok_or_else(|| RefactoringError::Analysis("Could not find variable value".to_string()))?;

    let name = name_node.utf8_text(source.as_bytes()).unwrap().to_string();
    let value = value_node.utf8_text(source.as_bytes()).unwrap().to_string();

    Ok((name, value, declaration_node))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_java_variable() {
        let source = r#"
class Main {
    public static void main(String[] args) {
        int x = 10 + 20;
        System.out.println(x);
    }
}"#;
        let plan = plan_extract_variable(source, 4, 16, 4, 23, Some("sum".to_string()), "Main.java").unwrap();
        assert_eq!(plan.edits.len(), 2);
        let insert_edit = &plan.edits[0];
        // The expected text includes the indentation of the line where the insertion happens.
        assert_eq!(insert_edit.new_text, "        var sum = 10 + 20;\n");
        let replace_edit = &plan.edits[1];
        assert_eq!(replace_edit.new_text, "sum");
    }

    #[test]
    fn test_extract_java_method() {
        let source = r#"
class Main {
    public static void main(String[] args) {
        System.out.println("Hello, World!");
    }
}"#;
        let range = CodeRange { start_line: 4, start_col: 8, end_line: 4, end_col: 42 };
        let plan = plan_extract_function(source, &range, "greet", "Main.java").unwrap();
        assert_eq!(plan.edits.len(), 2);
        let insert_edit = &plan.edits[0];
        assert!(insert_edit.new_text.contains("private void greet()"));
        let replace_edit = &plan.edits[1];
        assert_eq!(replace_edit.new_text, "greet();");
    }

    #[test]
    fn test_inline_java_variable() {
        let source = r#"
class Main {
    public static void main(String[] args) {
        String greeting = "Hello";
        System.out.println(greeting);
    }
}"#;
        let plan = plan_inline_variable(source, 4, 15, "Main.java").unwrap();
        assert_eq!(plan.edits.len(), 2);

        let inline_edit = plan.edits.iter().find(|e| e.edit_type == EditType::Replace).unwrap();
        assert_eq!(inline_edit.new_text, r#""Hello""#);

        let delete_edit = plan.edits.iter().find(|e| e.edit_type == EditType::Delete).unwrap();
        assert!(delete_edit.edit_type == EditType::Delete);
    }
}