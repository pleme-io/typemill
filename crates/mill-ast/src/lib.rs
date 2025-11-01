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

// Analyzer
pub use analyzer::plan_refactor;

// Cache
pub use cache::{AstCache, CacheKey, CacheSettings, CachedEntry};

// Error types
pub use error::{AstError, AstResult};

// Import utilities
pub use import_updater::{find_project_files, update_imports_for_rename, ImportPathResolver};

// Parser
pub use parser::{build_dependency_graph, build_import_graph, DependencyGraph};

// Refactoring (keep comprehensive for now - large surface)
pub use refactoring::*;

// Transformer (keep comprehensive for now - large surface)
pub use transformer::*;
