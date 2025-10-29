use mill_plugin_api::{ParsedSource, Symbol, SymbolKind, SourceLocation};
use tree_sitter::{Node, Parser, Tree};

fn get_language() -> tree_sitter::Language {
    extern "C" {
        fn tree_sitter_c() -> tree_sitter::Language;
    }
    unsafe { tree_sitter_c() }
}

pub fn parse_source(source: &str) -> ParsedSource {
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

fn traverse_tree(tree: &Tree, symbols: &mut Vec<Symbol>, source: &str) {
    visit_node(&tree.root_node(), symbols, source);
}

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