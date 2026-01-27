//! AST-based symbol extraction for dead code analysis.
//!
//! This module provides language-specific AST parsers to extract symbols
//! from source files. AST parsing is more reliable than LSP workspace symbols
//! for some cases (especially for visibility detection).

pub mod rust;
pub mod typescript;

pub use rust::RustSymbolExtractor;
pub use typescript::TypeScriptSymbolExtractor;

use mill_analysis_common::graph::SymbolNode;
use std::path::Path;

/// Trait for extracting symbols from source files using AST parsing.
pub trait SymbolExtractor {
    /// Extract all symbols from the given file.
    fn extract_symbols(
        &self,
        file_path: &Path,
        workspace_root: &Path,
    ) -> Result<Vec<SymbolNode>, std::io::Error>;
}
