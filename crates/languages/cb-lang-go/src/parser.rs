//! Go import parsing and symbol extraction logic for the cb-lang-go plugin.

use cb_plugin_api::{PluginError, PluginResult, Symbol, SymbolKind};
use cb_protocol::{ImportGraph, ImportGraphMetadata, ImportInfo, ImportType, SourceLocation};
use cb_lang_common::{SubprocessAstTool, run_ast_tool, parse_with_fallback, ImportGraphBuilder};
use serde::Deserialize;
use std::path::Path;

/// Analyzes Go source code to produce an import graph.
/// It attempts to use an AST-based approach first, falling back to regex on failure.
pub fn analyze_imports(source: &str, file_path: Option<&Path>) -> PluginResult<ImportGraph> {
    let imports = parse_with_fallback(
        || parse_go_imports_ast(source),
        || parse_go_imports_regex(source),
        "Go import parsing"
    )?;

    Ok(ImportGraphBuilder::new("go")
        .with_source_file(file_path)
        .with_imports(imports)
        .extract_external_dependencies(|path| is_external_dependency(path))
        .with_parser_version("0.1.0-plugin")
        .build())
}

/// Spawns the bundled `ast_tool.go` script to parse imports from source.
fn parse_go_imports_ast(source: &str) -> Result<Vec<ImportInfo>, PluginError> {
    const AST_TOOL_GO: &str = include_str!("../resources/ast_tool.go");

    let tool = SubprocessAstTool::builder()
        .command("go")
        .args(vec!["run".to_string()])
        .script_content(AST_TOOL_GO)
        .script_extension("go")
        .script_args(vec!["analyze-imports".to_string()])
        .prefix("codebuddy-go-ast")
        .build()?;

    run_ast_tool(tool, source)
}

/// Parse Go imports using regex (fallback implementation)
fn parse_go_imports_regex(source: &str) -> PluginResult<Vec<ImportInfo>> {
    let mut imports = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        if line.starts_with("//") || line.starts_with("/*") || line.is_empty() {
            i += 1;
            continue;
        }

        if line.starts_with("import ") && line.contains('"') && !line.contains("(") {
            if let Some(import_info) = parse_go_single_import(line, i as u32)? {
                imports.push(import_info);
            }
            i += 1;
        } else if line.starts_with("import (") || line == "import (" {
            i += 1;
            while i < lines.len() {
                let block_line = lines[i].trim();
                if block_line == ")" || block_line.starts_with(")") {
                    i += 1;
                    break;
                }
                if block_line.contains('"') && !block_line.is_empty() {
                    if let Some(import_info) = parse_go_block_import(block_line, i as u32)? {
                        imports.push(import_info);
                    }
                }
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    Ok(imports)
}

/// Parse a single Go import statement
fn parse_go_single_import(line: &str, line_num: u32) -> PluginResult<Option<ImportInfo>> {
    let import_part = &line[6..];
    let import_part = import_part.trim();

    if let Some(start_quote) = import_part.find('"') {
        if let Some(end_quote) = import_part[start_quote + 1..].find('"') {
            let package_path = &import_part[start_quote + 1..start_quote + 1 + end_quote];
            let alias = if start_quote > 0 {
                let alias_part = import_part[..start_quote].trim();
                if alias_part == "." {
                    Some(".".to_string())
                } else if alias_part == "_" {
                    Some("_".to_string())
                } else if !alias_part.is_empty() {
                    Some(alias_part.to_string())
                } else {
                    None
                }
            } else {
                None
            };

            return Ok(Some(ImportInfo {
                module_path: package_path.to_string(),
                import_type: ImportType::EsModule,
                named_imports: Vec::new(),
                default_import: alias.clone(),
                namespace_import: if alias.is_some() {
                    None
                } else {
                    Some(
                        package_path
                            .split('/')
                            .next_back()
                            .unwrap_or(package_path)
                            .to_string(),
                    )
                },
                type_only: false,
                location: SourceLocation {
                    start_line: line_num,
                    start_column: 0,
                    end_line: line_num,
                    end_column: line.len() as u32,
                },
            }));
        }
    }
    Ok(None)
}

/// Parse Go import from within an import block
fn parse_go_block_import(line: &str, line_num: u32) -> PluginResult<Option<ImportInfo>> {
    let line = line.trim();

    if let Some(start_quote) = line.find('"') {
        if let Some(end_quote) = line[start_quote + 1..].find('"') {
            let package_path = &line[start_quote + 1..start_quote + 1 + end_quote];
            let alias = if start_quote > 0 {
                let alias_part = line[..start_quote].trim();
                if alias_part == "." {
                    Some(".".to_string())
                } else if alias_part == "_" {
                    Some("_".to_string())
                } else if !alias_part.is_empty() {
                    Some(alias_part.to_string())
                } else {
                    None
                }
            } else {
                None
            };

            return Ok(Some(ImportInfo {
                module_path: package_path.to_string(),
                import_type: ImportType::EsModule,
                named_imports: Vec::new(),
                default_import: alias.clone(),
                namespace_import: if alias.is_some() {
                    None
                } else {
                    Some(
                        package_path
                            .split('/')
                            .next_back()
                            .unwrap_or(package_path)
                            .to_string(),
                    )
                },
                type_only: false,
                location: SourceLocation {
                    start_line: line_num,
                    start_column: 0,
                    end_line: line_num,
                    end_column: line.len() as u32,
                },
            }));
        }
    }
    Ok(None)
}

/// Check if a module path represents an external dependency
fn is_external_dependency(module_path: &str) -> bool {
    // Go external dependencies typically have domain names (e.g., github.com/...)
    // Internal/relative imports would be relative to the current module
    if module_path.starts_with("./") || module_path.starts_with("../") {
        return false;
    }

    // Standard library packages don't have domain names
    // External packages typically have at least one slash and a domain
    module_path.contains('.') || module_path.contains('/')
}

// ============================================================================
// Symbol Extraction
// ============================================================================

/// Go symbol information from AST tool
#[derive(Debug, Deserialize)]
struct GoSymbolInfo {
    name: String,
    kind: String,
    location: GoLocation,
    documentation: Option<String>,
    #[allow(dead_code)]
    receiver: Option<String>,
}

/// Location information from Go AST tool
#[derive(Debug, Deserialize)]
struct GoLocation {
    start_line: usize,
    start_column: usize,
    #[allow(dead_code)]
    end_line: usize,
    #[allow(dead_code)]
    end_column: usize,
}

/// Extract symbols from Go source code using AST-based parsing.
/// Falls back to empty list if Go is not available.
pub fn extract_symbols(source: &str) -> PluginResult<Vec<Symbol>> {
    match extract_symbols_ast(source) {
        Ok(symbols) => Ok(symbols),
        Err(e) => {
            tracing::debug!(error = %e, "Go symbol extraction failed, returning empty list");
            // Return empty list instead of failing - symbol extraction is optional
            Ok(Vec::new())
        }
    }
}

/// Spawns the bundled `ast_tool.go` script to extract symbols from source.
fn extract_symbols_ast(source: &str) -> Result<Vec<Symbol>, PluginError> {
    const AST_TOOL_GO: &str = include_str!("../resources/ast_tool.go");

    let tmp_dir = Builder::new()
        .prefix("codebuddy-go-ast")
        .tempdir()
        .map_err(|e| PluginError::internal(format!("Failed to create temp dir: {}", e)))?;
    let tool_path = tmp_dir.path().join("ast_tool.go");
    std::fs::write(&tool_path, AST_TOOL_GO).map_err(|e| {
        PluginError::internal(format!("Failed to write Go tool to temp file: {}", e))
    })?;

    let mut child = Command::new("go")
        .arg("run")
        .arg(&tool_path)
        .arg("extract-symbols")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| PluginError::parse(format!("Failed to spawn Go AST tool: {}", e)))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(source.as_bytes()).map_err(|e| {
            PluginError::parse(format!("Failed to write to Go AST tool stdin: {}", e))
        })?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| PluginError::parse(format!("Failed to wait for Go AST tool: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PluginError::parse(format!(
            "Go AST tool failed: {}",
            stderr
        )));
    }

    let go_symbols: Vec<GoSymbolInfo> = serde_json::from_slice(&output.stdout)
        .map_err(|e| PluginError::parse(format!("Failed to parse Go AST tool output: {}", e)))?;

    // Convert GoSymbolInfo to cb_plugin_api::Symbol
    let symbols = go_symbols
        .into_iter()
        .map(|s| Symbol {
            name: s.name,
            kind: match s.kind.as_str() {
                "function" => SymbolKind::Function,
                "method" => SymbolKind::Method,
                "struct" => SymbolKind::Struct,
                "interface" => SymbolKind::Interface,
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
    fn test_parse_go_imports_regex() {
        let source = r#"package main

import "fmt"
import alias "github.com/user/repo"
import (
    "os"
    "path/filepath"
    . "net/http"
    _ "database/sql/driver"
    json "encoding/json"
    "github.com/external/lib"
)

func main() {
    fmt.Println("Hello")
}"#;
        let imports = parse_go_imports_regex(source).unwrap();

        assert_eq!(imports.len(), 8);
        assert_eq!(imports[0].module_path, "fmt");
        assert_eq!(imports[0].namespace_import, Some("fmt".to_string()));
        assert_eq!(imports[0].default_import, None);

        assert_eq!(imports[1].module_path, "github.com/user/repo");
        assert_eq!(imports[1].default_import, Some("alias".to_string()));
        assert_eq!(imports[1].namespace_import, None);

        assert_eq!(imports[2].module_path, "os");
        assert_eq!(imports[2].namespace_import, Some("os".to_string()));

        assert_eq!(imports[3].module_path, "path/filepath");
        assert_eq!(imports[3].namespace_import, Some("filepath".to_string()));

        assert_eq!(imports[4].module_path, "net/http");
        assert_eq!(imports[4].default_import, Some(".".to_string()));

        assert_eq!(imports[5].module_path, "database/sql/driver");
        assert_eq!(imports[5].default_import, Some("_".to_string()));

        assert_eq!(imports[6].module_path, "encoding/json");
        assert_eq!(imports[6].default_import, Some("json".to_string()));

        assert_eq!(imports[7].module_path, "github.com/external/lib");
        assert_eq!(imports[7].namespace_import, Some("lib".to_string()));
    }

    #[test]
    fn test_is_external_dependency() {
        assert!(is_external_dependency("github.com/user/repo"));
        assert!(is_external_dependency("golang.org/x/tools"));
        assert!(!is_external_dependency("./local"));
        assert!(!is_external_dependency("../parent"));
    }

    #[test]
    fn test_analyze_imports() {
        let source = r#"package main
import "fmt"
func main() {}"#;

        let graph = analyze_imports(source, None).unwrap();
        assert_eq!(graph.metadata.language, "go");
        assert_eq!(graph.source_file, "in-memory.go");
        assert_eq!(graph.imports.len(), 1);
        assert_eq!(graph.imports[0].module_path, "fmt");
    }

    #[test]
    fn test_extract_symbols_graceful_fallback() {
        let source = r#"package main

// HelloWorld prints hello
func HelloWorld() {
    println("Hello")
}

// User represents a user
type User struct {
    Name string
}

const MaxUsers = 100
"#;

        // This should either succeed with symbols (if Go is available)
        // or return an empty list (if Go is not available)
        let result = extract_symbols(source);
        assert!(result.is_ok(), "extract_symbols should not fail");

        // If symbols were extracted, verify they're correct
        let symbols = result.unwrap();
        if !symbols.is_empty() {
            assert!(symbols.iter().any(|s| s.name == "HelloWorld"));
            assert!(symbols.iter().any(|s| s.name == "User"));
            assert!(symbols.iter().any(|s| s.name == "MaxUsers"));
        }
    }
}
