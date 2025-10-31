//! Import-related utilities for Rust code
//!
//! This module provides utilities for handling Rust imports during refactoring operations:
//! - Module path computation from file paths
//! - Crate name extraction from Cargo.toml files
//!
//! # Organization
//!
//! - `crate_name` - Extract crate names from Cargo.toml manifests
//! - `module_path` - Compute fully-qualified module paths from file paths

mod crate_name;
mod module_path;

// Re-export public API
pub use crate_name::find_crate_name_from_cargo_toml;
pub use module_path::compute_module_path_from_file;
