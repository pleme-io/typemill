//! Import support implementation for Rust language plugin
//!
//! This module implements the `ImportSupport` trait for Rust, providing
//! synchronous methods for parsing, analyzing, and rewriting import statements.

use cb_lang_common::import_helpers::{
    find_last_matching_line, insert_line_at, remove_lines_matching,
};
use cb_plugin_api::import_support::ImportSupport;
use std::path::Path;
use tracing::debug;

/// Rust import support implementation
#[derive(Default)]
pub struct RustImportSupport;

impl ImportSupport for RustImportSupport {
    fn parse_imports(&self, content: &str) -> Vec<String> {
        debug!("Parsing Rust imports from content");

        // Use our parser module to parse imports
        match crate::parser::parse_imports(content) {
            Ok(imports) => {
                let module_paths: Vec<String> =
                    imports.iter().map(|imp| imp.module_path.clone()).collect();

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
        tracing::info!(
            old_name = %old_name,
            new_name = %new_name,
            content = %content,
            "RustImportSupport::rewrite_imports_for_rename ENTRY"
        );

        let mut result = String::new();
        let mut changes_count = 0;
        let lines: Vec<&str> = content.lines().collect();

        tracing::info!(lines_count = lines.len(), "Split content into lines");

        // Process line by line, using AST-based rewriting for use statements only
        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            tracing::info!(
                idx = idx,
                line = %line,
                trimmed = %trimmed,
                starts_with_use = trimmed.starts_with("use "),
                contains_old_name = trimmed.contains(old_name),
                "Processing line"
            );

            // Check if this line is a use statement containing our module path
            // Also handle crate:: prefix (e.g., "use crate::core::types" should match "mylib::core::types")
            // Also handle crate-relative imports (e.g., "use utils::helpers" should match "mylib::utils::helpers")
            let is_use_statement = trimmed.starts_with("use ");
            let contains_old_module = trimmed.contains(old_name);

            // Check if this is a crate:: import that matches our module path
            // Extract suffix from old_name (e.g., "mylib::core::types" → "core::types")
            let crate_import_matches = if old_name.contains("::") {
                old_name
                    .split_once("::")
                    .map(|(_, suffix)| {
                        let crate_pattern = format!("crate::{}", suffix);
                        trimmed.contains(&crate_pattern)
                    })
                    .unwrap_or(false)
            } else {
                false
            };

            // Check if this is a crate-relative import (e.g., "use utils::helpers::process" matches "mylib::utils::helpers")
            // This handles imports from lib.rs or other crate root files
            let relative_import_matches = if old_name.contains("::") {
                old_name
                    .split_once("::")
                    .map(|(_, suffix)| {
                        let relative_pattern = format!("use {}::", suffix);
                        trimmed.starts_with(&relative_pattern) || {
                            // Also check for "use {suffix};" (no :: after, for leaf imports)
                            let leaf_pattern = format!("use {};", suffix);
                            trimmed.starts_with(&leaf_pattern)
                        }
                    })
                    .unwrap_or(false)
            } else {
                false
            };

            // Check for relative imports using super:: or self::
            // Extract the last component (module name) from old_name
            let old_module_name = old_name.split("::").last().unwrap_or("");
            let super_import_matches = !old_module_name.is_empty() &&
                (trimmed.contains(&format!("super::{}::", old_module_name)) ||
                 trimmed.contains(&format!("super::{}::*", old_module_name)));
            let self_import_matches = !old_module_name.is_empty() &&
                (trimmed.contains(&format!("self::{}::", old_module_name)) ||
                 trimmed.contains(&format!("self::{}::*", old_module_name)));

            if is_use_statement && (contains_old_module || crate_import_matches || relative_import_matches || super_import_matches || self_import_matches) {
                tracing::info!(
                    line = %trimmed,
                    old_name = %old_name,
                    super_import_matches = super_import_matches,
                    self_import_matches = self_import_matches,
                    "Found use statement containing old module name"
                );

                // For super:: and self:: imports, do simple string replacement
                // because they're relative and don't need full module path rewriting
                if super_import_matches || self_import_matches {
                    let new_module_name = new_name.split("::").last().unwrap_or("");
                    if !old_module_name.is_empty() && !new_module_name.is_empty() {
                        let mut new_line = trimmed.to_string();

                        // Replace all occurrences of the old module name in super:: and self:: contexts
                        new_line = new_line.replace(
                            &format!("super::{}::", old_module_name),
                            &format!("super::{}::", new_module_name)
                        );
                        new_line = new_line.replace(
                            &format!("super::{}::*", old_module_name),
                            &format!("super::{}::*", new_module_name)
                        );
                        new_line = new_line.replace(
                            &format!("self::{}::", old_module_name),
                            &format!("self::{}::", new_module_name)
                        );
                        new_line = new_line.replace(
                            &format!("self::{}::*", old_module_name),
                            &format!("self::{}::*", new_module_name)
                        );

                        // Preserve indentation
                        let indent = line.len() - trimmed.len();
                        let indent_str = &line[..indent];

                        result.push_str(indent_str);
                        result.push_str(&new_line);

                        // Add newline if not last line
                        if idx < lines.len() - 1 {
                            result.push('\n');
                        }
                        changes_count += 1;
                        continue;
                    }
                }

                // Extract just the use statement (up to and including the semicolon)
                // This handles cases like: "use foo::bar; fn main() {}"
                let use_stmt = if let Some(semi_idx) = trimmed.find(';') {
                    &trimmed[..=semi_idx]
                } else {
                    trimmed
                };

                // Try to parse just the use statement
                match syn::parse_str::<syn::ItemUse>(use_stmt) {
                    Ok(item_use) => {
                        tracing::info!(
                            "Successfully parsed use statement, calling rewrite_use_tree"
                        );

                        // Compute effective old and new module paths based on the import style
                        let (effective_old, effective_new) = if crate_import_matches {
                            // For crate:: imports, strip the crate name from both old and new
                            // e.g., old_name="mylib::core::types", new_name="mylib::core::models"
                            //   → effective_old="crate::core::types", effective_new="crate::core::models"
                            let old_suffix = old_name.split_once("::").map(|(_, s)| s).unwrap_or("");
                            let new_suffix = new_name.split_once("::").map(|(_, s)| s).unwrap_or("");
                            (
                                format!("crate::{}", old_suffix),
                                format!("crate::{}", new_suffix),
                            )
                        } else if relative_import_matches {
                            // For crate-relative imports (from lib.rs), use just the suffix
                            // e.g., old_name="mylib::utils::helpers", new_name="mylib::utils::support"
                            //   → effective_old="utils::helpers", effective_new="utils::support"
                            let old_suffix = old_name.split_once("::").map(|(_, s)| s).unwrap_or(old_name);
                            let new_suffix = new_name.split_once("::").map(|(_, s)| s).unwrap_or(new_name);
                            (old_suffix.to_string(), new_suffix.to_string())
                        } else {
                            (old_name.to_string(), new_name.to_string())
                        };

                        tracing::info!(
                            effective_old = %effective_old,
                            effective_new = %effective_new,
                            crate_import_matches = crate_import_matches,
                            relative_import_matches = relative_import_matches,
                            "Computed effective module paths for rewrite"
                        );

                        // Try to rewrite using AST-based transformation
                        let rewrite_result =
                            crate::parser::rewrite_use_tree(&item_use.tree, &effective_old, &effective_new);
                        tracing::info!(
                            rewrite_result = ?rewrite_result,
                            "rewrite_use_tree returned"
                        );
                        if let Some(new_tree) = rewrite_result {
                            // Preserve original indentation
                            let indent = line.len() - trimmed.len();
                            let indent_str = &line[..indent];

                            // Write rewritten use statement with original indentation
                            result.push_str(indent_str);
                            // quote! adds spaces around ::, so we need to remove them
                            let formatted = format!("use {};", quote::quote!(#new_tree));
                            let normalized = formatted.replace(" :: ", "::");
                            result.push_str(&normalized);

                            // If there was code after the semicolon, preserve it
                            if let Some(semi_idx) = trimmed.find(';') {
                                if semi_idx + 1 < trimmed.len() {
                                    let remainder = &trimmed[semi_idx + 1..];
                                    if !remainder.trim().is_empty() {
                                        result.push(' ');
                                        result.push_str(remainder.trim());
                                    }
                                }
                            }

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
        let last_import_idx =
            find_last_matching_line(content, |line| line.trim().starts_with("use "));

        // Build the new import statement
        let import_stmt = format!("use {};", module);

        if let Some(idx) = last_import_idx {
            // Insert after the last import (idx + 1)
            insert_line_at(content, idx + 1, &import_stmt)
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

        // Remove all lines that are use statements matching the module
        let (result, removed_count) = remove_lines_matching(content, |line| {
            let trimmed = line.trim();

            // Check if this is a use statement containing our module
            if trimmed.starts_with("use ") && trimmed.contains(module) {
                // Try to parse to ensure it's really this module
                if let Ok(item_use) = syn::parse_str::<syn::ItemUse>(trimmed) {
                    // Check if this use statement references our module
                    let use_tree_str = quote::quote!(#item_use.tree).to_string();
                    if use_tree_str.contains(module) {
                        return true; // Remove this line
                    }
                }
            }

            false // Keep this line
        });

        debug!(removed = removed_count, "Removed import lines");
        result
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

        let (result, changes) =
            support.rewrite_imports_for_rename(content, "old_crate", "new_crate");
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

    #[test]
    fn test_rewrite_crate_prefix_imports() {
        let support = RustImportSupport;

        // Test rewriting crate:: prefixed imports
        // Scenario: Moving mylib/src/core/types.rs → mylib/src/core/models.rs
        // Import: use crate::core::types::Entity;
        // Expected: use crate::core::models::Entity;
        let content = r#"use crate::core::types::Entity;

pub fn lib_fn() -> Entity {
    Entity::new()
}"#;

        // The old_name and new_name will be full module paths like "mylib::core::types"
        let (result, changes) = support.rewrite_imports_for_rename(
            content,
            "mylib::core::types",
            "mylib::core::models",
        );

        assert_eq!(changes, 1, "Should have changed 1 import");
        assert!(
            result.contains("use crate::core::models::Entity;"),
            "Should contain updated crate:: import. Actual:\n{}",
            result
        );
        assert!(
            !result.contains("use crate::core::types::Entity;"),
            "Should NOT contain old crate:: import. Actual:\n{}",
            result
        );
    }

    #[test]
    fn test_rewrite_crate_relative_imports() {
        let support = RustImportSupport;

        // Test rewriting crate-relative imports (from lib.rs)
        // Scenario: Renaming src/utils/helpers.rs → src/utils/support.rs
        // Import in lib.rs: use utils::helpers::process;
        // Expected: use utils::support::process;
        let content = r#"pub mod utils;

use utils::helpers::process;

pub fn lib_fn() {
    process();
}
"#;

        // The old_name and new_name will be full module paths like "test_project::utils::helpers"
        let (result, changes) = support.rewrite_imports_for_rename(
            content,
            "test_project::utils::helpers",
            "test_project::utils::support",
        );

        assert_eq!(changes, 1, "Should have changed 1 import. Actual:\n{}", result);
        assert!(
            result.contains("use utils::support::process;"),
            "Should contain updated crate-relative import. Actual:\n{}",
            result
        );
        assert!(
            !result.contains("use utils::helpers::process;"),
            "Should NOT contain old crate-relative import. Actual:\n{}",
            result
        );
    }
}
