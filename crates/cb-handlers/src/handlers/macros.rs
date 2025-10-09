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
