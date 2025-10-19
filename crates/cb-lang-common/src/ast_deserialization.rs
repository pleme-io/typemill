//! AST tool output deserialization
//!
//! Common structures for deserializing output from subprocess AST parsing tools.
//! Provides standard formats that language plugins can use when implementing
//! their AST tools in Python, Node.js, Go, Java, etc.

use cb_plugin_api::{PluginResult, SourceLocation, Symbol, SymbolKind};
use codebuddy_foundation::protocol::ImportInfo;
use serde::{Deserialize, Serialize};

/// Standard symbol representation from AST tools
///
/// AST tools (Python/Node/Go/Java subprocesses) should output this format
/// for maximum compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstSymbol {
    /// Symbol name (e.g., function name, class name)
    pub name: String,

    /// Symbol kind as string (converted to SymbolKind)
    ///
    /// Supported values: "function", "class", "interface", "struct", "enum",
    /// "constant", "variable", "method", "module", "type", "trait"
    pub kind: String,

    /// Line number (0-based or 1-based, configurable)
    pub line: usize,

    /// Optional column number
    #[serde(default)]
    pub column: Option<usize>,

    /// Optional documentation/docstring
    #[serde(default)]
    pub documentation: Option<String>,

    /// Optional end line for multi-line symbols
    #[serde(default)]
    pub end_line: Option<usize>,
}

impl From<AstSymbol> for Symbol {
    fn from(ast: AstSymbol) -> Self {
        Symbol {
            name: ast.name,
            kind: parse_symbol_kind(&ast.kind),
            location: SourceLocation {
                line: ast.line,
                column: ast.column.unwrap_or(0),
            },
            documentation: ast.documentation,
        }
    }
}

/// Parse symbol kind string to SymbolKind enum
///
/// Case-insensitive matching with common aliases
fn parse_symbol_kind(kind_str: &str) -> SymbolKind {
    match kind_str.to_lowercase().as_str() {
        "function" | "func" | "fn" => SymbolKind::Function,
        "class" => SymbolKind::Class,
        "interface" => SymbolKind::Interface,
        "struct" | "structure" => SymbolKind::Struct,
        "enum" | "enumeration" => SymbolKind::Enum,
        "constant" | "const" => SymbolKind::Constant,
        "variable" | "var" | "let" => SymbolKind::Variable,
        "method" => SymbolKind::Method,
        "module" | "mod" | "namespace" => SymbolKind::Module,
        "field" => SymbolKind::Field,
        // Map type aliases and traits to Other since they're not in the enum
        "type" | "typedef" | "trait" => SymbolKind::Other,
        _ => SymbolKind::Other,
    }
}

/// Standard output format from AST parsing tools
///
/// AST tools should return JSON in this format for consistency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstToolOutput {
    /// List of symbols found in the source
    pub symbols: Vec<AstSymbol>,

    /// Optional import information
    #[serde(default)]
    pub imports: Option<Vec<ImportInfo>>,

    /// Optional additional metadata
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

impl AstToolOutput {
    /// Convert to cb-plugin-api Symbol list
    pub fn into_symbols(self) -> Vec<Symbol> {
        self.symbols.into_iter().map(Symbol::from).collect()
    }

    /// Extract both symbols and imports
    pub fn into_parts(self) -> (Vec<Symbol>, Vec<ImportInfo>) {
        let symbols = self.symbols.into_iter().map(Symbol::from).collect();
        let imports = self.imports.unwrap_or_default();
        (symbols, imports)
    }
}

/// Helper function to deserialize AST tool JSON output
///
/// # Example
///
/// ```rust,ignore
/// use cb_lang_common::ast_deserialization::parse_ast_output;
///
/// let json = r#"{"symbols": [{"name": "foo", "kind": "function", "line": 0}]}"#;
/// let output = parse_ast_output(json)?;
/// let symbols = output.into_symbols();
/// ```
pub fn parse_ast_output(json: &str) -> PluginResult<AstToolOutput> {
    serde_json::from_str(json).map_err(|e| {
        cb_plugin_api::PluginError::parse(format!("Failed to parse AST tool output: {}", e))
    })
}

/// Helper for AST tools that only return symbol arrays
///
/// Wraps a simple symbol array in the standard AstToolOutput structure
pub fn parse_symbol_array(json: &str) -> PluginResult<Vec<Symbol>> {
    let symbols: Vec<AstSymbol> = serde_json::from_str(json).map_err(|e| {
        cb_plugin_api::PluginError::parse(format!("Failed to parse symbol array: {}", e))
    })?;

    Ok(symbols.into_iter().map(Symbol::from).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_symbol_kind() {
        assert_eq!(parse_symbol_kind("function"), SymbolKind::Function);
        assert_eq!(parse_symbol_kind("FUNCTION"), SymbolKind::Function);
        assert_eq!(parse_symbol_kind("func"), SymbolKind::Function);
        assert_eq!(parse_symbol_kind("class"), SymbolKind::Class);
        assert_eq!(parse_symbol_kind("interface"), SymbolKind::Interface);
        assert_eq!(parse_symbol_kind("unknown"), SymbolKind::Other);
    }

    #[test]
    fn test_ast_symbol_to_symbol() {
        let ast = AstSymbol {
            name: "test_function".to_string(),
            kind: "function".to_string(),
            line: 42,
            column: Some(10),
            documentation: Some("Test doc".to_string()),
            end_line: None,
        };

        let symbol: Symbol = ast.into();
        assert_eq!(symbol.name, "test_function");
        assert_eq!(symbol.kind, SymbolKind::Function);
        assert_eq!(symbol.location.line, 42);
        assert_eq!(symbol.location.column, 10);
        assert_eq!(symbol.documentation, Some("Test doc".to_string()));
    }

    #[test]
    fn test_parse_ast_output() {
        let json = r#"{
            "symbols": [
                {"name": "foo", "kind": "function", "line": 1},
                {"name": "Bar", "kind": "class", "line": 10}
            ]
        }"#;

        let output = parse_ast_output(json).unwrap();
        assert_eq!(output.symbols.len(), 2);

        let symbols = output.into_symbols();
        assert_eq!(symbols[0].name, "foo");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
        assert_eq!(symbols[1].name, "Bar");
        assert_eq!(symbols[1].kind, SymbolKind::Class);
    }

    #[test]
    fn test_parse_symbol_array() {
        let json = r#"[
            {"name": "test", "kind": "function", "line": 5}
        ]"#;

        let symbols = parse_symbol_array(json).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "test");
    }

    #[test]
    fn test_ast_output_with_imports() {
        let json = r#"{
            "symbols": [
                {"name": "foo", "kind": "function", "line": 1}
            ],
            "imports": [],
            "metadata": {"language": "test"}
        }"#;

        let output = parse_ast_output(json).unwrap();
        assert!(output.imports.is_some());
        assert!(output.metadata.is_some());
    }
}