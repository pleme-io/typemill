//! TypeScript/JavaScript import parsing and symbol extraction logic.

use cb_plugin_api::{PluginError, PluginResult, Symbol, SymbolKind};
use cb_protocol::{ImportGraph, ImportInfo, ImportType, SourceLocation};
use cb_lang_common::{SubprocessAstTool, run_ast_tool, parse_with_fallback, ImportGraphBuilder};
use serde::Deserialize;
use std::path::Path;

/// Analyzes TypeScript/JavaScript source code to produce an import graph.
/// It attempts to use an AST-based approach first, falling back to regex on failure.
pub fn analyze_imports(source: &str, file_path: Option<&Path>) -> PluginResult<ImportGraph> {
    let imports = parse_with_fallback(
        || parse_imports_ast(source),
        || parse_imports_regex(source),
        "TypeScript import parsing"
    )?;

    Ok(ImportGraphBuilder::new("typescript")
        .with_source_file(file_path)
        .with_imports(imports)
        .extract_external_dependencies(|path| is_external_dependency(path))
        .with_parser_version("0.1.0-plugin")
        .build())
}

/// TypeScript import information from AST tool
#[derive(Debug, Deserialize)]
struct TsImportInfo {
    module_path: String,
    import_type: String,
    named_imports: Vec<TsNamedImport>,
    default_import: Option<String>,
    namespace_import: Option<String>,
    type_only: bool,
    location: TsLocation,
}

#[derive(Debug, Deserialize)]
struct TsNamedImport {
    name: String,
    alias: Option<String>,
    type_only: bool,
}

#[derive(Debug, Deserialize)]
struct TsLocation {
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
}

/// Spawns the bundled `ast_tool.js` script to parse imports from source.
fn parse_imports_ast(source: &str) -> Result<Vec<ImportInfo>, PluginError> {
    const AST_TOOL_JS: &str = include_str!("../resources/ast_tool.js");

    let tool = SubprocessAstTool::new("node")
        .with_embedded_str(AST_TOOL_JS)
        .with_temp_filename("ast_tool.js")
        .with_arg("analyze-imports");

    let ts_imports: Vec<TsImportInfo> = run_ast_tool(tool, source)?;

    // Convert TsImportInfo to ImportInfo
    Ok(ts_imports
        .into_iter()
        .map(|imp| ImportInfo {
            module_path: imp.module_path,
            import_type: match imp.import_type.as_str() {
                "es_module" => ImportType::EsModule,
                "commonjs" => ImportType::CommonJs,
                "dynamic" => ImportType::Dynamic,
                _ => ImportType::EsModule,
            },
            named_imports: imp
                .named_imports
                .iter()
                .map(|n| cb_protocol::NamedImport {
                    name: n.name.clone(),
                    alias: n.alias.clone(),
                    type_only: n.type_only,
                })
                .collect(),
            default_import: imp.default_import,
            namespace_import: imp.namespace_import,
            type_only: imp.type_only,
            location: SourceLocation {
                start_line: imp.location.start_line as u32,
                start_column: imp.location.start_column as u32,
                end_line: imp.location.end_line as u32,
                end_column: imp.location.end_column as u32,
            },
        })
        .collect())
}

/// Parse TypeScript/JavaScript imports using regex (fallback implementation)
fn parse_imports_regex(source: &str) -> PluginResult<Vec<ImportInfo>> {
    let mut imports = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    // ES6 import pattern
    let es6_import_re = regex::Regex::new(r#"^import\s+.*?from\s+['"]([^'"]+)['"]"#).unwrap();
    // CommonJS require pattern
    let require_re = regex::Regex::new(r#"require\s*\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap();
    // Dynamic import pattern
    let dynamic_import_re = regex::Regex::new(r#"import\s*\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap();

    for (line_idx, line) in lines.iter().enumerate() {
        let line_num = (line_idx + 1) as u32;

        // Check for ES6 import
        if let Some(caps) = es6_import_re.captures(line) {
            let module_path = caps.get(1).unwrap().as_str().to_string();
            imports.push(ImportInfo {
                module_path,
                import_type: ImportType::EsModule,
                named_imports: Vec::new(),
                default_import: None,
                namespace_import: None,
                type_only: line.contains("import type"),
                location: SourceLocation {
                    start_line: line_num,
                    start_column: 0,
                    end_line: line_num,
                    end_column: line.len() as u32,
                },
            });
        }

        // Check for require()
        if let Some(caps) = require_re.captures(line) {
            let module_path = caps.get(1).unwrap().as_str().to_string();
            let start_col = line.find("require").unwrap_or(0) as u32;
            imports.push(ImportInfo {
                module_path,
                import_type: ImportType::CommonJs,
                named_imports: Vec::new(),
                default_import: None,
                namespace_import: None,
                type_only: false,
                location: SourceLocation {
                    start_line: line_num,
                    start_column: start_col,
                    end_line: line_num,
                    end_column: line.len() as u32,
                },
            });
        }

        // Check for dynamic import()
        if let Some(caps) = dynamic_import_re.captures(line) {
            let module_path = caps.get(1).unwrap().as_str().to_string();
            let start_col = line.find("import(").unwrap_or(0) as u32;
            imports.push(ImportInfo {
                module_path,
                import_type: ImportType::Dynamic,
                named_imports: Vec::new(),
                default_import: None,
                namespace_import: None,
                type_only: false,
                location: SourceLocation {
                    start_line: line_num,
                    start_column: start_col,
                    end_line: line_num,
                    end_column: line.len() as u32,
                },
            });
        }
    }

    Ok(imports)
}

/// Check if a module path represents an external dependency
fn is_external_dependency(module_path: &str) -> bool {
    // Relative imports are internal
    if module_path.starts_with("./") || module_path.starts_with("../") {
        return false;
    }

    // Absolute paths are internal
    if module_path.starts_with('/') {
        return false;
    }

    // Node built-ins are external but special
    const NODE_BUILTINS: &[&str] = &[
        "fs",
        "path",
        "http",
        "https",
        "crypto",
        "util",
        "events",
        "stream",
        "buffer",
        "process",
        "os",
        "child_process",
        "url",
        "querystring",
    ];

    if NODE_BUILTINS.contains(&module_path) {
        return true;
    }

    // Scoped packages (@org/package) are external
    if module_path.starts_with('@') {
        return true;
    }

    // If it doesn't start with . or /, it's probably an npm package
    !module_path.contains('/') || module_path.starts_with('@')
}

// ============================================================================
// Symbol Extraction
// ============================================================================

/// TypeScript symbol information from AST tool
#[derive(Debug, Deserialize)]
struct TsSymbolInfo {
    name: String,
    kind: String,
    location: TsLocation,
    documentation: Option<String>,
}

/// Extract symbols from TypeScript/JavaScript source code using AST-based parsing.
/// Falls back to empty list if Node.js is not available.
pub fn extract_symbols(source: &str) -> PluginResult<Vec<Symbol>> {
    match extract_symbols_ast(source) {
        Ok(symbols) => Ok(symbols),
        Err(e) => {
            tracing::debug!(error = %e, "TypeScript symbol extraction failed, returning empty list");
            // Return empty list instead of failing - symbol extraction is optional
            Ok(Vec::new())
        }
    }
}

/// Spawns the bundled `ast_tool.js` script to extract symbols from source.
fn extract_symbols_ast(source: &str) -> Result<Vec<Symbol>, PluginError> {
    const AST_TOOL_JS: &str = include_str!("../resources/ast_tool.js");

    let tool = SubprocessAstTool::new("node")
        .with_embedded_str(AST_TOOL_JS)
        .with_temp_filename("ast_tool.js")
        .with_arg("extract-symbols");

    let ts_symbols: Vec<TsSymbolInfo> = run_ast_tool(tool, source)?;

    // Convert TsSymbolInfo to cb_plugin_api::Symbol
    let symbols = ts_symbols
        .into_iter()
        .map(|s| Symbol {
            name: s.name,
            kind: match s.kind.as_str() {
                "function" | "async_function" => SymbolKind::Function,
                "class" => SymbolKind::Class,
                "interface" => SymbolKind::Interface,
                "enum" => SymbolKind::Enum,
                "type_alias" => SymbolKind::Other,
                "constant" => SymbolKind::Constant,
                "variable" => SymbolKind::Variable,
                _ => SymbolKind::Other,
            },
            location: cb_plugin_api::SourceLocation {
                line: s.location.start_line,
                column: s.location.start_column,
            },
            documentation: s.documentation,
        })
        .collect();

    Ok(symbols)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_imports_regex() {
        let source = r#"
import React from 'react';
import { useState, useEffect } from 'react';
import * as Utils from './utils';
const fs = require('fs');
const path = require('path');
import('./dynamic-module');
"#;

        let imports = parse_imports_regex(source).unwrap();
        assert_eq!(imports.len(), 6);

        // Check ES6 imports
        assert!(imports
            .iter()
            .any(|i| i.module_path == "react" && matches!(i.import_type, ImportType::EsModule)));
        assert!(imports
            .iter()
            .any(|i| i.module_path == "./utils" && matches!(i.import_type, ImportType::EsModule)));

        // Check CommonJS
        assert!(imports
            .iter()
            .any(|i| i.module_path == "fs" && matches!(i.import_type, ImportType::CommonJs)));
        assert!(imports
            .iter()
            .any(|i| i.module_path == "path" && matches!(i.import_type, ImportType::CommonJs)));

        // Check dynamic
        assert!(imports
            .iter()
            .any(|i| i.module_path == "./dynamic-module"
                && matches!(i.import_type, ImportType::Dynamic)));
    }

    #[test]
    fn test_is_external_dependency() {
        assert!(is_external_dependency("react"));
        assert!(is_external_dependency("@types/node"));
        assert!(is_external_dependency("lodash"));
        assert!(is_external_dependency("fs"));

        assert!(!is_external_dependency("./local"));
        assert!(!is_external_dependency("../parent"));
        assert!(!is_external_dependency("/absolute/path"));
    }

    #[test]
    fn test_analyze_imports() {
        let source = r#"
import React from 'react';
import './styles.css';
const lodash = require('lodash');
"#;

        let graph = analyze_imports(source, None).unwrap();
        assert_eq!(graph.metadata.language, "typescript");
        assert_eq!(graph.source_file, "in-memory.ts");
        // Regex fallback may not catch all imports
        assert!(graph.imports.len() >= 2);

        // Check external dependencies
        assert!(
            graph
                .metadata
                .external_dependencies
                .contains(&"react".to_string())
                || graph
                    .metadata
                    .external_dependencies
                    .contains(&"lodash".to_string())
        );
    }

    #[test]
    fn test_extract_symbols_graceful_fallback() {
        let source = r#"
function hello() {
    console.log("Hello");
}

class MyClass {
    constructor() {}
}

interface IUser {
    name: string;
}

type UserId = string;

enum Status {
    Active,
    Inactive
}
"#;

        // This should either succeed with symbols (if Node is available)
        // or return an empty list (if Node is not available)
        let result = extract_symbols(source);
        assert!(result.is_ok(), "extract_symbols should not fail");

        let symbols = result.unwrap();
        // If symbols were extracted, verify they're correct
        if !symbols.is_empty() {
            assert!(symbols.iter().any(|s| s.name == "hello"));
            assert!(symbols.iter().any(|s| s.name == "MyClass"));
            assert!(symbols.iter().any(|s| s.name == "IUser"));
            assert!(symbols.iter().any(|s| s.name == "Status"));
        }
    }
}
