// Example: ImportSupport Trait Implementation
// Purpose: Enable import analysis and rewriting

use mill_plugin_api::{ ImportSupport , ModuleReference , ReferenceKind };
use std::path::Path;

pub struct MyLanguageImportSupport;

impl ImportSupport for MyLanguageImportSupport {
    /// Parse all import statements from source code
    fn parse_imports(&self, content: &str) -> Vec<String> {
        let mut imports = Vec::new();

        for line in content.lines() {
            let line = line.trim();

            // Match: import foo
            if line.starts_with("import ") {
                if let Some(module) = line.strip_prefix("import ") {
                    let module = module.split_whitespace().next().unwrap_or("");
                    imports.push(module.to_string());
                }
            }

            // Match: from foo import bar
            if line.starts_with("from ") {
                if let Some(rest) = line.strip_prefix("from ") {
                    if let Some(module) = rest.split_whitespace().next() {
                        imports.push(module.to_string());
                    }
                }
            }
        }

        imports
    }

    /// Rewrite imports when a file moves
    fn rewrite_imports_for_move(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
    ) -> (String, usize) {
        // Convert paths to module names
        let old_module = path_to_module(old_path);
        let new_module = path_to_module(new_path);

        self.rewrite_imports_for_rename(content, &old_module, &new_module)
    }

    /// Rewrite imports when a module is renamed
    fn rewrite_imports_for_rename(
        &self,
        content: &str,
        old_name: &str,
        new_name: &str,
    ) -> (String, usize) {
        let mut result = String::new();
        let mut count = 0;

        for line in content.lines() {
            if line.contains(&format!("import {}", old_name)) {
                result.push_str(&line.replace(old_name, new_name));
                count += 1;
            } else if line.contains(&format!("from {}", old_name)) {
                result.push_str(&line.replace(old_name, new_name));
                count += 1;
            } else {
                result.push_str(line);
            }
            result.push('\n');
        }

        (result, count)
    }

    /// Find all references to a specific module
    fn find_module_references(
        &self,
        content: &str,
        module_to_find: &str,
    ) -> Vec<ModuleReference> {
        let mut references = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            if line.contains(&format!("import {}", module_to_find))
                || line.contains(&format!("from {}", module_to_find)) {
                references.push(ModuleReference {
                    line: line_num + 1,
                    column: 0,
                    length: line.len(),
                    text: line.to_string(),
                    kind: ReferenceKind::Declaration,
                });
            }
        }

        references
    }
}

fn path_to_module(path: &Path) -> String {
    // Convert file path to module name
    path.with_extension("")
        .to_string_lossy()
        .replace('/', ".")
}