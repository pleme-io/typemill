//! Import support implementation for TypeScript/JavaScript
//!
//! Provides synchronous import parsing, analysis, and rewriting capabilities
//! for TypeScript and JavaScript source code.

use crate::imports::{remove_named_import_from_line, update_import_reference_ast};
use mill_foundation::protocol::DependencyUpdate;
use mill_lang_common::import_helpers::{
    find_last_matching_line, insert_line_at, remove_lines_matching,
};
use mill_plugin_api::{
    import_support::{
        ImportAdvancedSupport, ImportMoveSupport, ImportMutationSupport, ImportParser,
        ImportRenameSupport,
    },
    path_alias_resolver::PathAliasResolver,
    PluginResult,
};
use std::path::Path;
use tracing::{debug, warn};

/// TypeScript/JavaScript import support implementation
pub struct TypeScriptImportSupport;

impl TypeScriptImportSupport {
    /// Creates a new TypeScript/JavaScript import support instance.
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
        use crate::regex_patterns::{DYNAMIC_IMPORT_RE, ES6_IMPORT_RE, REQUIRE_RE};

        // Use shared lazy regexes to parse all imports, then check if module is present
        // This is whitespace-tolerant (handles require( 'module' ), etc.)

        // Check ES6 imports: import ... from 'module'
        for caps in ES6_IMPORT_RE.captures_iter(content) {
            if let Some(imported_module) = caps.get(1) {
                if imported_module.as_str() == module {
                    return true;
                }
            }
        }

        // Check CommonJS: require('module')
        for caps in REQUIRE_RE.captures_iter(content) {
            if let Some(imported_module) = caps.get(1) {
                if imported_module.as_str() == module {
                    return true;
                }
            }
        }

        // Check dynamic imports: import('module')
        for caps in DYNAMIC_IMPORT_RE.captures_iter(content) {
            if let Some(imported_module) = caps.get(1) {
                if imported_module.as_str() == module {
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
        // Note: Path alias resolver not available in trait method context
        rewrite_imports_for_move_with_context(content, old_path, new_path, old_path, None, None)
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
    use crate::regex_patterns::{DYNAMIC_IMPORT_RE, ES6_IMPORT_RE, REQUIRE_RE};

    let mut imports = Vec::new();

    // ES6 import pattern
    for caps in ES6_IMPORT_RE.captures_iter(content) {
        if let Some(module) = caps.get(1) {
            imports.push(module.as_str().to_string());
        }
    }

    // CommonJS require pattern
    for caps in REQUIRE_RE.captures_iter(content) {
        if let Some(module) = caps.get(1) {
            imports.push(module.as_str().to_string());
        }
    }

    // Dynamic import pattern
    for caps in DYNAMIC_IMPORT_RE.captures_iter(content) {
        if let Some(module) = caps.get(1) {
            imports.push(module.as_str().to_string());
        }
    }

    imports
}

/// Rewrite imports when a file is moved, with full context
/// This is a standalone function that can be used without the ImportSupport trait
///
/// # Arguments
///
/// * `content` - The source code content to rewrite
/// * `old_path` - The original file/directory path that was moved
/// * `new_path` - The new file/directory path after the move
/// * `importing_file` - The file containing the imports to rewrite
/// * `path_alias_resolver` - Optional path alias resolver for TypeScript path mappings
/// * `project_root` - Optional project root directory (required if path_alias_resolver is provided)
///
/// # Returns
///
/// A tuple of (updated_content, number_of_changes)
pub(crate) fn rewrite_imports_for_move_with_context(
    content: &str,
    old_path: &Path,
    new_path: &Path,
    importing_file: &Path,
    path_alias_resolver: Option<&crate::path_alias_resolver::TypeScriptPathAliasResolver>,
    project_root: Option<&Path>,
) -> (String, usize) {
    let mut new_content = content.to_string();
    let mut changes = 0;

    // Step 1: Handle path alias imports if resolver is provided
    if let (Some(resolver), Some(root)) = (path_alias_resolver, project_root) {
        // Parse all imports from the content
        match crate::parser::analyze_imports(content, Some(importing_file)) {
            Ok(import_graph) => {
                for import in &import_graph.imports {
                    let specifier = &import.module_path;

                    // Check if this looks like a path alias
                    if !resolver.is_potential_alias(specifier) {
                        continue;
                    }

                    // Try to resolve the alias to an absolute path
                    if let Some(resolved_path_str) =
                        resolver.resolve_alias(specifier, importing_file, root)
                    {
                        let resolved_path = Path::new(&resolved_path_str);

                        // Check if the resolved path is inside the old_path directory
                        // (for directory moves) or equals old_path (for file moves)
                        let is_affected = if old_path.is_dir()
                            || old_path.to_string_lossy().contains("src/")
                        {
                            // Directory move: check if resolved path is inside old directory
                            resolved_path.starts_with(old_path)
                        } else {
                            // File move: check if resolved path equals old file (with or without extension)
                            resolved_path == old_path
                                || resolved_path.with_extension("") == old_path.with_extension("")
                        };

                        if !is_affected {
                            continue;
                        }

                        // Calculate the new path after the move
                        let new_resolved_path =
                            if old_path.is_dir() || old_path.to_string_lossy().contains("src/") {
                                // For directory moves, preserve the relative structure within
                                if let Ok(relative) = resolved_path.strip_prefix(old_path) {
                                    new_path.join(relative)
                                } else {
                                    continue;
                                }
                            } else {
                                // For file moves, use the new path directly
                                new_path.to_path_buf()
                            };

                        // Try to convert the new path back to an alias
                        if let Some(new_alias) =
                            resolver.path_to_alias(&new_resolved_path, importing_file, root)
                        {
                            // Replace the import in all forms (ES6, CommonJS, dynamic)
                            for quote_char in &['\'', '"'] {
                                // ES6 imports: from 'old_alias'
                                let old_str =
                                    format!("from {}{}{}", quote_char, specifier, quote_char);
                                let new_str =
                                    format!("from {}{}{}", quote_char, new_alias, quote_char);
                                if new_content.contains(&old_str) {
                                    new_content = new_content.replace(&old_str, &new_str);
                                    changes += 1;
                                    debug!(
                                        old_alias = %specifier,
                                        new_alias = %new_alias,
                                        "Rewrote path alias import"
                                    );
                                }

                                // CommonJS: require('old_alias')
                                let old_str =
                                    format!("require({}{}{})", quote_char, specifier, quote_char);
                                let new_str =
                                    format!("require({}{}{})", quote_char, new_alias, quote_char);
                                if new_content.contains(&old_str) {
                                    new_content = new_content.replace(&old_str, &new_str);
                                    changes += 1;
                                }

                                // Dynamic import: import('old_alias')
                                let old_str =
                                    format!("import({}{}{})", quote_char, specifier, quote_char);
                                let new_str =
                                    format!("import({}{}{})", quote_char, new_alias, quote_char);
                                if new_content.contains(&old_str) {
                                    new_content = new_content.replace(&old_str, &new_str);
                                    changes += 1;
                                }
                            }
                        } else {
                            // Cannot convert to alias - fall back to relative path
                            let new_relative =
                                calculate_relative_import(importing_file, &new_resolved_path);
                            for quote_char in &['\'', '"'] {
                                let old_str =
                                    format!("from {}{}{}", quote_char, specifier, quote_char);
                                let new_str =
                                    format!("from {}{}{}", quote_char, new_relative, quote_char);
                                if new_content.contains(&old_str) {
                                    new_content = new_content.replace(&old_str, &new_str);
                                    changes += 1;
                                    debug!(
                                        old_alias = %specifier,
                                        new_relative = %new_relative,
                                        "Converted path alias to relative import (no matching alias pattern)"
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to parse imports for path alias rewriting");
            }
        }
    }

    // Step 2: Handle relative path imports (existing logic)
    let old_import = calculate_relative_import(importing_file, old_path);
    let new_import = calculate_relative_import(importing_file, new_path);

    if old_import != new_import {
        // TypeScript ESM commonly uses .js extensions in imports even for .ts files
        // We need to check for imports both with and without extensions
        let import_variants = vec![
            (old_import.clone(), new_import.clone()),                    // No extension: ./utils/timer
            (format!("{}.js", old_import), format!("{}.js", new_import)), // .js extension: ./utils/timer.js
            (format!("{}.ts", old_import), format!("{}.ts", new_import)), // .ts extension (rare but possible)
        ];

        for (old_variant, new_variant) in import_variants {
            // ES6 imports: from 'old_path' or "old_path"
            for quote_char in &['\'', '"'] {
                let old_str = format!("from {}{}{}", quote_char, old_variant, quote_char);
                let new_str = format!("from {}{}{}", quote_char, new_variant, quote_char);
                if new_content.contains(&old_str) {
                    new_content = new_content.replace(&old_str, &new_str);
                    changes += 1;
                }
            }

            // CommonJS require: require('old_path') or require("old_path")
            for quote_char in &['\'', '"'] {
                let old_str = format!("require({}{}{})", quote_char, old_variant, quote_char);
                let new_str = format!("require({}{}{})", quote_char, new_variant, quote_char);
                if new_content.contains(&old_str) {
                    new_content = new_content.replace(&old_str, &new_str);
                    changes += 1;
                }
            }

            // Dynamic import: import('old_path') or import("old_path")
            for quote_char in &['\'', '"'] {
                let old_str = format!("import({}{}{})", quote_char, old_variant, quote_char);
                let new_str = format!("import({}{}{})", quote_char, new_variant, quote_char);
                if new_content.contains(&old_str) {
                    new_content = new_content.replace(&old_str, &new_str);
                    changes += 1;
                }
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

    // FIXED: Use pathdiff::diff_paths directly on raw paths first
    // This works correctly even when target_file doesn't exist yet (during planning)
    // and avoids macOS /private vs /var canonicalization mismatches
    let relative = pathdiff::diff_paths(to_file, from_dir).unwrap_or_else(|| {
        // Fallback: manually compute relative path only if diff_paths fails
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
    });

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

        let (updated, changes) = rewrite_imports_for_move_with_context(
            source,
            &old_path,
            &new_path,
            &importing_file,
            None,
            None,
        );
        assert!(
            updated.contains("from './new/path'"),
            "Expected single quotes preserved, got: {}",
            updated
        );
        assert!(updated.contains("require('./new/path')"));
        assert!(updated.contains("import('./new/path')"));
        assert!(changes > 0);
    }

    #[test]
    fn test_rewrite_path_alias_sveltekit() {
        use crate::path_alias_resolver::TypeScriptPathAliasResolver;
        use tempfile::TempDir;

        // Create temporary directory with tsconfig.json
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create tsconfig.json with $lib mapping
        let tsconfig_content = r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "$lib/*": ["src/lib/*"]
    }
  }
}"#;
        std::fs::write(project_root.join("tsconfig.json"), tsconfig_content).unwrap();

        // Create directory structure
        std::fs::create_dir_all(project_root.join("src/lib/server/core")).unwrap();
        std::fs::create_dir_all(project_root.join("src/routes")).unwrap();
        std::fs::create_dir_all(project_root.join("packages/orchestrator/src/engine")).unwrap();

        // Create source file with $lib import
        let source = r#"
import { WorkflowStateMachine } from "$lib/server/core/orchestrator/workflow";
import { orchestrator } from "$lib/server/core/orchestrator/main";

export async function load() {
    const workflow = new WorkflowStateMachine();
    return { orchestrator };
}
"#;

        let importing_file = project_root.join("src/routes/page.ts");
        std::fs::write(&importing_file, source).unwrap();

        // Simulate moving src/lib/server/core/orchestrator → packages/orchestrator/src/engine
        let old_path = project_root.join("src/lib/server/core/orchestrator");
        let new_path = project_root.join("packages/orchestrator/src/engine");

        let resolver = TypeScriptPathAliasResolver::new();

        let (updated, changes) = rewrite_imports_for_move_with_context(
            source,
            &old_path,
            &new_path,
            &importing_file,
            Some(&resolver),
            Some(project_root),
        );

        // Since packages/ is outside src/lib, it should convert to relative imports
        // or maintain the alias if a new pattern exists
        assert!(
            changes > 0,
            "Should have updated at least one import, got 0 changes"
        );

        // Verify imports were updated (will be converted to relative paths since
        // packages/ is outside the $lib/* mapping)
        println!("Updated content:\n{}", updated);
        assert!(
            !updated.contains("$lib/server/core/orchestrator"),
            "Old $lib path should be replaced"
        );
    }

    #[test]
    fn test_rewrite_path_alias_within_same_mapping() {
        use crate::path_alias_resolver::TypeScriptPathAliasResolver;
        use tempfile::TempDir;

        // Test renaming within the same alias mapping (should preserve alias)
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create tsconfig.json
        let tsconfig_content = r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "$lib/*": ["src/lib/*"]
    }
  }
}"#;
        std::fs::write(project_root.join("tsconfig.json"), tsconfig_content).unwrap();

        // Create directory structure
        std::fs::create_dir_all(project_root.join("src/lib/utils")).unwrap();
        std::fs::create_dir_all(project_root.join("src/lib/helpers")).unwrap();
        std::fs::create_dir_all(project_root.join("src/routes")).unwrap();

        let source = r#"
import { format } from "$lib/utils/formatter";
import { validate } from "$lib/utils/validator";
"#;

        let importing_file = project_root.join("src/routes/page.ts");
        std::fs::write(&importing_file, source).unwrap();

        // Rename src/lib/utils → src/lib/helpers
        let old_path = project_root.join("src/lib/utils");
        let new_path = project_root.join("src/lib/helpers");

        let resolver = TypeScriptPathAliasResolver::new();

        let (updated, changes) = rewrite_imports_for_move_with_context(
            source,
            &old_path,
            &new_path,
            &importing_file,
            Some(&resolver),
            Some(project_root),
        );

        assert!(changes >= 2, "Should have updated both imports");

        // Should preserve $lib alias with new path
        assert!(
            updated.contains("$lib/helpers/formatter") || updated.contains("$lib/helpers"),
            "Should update to new $lib path: {}",
            updated
        );
        assert!(
            !updated.contains("$lib/utils"),
            "Should not contain old $lib/utils path"
        );
    }

    #[test]
    fn test_rewrite_path_alias_nextjs_style() {
        use crate::path_alias_resolver::TypeScriptPathAliasResolver;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Next.js style @ alias
        let tsconfig_content = r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"]
    }
  }
}"#;
        std::fs::write(project_root.join("tsconfig.json"), tsconfig_content).unwrap();

        std::fs::create_dir_all(project_root.join("src/components")).unwrap();
        std::fs::create_dir_all(project_root.join("src/ui")).unwrap();
        std::fs::create_dir_all(project_root.join("src/app")).unwrap();

        let source = r#"
import { Button } from "@/components/Button";
import { Input } from "@/components/Input";
"#;

        let importing_file = project_root.join("src/app/page.tsx");
        std::fs::write(&importing_file, source).unwrap();

        // Rename src/components → src/ui
        let old_path = project_root.join("src/components");
        let new_path = project_root.join("src/ui");

        let resolver = TypeScriptPathAliasResolver::new();

        let (updated, changes) = rewrite_imports_for_move_with_context(
            source,
            &old_path,
            &new_path,
            &importing_file,
            Some(&resolver),
            Some(project_root),
        );

        assert!(changes >= 2, "Should update both imports");
        assert!(
            updated.contains("@/ui/Button") || updated.contains("@/ui"),
            "Should update to @/ui path"
        );
        assert!(
            !updated.contains("@/components"),
            "Should not contain old @/components"
        );
    }

    #[test]
    fn test_rewrite_preserves_non_alias_imports() {
        use crate::path_alias_resolver::TypeScriptPathAliasResolver;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        let tsconfig_content = r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "$lib/*": ["src/lib/*"]
    }
  }
}"#;
        std::fs::write(project_root.join("tsconfig.json"), tsconfig_content).unwrap();

        std::fs::create_dir_all(project_root.join("src/lib/utils")).unwrap();
        std::fs::create_dir_all(project_root.join("src/routes")).unwrap();

        // Mix of alias and non-alias imports
        let source = r#"
import { format } from "$lib/utils/formatter";
import React from "react";
import { helper } from "./helper";
"#;

        let importing_file = project_root.join("src/routes/page.ts");
        std::fs::write(&importing_file, source).unwrap();

        let old_path = project_root.join("src/lib/utils");
        let new_path = project_root.join("src/lib/helpers");

        let resolver = TypeScriptPathAliasResolver::new();

        let (updated, changes) = rewrite_imports_for_move_with_context(
            source,
            &old_path,
            &new_path,
            &importing_file,
            Some(&resolver),
            Some(project_root),
        );

        // Should only update the $lib import
        assert_eq!(changes, 1, "Should only update one import");
        assert!(
            updated.contains("from \"react\""),
            "Should preserve bare specifier"
        );
        assert!(
            updated.contains("from \"./helper\""),
            "Should preserve relative import"
        );
        assert!(
            updated.contains("$lib/helpers") || !updated.contains("$lib/utils"),
            "Should update $lib import"
        );
    }
}
