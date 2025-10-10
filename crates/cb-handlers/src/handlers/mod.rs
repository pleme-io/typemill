//! MCP tool handlers module

pub mod analysis_handler;
pub mod dependency_handler;
pub mod file_operation_handler;
pub mod lsp_adapter;
pub mod macros;
pub mod plugin_dispatcher;
pub mod refactoring_handler;
pub mod system_handler;
pub mod tool_registry;
pub mod tools;
pub mod workflow_handler;
// Note: mcp_tools module removed - all functionality now handled by plugin system
// Note: dead_code module moved from system_handler to analysis_handler

pub use analysis_handler::AnalysisHandler;
pub use file_operation_handler::FileOperationHandler;
pub use lsp_adapter::DirectLspAdapter;
pub use plugin_dispatcher::{create_test_dispatcher, AppState, PluginDispatcher};
pub use refactoring_handler::RefactoringHandler;
pub use system_handler::SystemHandler;
pub use tool_registry::ToolRegistry;
pub use tools::{
    AdvancedToolsHandler, EditingToolsHandler, FileToolsHandler, LifecycleHandler,
    NavigationHandler, SystemToolsHandler, ToolHandler, ToolHandlerContext, WorkspaceToolsHandler,
};
pub use workflow_handler::WorkflowHandler;
// Note: register_all_tools is no longer needed - plugins auto-register
