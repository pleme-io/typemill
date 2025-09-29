//! Plugin system for language-specific code intelligence
//!
//! This crate provides the plugin architecture for codebuddy,
//! enabling language-specific implementations without core code modifications.

pub mod adapters;
pub mod capabilities;
pub mod error;
pub mod manager;
pub mod plugin;
pub mod protocol;
pub mod registry;
pub mod system_tools_plugin;

pub use adapters::lsp_adapter::{LspAdapterPlugin, LspService};
pub use capabilities::*;
pub use error::{PluginError, PluginResult};
pub use manager::PluginManager;
pub use plugin::{LanguagePlugin, PluginMetadata};
pub use protocol::{PluginRequest, PluginResponse, Position, Range};
pub use registry::PluginRegistry;

/// Plugin system version for compatibility checking
pub const PLUGIN_SYSTEM_VERSION: &str = "0.1.0";
