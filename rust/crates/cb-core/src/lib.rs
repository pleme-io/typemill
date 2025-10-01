//! cb-core: Core types, configuration, and error handling for Codeflow Buddy
//!
//! This crate provides the foundational types and utilities used across
//! the entire Codeflow Buddy Rust implementation.

pub mod config;
pub mod error;
pub mod language;
pub mod model;

pub use config::AppConfig;
pub use error::CoreError;
