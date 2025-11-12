//! Analysis handlers for TypeMill
//!
//! This crate contains all analysis-related MCP tool handlers, extracted from
//! mill-handlers for better modularity and faster compilation.

// Re-export handler API types
pub use mill_handler_api::{ToolHandler, ToolHandlerContext, AppState};

// Re-export AnalysisConfig for use by handlers
pub use config::AnalysisConfig;

// Analysis handler modules
pub mod batch;
pub mod batch_handler;
pub mod circular_dependencies;
pub mod config;
pub mod dead_code;
pub mod dependencies;
pub mod documentation;
pub mod engine;
pub mod helpers;
#[cfg(any(feature = "analysis-dead-code", feature = "analysis-deep-dead-code"))]
pub mod lsp_provider_adapter;
pub mod markdown_fixers;
pub mod module_dependencies;
pub mod quality;
pub mod structure;
pub mod suggestions;
pub mod tests_handler;

// Re-export commonly used types
pub use batch_handler::*;
pub use circular_dependencies::*;
pub use dead_code::*;
pub use dependencies::*;
pub use documentation::*;
pub use module_dependencies::*;
pub use quality::*;
pub use structure::*;
pub use tests_handler::*;
