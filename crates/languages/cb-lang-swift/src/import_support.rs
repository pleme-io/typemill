//! Import support implementation for Swift language plugin
//!
//! This module implements the `ImportSupport` trait for Swift, providing
//! synchronous methods for parsing, analyzing, and rewriting import statements
//! using a regex-based approach as a fallback.

use cb_plugin_api::import_support::ImportSupport;
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::Path;
use tracing::debug;

// Regex to capture the full module path from an import statement.
// NOTE: This is a simplified regex and does not handle special cases like
// `import class` or attributes. A proper AST-based parser is needed.
static IMPORT_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^\s*import\s+([a-zA-Z0-9_.]+)").unwrap());

/// Swift import support implementation
pub struct SwiftImportSupport;

impl ImportSupport for SwiftImportSupport {
    fn parse_imports(&self, content: &str) -> Vec<String> {
        debug!("Parsing Swift imports from content using regex, extracting base module.");
        IMPORT_REGEX
            .captures_iter(content)
            .filter_map(|cap| {
                cap.get(1).map(|m| {
                    m.as_str().split('.').next().unwrap_or("").to_string()
                })
            })
            .collect()
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
            "Rewriting Swift imports for rename"
        );
        let import_to_find = format!("import {}", old_name);
        let import_to_replace = format!("import {}", new_name);
        let new_content = content.replace(&import_to_find, &import_to_replace);
        let changes_count = if new_content != content { 1 } else { 0 };
        (new_content, changes_count)
    }

    fn rewrite_imports_for_move(
        &self,
        content: &str,
        _old_path: &Path,
        _new_path: &Path,
    ) -> (String, usize) {
        // Swift imports are module-based, not path-based, so moving a file
        // within a module does not typically require import changes.
        debug!("File move detected, but Swift uses module-based imports (no changes needed)");
        (content.to_string(), 0)
    }

    fn contains_import(&self, content: &str, module: &str) -> bool {
        let imports = self.parse_imports(content);
        imports.iter().any(|imp| imp == module)
    }

    fn add_import(&self, content: &str, module: &str) -> String {
        if self.contains_import(content, module) {
            return content.to_string();
        }

        let new_import_line = format!("import {}", module);
        let mut lines: Vec<&str> = content.lines().collect();

        // Find the last import statement to add the new one after it.
        let last_import_line_index = lines.iter().rposition(|line| IMPORT_REGEX.is_match(line));

        if let Some(index) = last_import_line_index {
            lines.insert(index + 1, &new_import_line);
            lines.join("\n")
        } else {
            // No imports found, add it at the top.
            format!("{}\n{}", new_import_line, content)
        }
    }

    fn remove_import(&self, content: &str, module: &str) -> String {
        let lines: Vec<&str> = content
            .lines()
            .filter(|line| {
                if let Some(caps) = IMPORT_REGEX.captures(line) {
                    if let Some(m) = caps.get(1) {
                        return m.as_str() != module;
                    }
                }
                true
            })
            .collect();
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_swift_imports() {
        let support = SwiftImportSupport;
        let content = r#"
import Foundation
import SwiftUI
import UIKit.UIGestureRecognizer

class MyView: UIView {}
"#;
        let imports = support.parse_imports(content);
        assert_eq!(imports, vec!["Foundation", "SwiftUI", "UIKit"]);
    }

    #[test]
    fn test_add_swift_import() {
        let support = SwiftImportSupport;
        let content = r#"import Foundation

class MyClass {}"#;
        let new_content = support.add_import(content, "SwiftUI");
        assert!(new_content.contains("import Foundation"));
        assert!(new_content.contains("import SwiftUI"));
    }

    #[test]
    fn test_add_import_to_empty_file() {
        let support = SwiftImportSupport;
        let content = "";
        let new_content = support.add_import(content, "Foundation");
        assert_eq!(new_content, "import Foundation\n");
    }

    #[test]
    fn test_remove_swift_import() {
        let support = SwiftImportSupport;
        let content = r#"
import Foundation
import SwiftUI
import UIKit
"#;
        let new_content = support.remove_import(content, "SwiftUI");
        assert!(new_content.contains("import Foundation"));
        assert!(!new_content.contains("import SwiftUI"));
        assert!(new_content.contains("import UIKit"));
    }

    #[test]
    fn test_contains_swift_import() {
        let support = SwiftImportSupport;
        let content = "import Foundation\nimport SwiftUI";
        assert!(support.contains_import(content, "Foundation"));
        assert!(support.contains_import(content, "SwiftUI"));
        assert!(!support.contains_import(content, "UIKit"));
    }

    #[test]
    fn test_rename_swift_import() {
        let support = SwiftImportSupport;
        let content = "import OldModule";
        let (new_content, count) = support.rewrite_imports_for_rename(content, "OldModule", "NewModule");
        assert_eq!(count, 1);
        assert_eq!(new_content, "import NewModule");
    }
}