//! MCP tool handlers module

pub mod analysis_handler;
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
pub mod quick_rename_handler;
pub mod refactoring_handler;
pub mod rename_handler;
pub mod reorder_handler;
pub mod system_handler;
pub mod tool_registry;
pub mod tools;
pub mod transform_handler;
pub mod workflow_handler;
pub mod workspace_apply_handler;
// Note: mcp_tools module removed - all functionality now handled by plugin system
// Note: dead_code module moved from system_handler to analysis_handler

pub use analysis_handler::AnalysisHandler;
pub use delete_handler::DeleteHandler;
pub use extract_handler::ExtractHandler;
pub use file_operation_handler::FileOperationHandler;
pub use inline_handler::InlineHandler;
pub use lsp_adapter::DirectLspAdapter;
pub use plugin_dispatcher::{create_test_dispatcher, AppState, PluginDispatcher};
pub use quick_rename_handler::QuickRenameHandler;
pub use r#move::MoveHandler;
pub use refactoring_handler::RefactoringHandler;
pub use rename_handler::RenameHandler;
pub use reorder_handler::ReorderHandler;
pub use system_handler::SystemHandler;
pub use tool_registry::ToolRegistry;
pub use tools::{
    AdvancedToolsHandler, FileToolsHandler, LifecycleHandler, NavigationHandler,
    SystemToolsHandler, ToolHandler, ToolHandlerContext, WorkspaceToolsHandler,
};
pub use transform_handler::TransformHandler;
pub use workflow_handler::WorkflowHandler;
pub use workspace_apply_handler::WorkspaceApplyHandler;
// Note: register_all_tools is no longer needed - plugins auto-register
