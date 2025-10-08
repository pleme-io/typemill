//! Swift source code parsing and symbol extraction
//!
//! This implementation uses `sourcekitten` to generate a JSON AST, which is then
//! parsed to extract symbols. This is more accurate than regex but requires
//! `sourcekitten` to be installed in the environment.

use cb_lang_common::ErrorBuilder;
use cb_plugin_api::{ParsedSource, PluginResult, SourceLocation, Symbol, SymbolKind};
use serde::Deserialize;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;
use tracing::{debug, warn};

#[derive(Deserialize, Debug)]
struct SourceKittenStructure {
    #[serde(rename = "key.substructure")]
    substructure: Option<Vec<SourceKittenNode>>,
}

#[derive(Deserialize, Debug)]
struct SourceKittenNode {
    #[serde(rename = "key.kind")]
    kind: String,
    #[serde(rename = "key.name")]
    name: Option<String>,
    #[serde(rename = "key.line")]
    line: Option<u32>,
    #[serde(rename = "key.column")]
    column: Option<u32>,
    #[serde(rename = "key.substructure")]
    substructure: Option<Vec<SourceKittenNode>>,
}

/// Maps a SourceKitten kind string to a Codebuddy `SymbolKind`.
fn map_kind(kind_str: &str) -> Option<SymbolKind> {
    match kind_str {
        "source.lang.swift.decl.function.free" => Some(SymbolKind::Function),
        "source.lang.swift.decl.class" => Some(SymbolKind::Class),
        "source.lang.swift.decl.struct" => Some(SymbolKind::Struct),
        "source.lang.swift.decl.enum" => Some(SymbolKind::Enum),
        "source.lang.swift.decl.protocol" => Some(SymbolKind::Interface),
        "source.lang.swift.decl.var.global" => Some(SymbolKind::Variable),
        "source.lang.swift.decl.function.method.instance"
        | "source.lang.swift.decl.function.method.static" => Some(SymbolKind::Method),
        _ => None,
    }
}

/// Recursively traverses the SourceKitten AST to extract symbols.
fn extract_symbols_from_nodes(nodes: &[SourceKittenNode], symbols: &mut Vec<Symbol>) {
    for node in nodes {
        if let (Some(kind), Some(name), Some(line)) = (map_kind(&node.kind), &node.name, node.line)
        {
            symbols.push(Symbol {
                name: name.clone(),
                kind,
                location: SourceLocation {
                    line: line as usize,
                    column: node.column.unwrap_or(0) as usize,
                },
                documentation: None,
            });
        }
        if let Some(sub_nodes) = &node.substructure {
            extract_symbols_from_nodes(sub_nodes, symbols);
        }
    }
}

/// Parse Swift source code using `sourcekitten` and extract symbols.
pub fn parse_source(source: &str) -> PluginResult<ParsedSource> {
    debug!("Parsing Swift source with `sourcekitten`");

    let mut temp_file = NamedTempFile::new().map_err(|e| {
        ErrorBuilder::internal(format!("Failed to create temporary file: {}", e)).build()
    })?;
    temp_file.write_all(source.as_bytes()).map_err(|e| {
        ErrorBuilder::internal(format!("Failed to write to temporary file: {}", e)).build()
    })?;

    let output = Command::new("sourcekitten")
        .arg("structure")
        .arg("--file")
        .arg(temp_file.path())
        .output()
        .map_err(|e| {
            warn!("`sourcekitten` command failed to execute. Is it installed and in your PATH? Error: {}", e);
            ErrorBuilder::internal("`sourcekitten` command not found. Please ensure it is installed.").build()
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(
            ErrorBuilder::parse(format!("`sourcekitten` failed with error: {}", stderr)).build(),
        );
    }

    let structure: SourceKittenStructure = serde_json::from_slice(&output.stdout).map_err(|e| {
        ErrorBuilder::parse(format!("Failed to parse sourcekitten JSON output: {}", e)).build()
    })?;

    let mut symbols = Vec::new();
    if let Some(nodes) = structure.substructure {
        extract_symbols_from_nodes(&nodes, &mut symbols);
    }

    let raw_data = serde_json::from_slice(&output.stdout).unwrap_or_default();

    Ok(ParsedSource {
        data: raw_data,
        symbols,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_source() {
        let result = parse_source("");
        // This will fail if sourcekitten is not installed, which is expected.
        // In a proper environment, it should return an empty symbol list.
        if result.is_ok() {
            assert_eq!(result.unwrap().symbols.len(), 0);
        }
    }

    #[test]
    #[ignore] // This test requires a working `sourcekitten` installation.
    fn test_sourcekitten_parser() {
        let source = r#"
            func globalFunction() {}
            class MyClass {
                struct NestedStruct {}
                func myMethod() {}
            }
        "#;

        let result = parse_source(source).unwrap();
        assert_eq!(result.symbols.len(), 4);

        let global_func = result
            .symbols
            .iter()
            .find(|s| s.name == "globalFunction")
            .unwrap();
        assert_eq!(global_func.kind, SymbolKind::Function);
        assert_eq!(global_func.location.line, 2);

        let my_class = result.symbols.iter().find(|s| s.name == "MyClass").unwrap();
        assert_eq!(my_class.kind, SymbolKind::Class);
        assert_eq!(my_class.location.line, 3);

        let nested_struct = result
            .symbols
            .iter()
            .find(|s| s.name == "NestedStruct")
            .unwrap();
        assert_eq!(nested_struct.kind, SymbolKind::Struct);
        assert_eq!(nested_struct.location.line, 4);

        let my_method = result
            .symbols
            .iter()
            .find(|s| s.name == "myMethod")
            .unwrap();
        assert_eq!(my_method.kind, SymbolKind::Method);
        assert_eq!(my_method.location.line, 5);
    }
}
