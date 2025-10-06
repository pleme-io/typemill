//! Common utilities and helpers for language plugin implementations
//!
//! This crate provides shared functionality to reduce code duplication across
//! language plugin implementations (Rust, Python, Go, TypeScript, Java, etc.).
//!
//! # Modules
//!
//! - [`subprocess`] - Subprocess-based AST parsing utilities for Python/Node/Go/Java
//! - [`refactoring`] - Common refactoring primitives (CodeRange, line extraction)
//! - [`trait_helpers`] - Macros to reduce boilerplate in trait implementations
//! - [`manifest_common`] - TOML/JSON workspace manifest utilities
//!
//! # Example: Using subprocess utilities
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
//! # Example: Using refactoring primitives
//!
//! ```rust,ignore
//! use cb_lang_common::refactoring::{CodeRange, LineExtractor};
//!
//! let range = CodeRange::from_lines(10, 15);
//! let code = LineExtractor::extract_lines(source, range);
//! let indent = LineExtractor::get_indentation(source, 10);
//! ```
//!
//! # Example: Using trait helper macros
//!
//! ```rust,ignore
//! use cb_lang_common::{import_support_impl, workspace_support_impl};
//!
//! pub struct MyImportSupport;
//!
//! // Instead of manually implementing all trait methods,
//! // just implement the "_internal" methods and use the macro:
//! import_support_impl!(MyImportSupport);
//! ```
//!
//! # Example: Using manifest utilities
//!
//! ```rust,ignore
//! use cb_lang_common::manifest_common::{TomlWorkspace, JsonWorkspace};
//!
//! // Add member to Cargo.toml/pyproject.toml
//! let updated = TomlWorkspace::add_member(content, "new-package")?;
//!
//! // Add member to package.json
//! let updated = JsonWorkspace::add_member(content, "new-package")?;
//! ```

pub mod subprocess;
pub mod refactoring;
pub mod trait_helpers;
pub mod manifest_common;

// Re-export commonly used types for convenience
pub use refactoring::{CodeRange, LineExtractor, IndentationDetector};
pub use subprocess::{SubprocessAstTool, run_ast_tool, run_ast_tool_raw};
pub use manifest_common::{TomlWorkspace, JsonWorkspace};

// Re-export macros
pub use trait_helpers::{ImportSupportInternal, WorkspaceSupportInternal};
