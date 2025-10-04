use syn::visit::{self, Visit};
use syn::{File, UseTree};

#[derive(Debug, Clone)]
struct ModuleReference {
    line: usize,
    column: usize,
    text: String,
    kind: ReferenceKind,
}

#[derive(Debug, Clone, PartialEq)]
enum ReferenceKind {
    Declaration,
    QualifiedPath,
    StringLiteral,
}

/// Visitor that finds all references to a specific module
struct ModuleFinder<'a> {
    module_to_find: &'a str,
    references: Vec<ModuleReference>,
}

impl<'a> ModuleFinder<'a> {
    fn new(module_to_find: &'a str) -> Self {
        Self {
            module_to_find,
            references: Vec::new(),
        }
    }

    fn into_references(self) -> Vec<ModuleReference> {
        self.references
    }
}

impl<'ast, 'a> Visit<'ast> for ModuleFinder<'a> {
    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        // Check the use tree for our module
        self.check_use_tree(&node.tree);

        // Continue visiting child nodes
        visit::visit_item_use(self, node);
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        // Check if this path references our module
        if let Some(segment) = node.path.segments.first() {
            if segment.ident == self.module_to_find {
                self.references.push(ModuleReference {
                    line: 0,
                    column: 0,
                    text: quote::quote!(#node).to_string(),
                    kind: ReferenceKind::QualifiedPath,
                });
            }
        }

        // Continue visiting
        visit::visit_expr_path(self, node);
    }

    fn visit_expr_lit(&mut self, node: &'ast syn::ExprLit) {
        // Check string literals
        if let syn::Lit::Str(lit_str) = &node.lit {
            let value = lit_str.value();
            if value.contains(self.module_to_find) {
                self.references.push(ModuleReference {
                    line: 0,
                    column: 0,
                    text: value.clone(),
                    kind: ReferenceKind::StringLiteral,
                });
            }
        }

        // Continue visiting
        visit::visit_expr_lit(self, node);
    }
}

impl<'a> ModuleFinder<'a> {
    fn check_use_tree(&mut self, tree: &UseTree) {
        match tree {
            UseTree::Path(path) => {
                if path.ident == self.module_to_find {
                    self.references.push(ModuleReference {
                        line: 0,
                        column: 0,
                        text: self.module_to_find.to_string(),
                        kind: ReferenceKind::Declaration,
                    });
                }
                self.check_use_tree(&path.tree);
            }
            UseTree::Name(name) => {
                if name.ident == self.module_to_find {
                    self.references.push(ModuleReference {
                        line: 0,
                        column: 0,
                        text: self.module_to_find.to_string(),
                        kind: ReferenceKind::Declaration,
                    });
                }
            }
            UseTree::Group(group) => {
                for item in &group.items {
                    self.check_use_tree(item);
                }
            }
            _ => {}
        }
    }
}

fn find_module_references(code: &str, module_to_find: &str) -> Vec<ModuleReference> {
    let syntax_tree: File = match syn::parse_str(code) {
        Ok(tree) => tree,
        Err(e) => {
            eprintln!("Failed to parse code: {}", e);
            return Vec::new();
        }
    };

    let mut finder = ModuleFinder::new(module_to_find);
    finder.visit_file(&syntax_tree);
    finder.into_references()
}

fn main() {
    let test_code = r#"
        use my_module::Type;
        use std::collections::HashMap;

        fn test() {
            use my_module::nested::Item;
            let x = my_module::function();
            let path = "path/to/my_module/file.rs";
        }

        mod inner {
            use my_module::AnotherType;
        }
    "#;

    println!("Testing syn visitor pattern for finding 'my_module' references...\n");

    let references = find_module_references(test_code, "my_module");

    println!("Found {} references:", references.len());
    for (i, ref_item) in references.iter().enumerate() {
        println!("  {}. {:?} - {}", i + 1, ref_item.kind, ref_item.text);
    }

    println!("\n✅ Visitor pattern test completed successfully!");
}
