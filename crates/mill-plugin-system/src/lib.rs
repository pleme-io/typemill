//! Plugin system for language-specific code intelligence
//!
//! This crate consolidates runtime plugin management from cb-plugins.
//! Static plugin registration (inventory) has been moved to mill-plugin-api (Layer 0).

pub mod adapters;
pub mod capabilities;
pub mod error;
pub mod manager;
pub mod mcp;
pub mod plugin;
pub mod process_manager;
pub mod protocol;
pub mod registry;
pub mod rpc_adapter;
pub mod system_tools_plugin;

pub use adapters::lsp_adapter::{LspAdapterPlugin, LspService};
pub use capabilities::*;
pub use error::{PluginError, PluginResult};
pub use manager::PluginManager;
pub use plugin::{LanguagePlugin, PluginMetadata};
pub use process_manager::PluginProcessManager;
pub use protocol::{PluginRequest, PluginResponse, Position, Range};
pub use registry::PluginRegistry;

/// Plugin system version for compatibility checking
pub const PLUGIN_SYSTEM_VERSION: &str = env!("CARGO_PKG_VERSION");

// Re-export static plugin registration from mill-plugin-api for backward compatibility
pub use mill_plugin_api::{iter_plugins, mill_plugin, PluginDescriptor};
