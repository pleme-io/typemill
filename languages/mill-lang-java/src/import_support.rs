//! Java import support using AST-based JavaParser tool

use mill_lang_common::subprocess::{run_ast_tool_raw, SubprocessAstTool};
use mill_plugin_api::{
    ImportAdvancedSupport, ImportMoveSupport, ImportMutationSupport, ImportParser,
    ImportRenameSupport,
};
use serde::Deserialize;
use std::path::Path;
use tracing::{debug, warn};

/// Embedded JavaParser JAR (optional - empty if Maven not available during build)
#[cfg(java_parser_jar_exists)]
const JAVA_PARSER_JAR: &[u8] =
    include_bytes!("../resources/java-parser/target/java-parser-1.0.0.jar");

#[cfg(not(java_parser_jar_exists))]
const JAVA_PARSER_JAR: &[u8] = &[];

/// Java import support implementation using AST parsing
#[derive(Default)]
pub struct JavaImportSupport;

impl JavaImportSupport {
    /// Creates a new Java import support instance.
    pub fn new() -> Self {
        Self
    }

    /// Run JavaParser command and return output
    fn run_parser_command(
        &self,
        command: &str,
        source: &str,
        args: &[&str],
    ) -> Result<String, String> {
        // Build tool configuration
        let mut tool = SubprocessAstTool::new("java")
            .with_embedded_bytes(JAVA_PARSER_JAR)
            .with_temp_filename("java-parser.jar")
            .with_temp_prefix("mill-java-parser")
            .with_arg(command);

        // Add additional args
        for arg in args {
            tool = tool.with_arg(*arg);
        }

        // Execute and return output
        run_ast_tool_raw(tool, source).map_err(|e| e.to_string())
    }
}

#[derive(Debug, Deserialize)]
struct ImportInfo {
    path: String,
    #[serde(rename = "isStatic")]
    is_static: bool,
    #[allow(dead_code)]
    #[serde(rename = "isWildcard")]
    is_wildcard: bool,
}

impl ImportParser for JavaImportSupport {
    fn parse_imports(&self, content: &str) -> Vec<String> {
        match self.run_parser_command("parse-imports", content, &[]) {
            Ok(json_output) => match serde_json::from_str::<Vec<ImportInfo>>(&json_output) {
                Ok(imports) => imports
                    .into_iter()
                    .map(|i| {
                        if i.is_static {
                            format!("static {}", i.path)
                        } else {
                            i.path
                        }
                    })
                    .collect(),
                Err(e) => {
                    warn!(error = %e, "Failed to parse imports JSON");
                    Vec::new()
                }
            },
            Err(e) => {
                warn!(error = %e, "Failed to parse imports");
                Vec::new()
            }
        }
    }

    fn contains_import(&self, content: &str, module: &str) -> bool {
        let imports = self.parse_imports(content);
        imports.iter().any(|imp| {
            // Exact match
            imp == module ||
            // Package prefix match (e.g., "com.example.utils.Helper" contains "com.example.utils")
            imp.starts_with(&format!("{}.", module)) ||
            // Subpackage match
            imp.ends_with(&format!(".{}", module)) ||
            // Wildcard match
            (imp.ends_with(".*") && module.starts_with(&imp[..imp.len()-2]))
        })
    }
}

impl ImportRenameSupport for JavaImportSupport {
    fn rewrite_imports_for_rename(
        &self,
        content: &str,
        old_name: &str,
        new_name: &str,
    ) -> (String, usize) {
        match self.run_parser_command("rewrite-imports", content, &[old_name, new_name]) {
            Ok(new_content) => {
                let changes = if new_content.trim() != content.trim() {
                    1
                } else {
                    0
                };
                (new_content, changes)
            }
            Err(e) => {
                warn!(error = %e, old_name = %old_name, new_name = %new_name,
                      "Failed to rewrite imports");
                (content.to_string(), 0)
            }
        }
    }
}

impl ImportMoveSupport for JavaImportSupport {
    fn rewrite_imports_for_move(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
    ) -> (String, usize) {
        // Convert file paths to Java package paths
        let old_package = file_path_to_package(old_path).unwrap_or_default();
        let new_package = file_path_to_package(new_path).unwrap_or_default();

        if old_package.is_empty() || new_package.is_empty() {
            debug!(old_path = %old_path.display(), new_path = %new_path.display(),
                   "Could not convert paths to packages");
            return (content.to_string(), 0);
        }

        self.rewrite_imports_for_rename(content, &old_package, &new_package)
    }
}

impl ImportAdvancedSupport for JavaImportSupport {}

impl ImportMutationSupport for JavaImportSupport {
    fn add_import(&self, content: &str, module: &str) -> String {
        if self.contains_import(content, module) {
            debug!(module = %module, "Import already exists, skipping");
            return content.to_string();
        }

        match self.run_parser_command("add-import", content, &[module]) {
            Ok(new_content) => new_content,
            Err(e) => {
                warn!(error = %e, module = %module, "Failed to add import");
                content.to_string()
            }
        }
    }

    fn remove_import(&self, content: &str, module: &str) -> String {
        match self.run_parser_command("remove-import", content, &[module]) {
            Ok(new_content) => new_content,
            Err(e) => {
                warn!(error = %e, module = %module, "Failed to remove import");
                content.to_string()
            }
        }
    }
}

/// Convert file path to Java package path
/// Example: src/main/java/com/example/Foo.java -> com.example.Foo
fn file_path_to_package(path: &Path) -> Option<String> {
    let path_str = path.to_str()?;

    // Find source root markers (try multiple patterns)
    let markers = ["src/main/java/", "src/test/java/", "src/"];

    for marker in &markers {
        if let Some(idx) = path_str.find(marker) {
            let package_part = &path_str[idx + marker.len()..];
            let package_path = package_part
                .trim_end_matches(".java")
                .replace(['/', '\\'], "."); // Windows support
            return Some(package_path);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_path_to_package() {
        let path = Path::new("src/main/java/com/example/UserService.java");
        let package = file_path_to_package(path);
        assert_eq!(package, Some("com.example.UserService".to_string()));
    }

    #[test]
    fn test_file_path_to_package_test_dir() {
        let path = Path::new("src/test/java/com/example/UserServiceTest.java");
        let package = file_path_to_package(path);
        assert_eq!(package, Some("com.example.UserServiceTest".to_string()));
    }

    #[test]
    fn test_file_path_to_package_no_standard_path() {
        let path = Path::new("com/example/Foo.java");
        let package = file_path_to_package(path);
        assert_eq!(package, None);
    }

    /// Helper to ensure Java parser JAR is built before running integration tests
    fn require_java_parser_jar() {
        #[cfg(not(java_parser_jar_exists))]
        panic!(
            "\n╔══════════════════════════════════════════════════════════════════╗\n\
             ║ Java parser JAR not found!                                       ║\n\
             ║                                                                  ║\n\
             ║ To run this test, build the JAR:                                ║\n\
             ║   cd languages/mill-lang-java/resources/java-parser             ║\n\
             ║   mvn package                                                    ║\n\
             ║                                                                  ║\n\
             ║ Then rebuild:                                                    ║\n\
             ║   cargo clean -p mill-lang-java                                 ║\n\
             ║   cargo test -p mill-lang-java                                  ║\n\
             ╚══════════════════════════════════════════════════════════════════╝\n"
        );
    }

    #[test]
    fn test_parse_imports_integration() {
        require_java_parser_jar();

        let support = JavaImportSupport::new();
        let source = r#"
package com.example;

import java.util.List;
import java.util.ArrayList;
import static org.junit.Assert.*;

public class Test {}
        "#;

        let imports = support.parse_imports(source);
        assert_eq!(imports.len(), 3);
        assert!(imports.contains(&"java.util.List".to_string()));
        assert!(imports.contains(&"java.util.ArrayList".to_string()));
        assert!(imports.contains(&"static org.junit.Assert".to_string()));
    }

    #[test]
    fn test_add_import_integration() {
        require_java_parser_jar();

        let support = JavaImportSupport::new();
        let source = r#"
package com.example;

import java.util.List;

public class Test {}
        "#;

        let result = support.add_import(source, "java.util.HashMap");
        assert!(result.contains("import java.util.HashMap"));
    }

    #[test]
    fn test_remove_import_integration() {
        require_java_parser_jar();

        let support = JavaImportSupport::new();
        let source = r#"
package com.example;

import java.util.List;
import java.util.ArrayList;

public class Test {}
        "#;

        let result = support.remove_import(source, "java.util.ArrayList");
        assert!(!result.contains("import java.util.ArrayList"));
        assert!(result.contains("import java.util.List"));
    }
}
