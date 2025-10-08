//! Import support implementation for Go language plugin
//!
//! Provides synchronous import parsing, analysis, and rewriting capabilities for Go source code.

use cb_lang_common::import_helpers::{
    insert_line_at, remove_lines_matching, replace_in_lines,
};
use cb_plugin_api::ImportSupport;
use std::path::Path;

/// Go-specific import support implementation
pub struct GoImportSupport;

impl ImportSupport for GoImportSupport {
    fn parse_imports(&self, content: &str) -> Vec<String> {
        // Use the existing parser to extract imports
        match crate::parser::analyze_imports(content, None) {
            Ok(graph) => graph.imports.into_iter().map(|i| i.module_path).collect(),
            Err(e) => {
                tracing::warn!(error = %e, "Failed to parse Go imports, returning empty list");
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
        // Go imports are module paths, so renaming affects the import path itself
        // Replace quoted module paths in import statements
        let old_quoted = format!("\"{}\"", old_name);
        let new_quoted = format!("\"{}\"", new_name);

        let (new_content, changes) = replace_in_lines(content, &old_quoted, &new_quoted);

        tracing::debug!(
            old_name = %old_name,
            new_name = %new_name,
            changes = changes,
            "Rewrote Go imports for symbol rename"
        );

        (new_content, changes)
    }

    fn rewrite_imports_for_move(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
    ) -> (String, usize) {
        // Extract package names from file paths
        // Go imports are based on package paths, not file paths
        // For simplicity, we'll use the directory name as the package identifier

        let old_package = old_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        let new_package = new_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        if old_package.is_empty() || new_package.is_empty() || old_package == new_package {
            return (content.to_string(), 0);
        }

        // Replace import paths containing the old package
        let old_quoted = format!("\"{}\"", old_package);
        let new_quoted = format!("\"{}\"", new_package);

        let (new_content, changes) = replace_in_lines(content, &old_quoted, &new_quoted);

        tracing::debug!(
            old_path = ?old_path,
            new_path = ?new_path,
            old_package = %old_package,
            new_package = %new_package,
            changes = changes,
            "Rewrote Go imports for file move"
        );

        (new_content, changes)
    }

    fn contains_import(&self, content: &str, module: &str) -> bool {
        // Parse imports and check if the module is present
        let imports = self.parse_imports(content);
        let found = imports.iter().any(|imp| imp == module || imp.ends_with(&format!("/{}", module)));

        tracing::debug!(
            module = %module,
            found = found,
            "Checked if Go file contains import"
        );

        found
    }

    fn add_import(&self, content: &str, module: &str) -> String {
        // Don't add if already present
        if self.contains_import(content, module) {
            tracing::debug!(module = %module, "Import already present, skipping");
            return content.to_string();
        }

        let lines: Vec<&str> = content.lines().collect();

        // Check if there's an import block
        let import_block_idx = lines
            .iter()
            .position(|line| line.trim().starts_with("import ("));

        if let Some(block_idx) = import_block_idx {
            // Add to existing import block (right after "import (")
            let new_import = format!("\t\"{}\"", module);
            let result = insert_line_at(content, block_idx + 1, &new_import);
            tracing::debug!(module = %module, "Added import to existing import block");
            return result;
        }

        // No import block - find package declaration and add after it
        let package_idx = lines
            .iter()
            .position(|line| line.trim().starts_with("package "));

        if let Some(pkg_idx) = package_idx {
            // Add new import statement after package declaration
            let new_import = format!("\nimport \"{}\"", module);
            let result = insert_line_at(content, pkg_idx + 1, &new_import);
            tracing::debug!(module = %module, "Added new import after package declaration");
            return result;
        }

        // Fallback: append at end (shouldn't happen in well-formed Go)
        let last_line_idx = if lines.is_empty() { 0 } else { lines.len() };
        let new_import = format!("\nimport \"{}\"", module);
        let result = insert_line_at(content, last_line_idx, &new_import);
        tracing::debug!(module = %module, "Added import at end of file");
        result
    }

    fn remove_import(&self, content: &str, module: &str) -> String {
        let quoted_module = format!("\"{}\"", module);

        // Remove lines that match the import pattern
        let (result, removed_count) = remove_lines_matching(content, |line| {
            let trimmed = line.trim();
            // Match single import: import "module"
            // Or import within block: "module" or alias "module"
            (trimmed.starts_with("import ") && trimmed.contains(&quoted_module))
                || (trimmed.starts_with("\"") && trimmed.contains(&quoted_module))
        });

        if removed_count > 0 {
            tracing::debug!(
                module = %module,
                removed_count = removed_count,
                "Removed import statement(s)"
            );
        } else {
            tracing::debug!(module = %module, "Import not found, content unchanged");
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_imports() {
        let support = GoImportSupport;
        let content = r#"package main

import "fmt"
import (
    "os"
    "github.com/user/repo"
)
"#;
        let imports = support.parse_imports(content);
        assert!(imports.contains(&"fmt".to_string()));
        assert!(imports.contains(&"os".to_string()));
        assert!(imports.contains(&"github.com/user/repo".to_string()));
    }

    #[test]
    fn test_contains_import() {
        let support = GoImportSupport;
        let content = r#"package main
import "fmt"
"#;
        assert!(support.contains_import(content, "fmt"));
        assert!(!support.contains_import(content, "os"));
    }

    #[test]
    fn test_add_import() {
        let support = GoImportSupport;
        let content = r#"package main

func main() {}
"#;
        let result = support.add_import(content, "fmt");
        assert!(result.contains("import \"fmt\""));
    }

    #[test]
    fn test_add_import_to_existing_block() {
        let support = GoImportSupport;
        let content = r#"package main

import (
    "fmt"
)

func main() {}
"#;
        let result = support.add_import(content, "os");
        assert!(result.contains("\"os\""));
    }

    #[test]
    fn test_remove_import() {
        let support = GoImportSupport;
        let content = r#"package main
import "fmt"
import "os"
"#;
        let result = support.remove_import(content, "fmt");
        assert!(!result.contains("import \"fmt\""));
        assert!(result.contains("import \"os\""));
    }

    #[test]
    fn test_rewrite_imports_for_rename() {
        let support = GoImportSupport;
        let content = r#"package main
import "oldpkg"
"#;
        let (result, changes) = support.rewrite_imports_for_rename(content, "oldpkg", "newpkg");
        assert!(result.contains("\"newpkg\""));
        assert!(!result.contains("\"oldpkg\""));
        assert!(changes > 0);
    }

    #[test]
    fn test_rewrite_imports_for_move() {
        let support = GoImportSupport;
        let content = r#"package main
import "oldfile"
"#;
        let old_path = Path::new("/src/oldfile.go");
        let new_path = Path::new("/src/newfile.go");
        let (result, changes) = support.rewrite_imports_for_move(content, old_path, new_path);
        assert!(result.contains("\"newfile\""));
        assert!(changes > 0);
    }
}
