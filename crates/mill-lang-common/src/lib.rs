//! Common utilities and helpers for language plugin implementations
//!
//! This crate provides shared functionality to reduce code duplication across
//! language plugin implementations (Rust, Python, Go, TypeScript, Java, etc.).
//!
//! # Migration to v2 API (2024-10)
//!
//! The following functions have been removed due to zero usage:
//! - `split_import_list` - Each language has unique import syntax, generic splitting was never used
//! - `ExternalDependencyDetector` - Overly complex for actual needs, no plugin ever used it
//!
//! See docs/design/CB_LANG_COMMON_API_V2.md for planned v2 utilities.
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
//! ## Additional Utilities
//! - [`import_graph`] - ImportGraph builder for consistent construction
//! - [`parsing`] - Common parsing patterns (fallback strategies)
//!
//! # Examples
//!
//! ## Subprocess utilities
//!
//! ```rust,ignore
//! use mill_lang_common::subprocess::{SubprocessAstTool, run_ast_tool};
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
//! use mill_lang_common::error_helpers::ErrorBuilder;
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
//! use mill_lang_common::io::{read_manifest, find_source_files};
//!
//! let content = read_manifest(Path::new("Cargo.toml")).await?;
//! let files = find_source_files(root, &["rs", "toml"]).await?;
//! ```
//!
//! ## Import parsing
//!
//! ```rust,ignore
//! use mill_lang_common::import_parsing::parse_import_alias;
//!
//! let (name, alias) = parse_import_alias("foo as bar");
//! ```

// Core modules (Phase 1)
pub mod manifest_common;
pub mod plugin_helpers;
pub mod refactoring;
pub mod subprocess;
pub mod trait_helpers;

// Phase 2 modules (High ROI)
pub mod error_helpers;
pub mod import_helpers;
pub mod import_parsing;
pub mod io;
pub mod project_factory;

// Phase 3 modules (Medium ROI)
pub mod ast_deserialization;
pub mod location;
pub mod manifest_templates;
pub mod versioning;

// Phase 4 modules (Quality of Life)
pub mod plugin_scaffold;
pub mod testing;

// Additional utility modules
pub mod import_graph;
pub mod parsing;

// LSP installation utilities (for plugin-based LSP installation)
pub mod lsp;

// Re-export commonly used types for convenience
pub use ast_deserialization::{parse_ast_output, AstSymbol, AstToolOutput};
pub use error_helpers::ErrorBuilder;
pub use import_graph::ImportGraphBuilder;
pub use import_helpers::{
    find_last_matching_line, insert_line_at, remove_lines_matching, replace_in_lines,
};
pub use import_parsing::{extract_package_name, parse_import_alias};
pub use location::{
    extract_text_at_location, offset_to_position, position_to_offset, LocationBuilder,
};
pub use manifest_common::{JsonWorkspace, TomlWorkspace};
pub use parsing::{parse_with_fallback, parse_with_optional_fallback, try_parsers};
pub use refactoring::{
    CodeRange, ExtractVariableAnalysis, ExtractableFunction, IndentationDetector,
    InlineVariableAnalysis, LineExtractor, VariableUsage,
};
pub use subprocess::{run_ast_tool, run_ast_tool_raw, SubprocessAstTool};
pub use versioning::{
    detect_dependency_source, extract_version_number, normalize_version, parse_git_url,
};

// Re-export IO utilities
pub use io::{file_path_to_module, find_source_files, read_manifest, read_source};

// Re-export project factory utilities
pub use project_factory::{
    derive_package_name, find_workspace_manifest, resolve_package_path, update_workspace_manifest,
    validate_package_path_not_exists, write_project_file, WorkspaceManifestDetector,
};

// Re-export macros
pub use trait_helpers::WorkspaceSupportInternal;

// ============================================================================
// Plugin Helper Macros (re-exported for discoverability)
// ============================================================================
//
// The following macros are defined with #[macro_export] in plugin_helpers module,
// which automatically places them at the crate root (mill_lang_common::macro_name).
//
// Available macros:
// - `define_language_plugin!` - Comprehensive plugin scaffolding generator
//   Generates: struct, METADATA, CAPABILITIES, new(), mill_plugin! registration
//
// - `impl_language_plugin_basics!` - Standard LanguagePlugin method delegation
//   Generates: metadata(), capabilities(), as_any()
//
// - `impl_capability_delegations!` - Capability trait method delegation
//   Generates: Optional trait delegation methods for import/workspace/etc.
//
// Usage:
// ```rust
// use mill_lang_common::{define_language_plugin, impl_capability_delegations};
//
// define_language_plugin! {
//     struct: MyPlugin,
//     name: "my-lang",
//     // ... configuration ...
// }
// ```
//
// Note: No explicit re-export needed - #[macro_export] makes them available
// at mill_lang_common:: namespace automatically.
