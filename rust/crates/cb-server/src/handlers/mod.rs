//! MCP tool handlers module

pub mod plugin_dispatcher;
pub mod dead_code;
pub mod tool_handler;
pub mod tool_registry;
pub mod file_operation_handler;
pub mod workflow_handler;
pub mod system_handler;
pub mod refactoring_handler;
// Note: mcp_tools module removed - all functionality now handled by plugin system

pub use plugin_dispatcher::{AppState, PluginDispatcher};
pub use tool_handler::{ToolHandler, ToolContext};
pub use tool_registry::ToolRegistry;
pub use file_operation_handler::FileOperationHandler;
pub use workflow_handler::WorkflowHandler;
pub use system_handler::SystemHandler;
pub use refactoring_handler::RefactoringHandler;
// Note: register_all_tools is no longer needed - plugins auto-register
