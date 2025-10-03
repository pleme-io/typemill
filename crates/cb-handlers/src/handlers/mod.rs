//! MCP tool handlers module

pub mod compat;
pub mod macros;
pub mod dependency_handler;
pub mod file_operation_handler;
pub mod lsp_adapter;
pub mod plugin_dispatcher;
pub mod refactoring_handler;
pub mod system_handler;
pub mod tool_registry;
pub mod tools;
pub mod workflow_handler;
// Note: mcp_tools module removed - all functionality now handled by plugin system
// Note: dead_code module removed - consolidated into system_handler

pub use file_operation_handler::FileOperationHandler;
pub use lsp_adapter::DirectLspAdapter;
pub use plugin_dispatcher::{create_test_dispatcher, AppState, PluginDispatcher};
pub use refactoring_handler::RefactoringHandler;
pub use system_handler::SystemHandler as LegacySystemHandler;
pub use tool_registry::ToolRegistry;
pub use tools::{
    AdvancedHandler, EditingHandler, FileOpsHandler, LifecycleHandler, NavigationHandler,
    SystemHandler, ToolHandler, ToolHandlerContext, WorkspaceHandler,
};
pub use workflow_handler::WorkflowHandler;
// Note: register_all_tools is no longer needed - plugins auto-register
