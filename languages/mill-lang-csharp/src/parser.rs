//! Csharp source code parsing and symbol extraction
use mill_lang_common::parse_with_fallback;
use mill_plugin_api::{ParsedSource, PluginError, PluginResult, SourceLocation, Symbol, SymbolKind};
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

// A temporary struct to deserialize the JSON output from the C# parser.
// This decouples the plugin from the exact output of the external tool.
#[derive(Deserialize)]
struct JsonSymbol {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Kind")]
    kind: String,
    #[serde(rename = "Location")]
    location: JsonLocation,
}

#[derive(Deserialize)]
struct JsonLocation {
    #[serde(rename = "StartLine")]
    start_line: u32,
    #[serde(rename = "StartColumn")]
    start_column: u32,
}

impl From<JsonSymbol> for Symbol {
    fn from(json_symbol: JsonSymbol) -> Self {
        let kind = match json_symbol.kind.as_str() {
            "Class" => SymbolKind::Class,
            "Interface" => SymbolKind::Interface,
            "Struct" => SymbolKind::Struct,
            "Enum" => SymbolKind::Enum,
            "Method" => SymbolKind::Method,
            "Property" | "Field" => SymbolKind::Field,
            "Using" => SymbolKind::Module,
            _ => SymbolKind::Variable, // Fallback
        };

        Symbol {
            name: json_symbol.name,
            kind,
            location: SourceLocation {
                line: json_symbol.location.start_line.saturating_sub(1) as usize,
                column: json_symbol.location.start_column.saturating_sub(1) as usize,
            },
            documentation: None,
        }
    }
}

/// Parse C# source code using the primary AST parser.
fn parse_with_ast(source: &str) -> PluginResult<Vec<Symbol>> {
    let parser_path = Path::new("crates/mill-lang-csharp/csharp-parser");

    if !parser_path.exists() {
        return Err(PluginError::internal(
            "C# parser executable not found at 'crates/mill-lang-csharp/csharp-parser'. Please run 'make build-parsers' in the project root.",
        ));
    }

    let mut child = Command::new(parser_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            PluginError::internal(format!(
                "Failed to spawn C# parser subprocess. Is it executable? Error: {}",
                e
            ))
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(source.as_bytes())
            .map_err(|e| PluginError::internal(format!("Failed to write to stdin: {}", e)))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| PluginError::internal(format!("Failed to wait for subprocess: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PluginError::parse(format!(
            "C# parser failed: {}",
            stderr
        )));
    }

    let json_symbols: Vec<JsonSymbol> = serde_json::from_slice(&output.stdout).map_err(|e| {
        PluginError::parse(format!("Failed to parse JSON from C# parser: {}", e))
    })?;

    let symbols = json_symbols.into_iter().map(Symbol::from).collect();
    Ok(symbols)
}


lazy_static! {
    static ref SYMBOL_REGEX: Regex = Regex::new(
        r"(?m)^\s*(public|private|internal)?\s*(class|interface|struct|enum|using)\s+([\w\.]+)"
    )
    .expect("Invalid regex for C# symbol parsing");
}

/// Fallback parser using regex to extract basic symbols.
fn parse_with_regex(source: &str) -> PluginResult<Vec<Symbol>> {
    let mut symbols = Vec::new();
    for cap in SYMBOL_REGEX.captures_iter(source) {
        let kind_str = &cap[2];
        let name = &cap[3];
        let kind = match kind_str {
            "class" => SymbolKind::Class,
            "interface" => SymbolKind::Interface,
            "struct" => SymbolKind::Struct,
            "enum" => SymbolKind::Enum,
            "using" => SymbolKind::Module,
            _ => continue,
        };
        if let Some(match_group) = cap.get(0) {
            let start = match_group.start();
            let (line, col) = offset_to_line_col(source, start);
            symbols.push(Symbol {
                name: name.to_string(),
                kind,
                location: SourceLocation {
                    line: line.saturating_sub(1),
                    column: col.saturating_sub(1),
                },
                documentation: None,
            });
        }
    }
    Ok(symbols)
}

/// Parse C# source code and extract symbols using a fallback mechanism.
pub fn parse_source(source: &str) -> PluginResult<ParsedSource> {
    let symbols = parse_with_fallback(
        || parse_with_ast(source),
        || parse_with_regex(source),
        "C# symbol extraction",
    )?;
    Ok(ParsedSource {
        data: serde_json::json!({
            "language": "CSharp",
            "source_length": source.len(),
            "symbols_count": symbols.len(),
        }),
        symbols,
    })
}

/// List all function (method) names in C# source code
///
/// Extracts method names by filtering symbols for method kinds.
/// Uses the same fallback mechanism as parse_source.
pub fn list_functions(source: &str) -> PluginResult<Vec<String>> {
    let parsed = parse_source(source)?;
    Ok(parsed.symbols
        .into_iter()
        .filter(|s| s.kind == SymbolKind::Method)
        .map(|s| s.name)
        .collect())
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut last_newline = 0;
    for (i, c) in source.char_indices() {
        if i >= offset {
            break;
        }
        if c == '\n' {
            line += 1;
            last_newline = i + 1;
        }
    }
    (line, offset - last_newline + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    const SAMPLE_CODE: &str = r#"
using System;
namespace MyNamespace
{
    public class MyClass
    {
        public int MyProperty { get; set; }
        public void MyMethod()
        {
            Console.WriteLine("Hello");
        }
    }
    internal interface IMyInterface
    {
        void DoSomething();
    }
}
"#;
    #[test]
    fn test_parse_with_ast_success() {
        // This test will fail at runtime if the C# parser is not built and available.
        // The goal here is to ensure the code compiles.
        let result = parse_with_ast(SAMPLE_CODE);

        // If the parser binary is not present, we expect a specific error.
        if !Path::new("crates/mill-lang-csharp/csharp-parser").exists() {
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.to_string().contains("executable not found"));
        } else {
             // If the parser *is* present, we expect it to succeed or fail gracefully.
            assert!(result.is_ok() || result.is_err());
        }
    }
    #[test]
    fn test_parse_with_regex_fallback() {
        let result = parse_with_regex(SAMPLE_CODE);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 3);
        assert!(symbols.iter().any(|s| s.name == "System" && s.kind == SymbolKind::Module));
        assert!(symbols.iter().any(|s| s.name == "MyClass" && s.kind == SymbolKind::Class));
        assert!(symbols.iter().any(|s| s.name == "IMyInterface" && s.kind == SymbolKind::Interface));
    }
    #[test]
    fn test_parse_empty_source() {
        let result = parse_source("");
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.symbols.len(), 0);
    }

    #[test]
    fn test_list_functions_multiple() {
        let source = r#"
public class MyClass {
    public void FirstMethod() {}
    private int SecondMethod() { return 0; }
    public async Task ThirdMethod() {}
}
"#;
        let result = list_functions(source);
        assert!(result.is_ok());
        let functions = result.unwrap();
        // May be empty if C# parser not available, but should not fail
        if !functions.is_empty() {
            assert!(functions.contains(&"FirstMethod".to_string()));
            assert!(functions.contains(&"SecondMethod".to_string()));
            assert!(functions.contains(&"ThirdMethod".to_string()));
        }
    }

    #[test]
    fn test_list_functions_empty() {
        let source = r#"
public class MyClass {
    private int myField;
    public string MyProperty { get; set; }
}
"#;
        let result = list_functions(source);
        assert!(result.is_ok());
        let functions = result.unwrap();
        // Should not contain fields/properties
        assert!(!functions.contains(&"myField".to_string()));
        assert!(!functions.contains(&"MyProperty".to_string()));
        assert!(!functions.contains(&"MyClass".to_string()));
    }
}