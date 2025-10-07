//! cb-core: Core types, configuration, and error handling for Codeflow Buddy
//!
//! This crate provides the foundational types and utilities used across
//! the entire Codeflow Buddy Rust implementation.

pub mod auth;
pub mod config;
pub mod dry_run;
pub mod language;
pub mod logging;
pub mod utils;
pub mod workspaces;

pub use config::AppConfig;
pub use dry_run::{execute_with_dry_run, DryRunnable};

// Re-export from cb-types for backwards compatibility
pub use cb_types::error::{ApiError, CoreError};
pub use cb_types::model;
