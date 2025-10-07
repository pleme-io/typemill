//! Python import support capability implementation
//!
//! Provides synchronous import parsing, analysis, and rewriting for Python code.
//! Implements the `ImportSupport` trait from cb-plugin-api.

use cb_plugin_api::import_support::ImportSupport;
use std::path::Path;
use tracing::debug;

use crate::parser;

/// Python import support implementation
///
/// Provides import operations for Python code including:
/// - Parsing import statements (import, from...import)
/// - Rewriting imports for rename and move operations
/// - Import manipulation (add, remove, check)
pub struct PythonImportSupport;

impl ImportSupport for PythonImportSupport {
    fn parse_imports(&self, content: &str) -> Vec<String> {
        debug!("Parsing Python imports from content");

        // Use analyze_imports helper (returns ImportGraph)
        match parser::analyze_imports(content, None) {
            Ok(graph) => {
                let module_paths: Vec<String> = graph
                    .imports
                    .into_iter()
                    .map(|info| info.module_path)
                    .collect();

                debug!(
                    imports_count = module_paths.len(),
                    "Parsed Python imports successfully"
                );
                module_paths
            }
            Err(e) => {
                debug!(error = %e, "Failed to parse Python imports, returning empty list");
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
            "Rewriting Python imports for rename"
        );

        let mut result = String::new();
        let mut changes = 0;

        for line in content.lines() {
            let trimmed = line.trim();

            // Check if this is an import line that contains the old name
            if (trimmed.starts_with("import ") || trimmed.starts_with("from "))
                && trimmed.contains(old_name)
            {
                // Simple string replacement for module names
                let new_line = line.replace(old_name, new_name);
                result.push_str(&new_line);
                changes += 1;
            } else {
                result.push_str(line);
            }
            result.push('\n');
        }

        debug!(
            changes_count = changes,
            "Completed import rewrite for rename"
        );
        (result, changes)
    }

    fn rewrite_imports_for_move(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
    ) -> (String, usize) {
        debug!(
            old_path = %old_path.display(),
            new_path = %new_path.display(),
            "Rewriting Python imports for file move"
        );

        // Convert file paths to Python module paths
        let old_module = path_to_python_module(old_path);
        let new_module = path_to_python_module(new_path);

        // If we couldn't determine module names, return unchanged
        if old_module.is_empty() || new_module.is_empty() {
            debug!("Could not determine module paths, returning unchanged");
            return (content.to_string(), 0);
        }

        // Use rename logic with module paths
        self.rewrite_imports_for_rename(content, &old_module, &new_module)
    }

    fn contains_import(&self, content: &str, module: &str) -> bool {
        debug!(module = %module, "Checking if content contains import");

        for line in content.lines() {
            let trimmed = line.trim();

            // Check for "import module" or "import module as ..."
            if trimmed.starts_with("import ") {
                let import_part = trimmed.strip_prefix("import ").unwrap_or("");
                // Split by 'as' and check the module name part
                let module_name = import_part.split(" as ").next().unwrap_or("").trim();
                if module_name == module || module_name.starts_with(&format!("{}.", module)) {
                    return true;
                }
            }

            // Check for "from module import ..."
            if trimmed.starts_with("from ") {
                let from_part = trimmed.strip_prefix("from ").unwrap_or("");
                let module_name = from_part.split(" import ").next().unwrap_or("").trim();
                if module_name == module || module_name.starts_with(&format!("{}.", module)) {
                    return true;
                }
            }
        }

        false
    }

    fn add_import(&self, content: &str, module: &str) -> String {
        debug!(module = %module, "Adding import to Python content");

        // If the import already exists, return unchanged
        if self.contains_import(content, module) {
            debug!("Import already exists, returning unchanged");
            return content.to_string();
        }

        // Find the position to insert the import
        // Python convention: imports go at the top after docstrings/comments
        let lines: Vec<&str> = content.lines().collect();
        let mut insert_pos = 0;
        let mut in_docstring = false;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Skip shebang
            if i == 0 && trimmed.starts_with("#!") {
                insert_pos = i + 1;
                continue;
            }

            // Track docstrings
            if trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''") {
                let quote = if trimmed.starts_with("\"\"\"") { "\"\"\"" } else { "'''" };

                // Check if it's a single-line docstring
                let after_opening = &trimmed[3..];
                if after_opening.contains(quote) {
                    // Single-line docstring (e.g., """Module docstring.""")
                    insert_pos = i + 1;
                    continue;
                } else {
                    // Multi-line docstring start
                    in_docstring = true;
                    continue;
                }
            }

            if in_docstring {
                // Check if this line closes the docstring
                if trimmed.ends_with("\"\"\"") || trimmed.ends_with("'''") {
                    in_docstring = false;
                    insert_pos = i + 1;
                }
                continue;
            }

            // Skip comments and empty lines at the top
            if trimmed.starts_with('#') || trimmed.is_empty() {
                insert_pos = i + 1;
                continue;
            }

            // Found first non-comment, non-docstring line
            // If it's an import, insert after all imports
            if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
                insert_pos = i + 1;
                continue;
            }

            // First non-import line, insert here
            break;
        }

        // Build the new content
        let mut result = String::new();
        for (i, line) in lines.iter().enumerate() {
            if i == insert_pos {
                result.push_str(&format!("import {}\n", module));
            }
            result.push_str(line);
            result.push('\n');
        }

        // If we never inserted (empty file or insert at end)
        if insert_pos >= lines.len() {
            result.push_str(&format!("import {}\n", module));
        }

        debug!("Import added successfully");
        result
    }

    fn remove_import(&self, content: &str, module: &str) -> String {
        debug!(module = %module, "Removing import from Python content");

        let mut result = String::new();
        let mut removed = false;

        for line in content.lines() {
            let trimmed = line.trim();
            let mut skip_line = false;

            // Check for "import module" or "import module as ..."
            if trimmed.starts_with("import ") {
                let import_part = trimmed.strip_prefix("import ").unwrap_or("");
                let module_name = import_part.split(" as ").next().unwrap_or("").trim();
                if module_name == module {
                    skip_line = true;
                    removed = true;
                }
            }

            // Check for "from module import ..."
            if trimmed.starts_with("from ") {
                let from_part = trimmed.strip_prefix("from ").unwrap_or("");
                let module_name = from_part.split(" import ").next().unwrap_or("").trim();
                if module_name == module {
                    skip_line = true;
                    removed = true;
                }
            }

            if !skip_line {
                result.push_str(line);
                result.push('\n');
            }
        }

        if removed {
            debug!("Import removed successfully");
        } else {
            debug!("Import not found, content unchanged");
        }

        result
    }
}

/// Convert a file path to a Python module path
///
/// Examples:
/// - `src/foo/bar.py` -> `foo.bar`
/// - `foo/bar/__init__.py` -> `foo.bar`
/// - `example.py` -> `example`
fn path_to_python_module(path: &Path) -> String {
    // Get the path without extension
    let path_no_ext = path.with_extension("");

    // Convert path components to module path
    let components: Vec<_> = path_no_ext
        .components()
        .filter_map(|c| {
            if let std::path::Component::Normal(s) = c {
                s.to_str()
            } else {
                None
            }
        })
        .filter(|s| *s != "src") // Filter out 'src' directory
        .collect();

    // Join with dots, remove __init__ if present
    let mut module = components.join(".");
    if module.ends_with(".__init__") {
        module = module.strip_suffix(".__init__").unwrap_or(&module).to_string();
    }

    module
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_imports() {
        let support = PythonImportSupport;
        let source = r#"
import os
import sys as system
from pathlib import Path
from typing import Dict, List
"#;

        let imports = support.parse_imports(source);
        assert_eq!(imports.len(), 4);
        assert!(imports.contains(&"os".to_string()));
        assert!(imports.contains(&"sys".to_string()));
        assert!(imports.contains(&"pathlib".to_string()));
        assert!(imports.contains(&"typing".to_string()));
    }

    #[test]
    fn test_rewrite_imports_for_rename() {
        let support = PythonImportSupport;
        let source = r#"import old_module
from old_module import something
from other_module import stuff
"#;

        let (result, changes) = support.rewrite_imports_for_rename(source, "old_module", "new_module");
        assert_eq!(changes, 2);
        assert!(result.contains("import new_module"));
        assert!(result.contains("from new_module import something"));
        assert!(result.contains("from other_module import stuff"));
    }

    #[test]
    fn test_contains_import() {
        let support = PythonImportSupport;
        let source = r#"
import os
from pathlib import Path
"#;

        assert!(support.contains_import(source, "os"));
        assert!(support.contains_import(source, "pathlib"));
        assert!(!support.contains_import(source, "sys"));
    }

    #[test]
    fn test_add_import() {
        let support = PythonImportSupport;
        let source = r#"import os

def main():
    pass
"#;

        let result = support.add_import(source, "sys");
        assert!(result.contains("import sys"));
        assert!(result.contains("import os"));

        // Should not add duplicate
        let result2 = support.add_import(&result, "sys");
        let sys_count = result2.matches("import sys").count();
        assert_eq!(sys_count, 1);
    }

    #[test]
    fn test_remove_import() {
        let support = PythonImportSupport;
        let source = r#"import os
import sys
from pathlib import Path

def main():
    pass
"#;

        let result = support.remove_import(source, "sys");
        assert!(!result.contains("import sys"));
        assert!(result.contains("import os"));
        assert!(result.contains("from pathlib import Path"));
    }

    #[test]
    fn test_path_to_python_module() {
        assert_eq!(
            path_to_python_module(Path::new("src/foo/bar.py")),
            "foo.bar"
        );
        assert_eq!(
            path_to_python_module(Path::new("foo/bar/__init__.py")),
            "foo.bar"
        );
        assert_eq!(
            path_to_python_module(Path::new("example.py")),
            "example"
        );
    }

    #[test]
    fn test_add_import_with_docstring() {
        let support = PythonImportSupport;
        let source = r#"#!/usr/bin/env python3
"""Module docstring."""

def main():
    pass
"#;

        let result = support.add_import(source, "os");
        assert!(result.contains("import os"));

        // Import should be after docstring (find last occurrence of closing quotes)
        let import_pos = result.find("import os").unwrap();
        let docstring_line = result.find("\"\"\"Module docstring.\"\"\"").unwrap();
        assert!(import_pos > docstring_line);
    }
}
