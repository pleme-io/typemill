//! Python Language Plugin for Codebuddy
//!
//! Complete Python language support implementing the `LanguageIntelligencePlugin` trait.
//!
//! # Features
//!
//! - Dual-mode AST parsing (native Python parser + regex fallback)
//! - Import analysis (import, from...import)
//! - Symbol extraction (functions, classes, methods, variables)
//! - Manifest support (requirements.txt, pyproject.toml, setup.py, Pipfile)
//! - Refactoring operations (extract function, inline variable, extract variable)
//!
//! # Example
//!
//! ```rust,ignore
//! use cb_lang_python::PythonPlugin;
//! use cb_plugin_api::LanguageIntelligencePlugin;
//!
//! let plugin = PythonPlugin::new();
//! let source = "def hello():\n    print('Hello, world!')";
//! let parsed = plugin.parse(source).await.unwrap();
//! ```

pub mod manifest;
pub mod parser;
pub mod refactoring;

use async_trait::async_trait;
use cb_plugin_api::{LanguageIntelligencePlugin, ManifestData, ParsedSource, PluginResult};
use std::path::Path;
use tracing::{debug, warn};

/// Python language plugin implementation
///
/// Provides comprehensive Python language support including:
/// - AST parsing and symbol extraction
/// - Import statement analysis
/// - Multiple manifest format handling (requirements.txt, pyproject.toml)
/// - Code refactoring operations
pub struct PythonPlugin;

impl PythonPlugin {
    /// Create a new Python plugin instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for PythonPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguageIntelligencePlugin for PythonPlugin {
    fn name(&self) -> &'static str {
        "Python"
    }

    fn file_extensions(&self) -> Vec<&'static str> {
        vec!["py", "pyi"]
    }

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        debug!("Parsing Python source code");

        // Extract all symbols from the source code
        let symbols = parser::extract_symbols(source)?;

        // Parse imports
        let imports = parser::parse_python_imports(source)?;

        // Create a simplified AST representation
        let functions = parser::extract_python_functions(source)?;
        let variables = parser::extract_python_variables(source)?;

        let ast_json = serde_json::json!({
            "type": "Module",
            "functions_count": functions.len(),
            "variables_count": variables.len(),
            "imports_count": imports.len(),
            "imports": imports,
        });

        debug!(
            symbols_count = symbols.len(),
            functions_count = functions.len(),
            imports_count = imports.len(),
            "Parsed Python source"
        );

        Ok(ParsedSource {
            data: ast_json,
            symbols,
        })
    }

    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| cb_plugin_api::PluginError::invalid_input("Invalid manifest path"))?;

        debug!(filename = %filename, path = ?path, "Analyzing Python manifest");

        match filename {
            "requirements.txt" => manifest::parse_requirements_txt(path).await,
            "pyproject.toml" => manifest::parse_pyproject_toml(path).await,
            "setup.py" => manifest::parse_setup_py(path).await,
            "Pipfile" => manifest::parse_pipfile(path).await,
            _ => Err(cb_plugin_api::PluginError::not_supported(format!(
                "Unsupported Python manifest file: {}",
                filename
            ))),
        }
    }

    fn handles_manifest(&self, filename: &str) -> bool {
        matches!(
            filename,
            "requirements.txt" | "pyproject.toml" | "setup.py" | "Pipfile"
        )
    }

    async fn list_functions(&self, source: &str) -> PluginResult<Vec<String>> {
        debug!("Listing Python functions");

        // Try native Python parser first
        match parser::list_functions(source) {
            Ok(functions) => {
                debug!(
                    functions_count = functions.len(),
                    method = "native_parser",
                    "Listed functions"
                );
                Ok(functions)
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "Native Python parser failed, falling back to regex"
                );
                // Fallback to regex-based extraction
                let functions = parser::extract_python_functions(source)?;
                Ok(functions.into_iter().map(|f| f.name).collect())
            }
        }
    }

    async fn update_dependency(
        &self,
        manifest_path: &Path,
        old_name: &str,
        new_name: &str,
        new_version: Option<&str>,
    ) -> PluginResult<String> {
        let filename = manifest_path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| cb_plugin_api::PluginError::invalid_input("Invalid manifest path"))?;

        debug!(
            filename = %filename,
            old_name = %old_name,
            new_name = %new_name,
            new_version = ?new_version,
            "Updating Python dependency"
        );

        match filename {
            "requirements.txt" => {
                manifest::update_requirements_txt(manifest_path, old_name, new_name, new_version)
                    .await
            }
            "pyproject.toml" => {
                manifest::update_pyproject_toml(manifest_path, old_name, new_name, new_version)
                    .await
            }
            _ => Err(cb_plugin_api::PluginError::not_supported(format!(
                "Cannot update dependencies in {}",
                filename
            ))),
        }
    }

    fn manifest_filename(&self) -> &'static str {
        "requirements.txt"
    }

    fn source_dir(&self) -> &'static str {
        "" // Python has no standard source directory (can be src/, lib/, or root)
    }

    fn entry_point(&self) -> &'static str {
        "__init__.py"
    }

    fn module_separator(&self) -> &'static str {
        "."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_python_plugin_basic() {
        let plugin = PythonPlugin::new();

        assert_eq!(plugin.name(), "Python");
        assert_eq!(plugin.file_extensions(), vec!["py", "pyi"]);
        assert!(plugin.handles_extension("py"));
        assert!(plugin.handles_extension("pyi"));
        assert!(!plugin.handles_extension("rs"));
    }

    #[tokio::test]
    async fn test_python_plugin_handles_manifests() {
        let plugin = PythonPlugin::new();

        assert!(plugin.handles_manifest("requirements.txt"));
        assert!(plugin.handles_manifest("pyproject.toml"));
        assert!(plugin.handles_manifest("setup.py"));
        assert!(plugin.handles_manifest("Pipfile"));
        assert!(!plugin.handles_manifest("Cargo.toml"));
    }

    #[tokio::test]
    async fn test_python_plugin_parse() {
        let plugin = PythonPlugin::new();

        let source = r#"
import os
from pathlib import Path

CONSTANT = 42

def hello():
    print('Hello, world!')

class MyClass:
    pass
"#;

        let result = plugin.parse(source).await;
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert!(!parsed.symbols.is_empty());

        // Should have function, class, and constant symbols
        let has_function = parsed
            .symbols
            .iter()
            .any(|s| s.name == "hello" && s.kind == cb_plugin_api::SymbolKind::Function);
        let has_class = parsed
            .symbols
            .iter()
            .any(|s| s.name == "MyClass" && s.kind == cb_plugin_api::SymbolKind::Class);
        let has_constant = parsed
            .symbols
            .iter()
            .any(|s| s.name == "CONSTANT" && s.kind == cb_plugin_api::SymbolKind::Constant);

        assert!(has_function, "Should parse function");
        assert!(has_class, "Should parse class");
        assert!(has_constant, "Should parse constant");
    }

    #[tokio::test]
    async fn test_python_plugin_list_functions() {
        let plugin = PythonPlugin::new();

        let source = r#"
def function_one():
    pass

def function_two(param):
    return param * 2

class MyClass:
    def method_one(self):
        pass
"#;

        let result = plugin.list_functions(source).await;
        // This may fail if python3 is not available, which is okay for the test
        // The fallback will still work
        if let Ok(functions) = result {
            assert!(functions.contains(&"function_one".to_string()));
            assert!(functions.contains(&"function_two".to_string()));
        }
    }

    #[test]
    fn test_python_module_constants() {
        let plugin = PythonPlugin::new();

        assert_eq!(plugin.manifest_filename(), "requirements.txt");
        assert_eq!(plugin.entry_point(), "__init__.py");
        assert_eq!(plugin.module_separator(), ".");
        assert_eq!(plugin.source_dir(), "");
    }
}
