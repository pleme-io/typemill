//! Macros for handler registration
//!
//! This module provides declarative macros for registering tool handlers,
//! eliminating boilerplate and ensuring consistency.

/// Register multiple tool handlers in a declarative way
///
/// # Example
///
/// ```text
/// register_handlers!(registry, {
///     SystemHandler,
///     LifecycleHandler,
///     WorkspaceHandler,
/// });
/// ```
///
/// This macro expands to:
/// ```text
/// registry.register(Arc::new(SystemHandler::new()));
/// registry.register(Arc::new(LifecycleHandler::new()));
/// registry.register(Arc::new(WorkspaceHandler::new()));
/// ```
#[macro_export]
macro_rules! register_handlers {
    ($registry:expr, { $($handler:ident),* $(,)? }) => {
        {
            use std::sync::Arc;
            $(
                let handler = Arc::new($handler::new());
                $registry.register(handler);
            )*
        }
    };
}

/// Register tool handlers with debug logging
///
/// This variant logs each handler registration for better visibility during initialization
/// and automatically captures the handler type name for diagnostics and CLI tools.
///
/// # Example
///
/// ```text
/// register_handlers_with_logging!(registry, {
///     SystemHandler => "SystemHandler with 3 tools (health_check, web_fetch, system_status)",
///     LifecycleHandler => "LifecycleHandler with 3 tools (notify_file_opened, notify_file_saved, notify_file_closed)",
/// });
/// ```
///
/// # Benefits
///
/// - **Automatic Type Tracking**: Handler type names are captured for `codebuddy list-tools`
/// - **Debug Visibility**: Logs each registration with tool count
/// - **Compile-Time Safety**: Ensures all handlers implement `ToolHandler` trait
#[macro_export]
macro_rules! register_handlers_with_logging {
    ($registry:expr, { $($handler:ident => $description:expr),* $(,)? }) => {
        {
            use std::sync::Arc;
            use tracing::debug;
            $(
                let handler = Arc::new($handler::new());
                let handler_name = stringify!($handler);
                $registry.register_with_name(handler, handler_name);
                debug!("Registered {}", $description);
            )*
        }
    };
}

/// Delegate a tool call to a legacy handler with automatic context conversion
///
/// This macro eliminates the boilerplate of converting from the new `compat::ToolContext`
/// to the legacy `ToolContext` format when delegating to wrapped legacy handlers.
///
/// # Example
///
/// ```text
/// async fn handle_tool(&self, tool_call: ToolCall, context: &compat::ToolContext) -> ServerResult<Value> {
///     delegate_to_legacy!(self, context, tool_call)
/// }
/// ```
///
/// This expands to:
/// ```text
/// {
///     let legacy_context = ToolContext {
///         app_state: context.app_state.clone(),
///         plugin_manager: context.plugin_manager.clone(),
///         lsp_adapter: context.lsp_adapter.clone(),
///     };
///     self.legacy_handler.handle_tool(tool_call.clone(), &legacy_context).await
/// }
/// ```
#[macro_export]
macro_rules! delegate_to_legacy {
    ($self:expr, $context:expr, $tool_call:expr) => {{
        let legacy_context = $crate::handlers::compat::ToolContext {
            app_state: $context.app_state.clone(),
            plugin_manager: $context.plugin_manager.clone(),
            lsp_adapter: $context.lsp_adapter.clone(),
        };
        $self
            .legacy_handler
            .handle_tool($tool_call.clone(), &legacy_context)
            .await
    }};
}
