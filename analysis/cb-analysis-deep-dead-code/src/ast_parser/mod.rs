// analysis/cb-analysis-deep-dead-code/src/ast_parser/mod.rs

//! This module is responsible for parsing source code into an Abstract Syntax Tree (AST)
//! and extracting symbol information from it.

pub mod typescript;

use cb_analysis_common::graph::{SymbolKind, SymbolNode};
use lsp_types::{Position, Range};
use std::fs;
use std::path::Path;
use syn::spanned::Spanned;
use syn::{File, Item, Visibility};
use tracing::{debug, warn};

/// Extracts symbols from a given Rust source file using an AST parser.
pub struct RustSymbolExtractor;

impl RustSymbolExtractor {
    /// Creates a new `SymbolExtractor`.
    pub fn new() -> Self {
        Self
    }

    /// Extracts all supported symbols from a Rust source file.
    pub fn extract_symbols(
        &self,
        file_path: &Path,
        workspace_root: &Path,
    ) -> Result<Vec<SymbolNode>, std::io::Error> {
        let source_code = fs::read_to_string(file_path)?;
        let ast: File = match syn::parse_file(&source_code) {
            Ok(file) => file,
            Err(e) => {
                warn!("Failed to parse file {:?}: {}", file_path, e);
                return Ok(Vec::new()); // Return empty vector if parsing fails
            }
        };

        let relative_path = pathdiff::diff_paths(file_path, workspace_root)
            .unwrap_or_else(|| file_path.to_path_buf());

        let mut symbols = Vec::new();
        for item in ast.items {
            if let Some(symbol_node) = self.item_to_symbol_node(&item, &relative_path) {
                symbols.push(symbol_node);
            }
        }

        Ok(symbols)
    }

    /// Converts a `syn::Item` to a `SymbolNode`, if it's a supported symbol type.
    fn item_to_symbol_node(&self, item: &Item, file_path: &Path) -> Option<SymbolNode> {
        let (name, kind, visibility) = match item {
            Item::Struct(s) => (s.ident.to_string(), SymbolKind::Struct, &s.vis),
            Item::Enum(e) => (e.ident.to_string(), SymbolKind::Enum, &e.vis),
            Item::Fn(f) => (f.sig.ident.to_string(), SymbolKind::Function, &f.vis),
            Item::Trait(t) => (t.ident.to_string(), SymbolKind::Trait, &t.vis),
            Item::Type(t) => (t.ident.to_string(), SymbolKind::TypeAlias, &t.vis),
            Item::Const(c) => (c.ident.to_string(), SymbolKind::Constant, &c.vis),
            Item::Mod(m) => (m.ident.to_string(), SymbolKind::Module, &m.vis),
            _ => return None, // Ignore unsupported items
        };

        let is_public = matches!(visibility, Visibility::Public(_));
        let range = self.span_to_range(item.span());

        let id = format!("{}::{}@L{}", file_path.display(), name, range.start.line);

        debug!("Extracted symbol: {}", id);

        Some(SymbolNode {
            id,
            name,
            kind,
            file_path: file_path.to_str().unwrap_or("").to_string(),
            is_public,
            range,
        })
    }

    /// Converts a `proc_macro2::Span` to an `lsp_types::Range`.
    fn span_to_range(&self, span: proc_macro2::Span) -> Range {
        let start = span.start();
        let end = span.end();
        Range {
            start: Position {
                line: (start.line - 1) as u32,
                character: start.column as u32,
            },
            end: Position {
                line: (end.line - 1) as u32,
                character: end.column as u32,
            },
        }
    }
}

#[cfg(test)]
mod tests;