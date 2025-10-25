//! mill-ast: Abstract Syntax Tree parsing, analysis, and transformation for TypeMill
//!
//! This crate provides sophisticated AST-based code analysis capabilities including
//! import graph building, project-wide refactoring planning, and intelligent caching
//! for performance optimization. Supports TypeScript/JavaScript with extensible
//! plugin architecture for additional languages.

pub mod analyzer;
pub mod cache;
pub mod complexity;
pub mod error;
pub mod import_updater;
pub mod package_extractor; // Now language-agnostic using capability-based dispatch
pub mod parser;
pub mod refactoring;
pub mod transformer;

pub use analyzer::*;
pub use cache::*;
pub use error::{AstError, AstResult};
pub use import_updater::{find_project_files, update_imports_for_rename, ImportPathResolver};
pub use parser::*;
pub use refactoring::*;
pub use transformer::*;
