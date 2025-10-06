//! Common utilities and helpers for language plugin implementations
//!
//! This crate provides shared functionality to reduce code duplication across
//! language plugin implementations (Rust, Python, Go, TypeScript, Java, etc.).
//!
//! # Modules
//!
//! ## Core Utilities
//! - [`subprocess`] - Subprocess-based AST parsing utilities for Python/Node/Go/Java
//! - [`refactoring`] - Common refactoring primitives (CodeRange, line extraction)
//! - [`trait_helpers`] - Macros to reduce boilerplate in trait implementations
//! - [`manifest_common`] - TOML/JSON workspace manifest utilities
//!
//! ## Phase 2 (High ROI)
//! - [`error_helpers`] - Error construction with rich context
//! - [`io`] - File system operations with standardized error handling
//! - [`import_parsing`] - Import statement parsing utilities
//!
//! ## Phase 3 (Medium ROI)
//! - [`location`] - SourceLocation builders and utilities
//! - [`versioning`] - Version parsing and dependency source detection
//! - [`ast_deserialization`] - AST tool output parsing
//! - [`manifest_templates`] - Manifest file generation
//!
//! ## Phase 4 (Quality of Life)
//! - [`testing`] - Test utilities and fixtures
//! - [`plugin_scaffold`] - Plugin code generation
//!
//! # Examples
//!
//! ## Subprocess utilities
//!
//! ```rust,ignore
//! use cb_lang_common::subprocess::{SubprocessAstTool, run_ast_tool};
//!
//! const PYTHON_TOOL: &str = include_str!("../resources/ast_tool.py");
//!
//! let tool = SubprocessAstTool::new("python3")
//!     .with_embedded_str(PYTHON_TOOL)
//!     .with_temp_filename("ast_tool.py")
//!     .with_arg("list-functions");
//!
//! let functions: Vec<String> = run_ast_tool(tool, source)?;
//! ```
//!
//! ## Error handling
//!
//! ```rust,ignore
//! use cb_lang_common::error_helpers::ErrorBuilder;
//!
//! let error = ErrorBuilder::parse("Invalid syntax")
//!     .with_path(&file_path)
//!     .with_line(42)
//!     .build();
//! ```
//!
//! ## File I/O
//!
//! ```rust,ignore
//! use cb_lang_common::io::{read_manifest, find_source_files};
//!
//! let content = read_manifest(Path::new("Cargo.toml")).await?;
//! let files = find_source_files(root, &["rs", "toml"]).await?;
//! ```
//!
//! ## Import parsing
//!
//! ```rust,ignore
//! use cb_lang_common::import_parsing::{parse_import_alias, ExternalDependencyDetector};
//!
//! let (name, alias) = parse_import_alias("foo as bar");
//! let detector = ExternalDependencyDetector::new()
//!     .with_relative_prefix("./")
//!     .with_relative_prefix("../");
//! ```

// Core modules (Phase 1)
pub mod subprocess;
pub mod refactoring;
pub mod trait_helpers;
pub mod manifest_common;

// Phase 2 modules (High ROI)
pub mod error_helpers;
pub mod io;
pub mod import_parsing;

// Phase 3 modules (Medium ROI)
pub mod location;
pub mod versioning;
pub mod ast_deserialization;
pub mod manifest_templates;

// Phase 4 modules (Quality of Life)
pub mod testing;
pub mod plugin_scaffold;

// Re-export commonly used types for convenience
pub use refactoring::{CodeRange, LineExtractor, IndentationDetector};
pub use subprocess::{SubprocessAstTool, run_ast_tool, run_ast_tool_raw};
pub use manifest_common::{TomlWorkspace, JsonWorkspace};
pub use error_helpers::ErrorBuilder;
pub use import_parsing::{parse_import_alias, split_import_list, ExternalDependencyDetector};
pub use location::LocationBuilder;
pub use versioning::detect_dependency_source;
pub use ast_deserialization::{AstSymbol, AstToolOutput, parse_ast_output};

// Re-export macros
pub use trait_helpers::{ImportSupportInternal, WorkspaceSupportInternal};
