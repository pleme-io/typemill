//! C#-specific refactoring operations
//!
//! Provides AST-based refactoring capabilities including:
//! - Extract Method (function)
//! - Extract Variable
//! - Inline Variable

use mill_foundation::protocol::{
    EditPlan, EditPlanMetadata, EditType, TextEdit, ValidationRule, ValidationType,
};
use mill_lang_common::CodeRange;
use tree_sitter::{Node, Parser, Point, Query, QueryCursor};

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

/// Generate edit plan for extract method refactoring
pub fn plan_extract_function(
    source: &str,
    range: &CodeRange,
    function_name: &str, // In C#, this is a method name
    file_path: &str,
) -> RefactoringResult<EditPlan> {
    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_c_sharp::language())
        .map_err(|e| RefactoringError::Parse(format!("Failed to load C# grammar: {}", e)))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| RefactoringError::Parse("Failed to parse C# source".to_string()))?;
    let root = tree.root_node();

    let start_point = Point::new(range.start_line as usize, range.start_col as usize);
    let end_point = Point::new(range.end_line as usize, range.end_col as usize);

    let start_node = find_node_at_point(root, start_point).ok_or_else(|| {
        RefactoringError::Analysis("Could not find a node at the start of the selection.".to_string())
    })?;
    let end_node = find_node_at_point(root, end_point).ok_or_else(|| {
        RefactoringError::Analysis("Could not find a node at the end of the selection.".to_string())
    })?;

    let selected_text = &source[start_node.start_byte()..end_node.end_byte()];

    let enclosing_method = find_ancestor_of_kind(start_node, "method_declaration")
        .ok_or_else(|| RefactoringError::Analysis("Selection is not inside a method.".to_string()))?;

    let indent = get_indentation(source, enclosing_method.start_position().row);
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
        location: range.clone().into(),
        original_text: selected_text.to_string(),
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
            parameters: Default::default(),
        }],
        metadata: EditPlanMetadata {
            intent_name: "extract_method".to_string(),
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
        .set_language(tree_sitter_c_sharp::language())
        .map_err(|e| RefactoringError::Parse(format!("Failed to load C# grammar: {}", e)))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| RefactoringError::Parse("Failed to parse C# source".to_string()))?;
    let root = tree.root_node();

    let start_point = Point::new(start_line as usize, start_col as usize);
    let end_point = Point::new(end_line as usize, end_col as usize);

    let selected_node = find_smallest_node_containing_range(root, start_point, end_point)
        .ok_or_else(|| {
            RefactoringError::Analysis("Could not find a node for the selection.".to_string())
        })?;

    let expression_text = selected_node
        .utf8_text(source.as_bytes())
        .unwrap()
        .to_string();

    let insertion_statement = find_ancestor_of_kind(selected_node, "local_declaration_statement")
        .or_else(|| find_ancestor_of_kind(selected_node, "expression_statement"))
        .or_else(|| find_ancestor_of_kind(selected_node, "return_statement"))
        .or_else(|| find_ancestor_of_kind(selected_node, "assignment_expression"))
        .or_else(|| find_ancestor_of_kind(selected_node, "argument"))
        .ok_or_else(|| {
            RefactoringError::Analysis(
                "Could not find an appropriate statement to insert the variable before."
                    .to_string(),
            )
        })?;

    let indent = get_indentation(source, insertion_statement.start_position().row);
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

    Ok(EditPlan {
        source_file: file_path.to_string(),
        edits: vec![insert_edit, replace_edit],
        dependency_updates: vec![],
        validations: vec![ValidationRule {
            rule_type: ValidationType::SyntaxCheck,
            description: "Verify syntax after extraction".to_string(),
            parameters: Default::default(),
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
        .set_language(tree_sitter_c_sharp::language())
        .map_err(|e| RefactoringError::Parse(format!("Failed to load C# grammar: {}", e)))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| RefactoringError::Parse("Failed to parse C# source".to_string()))?;
    let root = tree.root_node();
    let point = Point::new(variable_line as usize, variable_col as usize);

    let var_ident_node = find_node_at_point(root, point).ok_or_else(|| {
        RefactoringError::Analysis("Could not find variable at specified location.".to_string())
    })?;

    let (var_name, var_value, declaration_node) = extract_csharp_var_info(var_ident_node, source)?;

    let scope_node = find_ancestor_of_kind(declaration_node, "method_declaration")
        .ok_or_else(|| RefactoringError::Analysis("Variable is not inside a method.".to_string()))?;

    let mut edits = Vec::new();
    let query_str = format!(r#"((identifier) @ref (#eq? @ref "{}"))"#, var_name);
    let query = Query::new(tree_sitter_c_sharp::language(), &query_str)
        .map_err(|e| RefactoringError::Query(e.to_string()))?;
    let mut cursor = QueryCursor::new();

    for match_ in cursor.matches(&query, scope_node, source.as_bytes()) {
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
    }

    edits.push(TextEdit {
        file_path: Some(file_path.to_string()),
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
            parameters: Default::default(),
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

fn find_node_at_point<'a>(node: Node<'a>, point: Point) -> Option<Node<'a>> {
    find_smallest_node_containing_range(node, point, point)
}

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

fn get_indentation(source: &str, line: usize) -> String {
    source
        .lines()
        .nth(line)
        .map(|l| l.chars().take_while(|c| c.is_whitespace()).collect())
        .unwrap_or_default()
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
) -> RefactoringResult<(String, String, Node<'a>)> {
    let declaration_statement = find_ancestor_of_kind(node, "local_declaration_statement")
        .ok_or_else(|| {
            RefactoringError::Analysis(format!(
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
            RefactoringError::Analysis(format!(
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
            RefactoringError::Analysis("Invalid declaration: missing variable_declarator".to_string())
        })?;

    // Get the identifier (variable name) from declarator
    let mut cursor_name = declarator.walk();
    let name_node = declarator
        .children(&mut cursor_name)
        .find(|n| n.kind() == "identifier")
        .ok_or_else(|| RefactoringError::Analysis("Could not find variable name".to_string()))?;

    // Get the value from equals_value_clause
    let mut cursor_value = declarator.walk();
    let equals_clause = declarator
        .children(&mut cursor_value)
        .find(|n| n.kind() == "equals_value_clause")
        .ok_or_else(|| {
            RefactoringError::Analysis("Could not find equals_value_clause".to_string())
        })?;

    let mut cursor_expr = equals_clause.walk();
    let value_node = equals_clause
        .children(&mut cursor_expr)
        .find(|n| n.kind() != "=")
        .ok_or_else(|| {
            RefactoringError::Analysis("Could not find variable initializer value".to_string())
        })?;

    let name = name_node.utf8_text(source.as_bytes()).unwrap().to_string();
    let value = value_node.utf8_text(source.as_bytes()).unwrap().to_string();

    Ok((name, value, declaration_statement))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let plan =
            plan_extract_variable(source, 5, 16, 5, 23, Some("sum".to_string()), "test.cs")
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
}