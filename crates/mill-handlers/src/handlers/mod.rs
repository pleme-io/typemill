//! MCP tool handlers module

pub mod common;
pub mod file_operation_handler;
pub mod lsp_adapter;
pub mod macros;
pub mod plugin_dispatcher;
pub mod prune_ops;
pub mod refactor_extract;
pub mod refactor_inline;
#[path = "relocate_ops/mod.rs"]
pub mod relocate_ops;
pub mod rename_ops;
pub mod system_handler;
pub mod tool_definitions;
pub mod tool_registry;
pub mod tools;
pub mod workflow_handler;
pub mod workspace;

// Magnificent Seven handlers
pub mod inspect_handler;
pub mod prune_handler;
pub mod refactor_handler;
pub mod relocate_handler;
pub mod rename_all_handler;
pub mod search_handler;
pub mod workspace_handler;

#[cfg(test)]
mod lsp_will_rename_test;

// Note: mcp_tools module removed - all functionality now handled by plugin system
pub use file_operation_handler::FileOperationHandler;
pub use lsp_adapter::DirectLspAdapter;
pub use plugin_dispatcher::{create_test_dispatcher, AppState, PluginDispatcher};
pub use system_handler::SystemHandler;
pub use tool_definitions::{get_all_tool_definitions, is_public_tool, PUBLIC_TOOLS};
pub use tool_registry::ToolRegistry;
pub use tools::{
    AdvancedToolsHandler, FileToolsHandler, LifecycleHandler, PlanToolsHandler, ToolHandler,
    ToolHandlerContext,
};
pub use workflow_handler::WorkflowHandler;

// Export new Magnificent Seven handlers
pub use inspect_handler::InspectHandler;
pub use prune_handler::PruneHandler;
pub use refactor_handler::RefactorHandler;
pub use relocate_handler::RelocateHandler;
pub use rename_all_handler::RenameAllHandler;
pub use search_handler::SearchHandler;
pub use workspace_handler::WorkspaceHandler;
// Note: register_all_tools is no longer needed - plugins auto-register
