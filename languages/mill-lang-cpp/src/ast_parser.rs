//! AST parsing for C++ source code using tree-sitter
//!
//! This module provides functionality to parse C++ source code into an Abstract Syntax Tree (AST)
//! and extract symbols such as classes, functions, namespaces, and structs. It uses the
//! tree-sitter-cpp grammar with support for C++11 through C++20 features.

use mill_plugin_api::{ParsedSource, SourceLocation, Symbol, SymbolKind};
use tree_sitter::{Node, Parser, Query, QueryCursor, StreamingIterator};

/// Get the tree-sitter C++ language grammar
///
/// Loads the compiled C++ grammar for tree-sitter parsing. The grammar is built
/// during compilation via build.rs.
///
/// # Returns
///
/// The tree-sitter Language object for C++
pub fn get_cpp_language() -> tree_sitter::Language {
    // The tree-sitter-cpp grammar is compiled via build.rs and linked
    // This extern function is provided by the compiled C code
    use tree_sitter::ffi::TSLanguage;
    extern "C" {
        fn tree_sitter_cpp() -> *const TSLanguage;
    }
    unsafe { tree_sitter::Language::from_raw(tree_sitter_cpp()) }
}

/// Get the tree-sitter query for extracting symbols
///
/// Returns a query string that matches classes, structs, unions, namespaces,
/// and function definitions in C++ code
fn get_symbol_query() -> &'static str {
    r#"
    (class_specifier name: (_) @name) @node
    (struct_specifier name: (_) @name) @node
    (union_specifier name: (_) @name) @node
    (namespace_definition name: (_) @name) @node
    (function_definition declarator: (function_declarator declarator: (_) @name)) @node
    "#
}

/// Convert a tree-sitter node type to a SymbolKind
///
/// Maps C++ AST node kinds to TypeMill symbol kinds
fn node_to_symbol_kind(node: &Node) -> SymbolKind {
    match node.kind() {
        "class_specifier" => SymbolKind::Class,
        "struct_specifier" => SymbolKind::Struct,
        "union_specifier" => SymbolKind::Other, // No Union in SymbolKind
        "namespace_definition" => SymbolKind::Module,
        "function_definition" => SymbolKind::Function,
        _ => SymbolKind::Other,
    }
}

/// Parse C++ source code into a ParsedSource with extracted symbols
///
/// Uses tree-sitter to parse the source and run queries to extract classes,
/// structs, namespaces, and functions
///
/// # Arguments
///
/// * `source` - The C++ source code to parse
///
/// # Returns
///
/// A `ParsedSource` containing all extracted symbols with their locations
///
/// # Panics
///
/// Panics if the tree-sitter C++ grammar fails to load or parsing fails
pub fn parse_source(source: &str) -> ParsedSource {
    let mut parser = Parser::new();
    parser
        .set_language(&get_cpp_language())
        .expect("Error loading C++ grammar");

    let tree = parser.parse(source, None).unwrap();
    let query = Query::new(&get_cpp_language(), get_symbol_query()).unwrap();

    let mut query_cursor = QueryCursor::new();
    let mut symbols = Vec::new();
    query_cursor
        .matches(&query, tree.root_node(), source.as_bytes())
        .for_each(|m| {
            let node = m.captures[0].node;
            let name_node = m.captures[1].node;
            let range = node.range();

            symbols.push(Symbol {
                name: source[name_node.range().start_byte..name_node.range().end_byte].to_string(),
                kind: node_to_symbol_kind(&node),
                location: SourceLocation {
                    line: range.start_point.row + 1,
                    column: range.start_point.column,
                },
                documentation: None,
            });
        });

    ParsedSource {
        data: serde_json::Value::Null,
        symbols,
    }
}

/// List all function names in C++ source code
///
/// Extracts function names using tree-sitter AST parsing.
pub fn list_functions(source: &str) -> Vec<String> {
    let parsed = parse_source(source);
    parsed
        .symbols
        .into_iter()
        .filter(|s| s.kind == mill_plugin_api::SymbolKind::Function)
        .map(|s| s.name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // List functions tests moved to mill-test-support/tests/list_functions_harness_integration.rs
}
