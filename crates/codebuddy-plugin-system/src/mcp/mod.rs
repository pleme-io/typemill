//! MCP Proxy Plugin System
//!
//! Optional plugin that proxies requests to external MCP servers
//! like context7, allowing codebuddy to integrate with any MCP-compatible tool.

pub mod client;
pub mod error;
pub mod manager;
pub mod plugin;
pub mod presets;
pub mod protocol;

pub use client::ExternalMcpClient;
pub use error::{McpProxyError, McpProxyResult};
pub use manager::ExternalMcpManager;
pub use plugin::McpProxyPlugin;
