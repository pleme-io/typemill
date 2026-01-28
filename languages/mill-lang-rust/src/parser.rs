//! Rust source code parsing using syn
//!
//! This module provides functionality for parsing Rust source code into ASTs,
//! extracting symbols, and analyzing imports.
use mill_foundation::protocol::{ImportGraph, ImportInfo, ImportType, NamedImport};
use mill_lang_common::ImportGraphBuilder;
use mill_plugin_api::{PluginApiError, PluginResult, SourceLocation, Symbol, SymbolKind};
use syn::{spanned::Spanned, visit::Visit, File, Item, ItemUse, UseTree};
/// A visitor that walks the AST and collects function names
struct FunctionVisitor {
    functions: Vec<String>,
}
impl<'ast> Visit<'ast> for FunctionVisitor {
    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        self.functions.push(i.sig.ident.to_string());
        syn::visit::visit_item_fn(self, i);
    }
    fn visit_impl_item_fn(&mut self, i: &'ast syn::ImplItemFn) {
        self.functions.push(i.sig.ident.to_string());
        syn::visit::visit_impl_item_fn(self, i);
    }
}
/// A visitor that collects all symbols (functions, structs, enums, etc.)
struct SymbolVisitor {
    symbols: Vec<Symbol>,
}
impl<'ast> Visit<'ast> for SymbolVisitor {
    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        let span = i.span();
        let start = span.start();
        let end = span.end();
        self.symbols.push(Symbol {
            name: i.sig.ident.to_string(),
            kind: SymbolKind::Function,
            location: SourceLocation {
                line: start.line.saturating_sub(1),
                column: start.column,
            },
            end_location: Some(SourceLocation {
                line: end.line.saturating_sub(1),
                column: end.column,
            }),
            documentation: extract_doc_comments(&i.attrs),
        });
        syn::visit::visit_item_fn(self, i);
    }
    fn visit_item_struct(&mut self, i: &'ast syn::ItemStruct) {
        let span = i.span();
        let start = span.start();
        let end = span.end();
        self.symbols.push(Symbol {
            name: i.ident.to_string(),
            kind: SymbolKind::Struct,
            location: SourceLocation {
                line: start.line.saturating_sub(1),
                column: start.column,
            },
            end_location: Some(SourceLocation {
                line: end.line.saturating_sub(1),
                column: end.column,
            }),
            documentation: extract_doc_comments(&i.attrs),
        });
        syn::visit::visit_item_struct(self, i);
    }
    fn visit_item_enum(&mut self, i: &'ast syn::ItemEnum) {
        let span = i.span();
        let start = span.start();
        let end = span.end();
        self.symbols.push(Symbol {
            name: i.ident.to_string(),
            kind: SymbolKind::Enum,
            location: SourceLocation {
                line: start.line.saturating_sub(1),
                column: start.column,
            },
            end_location: Some(SourceLocation {
                line: end.line.saturating_sub(1),
                column: end.column,
            }),
            documentation: extract_doc_comments(&i.attrs),
        });
        syn::visit::visit_item_enum(self, i);
    }
    fn visit_item_const(&mut self, i: &'ast syn::ItemConst) {
        let span = i.span();
        let start = span.start();
        let end = span.end();
        self.symbols.push(Symbol {
            name: i.ident.to_string(),
            kind: SymbolKind::Constant,
            location: SourceLocation {
                line: start.line.saturating_sub(1),
                column: start.column,
            },
            end_location: Some(SourceLocation {
                line: end.line.saturating_sub(1),
                column: end.column,
            }),
            documentation: extract_doc_comments(&i.attrs),
        });
        syn::visit::visit_item_const(self, i);
    }
    fn visit_item_static(&mut self, i: &'ast syn::ItemStatic) {
        let span = i.span();
        let start = span.start();
        let end = span.end();
        self.symbols.push(Symbol {
            name: i.ident.to_string(),
            kind: SymbolKind::Variable,
            location: SourceLocation {
                line: start.line.saturating_sub(1),
                column: start.column,
            },
            end_location: Some(SourceLocation {
                line: end.line.saturating_sub(1),
                column: end.column,
            }),
            documentation: extract_doc_comments(&i.attrs),
        });
        syn::visit::visit_item_static(self, i);
    }
    fn visit_item_mod(&mut self, i: &'ast syn::ItemMod) {
        let span = i.span();
        let start = span.start();
        let end = span.end();
        self.symbols.push(Symbol {
            name: i.ident.to_string(),
            kind: SymbolKind::Module,
            location: SourceLocation {
                line: start.line.saturating_sub(1),
                column: start.column,
            },
            end_location: Some(SourceLocation {
                line: end.line.saturating_sub(1),
                column: end.column,
            }),
            documentation: extract_doc_comments(&i.attrs),
        });
        syn::visit::visit_item_mod(self, i);
    }
    fn visit_impl_item_fn(&mut self, i: &'ast syn::ImplItemFn) {
        let span = i.span();
        let start = span.start();
        let end = span.end();
        self.symbols.push(Symbol {
            name: i.sig.ident.to_string(),
            kind: SymbolKind::Method,
            location: SourceLocation {
                line: start.line.saturating_sub(1),
                column: start.column,
            },
            end_location: Some(SourceLocation {
                line: end.line.saturating_sub(1),
                column: end.column,
            }),
            documentation: extract_doc_comments(&i.attrs),
        });
        syn::visit::visit_impl_item_fn(self, i);
    }
}
/// Extract documentation from attributes
fn extract_doc_comments(attrs: &[syn::Attribute]) -> Option<String> {
    let docs: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                attr.meta.require_name_value().ok().and_then(|nv| {
                    if let syn::Expr::Lit(expr_lit) = &nv.value {
                        if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                            return Some(lit_str.value().trim().to_string());
                        }
                    }
                    None
                })
            } else {
                None
            }
        })
        .collect();
    if docs.is_empty() {
        None
    } else {
        Some(docs.join("\n"))
    }
}
/// Parses Rust source code and returns a list of all function and method names
pub fn list_functions(source: &str) -> PluginResult<Vec<String>> {
    let ast: File = syn::parse_file(source)
        .map_err(|e| PluginApiError::parse(format!("Failed to parse Rust code: {}", e)))?;
    let mut visitor = FunctionVisitor {
        functions: Vec::new(),
    };
    visitor.visit_file(&ast);
    Ok(visitor.functions)
}
/// Parses Rust source code and extracts all symbols
pub fn extract_symbols(source: &str) -> PluginResult<Vec<Symbol>> {
    let ast: File = syn::parse_file(source)
        .map_err(|e| PluginApiError::parse(format!("Failed to parse Rust code: {}", e)))?;
    let mut visitor = SymbolVisitor {
        symbols: Vec::new(),
    };
    visitor.visit_file(&ast);
    Ok(visitor.symbols)
}
/// Parse Rust imports using AST analysis with syn
pub fn parse_imports(source: &str) -> PluginResult<Vec<ImportInfo>> {
    let syntax_tree: File = syn::parse_str(source)
        .map_err(|e| PluginApiError::parse(format!("Failed to parse Rust source: {}", e)))?;
    struct ImportVisitor {
        imports: Vec<ImportInfo>,
        current_line: u32,
    }
    impl<'ast> Visit<'ast> for ImportVisitor {
        fn visit_item_use(&mut self, node: &'ast ItemUse) {
            self.extract_use_tree(&node.tree, String::new(), self.current_line);
        }
        fn visit_item(&mut self, node: &'ast Item) {
            self.current_line += 1;
            syn::visit::visit_item(self, node);
        }
    }
    impl ImportVisitor {
        fn extract_use_tree(&mut self, tree: &UseTree, prefix: String, line: u32) {
            match tree {
                UseTree::Path(path) => {
                    let new_prefix = if prefix.is_empty() {
                        path.ident.to_string()
                    } else {
                        format!("{}::{}", prefix, path.ident)
                    };
                    self.extract_use_tree(&path.tree, new_prefix, line);
                }
                UseTree::Name(name) => {
                    let module_path = if prefix.is_empty() {
                        name.ident.to_string()
                    } else {
                        prefix.clone()
                    };
                    self.imports.push(ImportInfo {
                        module_path: module_path.clone(),
                        import_type: ImportType::EsModule,
                        named_imports: vec![NamedImport {
                            name: name.ident.to_string(),
                            alias: None,
                            type_only: false,
                        }],
                        default_import: None,
                        namespace_import: None,
                        type_only: false,
                        location: mill_foundation::protocol::SourceLocation {
                            start_line: line,
                            start_column: 0,
                            end_line: line,
                            end_column: 0,
                        },
                    });
                }
                UseTree::Rename(rename) => {
                    let module_path = prefix.clone();
                    self.imports.push(ImportInfo {
                        module_path: module_path.clone(),
                        import_type: ImportType::EsModule,
                        named_imports: vec![NamedImport {
                            name: rename.ident.to_string(),
                            alias: Some(rename.rename.to_string()),
                            type_only: false,
                        }],
                        default_import: None,
                        namespace_import: None,
                        type_only: false,
                        location: mill_foundation::protocol::SourceLocation {
                            start_line: line,
                            start_column: 0,
                            end_line: line,
                            end_column: 0,
                        },
                    });
                }
                UseTree::Glob(_) => {
                    self.imports.push(ImportInfo {
                        module_path: prefix.clone(),
                        import_type: ImportType::EsModule,
                        named_imports: Vec::new(),
                        default_import: None,
                        namespace_import: Some(prefix),
                        type_only: false,
                        location: mill_foundation::protocol::SourceLocation {
                            start_line: line,
                            start_column: 0,
                            end_line: line,
                            end_column: 0,
                        },
                    });
                }
                UseTree::Group(group) => {
                    for tree in &group.items {
                        self.extract_use_tree(tree, prefix.clone(), line);
                    }
                }
            }
        }
    }
    let mut visitor = ImportVisitor {
        imports: Vec::new(),
        current_line: 0,
    };
    visitor.visit_file(&syntax_tree);
    Ok(visitor.imports)
}
/// Rewrite a use tree to replace an old module path with a new one.
/// Supports multi-segment replacements (e.g., "common::utils" → "new_utils").
pub fn rewrite_use_tree(tree: &UseTree, old_module: &str, new_module: &str) -> Option<UseTree> {
    // Split module paths into segments
    let old_segments: Vec<&str> = old_module.split("::").collect();
    let new_segments: Vec<&str> = new_module.split("::").collect();

    rewrite_use_tree_with_segments(tree, &old_segments, &new_segments, 0)
}

/// Helper function that performs segment-aware matching and replacement.
fn rewrite_use_tree_with_segments(
    tree: &UseTree,
    old_segments: &[&str],
    new_segments: &[&str],
    depth: usize,
) -> Option<UseTree> {
    match tree {
        UseTree::Path(path) => {
            // Check if current segment matches
            if depth < old_segments.len() && path.ident == old_segments[depth] {
                // Matched current segment
                if depth + 1 == old_segments.len() {
                    // Matched all old segments! Replace with new segments.
                    tracing::info!(
                        old_segments = ?old_segments,
                        new_segments = ?new_segments,
                        depth = depth,
                        "Matched full module path, replacing"
                    );

                    // Build new path tree from NEW segments + remaining original tree
                    // CRITICAL: Check if this is a cross-crate move (first segment changed)
                    // - Cross-crate (common::utils → new_utils): use ALL new segments
                    // - Same-crate (self::utils → self::helpers): slice from depth to avoid duplication
                    let use_segments = if !old_segments.is_empty()
                        && !new_segments.is_empty()
                        && old_segments[0] != new_segments[0]
                    {
                        // Cross-crate: first segment changed, use full new path
                        new_segments
                    } else {
                        // Same-crate: first segment unchanged, slice from current depth
                        &new_segments[depth..]
                    };
                    build_new_use_tree(use_segments, &path.tree)
                } else {
                    // Partial match, continue matching next level
                    // If the recursion returns a replacement, we need to handle it differently
                    // depending on whether we're at the root (depth=0) or not
                    if let Some(new_subtree) = rewrite_use_tree_with_segments(
                        &path.tree,
                        old_segments,
                        new_segments,
                        depth + 1,
                    ) {
                        if depth == 0
                            && !old_segments.is_empty()
                            && !new_segments.is_empty()
                            && old_segments[0] != new_segments[0]
                        {
                            // At root level with cross-crate move (first segment changed):
                            // return replacement as-is (don't wrap with current segment)
                            // Example: common::utils -> new_utils (common != new_utils)
                            Some(new_subtree)
                        } else {
                            // Not at root: preserve the current path segment and replace only the subtree
                            let mut new_path = path.clone();
                            new_path.tree = Box::new(new_subtree);
                            Some(UseTree::Path(new_path))
                        }
                    } else {
                        None
                    }
                }
            } else {
                // No match at this depth, check subtree recursively (reset depth to 0)
                if let Some(new_subtree) =
                    rewrite_use_tree_with_segments(&path.tree, old_segments, new_segments, 0)
                {
                    let mut new_path = path.clone();
                    new_path.tree = Box::new(new_subtree);
                    Some(UseTree::Path(new_path))
                } else {
                    None
                }
            }
        }
        UseTree::Name(name) => {
            // Handle terminal UseName nodes
            // Check if this name matches the segment we're looking for at this depth
            if depth < old_segments.len() && name.ident == old_segments[depth] {
                // Check if this completes the match (all segments matched)
                if depth + 1 == old_segments.len() {
                    // This is the final segment - replace it with the new name
                    let mut new_name = name.clone();
                    new_name.ident = syn::Ident::new(new_segments[depth], name.ident.span());
                    Some(UseTree::Name(new_name))
                } else {
                    // More segments expected but we're at a terminal node - no match
                    None
                }
            } else {
                // Name doesn't match the segment we're looking for
                None
            }
        }
        UseTree::Rename(rename) => {
            // Handle rename patterns: use old_name as alias;
            // Check if the renamed ident matches our old module at this depth
            if depth < old_segments.len() && rename.ident == old_segments[depth] {
                // Check if this completes the match (all segments matched)
                if depth + 1 == old_segments.len() {
                    // Simple case: single segment replacement
                    // Replace the ident with the new name (keeping the alias)
                    let mut new_rename = rename.clone();
                    new_rename.ident = syn::Ident::new(new_segments[depth], rename.ident.span());
                    Some(UseTree::Rename(new_rename))
                } else {
                    // Multi-segment paths in rename context not supported yet
                    None
                }
            } else {
                None
            }
        }
        UseTree::Glob(_) => None,
        UseTree::Group(group) => {
            let mut modified = false;
            let new_items: Vec<UseTree> = group
                .items
                .iter()
                .map(|item| {
                    if let Some(new_item) =
                        rewrite_use_tree_with_segments(item, old_segments, new_segments, 0)
                    {
                        modified = true;
                        new_item
                    } else {
                        item.clone()
                    }
                })
                .collect();
            if modified {
                let mut new_group = group.clone();
                new_group.items = new_items.into_iter().collect();
                Some(UseTree::Group(new_group))
            } else {
                None
            }
        }
    }
}

/// Build a new UseTree from segments and attach the remainder
fn build_new_use_tree(segments: &[&str], remainder: &UseTree) -> Option<UseTree> {
    if segments.is_empty() {
        // No segments to add, return the remainder
        return Some(remainder.clone());
    }

    // Build nested Path nodes for all segments
    // Parse a simple template to get proper span/token types
    let template = "use a::b;";
    let parsed: syn::ItemUse = syn::parse_str(template).ok()?;

    // Extract span and token from the template
    let (span, colon2) = if let UseTree::Path(p) = &parsed.tree {
        (p.ident.span(), p.colon2_token)
    } else {
        return None;
    };

    // Build nested Path nodes for all segments
    let mut current = remainder.clone();
    for segment in segments.iter().rev() {
        current = UseTree::Path(syn::UsePath {
            ident: syn::Ident::new(segment, span),
            colon2_token: colon2,
            tree: Box::new(current),
        });
    }

    Some(current)
}
/// Analyzes Rust source code to produce an import graph.
/// Uses native syn AST parsing (no subprocess required).
pub(crate) fn analyze_imports(
    source: &str,
    file_path: Option<&std::path::Path>,
) -> PluginResult<ImportGraph> {
    let imports = parse_imports(source)?;
    Ok(ImportGraphBuilder::new("rust")
        .with_source_file(file_path)
        .with_imports(imports)
        .extract_external_dependencies(is_external_dependency)
        .with_parser_version("0.1.0-plugin")
        .build())
}
/// Check if a module path represents an external dependency
fn is_external_dependency(module_path: &str) -> bool {
    !module_path.starts_with("crate")
        && !module_path.starts_with("self")
        && !module_path.starts_with("super")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_list_functions_and_methods() {
        let source = r#"
fn top_level() {}

struct MyStruct;

impl MyStruct {
    fn my_method() {}
    fn another_method(&self) {}
}

mod my_mod {
    fn function_in_mod() {}
}
"#;
        let functions = list_functions(source).unwrap();
        assert_eq!(functions.len(), 4);
        assert!(functions.contains(&"top_level".to_string()));
        assert!(functions.contains(&"my_method".to_string()));
        assert!(functions.contains(&"another_method".to_string()));
        assert!(functions.contains(&"function_in_mod".to_string()));
    }
    #[test]
    fn test_list_nested_functions() {
        let source = r#"
fn outer() {
    fn inner() {}
}
"#;
        let functions = list_functions(source).unwrap();
        assert_eq!(functions.len(), 2);
        assert!(functions.contains(&"outer".to_string()));
        assert!(functions.contains(&"inner".to_string()));
    }
    #[test]
    fn test_syntax_error() {
        let source = "fn my_func {";
        let result = list_functions(source);
        assert!(result.is_err());
    }
    #[test]
    fn test_extract_symbols() {
        let source = r#"
/// A top-level function
fn my_function() {}

/// A struct
struct MyStruct {
    field: i32,
}

/// An enum
enum MyEnum {
    Variant1,
    Variant2,
}

const MY_CONST: i32 = 42;
"#;
        let symbols = extract_symbols(source).unwrap();
        assert!(symbols
            .iter()
            .any(|s| s.name == "my_function" && s.kind == SymbolKind::Function));
        assert!(symbols
            .iter()
            .any(|s| s.name == "MyStruct" && s.kind == SymbolKind::Struct));
        assert!(symbols
            .iter()
            .any(|s| s.name == "MyEnum" && s.kind == SymbolKind::Enum));
        assert!(symbols
            .iter()
            .any(|s| s.name == "MY_CONST" && s.kind == SymbolKind::Constant));
    }
    #[test]
    fn test_parse_imports() {
        let source = r#"
use std::collections::HashMap;
use std::fs::{File, read_to_string};
use crate::my_module::*;
"#;
        let imports = parse_imports(source).unwrap();
        assert_eq!(imports.len(), 4);
    }
}
