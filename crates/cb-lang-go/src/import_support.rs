//! Import support implementation for Go language plugin
//!
//! Provides synchronous import parsing, analysis, and rewriting capabilities for Go source code.

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
        // We need to replace the old module path with the new one in import statements

        let mut new_content = content.to_string();
        let mut changes = 0;

        // Handle single import: import "old/path"
        let old_single = format!("import \"{}\"", old_name);
        let new_single = format!("import \"{}\"", new_name);
        if new_content.contains(&old_single) {
            new_content = new_content.replace(&old_single, &new_single);
            changes += 1;
        }

        // Handle aliased import: import alias "old/path"
        let old_aliased = format!("\"{}\"", old_name);
        let new_aliased = format!("\"{}\"", new_name);

        // Count and replace occurrences in import blocks
        for line in content.lines() {
            let trimmed = line.trim();
            if (trimmed.starts_with("import") || trimmed.starts_with("\""))
                && trimmed.contains(&old_aliased)
                && !trimmed.contains(&new_aliased) {
                    changes += 1;
                }
        }

        new_content = new_content.replace(&old_aliased, &new_aliased);

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

        let mut new_content = content.to_string();
        let mut changes = 0;

        // Replace import paths containing the old package
        let old_import = format!("\"{}\"", old_package);
        let new_import = format!("\"{}\"", new_package);

        for line in content.lines() {
            if line.contains("import") && line.contains(&old_import) {
                changes += 1;
            }
        }

        new_content = new_content.replace(&old_import, &new_import);

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
        let mut result = String::new();
        let mut import_added = false;

        for (i, line) in lines.iter().enumerate() {
            result.push_str(line);
            result.push('\n');

            // Add after package declaration
            if !import_added && line.trim().starts_with("package ") {
                // Look ahead to see if there's already an import block
                let has_import_block = lines.iter().skip(i + 1).any(|l| {
                    let trimmed = l.trim();
                    trimmed.starts_with("import (") || trimmed.starts_with("import \"")
                });

                if !has_import_block {
                    // Add a new import statement
                    result.push('\n');
                    result.push_str(&format!("import \"{}\"\n", module));
                    import_added = true;
                    tracing::debug!(module = %module, "Added new import after package declaration");
                } else {
                    // Find the import block and add there
                    // This will be handled in subsequent iterations
                }
            }

            // Add to existing import block
            if !import_added && line.trim().starts_with("import (") {
                result.push_str(&format!("\t\"{}\"\n", module));
                import_added = true;
                tracing::debug!(module = %module, "Added import to existing import block");
            }
        }

        // If we still haven't added it, append at the end (shouldn't happen in well-formed Go)
        if !import_added {
            result.push_str(&format!("\nimport \"{}\"\n", module));
            tracing::debug!(module = %module, "Added import at end of file");
        }

        result
    }

    fn remove_import(&self, content: &str, module: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();
        let mut in_import_block = false;
        let mut removed = false;

        for line in lines {
            let trimmed = line.trim();

            // Start of import block
            if trimmed.starts_with("import (") {
                in_import_block = true;
                result.push(line.to_string());
                continue;
            }

            // End of import block
            if in_import_block && trimmed == ")" {
                in_import_block = false;
                result.push(line.to_string());
                continue;
            }

            // Single import statement
            if trimmed.starts_with("import ") && trimmed.contains(&format!("\"{}\"", module)) {
                removed = true;
                tracing::debug!(module = %module, "Removed single import statement");
                continue; // Skip this line
            }

            // Import within block
            if in_import_block && trimmed.contains(&format!("\"{}\"", module)) {
                removed = true;
                tracing::debug!(module = %module, "Removed import from block");
                continue; // Skip this line
            }

            result.push(line.to_string());
        }

        if !removed {
            tracing::debug!(module = %module, "Import not found, content unchanged");
        }

        result.join("\n")
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
