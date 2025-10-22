//! Import support implementation for TypeScript/JavaScript
//!
//! Provides synchronous import parsing, analysis, and rewriting capabilities
//! for TypeScript and JavaScript source code.

use crate::imports::{remove_named_import_from_line, update_import_reference_ast};
use cb_lang_common::import_helpers::{
    find_last_matching_line, insert_line_at, remove_lines_matching,
};
use mill_plugin_api::{ import_support::{ ImportAdvancedSupport , ImportMoveSupport , ImportMutationSupport , ImportParser , ImportRenameSupport , } , PluginResult , };
use mill_foundation::protocol::DependencyUpdate;
use std::path::Path;
use tracing::{debug, warn};

/// TypeScript/JavaScript import support implementation
pub struct TypeScriptImportSupport;

impl TypeScriptImportSupport {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TypeScriptImportSupport {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Segregated Trait Implementations
// ============================================================================

impl ImportParser for TypeScriptImportSupport {
    fn parse_imports(&self, content: &str) -> Vec<String> {
        // Use the existing parser's analyze_imports function
        match crate::parser::analyze_imports(content, None) {
            Ok(graph) => graph
                .imports
                .into_iter()
                .map(|imp| imp.module_path)
                .collect(),
            Err(e) => {
                warn!(error = %e, "Failed to parse imports, falling back to regex");
                // Fallback to simple regex parsing
                parse_imports_simple(content)
            }
        }
    }

    fn contains_import(&self, content: &str, module: &str) -> bool {
        // Check for various import patterns
        let patterns = [
            format!(r#"from\s+['"]{module}['"]"#, module = regex::escape(module)),
            format!(
                r#"require\s*\(\s*['"]{module}['"]\s*\)"#,
                module = regex::escape(module)
            ),
            format!(
                r#"import\s*\(\s*['"]{module}['"]\s*\)"#,
                module = regex::escape(module)
            ),
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if re.is_match(content) {
                    return true;
                }
            }
        }

        false
    }
}

impl ImportRenameSupport for TypeScriptImportSupport {
    fn rewrite_imports_for_rename(
        &self,
        content: &str,
        old_name: &str,
        new_name: &str,
    ) -> (String, usize) {
        // In TypeScript, we're renaming symbols (e.g., function names, class names)
        // This affects named imports and their usage
        let mut new_content = content.to_string();
        let mut changes = 0;

        // Pattern 1: Named imports - import { oldName } from '...'
        let named_import_pattern = format!(r"\{{\s*{}\s*\}}", regex::escape(old_name));
        if let Ok(re) = regex::Regex::new(&named_import_pattern) {
            let replaced = re.replace_all(&new_content, format!("{{ {} }}", new_name));
            if replaced != new_content {
                new_content = replaced.to_string();
                changes += 1;
            }
        }

        // Pattern 2: Named imports with alias - import { oldName as alias } from '...'
        let named_alias_pattern = format!(r"{}\s+as\s+", regex::escape(old_name));
        if let Ok(re) = regex::Regex::new(&named_alias_pattern) {
            let replaced = re.replace_all(&new_content, format!("{} as ", new_name));
            if replaced != new_content {
                new_content = replaced.to_string();
                changes += 1;
            }
        }

        // Pattern 3: Default imports - import oldName from '...'
        let default_import_pattern = format!(r"import\s+{}\s+from", regex::escape(old_name));
        if let Ok(re) = regex::Regex::new(&default_import_pattern) {
            let replaced = re.replace_all(&new_content, format!("import {} from", new_name));
            if replaced != new_content {
                new_content = replaced.to_string();
                changes += 1;
            }
        }

        (new_content, changes)
    }
}

impl ImportMoveSupport for TypeScriptImportSupport {
    fn rewrite_imports_for_move(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
    ) -> (String, usize) {
        // Simplified wrapper - delegates to context-aware version
        // Uses old_path as default importing_file location
        rewrite_imports_for_move_with_context(content, old_path, new_path, old_path)
    }
}

impl ImportMutationSupport for TypeScriptImportSupport {
    fn add_import(&self, content: &str, module: &str) -> String {
        // Don't add if already exists
        if self.contains_import(content, module) {
            debug!(module = %module, "Import already exists, skipping");
            return content.to_string();
        }

        // Find the last import statement using primitive
        let last_import_idx = find_last_matching_line(content, |line| {
            let trimmed = line.trim();
            trimmed.starts_with("import ")
                || (trimmed.starts_with("const ") && trimmed.contains("require("))
        });

        let new_import = format!("import {{ }} from '{}';", module);

        match last_import_idx {
            Some(idx) => {
                // Insert after the last import using primitive
                insert_line_at(content, idx + 1, &new_import)
            }
            None => {
                // No imports found, add at the beginning
                format!("{}\n{}", new_import, content)
            }
        }
    }

    fn remove_import(&self, content: &str, module: &str) -> String {
        // Use primitive to remove all lines that import the specified module
        let (new_content, _count) = remove_lines_matching(content, |line| {
            // Check if this line imports the specified module
            if let Some(_pos) = line.find(module) {
                // Verify it's actually an import statement
                let trimmed = line.trim();
                (trimmed.starts_with("import ") && trimmed.contains(&format!("'{}'", module)))
                    || (trimmed.starts_with("import ")
                        && trimmed.contains(&format!("\"{}\"", module)))
                    || (trimmed.contains("require(") && trimmed.contains(&format!("'{}'", module)))
                    || (trimmed.contains("require(")
                        && trimmed.contains(&format!("\"{}\"", module)))
            } else {
                false
            }
        });

        new_content
    }

    fn remove_named_import(&self, line: &str, import_name: &str) -> PluginResult<String> {
        remove_named_import_from_line(line, import_name)
    }
}

impl ImportAdvancedSupport for TypeScriptImportSupport {
    fn update_import_reference(
        &self,
        file_path: &Path,
        content: &str,
        update: &DependencyUpdate,
    ) -> PluginResult<String> {
        update_import_reference_ast(file_path, content, update)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Simple regex-based import parsing (fallback)
fn parse_imports_simple(content: &str) -> Vec<String> {
    let mut imports = Vec::new();

    // ES6 import pattern
    if let Ok(es6_re) = regex::Regex::new(r#"import\s+.*?from\s+['"]([^'"]+)['"]"#) {
        for caps in es6_re.captures_iter(content) {
            if let Some(module) = caps.get(1) {
                imports.push(module.as_str().to_string());
            }
        }
    }

    // CommonJS require pattern
    if let Ok(require_re) = regex::Regex::new(r#"require\s*\(\s*['"]([^'"]+)['"]\s*\)"#) {
        for caps in require_re.captures_iter(content) {
            if let Some(module) = caps.get(1) {
                imports.push(module.as_str().to_string());
            }
        }
    }

    // Dynamic import pattern
    if let Ok(dynamic_re) = regex::Regex::new(r#"import\s*\(\s*['"]([^'"]+)['"]\s*\)"#) {
        for caps in dynamic_re.captures_iter(content) {
            if let Some(module) = caps.get(1) {
                imports.push(module.as_str().to_string());
            }
        }
    }

    imports
}

/// Rewrite imports when a file is moved, with full context
/// This is a standalone function that can be used without the ImportSupport trait
pub fn rewrite_imports_for_move_with_context(
    content: &str,
    old_path: &Path,
    new_path: &Path,
    importing_file: &Path,
) -> (String, usize) {
    // Calculate relative import paths FROM the importing file
    let old_import = calculate_relative_import(importing_file, old_path);
    let new_import = calculate_relative_import(importing_file, new_path);

    if old_import == new_import {
        return (content.to_string(), 0);
    }

    let mut new_content = content.to_string();
    let mut changes = 0;

    // ES6 imports: from 'old_path' or "old_path"
    // Preserve the original quote style
    for quote_char in &['\'', '"'] {
        let es6_pattern = format!(
            r#"from\s+{}{}{}"#,
            quote_char,
            regex::escape(&old_import),
            quote_char
        );
        if let Ok(re) = regex::Regex::new(&es6_pattern) {
            let replacement = format!(r#"from {}{}{}"#, quote_char, new_import, quote_char);
            let replaced = re.replace_all(&new_content, replacement.as_str());
            if replaced != new_content {
                new_content = replaced.to_string();
                changes += 1;
            }
        }
    }

    // CommonJS require: require('old_path') or require("old_path")
    // Preserve the original quote style
    for quote_char in &['\'', '"'] {
        let require_pattern = format!(
            r#"require\s*\(\s*{}{}{}\s*\)"#,
            quote_char,
            regex::escape(&old_import),
            quote_char
        );
        if let Ok(re) = regex::Regex::new(&require_pattern) {
            let replacement = format!(r#"require({}{}{})"#, quote_char, new_import, quote_char);
            let replaced = re.replace_all(&new_content, replacement.as_str());
            if replaced != new_content {
                new_content = replaced.to_string();
                changes += 1;
            }
        }
    }

    // Dynamic import: import('old_path') or import("old_path")
    // Preserve the original quote style
    for quote_char in &['\'', '"'] {
        let dynamic_pattern = format!(
            r#"import\s*\(\s*{}{}{}\s*\)"#,
            quote_char,
            regex::escape(&old_import),
            quote_char
        );
        if let Ok(re) = regex::Regex::new(&dynamic_pattern) {
            let replacement = format!(r#"import({}{}{})"#, quote_char, new_import, quote_char);
            let replaced = re.replace_all(&new_content, replacement.as_str());
            if replaced != new_content {
                new_content = replaced.to_string();
                changes += 1;
            }
        }
    }

    (new_content, changes)
}

/// Convert a file path to an import string
/// Calculate relative import path from importing_file to target_file
/// Returns a string like "./helper" or "../utils/helper"
fn calculate_relative_import(importing_file: &Path, target_file: &Path) -> String {
    // Get parent directories
    let from_dir = importing_file.parent().unwrap_or(Path::new(""));
    let to_file = target_file;

    // Try to compute relative path
    let relative = if let (Ok(from), Ok(to)) = (from_dir.canonicalize(), to_file.canonicalize()) {
        pathdiff::diff_paths(to, from).unwrap_or_else(|| to_file.to_path_buf())
    } else {
        // Fallback: manually compute relative path
        let from_components: Vec<_> = from_dir.components().collect();
        let to_components: Vec<_> = to_file.components().collect();

        // Find common prefix
        let mut common = 0;
        for (a, b) in from_components.iter().zip(to_components.iter()) {
            if a == b {
                common += 1;
            } else {
                break;
            }
        }

        // Build relative path
        let mut result = std::path::PathBuf::new();

        // Add ../ for each directory we need to go up
        for _ in common..from_components.len() {
            result.push("..");
        }

        // Add the remaining path components from target
        for component in &to_components[common..] {
            result.push(component);
        }

        if result.as_os_str().is_empty() {
            to_file.to_path_buf()
        } else {
            result
        }
    };

    // Convert to string and remove file extension
    let mut import_str = relative.to_string_lossy().to_string();

    // Remove common file extensions
    for ext in &[".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"] {
        if import_str.ends_with(ext) {
            import_str = import_str[..import_str.len() - ext.len()].to_string();
            break;
        }
    }

    // Ensure it starts with ./ if it's a relative path in the same directory
    if !import_str.starts_with("./")
        && !import_str.starts_with("../")
        && !import_str.starts_with('/')
    {
        import_str = format!("./{}", import_str);
    }

    // Normalize path separators to forward slashes for imports
    import_str.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_imports() {
        let support = TypeScriptImportSupport::new();
        let source = r#"
import React from 'react';
import { useState, useEffect } from 'react';
import * as Utils from './utils';
const fs = require('fs');
const path = require('path');
"#;

        let imports = ImportParser::parse_imports(&support, source);
        assert!(imports.contains(&"react".to_string()));
        assert!(imports.contains(&"./utils".to_string()));
        assert!(imports.contains(&"fs".to_string()));
        assert!(imports.contains(&"path".to_string()));
    }

    #[test]
    fn test_contains_import() {
        let support = TypeScriptImportSupport::new();
        let source = r#"
import React from 'react';
const fs = require('fs');
"#;

        assert!(ImportParser::contains_import(&support, source, "react"));
        assert!(ImportParser::contains_import(&support, source, "fs"));
        assert!(!ImportParser::contains_import(&support, source, "lodash"));
    }

    #[test]
    fn test_add_import() {
        let support = TypeScriptImportSupport::new();
        let source = r#"import React from 'react';

function App() {
    return <div>Hello</div>;
}
"#;

        let updated = ImportMutationSupport::add_import(&support, source, "lodash");
        assert!(updated.contains("import { } from 'lodash';"));
        assert!(updated.contains("import React from 'react';"));
    }

    #[test]
    fn test_remove_import() {
        let support = TypeScriptImportSupport::new();
        let source = r#"import React from 'react';
import { useState } from 'react';
const fs = require('fs');

function App() {
    return <div>Hello</div>;
}
"#;

        let updated = ImportMutationSupport::remove_import(&support, source, "react");
        assert!(!updated.contains("import React from 'react';"));
        assert!(!updated.contains("import { useState } from 'react';"));
        assert!(updated.contains("const fs = require('fs');"));
    }

    #[test]
    fn test_rewrite_imports_for_rename() {
        let support = TypeScriptImportSupport::new();
        let source = r#"import { oldFunction } from './utils';
import oldFunction from './utils';
import { oldFunction as alias } from './utils';
"#;

        let (updated, changes) = ImportRenameSupport::rewrite_imports_for_rename(
            &support,
            source,
            "oldFunction",
            "newFunction",
        );
        assert!(updated.contains("{ newFunction }"));
        assert!(updated.contains("import newFunction from"));
        assert!(updated.contains("newFunction as alias"));
        assert!(changes > 0);
    }

    #[test]
    fn test_rewrite_imports_for_move() {
        use std::path::PathBuf;

        let source = r#"import { foo } from './old/path';
const bar = require('./old/path');
import('./old/path');
"#;

        let workspace = PathBuf::from("workspace");
        let old_path = workspace.join("old").join("path.ts");
        let new_path = workspace.join("new").join("path.ts");
        let importing_file = workspace.join("main.ts");

        let (updated, changes) =
            rewrite_imports_for_move_with_context(source, &old_path, &new_path, &importing_file);
        assert!(
            updated.contains("from './new/path'"),
            "Expected single quotes preserved, got: {}",
            updated
        );
        assert!(updated.contains("require('./new/path')"));
        assert!(updated.contains("import('./new/path')"));
        assert!(changes > 0);
    }
}