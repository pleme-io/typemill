//! cb-ast: Abstract Syntax Tree parsing, analysis, and transformation for Codeflow Buddy
//!
//! This crate provides sophisticated AST-based code analysis capabilities including
//! import graph building, project-wide refactoring planning, and intelligent caching
//! for performance optimization. It supports multiple languages (TypeScript, Python, etc.)
//! and enables safe, automated code transformations.

pub mod analyzer;
pub mod cache;
pub mod error;
pub mod import_updater;
pub mod language;
pub mod package_extractor;
pub mod parser;
pub mod python_parser;
pub mod refactoring;
pub mod rust_parser;
pub mod transformer;

#[cfg(test)]
mod python_refactoring_test;

pub use analyzer::*;
pub use cache::*;
pub use error::{AstError, AstResult};
pub use import_updater::{find_project_files, update_imports_for_rename, ImportPathResolver};
pub use language::{GoAdapter, JavaAdapter, LanguageAdapter, PythonAdapter, RustAdapter, TypeScriptAdapter};
pub use parser::*;
pub use refactoring::*;
pub use transformer::*;
