//! Module reference scanner for Java source code
//!
//! This module provides functionality for scanning Java source files
//! to find references to specific modules/packages in import statements
//! and qualified paths.

use mill_plugin_api::{
    ModuleReference, ModuleReferenceScanner, PluginResult, ReferenceKind, ScanScope,
};
use tracing::debug;

/// Java module reference scanner
#[derive(Default, Clone)]
pub struct JavaModuleReferenceScanner;

impl ModuleReferenceScanner for JavaModuleReferenceScanner {
    fn scan_references(
        &self,
        content: &str,
        module_name: &str,
        scope: ScanScope,
    ) -> PluginResult<Vec<ModuleReference>> {
        debug!(
            module_name = %module_name,
            scope = ?scope,
            content_length = content.len(),
            "Scanning Java source for module references"
        );

        let mut references = Vec::new();

        // Scan line by line
        for (line_idx, line) in content.lines().enumerate() {
            let line_number = line_idx + 1; // 1-indexed

            // Skip comments
            if is_comment_line(line) {
                continue;
            }

            // Remove string literals to avoid false matches
            let line_no_strings = remove_string_literals(line);

            // Track if this is an import line to avoid double-counting
            let is_import_line = line_no_strings.trim().starts_with("import ");

            // Look for import statements
            if is_import_line {
                if let Some(module_ref) =
                    scan_import_statement(&line_no_strings, module_name, line_number)
                {
                    references.push(module_ref);
                }
            }

            // Look for qualified paths in code (if scope allows)
            // Skip qualified path scanning for import lines to avoid duplicates
            if !is_import_line && matches!(scope, ScanScope::QualifiedPaths | ScanScope::All) {
                references.extend(scan_qualified_paths(
                    &line_no_strings,
                    module_name,
                    line_number,
                ));
            }
        }

        debug!(
            references_found = references.len(),
            "Completed Java module reference scan"
        );

        Ok(references)
    }
}

/// Check if a line is a comment line
fn is_comment_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*')
}

/// Remove string literals from a line to avoid false matches
fn remove_string_literals(line: &str) -> String {
    let mut result = String::new();
    let mut in_string = false;
    let mut in_char = false;
    let mut escape = false;
    let chars = line.chars();

    for ch in chars {
        if escape {
            escape = false;
            if !in_string && !in_char {
                result.push(ch);
            }
            continue;
        }

        match ch {
            '\\' => {
                escape = true;
                if !in_string && !in_char {
                    result.push(ch);
                }
            }
            '"' => {
                if !in_char {
                    in_string = !in_string;
                    result.push(' '); // Replace with space
                }
            }
            '\'' => {
                if !in_string {
                    in_char = !in_char;
                    result.push(' '); // Replace with space
                }
            }
            _ => {
                if !in_string && !in_char {
                    result.push(ch);
                }
            }
        }
    }

    result
}

/// Scan an import statement for module references
fn scan_import_statement(
    line: &str,
    module_name: &str,
    line_number: usize,
) -> Option<ModuleReference> {
    let trimmed = line.trim();

    // Remove "import " prefix
    let import_part = trimmed.strip_prefix("import ")?.trim();

    // Remove "static " if present
    let import_part = import_part
        .strip_prefix("static ")
        .unwrap_or(import_part)
        .trim();

    // Remove trailing semicolon
    let import_path = import_part.strip_suffix(';').unwrap_or(import_part).trim();

    // Check if this import references the module
    // Match patterns:
    // - import com.example.Module;
    // - import com.example.*;
    // - import static com.example.Module.method;
    if import_path.contains(module_name) {
        // Find the position of module_name in the original line
        if let Some(start) = line.find(module_name) {
            return Some(ModuleReference {
                line: line_number,
                column: start,
                length: module_name.len(),
                text: module_name.to_string(),
                kind: ReferenceKind::Declaration,
            });
        }
    }

    None
}

/// Scan for qualified paths in code (e.g., Module.method())
fn scan_qualified_paths(line: &str, module_name: &str, line_number: usize) -> Vec<ModuleReference> {
    let mut references = Vec::new();

    // Look for patterns like "module_name." or "module_name::"
    // In Java, qualified paths use dots: Module.method()
    let mut search_pos = 0;
    while let Some(pos) = line[search_pos..].find(module_name) {
        let absolute_pos = search_pos + pos;

        // Check if this is a qualified path (followed by . or::)
        let next_pos = absolute_pos + module_name.len();
        if next_pos < line.len() {
            let next_char = line.chars().nth(next_pos);
            if next_char == Some('.')
                || (next_pos + 1 < line.len() && &line[next_pos..next_pos + 2] == "::")
            {
                // Make sure it's not part of a string or comment
                // (already handled by remove_string_literals, but double-check)
                references.push(ModuleReference {
                    line: line_number,
                    column: absolute_pos,
                    length: module_name.len(),
                    text: module_name.to_string(),
                    kind: ReferenceKind::QualifiedPath,
                });
            }
        }

        search_pos = absolute_pos + 1;
    }

    references
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_single_import() {
        let source = r#"
package com.example;

import com.example.utils.Helper;
import java.util.List;
"#;
        let scanner = JavaModuleReferenceScanner;
        let refs = scanner
            .scan_references(source, "utils", ScanScope::TopLevelOnly)
            .unwrap();

        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].line, 4);
        assert_eq!(refs[0].text, "utils");
        assert_eq!(refs[0].kind, ReferenceKind::Declaration);
    }

    #[test]
    fn test_scan_wildcard_import() {
        let source = r#"
import com.example.utils.*;
"#;
        let scanner = JavaModuleReferenceScanner;
        let refs = scanner
            .scan_references(source, "utils", ScanScope::TopLevelOnly)
            .unwrap();

        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].line, 2);
        assert_eq!(refs[0].text, "utils");
    }

    #[test]
    fn test_scan_static_import() {
        let source = r#"
import static com.example.Helper.doSomething;
"#;
        let scanner = JavaModuleReferenceScanner;
        let refs = scanner
            .scan_references(source, "Helper", ScanScope::TopLevelOnly)
            .unwrap();

        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].line, 2);
        assert_eq!(refs[0].text, "Helper");
    }

    #[test]
    fn test_exclude_comments() {
        let source = r#"
// import com.example.utils.Helper;
/* import com.example.utils.Test; */
import java.util.List;
"#;
        let scanner = JavaModuleReferenceScanner;
        let refs = scanner
            .scan_references(source, "utils", ScanScope::TopLevelOnly)
            .unwrap();

        assert_eq!(refs.len(), 0, "Comments should be ignored");
    }

    #[test]
    fn test_exclude_strings() {
        let source = r#"
import java.util.List;
String path = "com.example.utils.Helper";
"#;
        let scanner = JavaModuleReferenceScanner;
        let refs = scanner
            .scan_references(source, "utils", ScanScope::TopLevelOnly)
            .unwrap();

        assert_eq!(refs.len(), 0, "String literals should be ignored");
    }

    #[test]
    fn test_qualified_path_references() {
        let source = r#"
import com.example.Helper;

public class Main {
    public void test() {
        Helper.doSomething();
    }
}
"#;
        let scanner = JavaModuleReferenceScanner;
        let refs = scanner
            .scan_references(source, "Helper", ScanScope::QualifiedPaths)
            .unwrap();

        assert_eq!(refs.len(), 2); // import + qualified path
        assert_eq!(refs[0].kind, ReferenceKind::Declaration);
        assert_eq!(refs[1].kind, ReferenceKind::QualifiedPath);
        assert_eq!(refs[1].line, 6);
    }

    #[test]
    fn test_multiple_references() {
        let source = r#"
import com.example.utils.Helper;
import com.example.utils.Another;
import java.util.List;
"#;
        let scanner = JavaModuleReferenceScanner;
        let refs = scanner
            .scan_references(source, "utils", ScanScope::TopLevelOnly)
            .unwrap();

        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].line, 2);
        assert_eq!(refs[1].line, 3);
    }

    #[test]
    fn test_no_matches() {
        let source = r#"
import java.util.List;
import java.io.File;
"#;
        let scanner = JavaModuleReferenceScanner;
        let refs = scanner
            .scan_references(source, "utils", ScanScope::TopLevelOnly)
            .unwrap();

        assert_eq!(refs.len(), 0);
    }

    #[test]
    fn test_remove_string_literals() {
        let line = r#"String s = "hello"; int x = 5;"#;
        let result = remove_string_literals(line);
        assert!(!result.contains("hello"));
        assert!(result.contains("int x = 5"));
    }
}