//! Import support implementation for Rust language plugin
//!
//! This module implements the `ImportSupport` trait for Rust, providing
//! synchronous methods for parsing, analyzing, and rewriting import statements.

use cb_plugin_api::import_support::ImportSupport;
use std::path::Path;
use tracing::debug;

/// Rust import support implementation
pub struct RustImportSupport;

impl ImportSupport for RustImportSupport {
    fn parse_imports(&self, content: &str) -> Vec<String> {
        debug!("Parsing Rust imports from content");

        // Use our parser module to parse imports
        match crate::parser::parse_imports(content) {
            Ok(imports) => {
                let module_paths: Vec<String> = imports
                    .iter()
                    .map(|imp| imp.module_path.clone())
                    .collect();

                debug!(imports_count = module_paths.len(), "Parsed imports");
                module_paths
            }
            Err(e) => {
                debug!(error = %e, "Failed to parse imports");
                Vec::new()
            }
        }
    }

    fn rewrite_imports_for_rename(
        &self,
        content: &str,
        old_name: &str,
        new_name: &str,
    ) -> (String, usize) {
        debug!(
            old_name = %old_name,
            new_name = %new_name,
            "Rewriting Rust imports for rename"
        );

        let mut result = String::new();
        let mut changes_count = 0;
        let lines: Vec<&str> = content.lines().collect();

        // Process line by line, using AST-based rewriting for use statements only
        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Check if this line is a use statement containing our crate name
            if trimmed.starts_with("use ") && trimmed.contains(old_name) {
                // Try to parse this line as a use statement
                match syn::parse_str::<syn::ItemUse>(trimmed) {
                    Ok(item_use) => {
                        // Try to rewrite using AST-based transformation
                        if let Some(new_tree) =
                            crate::parser::rewrite_use_tree(&item_use.tree, old_name, new_name)
                        {
                            // Preserve original indentation
                            let indent = line.len() - trimmed.len();
                            let indent_str = &line[..indent];

                            // Write rewritten use statement with original indentation
                            result.push_str(indent_str);
                            result.push_str(&format!("use {};", quote::quote!(#new_tree)));

                            // Add newline if not last line
                            if idx < lines.len() - 1 {
                                result.push('\n');
                            }
                            changes_count += 1;
                            continue;
                        }
                    }
                    Err(_) => {
                        // If parsing fails (e.g., multi-line use statement), keep original
                        debug!(
                            line = %line,
                            "Could not parse use statement, keeping original"
                        );
                    }
                }
            }

            // Keep original line (either not a use statement, or parsing failed)
            result.push_str(line);

            // Add newline if not last line
            if idx < lines.len() - 1 {
                result.push('\n');
            }
        }

        debug!(changes = changes_count, "Rewrote Rust imports using AST");

        (result, changes_count)
    }

    fn rewrite_imports_for_move(
        &self,
        content: &str,
        _old_path: &Path,
        _new_path: &Path,
    ) -> (String, usize) {
        // For Rust, file moves typically don't require import rewriting
        // because imports use crate names, not file paths
        // This would need custom logic if we wanted to handle relative imports
        debug!("File move detected, but Rust uses crate-based imports (no changes needed)");
        (content.to_string(), 0)
    }

    fn contains_import(&self, content: &str, module: &str) -> bool {
        debug!(module = %module, "Checking if content contains import");

        // Parse all imports and check if any match the module
        let imports = self.parse_imports(content);
        imports.iter().any(|imp| imp.contains(module))
    }

    fn add_import(&self, content: &str, module: &str) -> String {
        debug!(module = %module, "Adding import to Rust code");

        // Find the position after the last import statement
        let lines: Vec<&str> = content.lines().collect();
        let mut last_import_idx = None;

        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("use ") {
                last_import_idx = Some(idx);
            }
        }

        // Build the new import statement
        let import_stmt = format!("use {};", module);

        if let Some(idx) = last_import_idx {
            // Insert after the last import
            let mut new_lines = lines.clone();
            new_lines.insert(idx + 1, &import_stmt);
            new_lines.join("\n")
        } else {
            // No existing imports, add at the top
            if content.is_empty() {
                import_stmt
            } else {
                format!("{}\n\n{}", import_stmt, content)
            }
        }
    }

    fn remove_import(&self, content: &str, module: &str) -> String {
        debug!(module = %module, "Removing import from Rust code");

        let lines: Vec<&str> = content.lines().collect();
        let mut result_lines = Vec::new();

        for line in lines {
            let trimmed = line.trim();

            // Skip lines that are use statements matching the module
            if trimmed.starts_with("use ") && trimmed.contains(module) {
                // Try to parse to ensure it's really this module
                if let Ok(item_use) = syn::parse_str::<syn::ItemUse>(trimmed) {
                    // Check if this use statement references our module
                    let use_tree_str = quote::quote!(#item_use.tree).to_string();
                    if use_tree_str.contains(module) {
                        debug!(line = %line, "Removing import line");
                        continue; // Skip this line
                    }
                }
            }

            result_lines.push(line);
        }

        result_lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_imports() {
        let support = RustImportSupport;
        let content = r#"
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::parser;
"#;

        let imports = support.parse_imports(content);
        // Parser returns module paths, not including the final symbol name
        // e.g., "use std::collections::HashMap" returns module_path="std::collections"
        assert!(imports.len() >= 3);
        assert!(imports.contains(&"std::collections".to_string()));
        assert!(imports.contains(&"serde".to_string()));
        assert!(imports.contains(&"crate".to_string()));
    }

    #[test]
    fn test_rewrite_imports_for_rename() {
        let support = RustImportSupport;
        let content = r#"
use old_crate::module::Thing;
use other::stuff;
"#;

        let (result, changes) = support.rewrite_imports_for_rename(content, "old_crate", "new_crate");
        assert_eq!(changes, 1);
        assert!(result.contains("new_crate"));
        assert!(!result.contains("use old_crate"));
        assert!(result.contains("other::stuff"));
    }

    #[test]
    fn test_contains_import() {
        let support = RustImportSupport;
        let content = r#"
use std::collections::HashMap;
use serde::Serialize;
"#;

        assert!(support.contains_import(content, "std::collections"));
        assert!(support.contains_import(content, "serde"));
        assert!(!support.contains_import(content, "tokio"));
    }

    #[test]
    fn test_add_import() {
        let support = RustImportSupport;
        let content = r#"use std::collections::HashMap;

fn main() {}"#;

        let result = support.add_import(content, "serde::Serialize");
        assert!(result.contains("use serde::Serialize;"));
        assert!(result.contains("use std::collections::HashMap;"));
    }

    #[test]
    fn test_remove_import() {
        let support = RustImportSupport;
        let content = r#"use std::collections::HashMap;
use serde::Serialize;
use tokio::runtime::Runtime;

fn main() {}"#;

        let result = support.remove_import(content, "serde");
        assert!(!result.contains("use serde::Serialize"));
        assert!(result.contains("use std::collections::HashMap"));
        assert!(result.contains("use tokio::runtime::Runtime"));
    }
}
