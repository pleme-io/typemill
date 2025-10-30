use mill_plugin_api::{ParsedSource, SourceLocation, Symbol, SymbolKind};
use tree_sitter::{Node, Parser, Query, QueryCursor, StreamingIterator};

pub fn get_cpp_language() -> tree_sitter::Language {
    // The tree-sitter-cpp grammar is compiled via build.rs and linked
    // This extern function is provided by the compiled C code
    use tree_sitter::ffi::TSLanguage;
    extern "C" {
        fn tree_sitter_cpp() -> *const TSLanguage;
    }
    unsafe { tree_sitter::Language::from_raw(tree_sitter_cpp()) }
}

fn get_symbol_query() -> &'static str {
    r#"
    (class_specifier name: (_) @name) @node
    (struct_specifier name: (_) @name) @node
    (union_specifier name: (_) @name) @node
    (namespace_definition name: (_) @name) @node
    (function_definition declarator: (function_declarator declarator: (_) @name)) @node
    "#
}

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