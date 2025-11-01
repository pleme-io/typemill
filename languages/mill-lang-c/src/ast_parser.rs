//! AST parsing for C source code using tree-sitter
//!
//! This module provides functionality to parse C source code into an Abstract Syntax Tree (AST)
//! and extract symbols such as function definitions. It uses the tree-sitter-c grammar for
//! accurate parsing of C99 and C11 code.

use mill_plugin_api::{ParsedSource, Symbol, SymbolKind, SourceLocation};
use tree_sitter::{Node, Parser, Tree};

/// Get the tree-sitter C language grammar
///
/// Loads the compiled C grammar for tree-sitter parsing
fn get_language() -> tree_sitter::Language {
    extern "C" {
        fn tree_sitter_c() -> tree_sitter::Language;
    }
    unsafe { tree_sitter_c() }
}

/// Parse C source code into a ParsedSource with extracted symbols
///
/// Uses tree-sitter to parse the source and traverse the AST to extract function definitions
///
/// # Arguments
///
/// * `source` - The C source code to parse
///
/// # Returns
///
/// A `ParsedSource` containing all extracted symbols with their locations
///
/// # Panics
///
/// Panics if the tree-sitter C grammar fails to load or parsing fails
pub(crate) fn parse_source(source: &str) -> ParsedSource {
    let mut parser = Parser::new();
    parser
        .set_language(&get_language())
        .expect("Failed to load C grammar");
    let tree = parser.parse(source, None).unwrap();
    let mut symbols = Vec::new();
    traverse_tree(&tree, &mut symbols, source);

    ParsedSource {
        data: serde_json::Value::Null,
        symbols,
    }
}

/// List all function names in C source code
///
/// Extracts function names using tree-sitter AST parsing.
pub(crate) fn list_functions(source: &str) -> Vec<String> {
    let parsed = parse_source(source);
    parsed.symbols
        .into_iter()
        .filter(|s| s.kind == mill_plugin_api::SymbolKind::Function)
        .map(|s| s.name)
        .collect()
}

/// Traverse the entire syntax tree to extract symbols
///
/// Walks the tree-sitter AST starting from the root node
fn traverse_tree(tree: &Tree, symbols: &mut Vec<Symbol>, source: &str) {
    visit_node(&tree.root_node(), symbols, source);
}

/// Recursively visit AST nodes to extract symbols
///
/// Identifies function definitions and extracts them as symbols
fn visit_node(node: &Node, symbols: &mut Vec<Symbol>, source: &str) {
    if node.kind() == "function_definition" {
        if let Some(symbol) = extract_function_symbol(node, source) {
            symbols.push(symbol);
        }
    }

    for child in node.children(&mut node.walk()) {
        visit_node(&child, symbols, source);
    }
}

/// Extract a Symbol from a function_definition node
///
/// Parses the function declarator to get the function name and location
fn extract_function_symbol(node: &Node, source: &str) -> Option<Symbol> {
    let declarator = node.child_by_field_name("declarator")?;

    let mut queue = vec![declarator];
    while let Some(current) = queue.pop() {
        if current.kind() == "function_declarator" {
            let name_node = current.child_by_field_name("declarator")?;
            let name = name_node.utf8_text(source.as_bytes()).ok()?.to_string();

            let start = name_node.start_position();
            let location = SourceLocation {
                line: start.row + 1,
                column: start.column,
            };

            return Some(Symbol {
                name,
                kind: SymbolKind::Function,
                location,
                documentation: None,
            });
        }
        queue.extend(current.children(&mut current.walk()));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_functions_multiple() {
        let source = r#"
void firstFunction() {
    printf("first");
}

int secondFunction(int x) {
    return x * 2;
}

static void thirdFunction() {
    // helper
}
"#;
        let functions = list_functions(source);
        assert_eq!(functions.len(), 3);
        assert!(functions.contains(&"firstFunction".to_string()));
        assert!(functions.contains(&"secondFunction".to_string()));
        assert!(functions.contains(&"thirdFunction".to_string()));
    }

    #[test]
    fn test_list_functions_empty() {
        let source = r#"
int myGlobal = 42;
struct Point { int x; int y; };
"#;
        let functions = list_functions(source);
        assert_eq!(functions.len(), 0);
    }
}