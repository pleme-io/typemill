//! TypeScript/JavaScript AST-based symbol extraction.

use super::SymbolExtractor;
use lsp_types::{Position, Range};
use mill_analysis_common::graph::{SymbolKind, SymbolNode};
use mill_lang_common::{run_ast_tool, SubprocessAstTool};
use serde::Deserialize;
use std::fs;
use std::path::Path;
use tracing::{debug, warn};

/// Extracts symbols from TypeScript/JavaScript source files using a Node.js AST tool.
pub struct TypeScriptSymbolExtractor;

impl TypeScriptSymbolExtractor {
    /// Creates a new `TypeScriptSymbolExtractor`.
    pub fn new() -> Self {
        Self
    }

    /// Converts a `TsSymbolInfo` to a `SymbolNode`.
    fn ts_symbol_to_symbol_node(&self, ts_symbol: &TsSymbolInfo, file_path: &Path) -> SymbolNode {
        let range = Range {
            start: Position {
                line: (ts_symbol.location.start_line - 1) as u32,
                character: ts_symbol.location.start_column as u32,
            },
            end: Position {
                line: (ts_symbol.location.end_line - 1) as u32,
                character: ts_symbol.location.end_column as u32,
            },
        };

        let id = format!(
            "{}::{}@L{}",
            file_path.display(),
            ts_symbol.name,
            range.start.line
        );

        debug!("Extracted TypeScript symbol: {}", id);

        SymbolNode {
            id,
            name: ts_symbol.name.clone(),
            kind: self.ts_kind_to_symbol_kind(&ts_symbol.kind),
            file_path: file_path.to_str().unwrap_or("").to_string(),
            is_public: ts_symbol.is_public,
            range,
        }
    }

    /// Maps the string-based kind from `ast_tool.js` to the `SymbolKind` enum.
    fn ts_kind_to_symbol_kind(&self, kind_str: &str) -> SymbolKind {
        match kind_str {
            "Function" => SymbolKind::Function,
            "Struct" => SymbolKind::Struct,
            "Trait" => SymbolKind::Trait,
            "Enum" => SymbolKind::Enum,
            "TypeAlias" => SymbolKind::TypeAlias,
            "Constant" => SymbolKind::Constant,
            "Module" => SymbolKind::Module,
            _ => SymbolKind::Function, // Default fallback
        }
    }
}

impl Default for TypeScriptSymbolExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolExtractor for TypeScriptSymbolExtractor {
    fn extract_symbols(
        &self,
        file_path: &Path,
        workspace_root: &Path,
    ) -> Result<Vec<SymbolNode>, std::io::Error> {
        let source_code = fs::read_to_string(file_path)?;

        const AST_TOOL_JS: &str =
            include_str!("../../../../languages/mill-lang-typescript/resources/ast_tool.js");

        let tool = SubprocessAstTool::new("node")
            .with_embedded_str(AST_TOOL_JS)
            .with_temp_filename("ast_tool.js")
            .with_arg("extract-symbols-deep");

        let ts_symbols: Vec<TsSymbolInfo> = match run_ast_tool(tool, &source_code) {
            Ok(symbols) => symbols,
            Err(e) => {
                warn!(
                    "Failed to run TypeScript AST tool for file {:?}: {}",
                    file_path, e
                );
                return Ok(Vec::new());
            }
        };

        let relative_path = pathdiff::diff_paths(file_path, workspace_root)
            .unwrap_or_else(|| file_path.to_path_buf());

        let symbols = ts_symbols
            .into_iter()
            .map(|s| self.ts_symbol_to_symbol_node(&s, &relative_path))
            .collect();

        Ok(symbols)
    }
}

/// Represents the detailed symbol information extracted by `ast_tool.js`.
#[derive(Debug, Deserialize)]
struct TsSymbolInfo {
    name: String,
    kind: String,
    is_public: bool,
    location: TsLocation,
}

/// Represents a location in the source code.
#[derive(Debug, Deserialize)]
struct TsLocation {
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
}
