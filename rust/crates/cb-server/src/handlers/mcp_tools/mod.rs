//! MCP tool implementations

pub mod util;
pub mod navigation;
pub mod editing;
pub mod filesystem;
pub mod intelligence;
pub mod analysis;
pub mod hierarchy;
pub mod batch;
pub mod diagnostics;
pub mod server_management;
pub mod monitoring;
pub mod refactoring;
pub mod duplicate_detection;

#[cfg(test)]
mod refactoring_tests;

#[cfg(test)]
mod debug_refactoring;

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
    monitoring::register_tools(dispatcher);
    refactoring::register_tools(dispatcher);
    duplicate_detection::register(dispatcher);
}