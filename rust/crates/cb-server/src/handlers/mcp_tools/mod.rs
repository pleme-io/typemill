//! MCP tool implementations

pub mod navigation;
pub mod editing;
pub mod filesystem;
pub mod intelligence;
pub mod analysis;
pub mod hierarchy;
pub mod batch;
pub mod diagnostics;
pub mod server_management;

use crate::handlers::McpDispatcher;

/// Register all MCP tools with the dispatcher
pub fn register_all_tools(dispatcher: &mut McpDispatcher) {
    navigation::register_tools(dispatcher);
    editing::register_tools(dispatcher);
    filesystem::register_tools(dispatcher);
    intelligence::register_tools(dispatcher);
    analysis::register_tools(dispatcher);
    hierarchy::register_tools(dispatcher);
    batch::register_tools(dispatcher);
    diagnostics::register_tools(dispatcher);
    server_management::register_tools(dispatcher);
}