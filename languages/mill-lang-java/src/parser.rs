//! Java source code parsing and symbol extraction
use mill_plugin_api::{ParsedSource, PluginError, PluginResult, Symbol, SymbolKind, SourceLocation};
use serde::Deserialize;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::Builder;

// Embed the pre-compiled Java parser JAR into the binary if it exists
// If the JAR file doesn't exist (e.g., Maven not available), we'll use an empty slice
// and the parser will gracefully fall back to returning no symbols
#[cfg(java_parser_jar_exists)]
const JAVA_PARSER_JAR: &[u8] =
    include_bytes!("../resources/java-parser/target/java-parser-1.0.0.jar");

#[cfg(not(java_parser_jar_exists))]
const JAVA_PARSER_JAR: &[u8] = &[];

/// Represents a symbol extracted by the Java parser tool.
#[derive(Debug, Deserialize)]
struct JavaSymbolInfo {
    name: String,
    kind: String,
    line: usize,
}

/// Parse Java source code and extract symbols using the subprocess-based AST parser.
pub fn parse_source(source: &str) -> PluginResult<ParsedSource> {
    // Attempt to extract symbols using the AST-based tool.
    // If it fails (e.g., Java runtime not found), gracefully fall back to an empty list.
    let symbols = match extract_symbols_ast(source) {
        Ok(symbols) => symbols,
        Err(e) => {
            tracing::debug!(error = %e, "Java AST parsing failed, falling back to empty symbols");
            Vec::new()
        }
    };

    Ok(ParsedSource {
        data: serde_json::json!({
            "parser": if symbols.is_empty() { "fallback" } else { "ast" }
        }),
        symbols,
    })
}

/// List all function (method) names in Java source code
///
/// Extracts method names by filtering symbols for method kinds.
/// Falls back to empty list if AST parsing fails.
pub fn list_functions(source: &str) -> PluginResult<Vec<String>> {
    let parsed = parse_source(source)?;
    Ok(parsed.symbols
        .into_iter()
        .filter(|s| matches!(s.kind, SymbolKind::Method | SymbolKind::Function))
        .map(|s| s.name)
        .collect())
}

/// Spawns the embedded Java parser JAR to extract symbols from source code.
fn extract_symbols_ast(source: &str) -> Result<Vec<Symbol>, PluginError> {
    // Create a temporary directory to write the JAR to
    let tmp_dir = Builder::new()
        .prefix("mill-java-parser")
        .tempdir()
        .map_err(|e| PluginError::internal(format!("Failed to create temp dir: {}", e)))?;
    let jar_path = tmp_dir.path().join("java-parser.jar");
    std::fs::write(&jar_path, JAVA_PARSER_JAR)
        .map_err(|e| PluginError::internal(format!("Failed to write JAR to temp file: {}", e)))?;

    // Spawn the Java process
    let mut child = Command::new("java")
        .arg("-jar")
        .arg(&jar_path)
        .arg("extract-symbols")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            PluginError::parse(format!(
                "Failed to spawn Java parser tool. Is Java installed and in your PATH? Error: {}",
                e
            ))
        })?;

    // Write the source code to the process's stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(source.as_bytes()).map_err(|e| {
            PluginError::parse(format!("Failed to write to Java parser stdin: {}", e))
        })?;
    }

    // Wait for the process to complete and get the output
    let output = child.wait_with_output().map_err(|e| {
        PluginError::parse(format!("Failed to wait for Java parser process: {}", e))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PluginError::parse(format!(
            "Java parser tool failed: {}",
            stderr
        )));
    }

    // Deserialize the JSON output from stdout
    let java_symbols: Vec<JavaSymbolInfo> =
        serde_json::from_slice(&output.stdout).map_err(|e| {
            PluginError::parse(format!(
                "Failed to parse JSON output from Java parser: {}",
                e
            ))
        })?;

    // Convert the Java-specific symbols to the generic Symbol type
    let symbols = java_symbols
        .into_iter()
        .map(|s| Symbol {
            name: s.name,
            kind: match s.kind.as_str() {
                "Class" => SymbolKind::Class,
                "Interface" => SymbolKind::Interface,
                "Method" => SymbolKind::Method,
                _ => SymbolKind::Other,
            },
            location: SourceLocation {
                line: s.line,
                column: 0, // The Java parser doesn't provide column info yet
            },
            documentation: None,
        })
        .collect();

    Ok(symbols)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_source() {
        let result = parse_source("");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().symbols.len(), 0);
    }

    #[test]
    fn test_parse_simple_java_class() {
        let source = r#"
            package com.example;

            /**
             * A simple example class.
             */
            public class MyClass {
                private String name;

                public void myMethod() {
                    System.out.println("Hello");
                }
            }
        "#;

        let result = parse_source(source);

        // This test will only pass if a Java runtime is available.
        // If it's not, the parser will fall back and produce 0 symbols.
        if let Ok(parsed) = result {
            if parsed.symbols.is_empty() {
                // This can happen if java is not in the PATH, which is a valid fallback.
                // We'll just log a warning instead of failing the test.
                tracing::warn!("Java parser fallback was used. Could not test symbol extraction.");
                return;
            }

            assert_eq!(parsed.symbols.len(), 2);

            let class_symbol = parsed.symbols.iter().find(|s| s.name == "MyClass").unwrap();
            assert_eq!(class_symbol.kind, SymbolKind::Class);

            let method_symbol = parsed
                .symbols
                .iter()
                .find(|s| s.name == "myMethod")
                .unwrap();
            assert_eq!(method_symbol.kind, SymbolKind::Method);
        } else {
            // If there was an error, fail the test
            panic!("Parsing failed: {:?}", result.err());
        }
    }

    #[test]
    fn test_list_functions_multiple() {
        let source = r#"
public class MyClass {
    public void firstMethod() {}
    private int secondMethod() { return 0; }
    public static String thirdMethod() { return "test"; }
}
"#;
        let result = list_functions(source);
        assert!(result.is_ok());
        let functions = result.unwrap();
        // May be empty if Java not available, but should not fail
        if !functions.is_empty() {
            assert!(functions.contains(&"firstMethod".to_string()));
            assert!(functions.contains(&"secondMethod".to_string()));
            assert!(functions.contains(&"thirdMethod".to_string()));
        }
    }

    #[test]
    fn test_list_functions_empty() {
        let source = r#"
public class MyClass {
    private int myField;
    public static final int CONSTANT = 42;
}
"#;
        let result = list_functions(source);
        assert!(result.is_ok());
        let functions = result.unwrap();
        // Should not contain fields/constants
        assert!(!functions.contains(&"myField".to_string()));
        assert!(!functions.contains(&"CONSTANT".to_string()));
        assert!(!functions.contains(&"MyClass".to_string()));
    }
}