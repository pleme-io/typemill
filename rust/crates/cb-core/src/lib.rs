//! cb-core: Core types, configuration, and error handling for Codeflow Buddy
//!
//! This crate provides the foundational types and utilities used across
//! the entire Codeflow Buddy Rust implementation.

pub mod config;
pub mod dry_run;
pub mod error;
pub mod language;
pub mod model;

pub use config::AppConfig;
pub use dry_run::{execute_with_dry_run, DryRunnable};
pub use error::CoreError;
