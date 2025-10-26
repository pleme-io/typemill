//! Import support implementation for Rust language plugin
//!
//! This module implements the segregated import traits for Rust, providing
//! synchronous methods for parsing, analyzing, and rewriting import statements.

use mill_lang_common::import_helpers::{
    find_last_matching_line, insert_line_at, remove_lines_matching,
};
use mill_plugin_api::{
    ImportAdvancedSupport, ImportMoveSupport, ImportMutationSupport, ImportParser,
    ImportRenameSupport,
};
use std::path::Path;
use tracing::debug;

/// Rust import support implementation
#[derive(Default)]
pub struct RustImportSupport;

// ============================================================================
// Segregated Trait Implementations
// ============================================================================

impl ImportParser for RustImportSupport {
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

    fn contains_import(&self, content: &str, module: &str) -> bool {
        debug!(module = %module, "Checking if content contains import");

        // Use parser module directly to avoid ambiguity with deprecated trait
        match crate::parser::parse_imports(content) {
            Ok(imports) => {
                let module_paths: Vec<String> =
                    imports.iter().map(|imp| imp.module_path.clone()).collect();

                // Use segment-based matching to avoid false positives
                // e.g., searching for "test" should NOT match "my_test_module"
                let search_segments: Vec<&str> = module.split("::").collect();

                module_paths.iter().any(|imp| {
                    let imp_segments: Vec<&str> = imp.split("::").collect();

                    // Check if search_segments matches as:
                    // 1. Exact match
                    if imp_segments == search_segments {
                        return true;
                    }

                    // 2. Prefix match (search is a prefix of import)
                    // e.g., "std::collections" contains "std"
                    if imp_segments.starts_with(&search_segments) {
                        return true;
                    }

                    // 3. Contiguous subsequence match
                    // e.g., "crate::foo::bar::baz" contains "bar::baz"
                    if search_segments.len() <= imp_segments.len() {
                        imp_segments
                            .windows(search_segments.len())
                            .any(|window| window == search_segments.as_slice())
                    } else {
                        false
                    }
                })
            }
            Err(_) => false,
        }
    }
}

impl ImportRenameSupport for RustImportSupport {
    fn rewrite_imports_for_rename(
        &self,
        content: &str,
        old_name: &str,
        new_name: &str,
    ) -> (String, usize) {
        tracing::info!(
            old_name = %old_name,
            new_name = %new_name,
            "RustImportSupport::rewrite_imports_for_rename ENTRY"
        );

        let mut result = String::new();
        let mut changes_count = 0;
        let lines: Vec<&str> = content.lines().collect();

        tracing::info!(lines_count = lines.len(), "Split content into lines");

        // State machine for collecting multi-line use statements
        let mut in_use_stmt = false;
        let mut use_stmt_lines: Vec<(usize, &str)> = Vec::new(); // (line_index, line_content)
        let mut use_stmt_indent = 0;

        let mut idx = 0;
        while idx < lines.len() {
            let line = lines[idx];
            let trimmed = line.trim();

            // PHASE 1: Handle extern crate declarations
            // Note: extern crate is a deprecated Rust 2015 pattern, but still needs support
            if trimmed.starts_with("extern crate ") {
                // Convert hyphenated crate names to underscored for Rust identifiers
                let old_rust_ident = old_name.replace('-', "_");
                let new_rust_ident = new_name.replace('-', "_");

                let pattern = format!("extern crate {}", old_rust_ident);
                if line.contains(&pattern) {
                    let updated_line =
                        line.replace(&pattern, &format!("extern crate {}", new_rust_ident));
                    result.push_str(&updated_line);
                    changes_count += 1;
                    if idx < lines.len() - 1 {
                        result.push('\n');
                    }
                    idx += 1;
                    continue;
                }
            }

            // Check if starting a new use statement (including pub use)
            if !in_use_stmt && (trimmed.starts_with("use ") || trimmed.starts_with("pub use ")) {
                in_use_stmt = true;
                use_stmt_lines.clear();
                use_stmt_indent = line.len() - trimmed.len();
                use_stmt_lines.push((idx, line));

                // Check if it's a single-line statement (has semicolon)
                if trimmed.contains(';') {
                    in_use_stmt = false;
                    // Process this complete use statement
                    if let Some(change) = self.process_use_statement(
                        &use_stmt_lines,
                        old_name,
                        new_name,
                        use_stmt_indent,
                        &lines,
                    ) {
                        result.push_str(&change.0);
                        changes_count += change.1;
                    } else {
                        // No change, keep original
                        for (_, original_line) in &use_stmt_lines {
                            result.push_str(original_line);
                            if idx < lines.len() - 1 {
                                result.push('\n');
                            }
                        }
                    }
                    use_stmt_lines.clear();
                }
                idx += 1;
                continue;
            }

            // If we're inside a use statement, continue collecting lines
            if in_use_stmt {
                use_stmt_lines.push((idx, line));

                // Check if this line completes the statement (has semicolon)
                if trimmed.contains(';') {
                    in_use_stmt = false;
                    // Process this complete multi-line use statement
                    if let Some(change) = self.process_use_statement(
                        &use_stmt_lines,
                        old_name,
                        new_name,
                        use_stmt_indent,
                        &lines,
                    ) {
                        result.push_str(&change.0);
                        changes_count += change.1;
                    } else {
                        // No change, keep original lines
                        for (line_idx, original_line) in &use_stmt_lines {
                            result.push_str(original_line);
                            if *line_idx < lines.len() - 1 {
                                result.push('\n');
                            }
                        }
                    }
                    use_stmt_lines.clear();
                }
                idx += 1;
                continue;
            }

            // Not a use statement, just copy the line
            result.push_str(line);
            if idx < lines.len() - 1 {
                result.push('\n');
            }
            idx += 1;
        }

        debug!(changes = changes_count, "Rewrote Rust imports using AST");

        // Second pass: Update qualified paths in code (not in use statements)
        // This catches inline qualified paths like: cb_ast::CacheSettings::new()
        let lines: Vec<String> = result.lines().map(|s| s.to_string()).collect();
        let mut final_result = String::new();
        let mut qualified_path_changes = 0;

        // Convert hyphenated crate names to underscored for Rust identifiers
        let old_rust_ident = old_name.replace('-', "_");
        let new_rust_ident = new_name.replace('-', "_");

        // Build regex pattern with word boundary to avoid false positives
        // Pattern: \b + old_rust_ident + \s*::
        // This matches "cb_ast::" but not "my_cb_ast::"
        let pattern = format!(r"\b{}\s*::", regex::escape(&old_rust_ident));
        let re = match regex::Regex::new(&pattern) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(error = %e, pattern = %pattern, "Failed to compile regex for qualified paths");
                return (result, changes_count);
            }
        };

        for line in lines {
            let trimmed = line.trim();

            // Skip use statements (already processed in first pass, including pub use)
            if trimmed.starts_with("use ") || trimmed.starts_with("pub use ") {
                final_result.push_str(&line);
                final_result.push('\n');
                continue;
            }

            // Search for qualified paths using regex
            if re.is_match(&line) {
                let updated_line = re.replace_all(&line, |_caps: &regex::Captures| {
                    format!("{}::", new_rust_ident)
                });

                if updated_line != line {
                    qualified_path_changes += 1;
                    tracing::debug!(
                        old_line = %line,
                        new_line = %updated_line,
                        "Updated qualified path reference"
                    );
                }

                final_result.push_str(&updated_line);
            } else {
                final_result.push_str(&line);
            }

            final_result.push('\n');
        }

        // Remove trailing newline if original didn't have one
        if !result.ends_with('\n') && final_result.ends_with('\n') {
            final_result.pop();
        }

        changes_count += qualified_path_changes;

        tracing::info!(
            use_statement_changes = changes_count - qualified_path_changes,
            qualified_path_changes = qualified_path_changes,
            total_changes = changes_count,
            "Completed import rewrite with qualified path updates"
        );

        (final_result, changes_count)
    }
}

impl ImportMoveSupport for RustImportSupport {
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
}

impl ImportMutationSupport for RustImportSupport {
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

/// Import prefix classification for consistent pattern matching
#[derive(Debug, PartialEq, Eq)]
enum ImportPrefix {
    /// `crate::` prefix (absolute path from crate root)
    Crate,
    /// `super::` prefix (parent module)
    Super,
    /// `self::` prefix (current module)
    SelfPath,
    /// External crate or relative path (no special prefix)
    External,
    /// Unknown/unclassified
    Unknown,
}

// Helper methods for RustImportSupport
impl RustImportSupport {
    /// Count the number of nested super/self prefixes in a UseTree.
    ///
    /// For `use super::super::utils::Thing;`, this returns (Super, 2).
    /// For `use super::utils::Thing;`, this returns (Super, 1).
    fn count_relative_prefix(tree: &syn::UseTree) -> (ImportPrefix, usize) {
        let mut count = 0;
        let mut current = tree;
        let mut prefix_type = ImportPrefix::Unknown;

        while let syn::UseTree::Path(path) = current {
            let ident_str = path.ident.to_string();
            match ident_str.as_str() {
                "super" => {
                    count += 1;
                    prefix_type = ImportPrefix::Super;
                    current = &*path.tree;
                }
                "self" => {
                    count += 1;
                    prefix_type = ImportPrefix::SelfPath;
                    current = &*path.tree;
                }
                "crate" => {
                    count = 1;
                    prefix_type = ImportPrefix::Crate;
                    break;
                }
                _ => {
                    // Hit a non-prefix segment
                    if count == 0 {
                        prefix_type = ImportPrefix::External;
                    }
                    break;
                }
            }
        }

        (prefix_type, count)
    }

    /// Normalize relative import paths to include the prefix keyword.
    ///
    /// For super/self imports, this extracts the suffix after the first "::"
    /// and prepends the appropriate prefix (accounting for nested super/self).
    ///
    /// Examples:
    /// - Input: old_name="parent::utils", new_name="parent::helpers", tree=super::utils
    ///   Output: ("super::utils", "super::helpers")
    /// - Input: old_name="grandparent::utils", new_name="grandparent::helpers", tree=super::super::utils
    ///   Output: ("super::super::utils", "super::super::helpers")
    /// - Input: old_name="parent::utils", new_name="parent::helpers::new", tree=super::utils
    ///   Output: ("super::utils", "super::helpers::new")
    fn normalize_relative_import_paths(
        old_name: &str,
        new_name: &str,
        tree: &syn::UseTree,
    ) -> (String, String) {
        let (prefix_type, prefix_count) = Self::count_relative_prefix(tree);

        // Extract the suffix after the first "::" (or use the whole name if no ::)
        // This preserves multi-segment renames like "parent::helpers::experimental"
        let old_suffix = old_name
            .split_once("::")
            .map(|(_, s)| s)
            .unwrap_or(old_name);
        let new_suffix = new_name
            .split_once("::")
            .map(|(_, s)| s)
            .unwrap_or(new_name);

        match prefix_type {
            ImportPrefix::Super => {
                // Build the prefix string with the correct number of super:: segments
                let super_prefix = "super::".repeat(prefix_count);
                (
                    format!("{}{}", super_prefix, old_suffix),
                    format!("{}{}", super_prefix, new_suffix),
                )
            }
            ImportPrefix::SelfPath => {
                // Build the prefix string with the correct number of self:: segments
                let self_prefix = "self::".repeat(prefix_count);
                (
                    format!("{}{}", self_prefix, old_suffix),
                    format!("{}{}", self_prefix, new_suffix),
                )
            }
            ImportPrefix::Crate => (
                format!("crate::{}", old_suffix),
                format!("crate::{}", new_suffix),
            ),
            _ => {
                // External or unknown - use the suffix as-is
                (old_suffix.to_string(), new_suffix.to_string())
            }
        }
    }

    /// Process a complete use statement (single-line or multi-line) and return the transformed version if changed.
    ///
    /// Returns: Option<(transformed_text, change_count)>
    fn process_use_statement(
        &self,
        use_stmt_lines: &[(usize, &str)],
        old_name: &str,
        new_name: &str,
        indent: usize,
        all_lines: &[&str],
    ) -> Option<(String, usize)> {
        if use_stmt_lines.is_empty() {
            return None;
        }

        // Concatenate all lines to form the complete use statement
        let concatenated: String = use_stmt_lines
            .iter()
            .map(|(_, line)| line.trim())
            .collect::<Vec<_>>()
            .join(" ");

        let trimmed = concatenated.trim();

        // Convert hyphenated crate names to underscored for Rust identifiers
        let old_rust_ident = old_name.replace('-', "_");
        let new_rust_ident = new_name.replace('-', "_");

        tracing::info!(
            use_statement = %trimmed,
            old_name = %old_name,
            new_name = %new_name,
            old_rust_ident = %old_rust_ident,
            new_rust_ident = %new_rust_ident,
            "Processing use statement"
        );

        // Check if this statement contains the old module name (as Rust identifier)
        let contains_old_module = trimmed.contains(&old_rust_ident);

        // Check for various import patterns
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

        let relative_import_matches = if old_name.contains("::") {
            old_name
                .split_once("::")
                .map(|(_, suffix)| {
                    let relative_pattern = format!("use {}::", suffix);
                    trimmed.starts_with(&relative_pattern) || {
                        let leaf_pattern = format!("use {};", suffix);
                        trimmed.starts_with(&leaf_pattern)
                    }
                })
                .unwrap_or(false)
        } else {
            false
        };

        let old_module_name = old_name.split("::").last().unwrap_or("");
        let super_import_matches = !old_module_name.is_empty()
            && (trimmed.contains(&format!("super::{}::", old_module_name))     // super::module::Thing
                || trimmed.contains(&format!("super::{}::*", old_module_name))  // super::module::*
                || trimmed.contains(&format!("super::{};", old_module_name))    // super::module;
                || trimmed.contains(&format!("super::{} as", old_module_name))  // super::module as alias
                || trimmed.contains(&format!("super::{}::{{", old_module_name))); // super::module::{...}
        let self_import_matches = !old_module_name.is_empty()
            && (trimmed.contains(&format!("self::{}::", old_module_name))     // self::module::Thing
                || trimmed.contains(&format!("self::{}::*", old_module_name))  // self::module::*
                || trimmed.contains(&format!("self::{};", old_module_name))    // self::module;
                || trimmed.contains(&format!("self::{} as", old_module_name))  // self::module as alias
                || trimmed.contains(&format!("self::{}::{{", old_module_name))); // self::module::{...}

        // If none of the patterns match, no change needed
        if !contains_old_module
            && !crate_import_matches
            && !relative_import_matches
            && !super_import_matches
            && !self_import_matches
        {
            return None;
        }

        // Handle super:: and self:: imports using AST transformation
        // This approach handles ALL UseTree variants (Path, Name, Rename, Glob, Group)
        // instead of just the limited patterns we had with string replacement
        if super_import_matches || self_import_matches {
            // Extract just the use statement for parsing
            let use_stmt = if let Some(semi_idx) = trimmed.find(';') {
                &trimmed[..=semi_idx]
            } else {
                trimmed
            };

            // Try to parse the use statement
            if let Ok(item_use) = syn::parse_str::<syn::ItemUse>(use_stmt) {
                // Normalize the module paths to include the prefix
                // e.g., "parent::utils" → "super::utils", "current::utils" → "self::utils"
                let (effective_old, effective_new) =
                    Self::normalize_relative_import_paths(old_name, new_name, &item_use.tree);

                tracing::info!(
                    effective_old = %effective_old,
                    effective_new = %effective_new,
                    original_statement = %trimmed,
                    "Normalized super/self import paths for AST transformation"
                );

                // Use AST rewriter to handle all UseTree variants
                if let Some(new_tree) =
                    crate::parser::rewrite_use_tree(&item_use.tree, &effective_old, &effective_new)
                {
                    let formatted = format!("use {};", quote::quote!(#new_tree));
                    // Normalize spacing: remove extra spaces around ::, {, }, and commas
                    let normalized = formatted
                        .replace(" :: ", "::")
                        .replace("{ ", "{")
                        .replace(" }", "}")
                        .replace(" , ", ", ");
                    let indent_str = " ".repeat(indent);
                    let mut result = format!("{}{}\n", indent_str, normalized);

                    // Don't add extra newline if this is the last line
                    let last_line_idx = use_stmt_lines.last().map(|(idx, _)| *idx).unwrap_or(0);
                    if last_line_idx >= all_lines.len() - 1 {
                        result.pop();
                    }

                    tracing::info!(
                        original = %trimmed,
                        transformed = %normalized,
                        "Successfully transformed super/self import via AST"
                    );

                    return Some((result, 1));
                } else {
                    tracing::warn!(
                        statement = %use_stmt,
                        "AST rewriter returned None for super/self import"
                    );
                }
            } else {
                tracing::warn!(
                    statement = %use_stmt,
                    "Failed to parse super/self import statement"
                );
            }

            // Fallback: return None to let the main AST path handle it
            // This prevents breaking on unparseable statements
            return None;
        }

        // Extract just the use statement (up to and including the semicolon)
        let use_stmt = if let Some(semi_idx) = trimmed.find(';') {
            &trimmed[..=semi_idx]
        } else {
            trimmed
        };

        // Skip AST rewrite if this appears to be inside a format string template
        // Format strings use {{ and }} to escape braces, which never appear in valid use syntax
        // This prevents breaking format string escaping when we reserialize via quote!
        if use_stmt.contains("{{") || use_stmt.contains("}}") {
            tracing::debug!(
                use_stmt = %use_stmt,
                "Skipping AST rewrite for format string template (contains escaped braces)"
            );

            // Apply regex replacement directly since AST rewrite would break escaping
            let old_rust_ident = old_name.replace('-', "_");
            let new_rust_ident = new_name.replace('-', "_");
            let pattern = format!(r"\b{}\s*::", regex::escape(&old_rust_ident));

            if let Ok(re) = regex::Regex::new(&pattern) {
                let new_content = re.replace_all(trimmed, |_caps: &regex::Captures| {
                    format!("{}::", new_rust_ident)
                });

                if new_content != trimmed {
                    let indent_str = " ".repeat(indent);
                    let mut result = format!("{}{}\n", indent_str, new_content);

                    // Don't add extra newline if this is the last line
                    let last_line_idx = use_stmt_lines.last().map(|(idx, _)| *idx).unwrap_or(0);
                    if last_line_idx >= all_lines.len() - 1 {
                        result.pop();
                    }

                    return Some((result, 1));
                }
            }

            return None;
        }

        // Try to parse the use statement
        match syn::parse_str::<syn::ItemUse>(use_stmt) {
            Ok(item_use) => {
                // Compute effective module paths
                let (effective_old, effective_new) = if crate_import_matches {
                    let old_suffix = old_name.split_once("::").map(|(_, s)| s).unwrap_or("");
                    let new_suffix = new_name.split_once("::").map(|(_, s)| s).unwrap_or("");
                    (
                        format!("crate::{}", old_suffix),
                        format!("crate::{}", new_suffix),
                    )
                } else if relative_import_matches {
                    let old_suffix = old_name
                        .split_once("::")
                        .map(|(_, s)| s)
                        .unwrap_or(old_name);
                    let new_suffix = new_name
                        .split_once("::")
                        .map(|(_, s)| s)
                        .unwrap_or(new_name);
                    (old_suffix.to_string(), new_suffix.to_string())
                } else {
                    (old_rust_ident.clone(), new_rust_ident.clone())
                };

                tracing::info!(
                    effective_old = %effective_old,
                    effective_new = %effective_new,
                    "Computed effective module paths"
                );

                // Try to rewrite using AST transformation
                if let Some(new_tree) =
                    crate::parser::rewrite_use_tree(&item_use.tree, &effective_old, &effective_new)
                {
                    // Preserve visibility (pub, pub(crate), etc.)
                    let vis_str = match &item_use.vis {
                        syn::Visibility::Public(_) => "pub ",
                        _ => "",
                    };
                    let formatted = format!("{}use {};", vis_str, quote::quote!(#new_tree));
                    let normalized = formatted.replace(" :: ", "::");
                    let indent_str = " ".repeat(indent);
                    let mut result = format!("{}{}\n", indent_str, normalized);

                    // Don't add extra newline if this is the last line
                    let last_line_idx = use_stmt_lines.last().map(|(idx, _)| *idx).unwrap_or(0);
                    if last_line_idx >= all_lines.len() - 1 {
                        result.pop(); // Remove trailing newline
                    }

                    tracing::info!(
                        original = %trimmed,
                        transformed = %normalized,
                        "Successfully transformed use statement"
                    );

                    return Some((result, 1));
                }
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    statement = %use_stmt,
                    "Failed to parse use statement"
                );
            }
        }

        None
    }
}

impl ImportAdvancedSupport for RustImportSupport {
    // Uses default implementation
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

        let imports = ImportParser::parse_imports(&support, content);
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

        let (result, changes) = ImportRenameSupport::rewrite_imports_for_rename(
            &support,
            content,
            "old_crate",
            "new_crate",
        );
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

        assert!(ImportParser::contains_import(
            &support,
            content,
            "std::collections"
        ));
        assert!(ImportParser::contains_import(&support, content, "serde"));
        assert!(!ImportParser::contains_import(&support, content, "tokio"));
    }

    #[test]
    fn test_contains_import_no_false_positives() {
        let support = RustImportSupport;
        let content = r#"
use my_test_module::Thing;
use testing_utils::Helper;
use scorecard::Report;
"#;

        // Should NOT match "test" as substring of "my_test_module" or "testing_utils"
        assert!(!ImportParser::contains_import(&support, content, "test"));

        // Should NOT match "core" as substring of "scorecard"
        assert!(!ImportParser::contains_import(&support, content, "core"));

        // Should match exact module names
        assert!(ImportParser::contains_import(
            &support,
            content,
            "my_test_module"
        ));
        assert!(ImportParser::contains_import(
            &support,
            content,
            "testing_utils"
        ));
        assert!(ImportParser::contains_import(
            &support,
            content,
            "scorecard"
        ));
    }

    #[test]
    fn test_contains_import_exact_match() {
        let support = RustImportSupport;
        let content = r#"
use std::collections::HashMap;
use crate::parser::utils;
"#;

        // Exact matches
        assert!(ImportParser::contains_import(&support, content, "std"));
        assert!(ImportParser::contains_import(
            &support,
            content,
            "std::collections"
        ));
        assert!(ImportParser::contains_import(&support, content, "crate"));
        assert!(ImportParser::contains_import(
            &support,
            content,
            "crate::parser"
        ));
    }

    #[test]
    fn test_contains_import_prefix_match() {
        let support = RustImportSupport;
        let content = r#"
use std::collections::HashMap;
use crate::foo::bar::baz::Thing;
"#;

        // Prefix matches
        assert!(ImportParser::contains_import(&support, content, "std"));
        assert!(ImportParser::contains_import(
            &support,
            content,
            "crate::foo"
        ));
        assert!(ImportParser::contains_import(
            &support,
            content,
            "crate::foo::bar"
        ));
    }

    #[test]
    fn test_contains_import_subsequence_match() {
        let support = RustImportSupport;
        let content = r#"
use crate::foo::bar::baz::Thing;
use std::collections::hash::map::HashMap;
"#;

        // Contiguous subsequence matches
        assert!(ImportParser::contains_import(&support, content, "bar::baz"));
        assert!(ImportParser::contains_import(
            &support,
            content,
            "foo::bar::baz"
        ));
        assert!(ImportParser::contains_import(
            &support,
            content,
            "hash::map"
        ));

        // Should NOT match non-contiguous subsequences
        assert!(!ImportParser::contains_import(
            &support, content, "foo::baz"
        )); // skips "bar"
        assert!(!ImportParser::contains_import(
            &support, content, "std::map"
        )); // skips "collections::hash"
    }

    #[test]
    fn test_contains_import_single_vs_multi_segment() {
        let support = RustImportSupport;
        let content = r#"
use std::collections::HashMap;
use my_std::other::Thing;
"#;

        // Single segment "std" should match "std::collections" but not "my_std"
        assert!(ImportParser::contains_import(&support, content, "std"));
        assert!(!ImportParser::contains_import(
            &support,
            content,
            "std::other"
        ));

        // "my_std" should match exactly
        assert!(ImportParser::contains_import(&support, content, "my_std"));
    }

    #[test]
    fn test_add_import() {
        let support = RustImportSupport;
        let content = r#"use std::collections::HashMap;

fn main() {}"#;

        let result = ImportMutationSupport::add_import(&support, content, "serde::Serialize");
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

        let result = ImportMutationSupport::remove_import(&support, content, "serde");
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
        let (result, changes) = ImportRenameSupport::rewrite_imports_for_rename(
            &support,
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
        let (result, changes) = ImportRenameSupport::rewrite_imports_for_rename(
            &support,
            content,
            "test_project::utils::helpers",
            "test_project::utils::support",
        );

        assert_eq!(
            changes, 1,
            "Should have changed 1 import. Actual:\n{}",
            result
        );
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

    #[test]
    fn test_rewrite_qualified_paths() {
        let support = RustImportSupport;
        let content = r#"
use std::sync::Arc;

pub fn example() {
    let cache = cb_ast::CacheSettings::default();
    let report = cb_ast::complexity::analyze_file_complexity(&path, &content, &symbols, lang);
    cb_ast::refactoring::extract_function::plan_extract_function(params).await?;
}
"#;

        let (result, changes) = support.rewrite_imports_for_rename(content, "cb_ast", "mill_ast");

        assert!(changes > 0, "Should detect changes");
        assert!(
            !result.contains("cb_ast::"),
            "Should replace all cb_ast:: occurrences. Result:\n{}",
            result
        );
        assert!(
            result.contains("mill_ast::CacheSettings"),
            "Should update CacheSettings reference"
        );
        assert!(
            result.contains("mill_ast::complexity::analyze_file_complexity"),
            "Should update complexity reference"
        );
        assert!(
            result.contains("mill_ast::refactoring::extract_function"),
            "Should update refactoring reference"
        );
    }

    #[test]
    fn test_qualified_paths_and_use_statements() {
        let support = RustImportSupport;
        let content = r#"
use cb_ast::CacheSettings;

pub fn example() {
    let cache = cb_ast::CacheSettings::default();
}
"#;

        let (result, changes) = support.rewrite_imports_for_rename(content, "cb_ast", "mill_ast");

        // Both the use statement AND the qualified path should be updated
        assert!(
            result.contains("use mill_ast::CacheSettings"),
            "Should update use statement. Result:\n{}",
            result
        );
        assert!(
            result.contains("mill_ast::CacheSettings::default()"),
            "Should update qualified path. Result:\n{}",
            result
        );
        assert!(
            !result.contains("cb_ast::"),
            "Should not contain any cb_ast:: references. Result:\n{}",
            result
        );
        assert!(
            changes >= 2,
            "Should update both use and qualified path, got {}",
            changes
        );
    }

    #[test]
    fn test_qualified_paths_word_boundary() {
        let support = RustImportSupport;
        let content = r#"
pub fn example() {
    let cache = cb_ast::CacheSettings::default();
    let my_cb_ast_thing = 123;  // Should NOT be changed
    let cb_ast_prefix = "test";  // Should NOT be changed
}
"#;

        let (result, _changes) = support.rewrite_imports_for_rename(content, "cb_ast", "mill_ast");

        assert!(
            result.contains("mill_ast::CacheSettings"),
            "Should update qualified path"
        );
        assert!(
            result.contains("my_cb_ast_thing"),
            "Should preserve variable names with cb_ast prefix"
        );
        assert!(
            result.contains("cb_ast_prefix"),
            "Should preserve variable names with cb_ast in them"
        );
        assert_eq!(
            result.matches("mill_ast").count(),
            1,
            "Should only replace the qualified path, not variable names"
        );
    }

    #[test]
    fn test_qualified_paths_with_spacing() {
        let support = RustImportSupport;
        let content = r#"
pub fn example() {
    let cache = cb_ast:: CacheSettings::default();
    let report = cb_ast::  complexity::analyze_file();
}
"#;

        let (result, changes) = support.rewrite_imports_for_rename(content, "cb_ast", "mill_ast");

        assert!(changes > 0, "Should detect changes");
        assert!(
            !result.contains("cb_ast"),
            "Should replace all cb_ast references regardless of spacing"
        );
        assert!(
            result.contains("mill_ast::"),
            "Should normalize to standard spacing"
        );
    }

    #[test]
    fn test_multiline_grouped_imports() {
        let support = RustImportSupport;

        // Test case that was failing: multi-line grouped imports
        let content = r#"use crate::handlers::tools::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use cb_services::{
    services::file_service::EditPlanResult, ChecksumValidator, DryRunGenerator, PlanConverter,
    PostApplyValidator, ValidationConfig, ValidationResult,
};
use mill_foundation::core::model::mcp::ToolCall;

pub fn example() {
    let validator = cb_services::ChecksumValidator::new();
}
"#;

        let (result, changes) =
            support.rewrite_imports_for_rename(content, "cb_services", "mill_services");

        println!("Result:\n{}", result);
        println!("Changes: {}", changes);

        // Should update both the multi-line import and the qualified path
        assert!(
            changes >= 2,
            "Should detect at least 2 changes (import + qualified path)"
        );
        assert!(
            !result.contains("cb_services"),
            "Should replace all cb_services references"
        );
        assert!(
            result.contains("use mill_services::{"),
            "Should update multi-line grouped import"
        );
        assert!(
            result.contains("mill_services::ChecksumValidator"),
            "Should update qualified path in code"
        );
    }

    #[test]
    fn test_multiline_imports_preserve_formatting() {
        let support = RustImportSupport;

        // Test that indentation is preserved
        let content = r#"    use cb_services::{
        ChecksumValidator,
        DryRunGenerator,
    };
"#;

        let (result, changes) =
            support.rewrite_imports_for_rename(content, "cb_services", "mill_services");

        assert_eq!(changes, 1, "Should detect 1 change");
        // Result should start with the same indentation (4 spaces)
        assert!(
            result.starts_with("    use mill_services"),
            "Should preserve leading indentation"
        );
    }

    #[test]
    fn test_extern_crate_rename() {
        let support = RustImportSupport;

        // Test extern crate declarations (deprecated Rust 2015 pattern)
        let content = r#"extern crate old_plugin_bundle;
extern crate other_crate;

fn main() {
    old_plugin_bundle::init();
}
"#;

        let (result, changes) = support.rewrite_imports_for_rename(
            content,
            "old-plugin-bundle", // Note: hyphenated input
            "new-plugin-bundle",
        );

        println!("Result:\n{}", result);
        println!("Changes: {}", changes);

        // Should update both extern crate and qualified path
        assert_eq!(
            changes, 2,
            "Should detect 2 changes (extern crate + qualified path)"
        );
        assert!(
            result.contains("extern crate new_plugin_bundle;"),
            "Should update extern crate with underscores"
        );
        assert!(
            !result.contains("old_plugin_bundle"),
            "Should replace all old references"
        );
        assert!(
            result.contains("new_plugin_bundle::init()"),
            "Should update qualified paths"
        );
        assert!(
            result.contains("extern crate other_crate;"),
            "Should preserve other extern crate declarations"
        );
    }

    #[test]
    fn test_pub_use_as_rename() {
        let support = RustImportSupport;

        // Test pub use ... as pattern (re-exports)
        let content = r#"pub use old_workspaces as workspaces;
use old_workspaces::Thing;

fn main() {
    old_workspaces::utility();
}
"#;

        let (result, changes) = support.rewrite_imports_for_rename(
            content,
            "old-workspaces", // Note: hyphenated input
            "new-workspaces",
        );

        println!("Result:\n{}", result);
        println!("Changes: {}", changes);

        // Should update pub use, regular use, and qualified path
        assert_eq!(changes, 3, "Should detect 3 changes");
        assert!(
            result.contains("pub use new_workspaces as workspaces;"),
            "Should update pub use with underscores"
        );
        assert!(
            result.contains("use new_workspaces::Thing;"),
            "Should update regular use statement"
        );
        assert!(
            result.contains("new_workspaces::utility()"),
            "Should update qualified paths"
        );
        assert!(
            !result.contains("old_workspaces"),
            "Should replace all old references"
        );
    }

    #[test]
    fn test_extern_crate_and_pub_use_combined() {
        let support = RustImportSupport;

        // Real-world test case combining both patterns
        let content = r#"// Force linker to include plugin-bundle
extern crate old_plugin_bundle;

// Re-export workspaces
pub use old_workspaces as workspaces;

use old_workspaces::WorkspaceManager;

fn init() {
    let _plugins = old_plugin_bundle::all_plugins();
    let manager = old_workspaces::WorkspaceManager::new();
}
"#;

        // Rename plugin-bundle
        let (result1, changes1) =
            support.rewrite_imports_for_rename(content, "old-plugin-bundle", "new-plugin-bundle");

        assert_eq!(changes1, 2, "Should update extern crate + qualified path");
        assert!(result1.contains("extern crate new_plugin_bundle;"));
        assert!(result1.contains("new_plugin_bundle::all_plugins()"));

        // Rename workspaces
        let (result2, changes2) =
            support.rewrite_imports_for_rename(&result1, "old-workspaces", "new-workspaces");

        assert_eq!(changes2, 3, "Should update pub use + use + qualified path");
        assert!(result2.contains("pub use new_workspaces as workspaces;"));
        assert!(result2.contains("use new_workspaces::WorkspaceManager;"));
        assert!(result2.contains("new_workspaces::WorkspaceManager::new()"));

        // Verify no old names remain
        assert!(!result2.contains("old_plugin_bundle"));
        assert!(!result2.contains("old_workspaces"));
    }

    #[test]
    fn test_format_string_template_escaping() {
        let support = RustImportSupport;

        // Simulates code generation templates like those in plugin_scaffold.rs
        // The use statement appears to be inside a string with escaped braces
        let content = "use cb_plugin_api::{{ ParsedSource, PluginResult }};\nuse std::path::Path;";

        let (result, changes) = support.rewrite_imports_for_rename(
            content,
            "cb-plugin-api", // Hyphenated crate name (will be converted to cb_plugin_api for matching)
            "mill-plugin-api",
        );

        // Should update via regex within process_use_statement because AST rewrite is skipped for {{ }}
        assert!(changes > 0);

        // The escaped braces {{ }} must be preserved
        assert!(result.contains("mill_plugin_api::{{ ParsedSource, PluginResult }}"));

        // Should not break into { {
        assert!(!result.contains("mill_plugin_api::{ { ParsedSource"));

        // Old name should be replaced
        assert!(!result.contains("cb_plugin_api"));
    }

    #[test]
    fn test_import_prefix_classification() {
        // Test that we can classify import prefixes and count nested levels using AST
        let crate_import = syn::parse_str::<syn::ItemUse>("use crate::foo::bar;").unwrap();
        let super_import = syn::parse_str::<syn::ItemUse>("use super::foo::bar;").unwrap();
        let super_super_import =
            syn::parse_str::<syn::ItemUse>("use super::super::foo::bar;").unwrap();
        let self_import = syn::parse_str::<syn::ItemUse>("use self::foo::bar;").unwrap();
        let external_import =
            syn::parse_str::<syn::ItemUse>("use std::collections::HashMap;").unwrap();

        let (prefix_type, count) = RustImportSupport::count_relative_prefix(&crate_import.tree);
        assert_eq!(prefix_type, ImportPrefix::Crate);
        assert_eq!(count, 1);

        let (prefix_type, count) = RustImportSupport::count_relative_prefix(&super_import.tree);
        assert_eq!(prefix_type, ImportPrefix::Super);
        assert_eq!(count, 1);

        let (prefix_type, count) =
            RustImportSupport::count_relative_prefix(&super_super_import.tree);
        assert_eq!(prefix_type, ImportPrefix::Super);
        assert_eq!(count, 2, "Should count nested super:: prefixes");

        let (prefix_type, count) = RustImportSupport::count_relative_prefix(&self_import.tree);
        assert_eq!(prefix_type, ImportPrefix::SelfPath);
        assert_eq!(count, 1);

        let (prefix_type, count) = RustImportSupport::count_relative_prefix(&external_import.tree);
        assert_eq!(prefix_type, ImportPrefix::External);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_consistent_pattern_matching() {
        // Verify that import pattern matching is consistent across all types
        let support = RustImportSupport;

        // Test crate:: imports
        let crate_content = r#"
use crate::utils::helper;
use crate::types::Thing;
"#;
        let (result, changes) = support.rewrite_imports_for_rename(
            crate_content,
            "test_project::utils",
            "test_project::helpers",
        );
        assert_eq!(changes, 1, "Should update crate:: import");
        assert!(result.contains("use crate::helpers::helper;"));

        // Test super:: imports (documenting current behavior)
        let super_content = r#"
use super::utils::*;
use super::utils::Thing;
"#;
        let (result, changes) =
            support.rewrite_imports_for_rename(super_content, "parent::utils", "parent::helpers");
        // Current implementation should handle these
        assert!(changes >= 2, "Should update super:: imports");
        assert!(result.contains("super::helpers::"));

        // Test self:: imports (documenting current behavior)
        let self_content = r#"
use self::utils::*;
use self::utils::Thing;
"#;
        let (result, changes) =
            support.rewrite_imports_for_rename(self_content, "current::utils", "current::helpers");
        // Current implementation should handle these
        assert!(changes >= 2, "Should update self:: imports");
        assert!(result.contains("self::helpers::"));
    }

    // ============================================================================
    // Comprehensive Super/Self Import Pattern Tests (Issue #1)
    // ============================================================================

    #[test]
    fn test_super_direct_module_import() {
        let support = RustImportSupport;
        let content = "use super::utils;";

        let (result, changes) =
            support.rewrite_imports_for_rename(content, "parent::utils", "parent::helpers");

        assert_eq!(
            changes, 1,
            "Should update direct super import. Result: {}",
            result
        );
        assert!(result.contains("use super::helpers;"), "Result: {}", result);
        assert!(!result.contains("super::utils"), "Result: {}", result);
    }

    #[test]
    fn test_super_aliased_import() {
        let support = RustImportSupport;
        let content = "use super::utils as util_helpers;";

        let (result, changes) =
            support.rewrite_imports_for_rename(content, "parent::utils", "parent::helpers");

        assert_eq!(changes, 1, "Should update aliased super import");
        assert!(
            result.contains("use super::helpers as util_helpers;"),
            "Result: {}",
            result
        );
        assert!(!result.contains("super::utils"), "Result: {}", result);
    }

    #[test]
    fn test_super_grouped_imports() {
        let support = RustImportSupport;
        let content = "use super::utils::{Thing1, Thing2, Thing3};";

        let (result, changes) =
            support.rewrite_imports_for_rename(content, "parent::utils", "parent::helpers");

        assert_eq!(changes, 1, "Should update grouped super import");
        assert!(
            result.contains("use super::helpers::{Thing1, Thing2, Thing3}"),
            "Result: {}",
            result
        );
        assert!(!result.contains("super::utils"), "Result: {}", result);
    }

    #[test]
    fn test_super_glob_import() {
        let support = RustImportSupport;
        let content = "use super::utils::*;";

        let (result, changes) =
            support.rewrite_imports_for_rename(content, "parent::utils", "parent::helpers");

        assert_eq!(changes, 1, "Should update glob super import");
        assert!(
            result.contains("use super::helpers::*;"),
            "Result: {}",
            result
        );
        assert!(!result.contains("super::utils"), "Result: {}", result);
    }

    #[test]
    fn test_super_nested_path() {
        let support = RustImportSupport;
        let content = "use super::utils::submodule::Thing;";

        let (result, changes) =
            support.rewrite_imports_for_rename(content, "parent::utils", "parent::helpers");

        assert_eq!(changes, 1, "Should update nested super import");
        assert!(
            result.contains("use super::helpers::submodule::Thing;"),
            "Result: {}",
            result
        );
        assert!(!result.contains("super::utils"), "Result: {}", result);
    }

    #[test]
    fn test_self_direct_module_import() {
        let support = RustImportSupport;
        let content = "use self::utils;";

        let (result, changes) =
            support.rewrite_imports_for_rename(content, "current::utils", "current::helpers");

        assert_eq!(changes, 1, "Should update direct self import");
        assert!(result.contains("use self::helpers;"), "Result: {}", result);
        assert!(!result.contains("self::utils"), "Result: {}", result);
    }

    #[test]
    fn test_self_aliased_import() {
        let support = RustImportSupport;
        let content = "use self::utils as my_utils;";

        let (result, changes) =
            support.rewrite_imports_for_rename(content, "current::utils", "current::helpers");

        assert_eq!(changes, 1, "Should update aliased self import");
        assert!(
            result.contains("use self::helpers as my_utils;"),
            "Result: {}",
            result
        );
        assert!(!result.contains("self::utils"), "Result: {}", result);
    }

    #[test]
    fn test_self_grouped_imports() {
        let support = RustImportSupport;
        let content = "use self::utils::{Foo, Bar};";

        let (result, changes) =
            support.rewrite_imports_for_rename(content, "current::utils", "current::helpers");

        assert_eq!(changes, 1, "Should update grouped self import");
        assert!(
            result.contains("use self::helpers::{Foo, Bar}"),
            "Result: {}",
            result
        );
        assert!(!result.contains("self::utils"), "Result: {}", result);
    }

    #[test]
    fn test_self_glob_import() {
        let support = RustImportSupport;
        let content = "use self::utils::*;";

        let (result, changes) =
            support.rewrite_imports_for_rename(content, "current::utils", "current::helpers");

        assert_eq!(changes, 1, "Should update glob self import");
        assert!(
            result.contains("use self::helpers::*;"),
            "Result: {}",
            result
        );
        assert!(!result.contains("self::utils"), "Result: {}", result);
    }

    #[test]
    fn test_multiline_super_imports() {
        let support = RustImportSupport;
        let content = r#"use super::utils::{
    Thing1,
    Thing2,
    Thing3
};"#;

        let (result, changes) =
            support.rewrite_imports_for_rename(content, "parent::utils", "parent::helpers");

        assert_eq!(changes, 1, "Should update multi-line super import");
        assert!(result.contains("super::helpers"), "Result: {}", result);
        assert!(!result.contains("super::utils"), "Result: {}", result);
    }

    #[test]
    fn test_mixed_super_self_patterns() {
        let support = RustImportSupport;
        let content = r#"
use super::utils;
use super::utils::Thing;
use super::utils::*;
use super::utils as u;
use self::helpers::Tool;
"#;

        let (result, changes) =
            support.rewrite_imports_for_rename(content, "parent::utils", "parent::renamed_utils");

        assert!(
            changes >= 4,
            "Should update at least 4 super:: imports, got {}",
            changes
        );
        assert!(
            result.contains("super::renamed_utils;"),
            "Result: {}",
            result
        );
        assert!(
            result.contains("super::renamed_utils::Thing;"),
            "Result: {}",
            result
        );
        assert!(
            result.contains("super::renamed_utils::*;"),
            "Result: {}",
            result
        );
        assert!(
            result.contains("super::renamed_utils as u;"),
            "Result: {}",
            result
        );
        // self::helpers should remain unchanged
        assert!(
            result.contains("self::helpers::Tool;"),
            "Result: {}",
            result
        );
    }
}
