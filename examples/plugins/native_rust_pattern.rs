// Example: Native Rust Parsing Pattern
// Best for: Zero subprocess overhead, pure Rust parsers

use syn::{File, Item, ItemFn, ItemStruct, Attribute};
use mill_plugin_api::{ Symbol , SymbolKind , PluginResult , PluginError };
use mill_foundation::protocol::SourceLocation;

pub fn extract_symbols(source: &str) -> PluginResult<Vec<Symbol>> {
    // Parse with syn (Rust) or tree-sitter (other languages)
    let ast: File = syn::parse_file(source)
        .map_err(|e| PluginError::parse(format!("Failed to parse: {}", e)))?;

    let mut symbols = Vec::new();

    for item in ast.items {
        match item {
            Item::Fn(func) => symbols.push(extract_function(&func)),
            Item::Struct(s) => symbols.push(extract_struct(&s)),
            Item::Enum(e) => symbols.push(extract_enum(&e)),
            _ => {}
        }
    }

    Ok(symbols)
}

fn extract_function(func: &ItemFn) -> Symbol {
    Symbol {
        name: func.sig.ident.to_string(),
        kind: SymbolKind::Function,
        location: extract_location(&func.sig.ident),
        documentation: extract_doc_comments(&func.attrs),
    }
}

fn extract_doc_comments(attrs: &[Attribute]) -> Option<String> {
    let doc_lines: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                // Extract doc comment text
                Some(extract_doc_text(attr))
            } else {
                None
            }
        })
        .collect();

    if doc_lines.is_empty() {
        None
    } else {
        Some(doc_lines.join("\n"))
    }
}