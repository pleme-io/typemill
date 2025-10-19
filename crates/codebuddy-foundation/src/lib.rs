//! Foundation Layer - Core types, protocol definitions, and configuration
//!
//! This crate provides the foundational building blocks for Codebuddy/TypeMill:
//! - Core data structures and types (from cb-types)
//! - MCP protocol definitions (from cb-protocol)
//! - Configuration and error handling (from codebuddy-core)
//!
//! After consolidation, this will contain the merged modules from:
//! - codebuddy-core
//! - cb-types
//! - cb-protocol

// ============================================================================
// TYPES MODULE (consolidated from cb-types)
// ============================================================================
pub mod error;
pub mod model;
pub mod protocol;

// Re-export commonly used types for convenience
pub use error::*;
pub use model::*;
