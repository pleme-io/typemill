//! Plugin system for language-specific code intelligence
//!
//! This crate provides the plugin architecture for codeflow-buddy,
//! enabling language-specific implementations without core code modifications.

pub mod plugin;
pub mod manager;
pub mod capabilities;
pub mod protocol;
pub mod registry;
pub mod error;
pub mod adapters;
pub mod system_tools_plugin;

pub use plugin::{LanguagePlugin, PluginMetadata};
pub use manager::PluginManager;
pub use capabilities::*;
pub use protocol::{PluginRequest, PluginResponse, Position, Range};
pub use registry::PluginRegistry;
pub use error::{PluginError, PluginResult};
pub use adapters::lsp_adapter::{LspAdapterPlugin, LspService};

/// Plugin system version for compatibility checking
pub const PLUGIN_SYSTEM_VERSION: &str = "0.1.0";