//! Rust AST-based symbol extraction using `syn`.

use super::SymbolExtractor;
use lsp_types::{Position, Range};
use mill_analysis_common::graph::{SymbolKind, SymbolNode};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{File, Item, Visibility};
use tracing::{debug, warn};

/// Visibility level extracted from Rust AST.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustVisibility {
    /// Fully public (`pub`)
    Public,
    /// Public within crate (`pub(crate)`)
    Crate,
    /// Public to parent module (`pub(super)`)
    Super,
    /// Public to a specific path (`pub(in path)`)
    Restricted,
    /// Private (no visibility modifier)
    Private,
}

impl RustVisibility {
    /// Parse visibility from syn's Visibility enum.
    pub fn from_syn(vis: &Visibility) -> Self {
        match vis {
            Visibility::Public(_) => RustVisibility::Public,
            Visibility::Restricted(restricted) => {
                let path_str = restricted
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                match path_str.as_str() {
                    "crate" => RustVisibility::Crate,
                    "super" => RustVisibility::Super,
                    "self" => RustVisibility::Private,
                    _ => RustVisibility::Restricted,
                }
            }
            Visibility::Inherited => RustVisibility::Private,
        }
    }

    /// Returns true if this is considered public for API boundary purposes.
    pub fn is_api_public(&self) -> bool {
        matches!(self, RustVisibility::Public)
    }
}

/// Extracts symbols from Rust source files using the `syn` AST parser.
pub struct RustSymbolExtractor;

impl RustSymbolExtractor {
    /// Creates a new `RustSymbolExtractor`.
    pub fn new() -> Self {
        Self
    }

    /// Converts a `syn::Item` to a `SymbolNode` with visibility info.
    fn item_to_symbol_node(&self, item: &Item, file_path: &Path) -> Option<(SymbolNode, RustVisibility)> {
        let (name, kind, syn_vis) = match item {
            Item::Struct(s) => (s.ident.to_string(), SymbolKind::Struct, &s.vis),
            Item::Enum(e) => (e.ident.to_string(), SymbolKind::Enum, &e.vis),
            Item::Fn(f) => (f.sig.ident.to_string(), SymbolKind::Function, &f.vis),
            Item::Trait(t) => (t.ident.to_string(), SymbolKind::Trait, &t.vis),
            Item::Type(t) => (t.ident.to_string(), SymbolKind::TypeAlias, &t.vis),
            Item::Const(c) => (c.ident.to_string(), SymbolKind::Constant, &c.vis),
            Item::Mod(m) => (m.ident.to_string(), SymbolKind::Module, &m.vis),
            _ => return None, // Ignore unsupported items
        };

        let visibility = RustVisibility::from_syn(syn_vis);
        let is_public = visibility.is_api_public();
        let range = self.span_to_range(item.span());

        let id = format!("{}::{}@L{}", file_path.display(), name, range.start.line);

        debug!(
            "Extracted Rust symbol: {} (visibility: {:?})",
            id, visibility
        );

        Some((
            SymbolNode {
                id,
                name,
                kind,
                file_path: file_path.to_str().unwrap_or("").to_string(),
                is_public,
                range,
            },
            visibility,
        ))
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

impl Default for RustSymbolExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// An intra-file call from one function to another.
#[derive(Debug, Clone)]
pub struct IntraFileCall {
    /// The caller function name.
    pub caller: String,
    /// The callee function name.
    pub callee: String,
}

/// Visitor to find function calls within a function body.
struct CallVisitor<'a> {
    #[allow(dead_code)] // Stored for potential future use in debugging
    caller: &'a str,
    calls: Vec<String>,
    /// Known function names in this file for filtering.
    known_functions: &'a HashSet<String>,
}

impl<'a> CallVisitor<'a> {
    fn new(caller: &'a str, known_functions: &'a HashSet<String>) -> Self {
        Self {
            caller,
            calls: Vec::new(),
            known_functions,
        }
    }
}

impl<'a> Visit<'_> for CallVisitor<'a> {
    fn visit_expr_call(&mut self, node: &syn::ExprCall) {
        // Extract function name from call expression
        if let syn::Expr::Path(path) = &*node.func {
            if let Some(ident) = path.path.get_ident() {
                let name = ident.to_string();
                // Only track calls to known local functions
                if self.known_functions.contains(&name) {
                    self.calls.push(name);
                }
            }
        }
        // Continue visiting nested expressions
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &syn::ExprMethodCall) {
        // Track method calls (e.g., self.foo())
        let name = node.method.to_string();
        if self.known_functions.contains(&name) {
            self.calls.push(name);
        }
        // Continue visiting nested expressions
        syn::visit::visit_expr_method_call(self, node);
    }
}

impl RustSymbolExtractor {
    /// Extract intra-file calls from a Rust source file.
    ///
    /// Returns a list of (caller, callee) pairs representing function calls
    /// within the same file.
    pub fn extract_calls(&self, file_path: &Path) -> Result<Vec<IntraFileCall>, std::io::Error> {
        let source_code = fs::read_to_string(file_path)?;
        let ast: File = match syn::parse_file(&source_code) {
            Ok(file) => file,
            Err(e) => {
                warn!("Failed to parse Rust file {:?}: {}", file_path, e);
                return Ok(Vec::new());
            }
        };

        // First, collect all function names in this file
        let known_functions: HashSet<String> = ast
            .items
            .iter()
            .filter_map(|item| {
                if let Item::Fn(f) = item {
                    Some(f.sig.ident.to_string())
                } else {
                    None
                }
            })
            .collect();

        // Now visit each function and find calls
        let mut calls = Vec::new();

        for item in &ast.items {
            if let Item::Fn(func) = item {
                let caller_name = func.sig.ident.to_string();
                let mut visitor = CallVisitor::new(&caller_name, &known_functions);

                // Visit the function body
                for stmt in &func.block.stmts {
                    visitor.visit_stmt(stmt);
                }

                // Add all discovered calls
                for callee in visitor.calls {
                    if callee != caller_name {
                        // Skip self-recursion
                        calls.push(IntraFileCall {
                            caller: caller_name.clone(),
                            callee,
                        });
                    }
                }
            }
        }

        debug!(
            file = %file_path.display(),
            call_count = calls.len(),
            "Extracted intra-file calls"
        );

        Ok(calls)
    }
}

impl SymbolExtractor for RustSymbolExtractor {
    fn extract_symbols(
        &self,
        file_path: &Path,
        _workspace_root: &Path,
    ) -> Result<Vec<SymbolNode>, std::io::Error> {
        let source_code = fs::read_to_string(file_path)?;
        let ast: File = match syn::parse_file(&source_code) {
            Ok(file) => file,
            Err(e) => {
                warn!("Failed to parse Rust file {:?}: {}", file_path, e);
                return Ok(Vec::new());
            }
        };

        // Use absolute path for SymbolNode.file_path so URIs work correctly
        // for intra-file call detection
        let absolute_path = file_path.to_path_buf();

        let mut symbols = Vec::new();
        for item in ast.items {
            if let Some((symbol_node, _visibility)) =
                self.item_to_symbol_node(&item, &absolute_path)
            {
                // Note: visibility is logged in item_to_symbol_node
                // SymbolNode.is_public correctly reflects whether it's `pub`
                symbols.push(symbol_node);
            }
        }

        Ok(symbols)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_extract_rust_symbols() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
pub fn public_function() {{}}
fn private_function() {{}}
pub struct MyStruct {{}}
enum MyEnum {{}}
"#
        )
        .unwrap();

        let extractor = RustSymbolExtractor::new();
        let symbols = extractor
            .extract_symbols(file.path(), file.path().parent().unwrap())
            .unwrap();

        assert_eq!(symbols.len(), 4);

        let pub_fn = symbols.iter().find(|s| s.name == "public_function").unwrap();
        assert!(pub_fn.is_public);

        let priv_fn = symbols
            .iter()
            .find(|s| s.name == "private_function")
            .unwrap();
        assert!(!priv_fn.is_public);
    }

    #[test]
    fn test_visibility_detection() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
pub fn fully_public() {{}}
pub(crate) fn crate_public() {{}}
pub(super) fn super_public() {{}}
pub(in crate::foo) fn path_restricted() {{}}
fn private() {{}}
"#
        )
        .unwrap();

        let extractor = RustSymbolExtractor::new();
        let symbols = extractor
            .extract_symbols(file.path(), file.path().parent().unwrap())
            .unwrap();

        assert_eq!(symbols.len(), 5);

        // Only `pub` should be marked as public
        let fully_pub = symbols.iter().find(|s| s.name == "fully_public").unwrap();
        assert!(fully_pub.is_public, "pub should be public");

        // pub(crate) should NOT be marked as public API
        let crate_pub = symbols.iter().find(|s| s.name == "crate_public").unwrap();
        assert!(!crate_pub.is_public, "pub(crate) should not be API-public");

        // pub(super) should NOT be marked as public
        let super_pub = symbols.iter().find(|s| s.name == "super_public").unwrap();
        assert!(!super_pub.is_public, "pub(super) should not be API-public");

        // pub(in path) should NOT be marked as public
        let path_restricted = symbols.iter().find(|s| s.name == "path_restricted").unwrap();
        assert!(!path_restricted.is_public, "pub(in path) should not be API-public");

        // Private should not be marked as public
        let private = symbols.iter().find(|s| s.name == "private").unwrap();
        assert!(!private.is_public, "private should not be public");
    }

    #[test]
    fn test_extract_intra_file_calls() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
pub fn entry_point() {{
    helper_one();
    helper_two();
}}

fn helper_one() {{
    helper_two();
}}

fn helper_two() {{}}

fn unused_function() {{}}
"#
        )
        .unwrap();

        let extractor = RustSymbolExtractor::new();
        let calls = extractor.extract_calls(file.path()).unwrap();

        // Should find: entry_point -> helper_one, entry_point -> helper_two, helper_one -> helper_two
        assert_eq!(calls.len(), 3, "Expected 3 calls, got {:?}", calls);

        // Check specific calls
        let entry_to_one = calls.iter().any(|c| c.caller == "entry_point" && c.callee == "helper_one");
        let entry_to_two = calls.iter().any(|c| c.caller == "entry_point" && c.callee == "helper_two");
        let one_to_two = calls.iter().any(|c| c.caller == "helper_one" && c.callee == "helper_two");

        assert!(entry_to_one, "Should find entry_point -> helper_one");
        assert!(entry_to_two, "Should find entry_point -> helper_two");
        assert!(one_to_two, "Should find helper_one -> helper_two");

        // unused_function should have no outgoing calls
        let from_unused = calls.iter().any(|c| c.caller == "unused_function");
        assert!(!from_unused, "unused_function should have no outgoing calls");
    }
}
