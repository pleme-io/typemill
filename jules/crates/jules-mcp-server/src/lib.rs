//! Jules MCP Server
//!
//! MCP server providing tools for interacting with the Jules API.

pub mod server;
pub mod config;
pub mod tools;
pub mod handlers;
pub mod mcp;

// Re-exports
pub use server::JulesMcpServer;
pub use config::Config;