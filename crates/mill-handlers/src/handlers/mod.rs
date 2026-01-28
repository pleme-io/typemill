//! MCP tool handlers module

pub mod common;
pub mod delete_handler;
pub mod dependency_handler;
pub mod extract_handler;
pub mod file_operation_handler;
pub mod inline_handler;
pub mod lsp_adapter;
pub mod macros;
#[path = "move/mod.rs"]
pub mod r#move;
pub mod plugin_dispatcher;
pub mod refactoring_handler;
pub mod rename_handler;
pub mod system_handler;
pub mod tool_definitions;
pub mod tool_registry;
pub mod tools;
pub mod workflow_handler;
pub mod workspace;

// New Magnificent Seven handlers
pub mod inspect_handler;
pub mod prune_handler;
pub mod refactor_handler;
pub mod relocate_handler;
pub mod rename_all_handler;
pub mod search_handler;
pub mod workspace_handler;
// Note: mcp_tools module removed - all functionality now handled by plugin system
pub use delete_handler::DeleteHandler;
pub use extract_handler::ExtractHandler;
pub use file_operation_handler::FileOperationHandler;
pub use inline_handler::InlineHandler;
pub use lsp_adapter::DirectLspAdapter;
pub use plugin_dispatcher::{create_test_dispatcher, AppState, PluginDispatcher};
pub use r#move::MoveHandler;
pub use refactoring_handler::RefactoringHandler;
pub use rename_handler::{RenameHandler, RenameOptions, RenameTarget, SymbolSelector};
pub use system_handler::SystemHandler;
pub use tool_definitions::{get_all_tool_definitions, is_public_tool, PUBLIC_TOOLS};
pub use tool_registry::ToolRegistry;
pub use tools::{
    AdvancedToolsHandler, FileToolsHandler, LifecycleHandler, NavigationHandler,
    SystemToolsHandler, ToolHandler, ToolHandlerContext, WorkspaceToolsHandler,
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
