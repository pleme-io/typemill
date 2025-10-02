use crate::error::{AstError, AstResult};
use cb_api::{ImportInfo, ImportType, NamedImport, SourceLocation};
use syn::{visit::Visit, File, Item, ItemUse, UseTree};

// A visitor that will walk the AST and collect the names of all functions it finds.
struct FunctionVisitor {
    functions: Vec<String>,
}

impl<'ast> Visit<'ast> for FunctionVisitor {
    // This method is called for every function, method, or function-like item.
    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        // Add the function's identifier (its name) to our list.
        self.functions.push(i.sig.ident.to_string());

        // Continue visiting to find nested functions inside this one.
        syn::visit::visit_item_fn(self, i);
    }

    // This method is called for every impl method.
    fn visit_impl_item_fn(&mut self, i: &'ast syn::ImplItemFn) {
        // Add the method's identifier (its name) to our list.
        self.functions.push(i.sig.ident.to_string());

        // Continue visiting to find nested functions inside this method.
        syn::visit::visit_impl_item_fn(self, i);
    }
}

/// Parses Rust source code and returns a list of all function and method names.
pub fn list_functions(source: &str) -> AstResult<Vec<String>> {
    // Parse the source code into a syn::File, which is the root of the AST.
    let ast: File =
        syn::parse_file(source).map_err(|e| AstError::analysis(format!("Failed to parse Rust code: {}", e)))?;

    // Create an instance of our visitor.
    let mut visitor = FunctionVisitor {
        functions: Vec::new(),
    };

    // Walk the AST, starting from the root file node.
    visitor.visit_file(&ast);

    // Return the collected function names.
    Ok(visitor.functions)
}

/// Parse Rust imports using AST analysis with syn
///
/// This function parses Rust source code and extracts all `use` statements,
/// returning detailed information about each import including the module path,
/// named imports, and location in the source.
pub fn parse_rust_imports_ast(source: &str) -> AstResult<Vec<ImportInfo>> {
    // Parse the Rust source file
    let syntax_tree: File = syn::parse_str(source).map_err(|e| {
        AstError::analysis(format!("Failed to parse Rust source: {}", e))
    })?;

    struct ImportVisitor {
        imports: Vec<ImportInfo>,
        current_line: u32,
    }

    impl<'ast> Visit<'ast> for ImportVisitor {
        fn visit_item_use(&mut self, node: &'ast ItemUse) {
            self.extract_use_tree(&node.tree, String::new(), self.current_line);
        }

        fn visit_item(&mut self, node: &'ast Item) {
            // Track line numbers
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
                        location: SourceLocation {
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
                        location: SourceLocation {
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
                        location: SourceLocation {
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

/// Rewrite a use tree to replace an old crate name with a new one
///
/// This function recursively traverses a `syn::UseTree` and replaces the leading
/// identifier if it matches the old crate name. It handles all UseTree variants:
/// - Path: Checks if the leading identifier matches old_crate and replaces it
/// - Name, Rename, Glob: Returns None (no changes needed at this level)
/// - Group: Recursively processes all items in the group
///
/// # Arguments
///
/// * `tree` - The use tree to potentially rewrite
/// * `old_crate` - The old crate name to replace (in snake_case)
/// * `new_crate` - The new crate name to use (in snake_case)
///
/// # Returns
///
/// `Some(UseTree)` if the tree was modified, `None` if no changes were needed
pub(crate) fn rewrite_use_tree(tree: &UseTree, old_crate: &str, new_crate: &str) -> Option<UseTree> {
    match tree {
        UseTree::Path(path) => {
            // Check if this is the leading identifier we need to replace
            if path.ident == old_crate {
                // Clone the path and replace the identifier
                let mut new_path = path.clone();
                new_path.ident = syn::Ident::new(new_crate, path.ident.span());

                // Recursively process the rest of the tree
                if let Some(new_subtree) = rewrite_use_tree(&path.tree, old_crate, new_crate) {
                    new_path.tree = Box::new(new_subtree);
                }

                Some(UseTree::Path(new_path))
            } else {
                // Not the crate we're looking for, but check the subtree
                if let Some(new_subtree) = rewrite_use_tree(&path.tree, old_crate, new_crate) {
                    let mut new_path = path.clone();
                    new_path.tree = Box::new(new_subtree);
                    Some(UseTree::Path(new_path))
                } else {
                    None
                }
            }
        }
        UseTree::Name(_) => {
            // Name nodes don't contain the crate identifier, no changes needed
            None
        }
        UseTree::Rename(_) => {
            // Rename nodes don't contain the crate identifier at this level
            None
        }
        UseTree::Glob(_) => {
            // Glob imports don't need modification at this level
            None
        }
        UseTree::Group(group) => {
            // Process each item in the group
            let mut modified = false;
            let new_items: Vec<UseTree> = group
                .items
                .iter()
                .map(|item| {
                    if let Some(new_item) = rewrite_use_tree(item, old_crate, new_crate) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_rust_functions_and_methods() {
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
    fn test_list_rust_nested_functions() {
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
    fn test_list_rust_syntax_error() {
        let source = "fn my_func {";
        let result = list_functions(source);
        assert!(result.is_err());
    }
}
