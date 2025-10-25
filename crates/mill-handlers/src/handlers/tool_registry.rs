//! Tool handler registry
//!
//! Central registry for all tool handlers with automatic routing based on tool names.

use super::tools::{ToolHandler, ToolHandlerContext};
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::protocol::{ ApiError as ServerError , ApiResult as ServerResult };
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, warn};

/// Registry for tool handlers providing automatic routing
///
/// The ToolRegistry maintains a mapping from tool names to their handlers,
/// enabling automatic dispatch without hardcoded routing logic.
///
/// # Internal Tools
///
/// Tools marked as "internal" (via `ToolHandler::is_internal()`) are:
/// - Hidden from `list_tools()` (MCP tool discovery)
/// - Still callable via `handle_tool()` (for backend use)
/// - Documented in `list_internal_tools()` for system visibility
pub struct ToolRegistry {
    /// Map from tool name to handler
    handlers: HashMap<String, Arc<dyn ToolHandler>>,
    /// Map from tool name to handler type name (for diagnostics)
    handler_names: HashMap<String, String>,
    /// Set of internal tool names (hidden from MCP listings)
    internal_tools: std::collections::HashSet<String>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            handler_names: HashMap::new(),
            internal_tools: std::collections::HashSet::new(),
        }
    }

    /// Register a tool handler with its type name for diagnostics
    ///
    /// All tools returned by `handler.tool_names()` will be registered.
    /// If a tool name is already registered, it will be replaced and a warning logged.
    ///
    /// # Arguments
    ///
    /// * `handler` - The handler to register
    /// * `handler_name` - The name of the handler type (e.g., "SystemHandler")
    pub fn register_with_name(&mut self, handler: Arc<dyn ToolHandler>, handler_name: &str) {
        let is_internal = handler.is_internal();

        for tool_name in handler.tool_names() {
            debug!(
                tool_name = %tool_name,
                handler_name = %handler_name,
                is_internal = %is_internal,
                "Registering tool handler"
            );

            if self
                .handlers
                .insert(tool_name.to_string(), handler.clone())
                .is_some()
            {
                warn!(
                    tool_name = %tool_name,
                    "Tool handler replaced (duplicate registration)"
                );
            }

            self.handler_names
                .insert(tool_name.to_string(), handler_name.to_string());

            // Track internal tools separately
            if is_internal {
                self.internal_tools.insert(tool_name.to_string());
            }
        }
    }

    /// Route a tool call to the appropriate handler
    ///
    /// # Arguments
    ///
    /// * `tool_call` - The tool call to handle
    /// * `context` - Context providing access to application services
    ///
    /// # Returns
    ///
    /// Returns the tool result on success, or an error if no handler is found
    /// or the handler fails.
    ///
    /// # Errors
    ///
    /// Returns `InvalidRequest` if attempting to call an internal tool.
    /// Internal tools are backend-only and not accessible via CLI/MCP.
    pub async fn handle_tool(
        &self,
        tool_call: ToolCall,
        context: &ToolHandlerContext,
    ) -> ServerResult<Value> {
        // Block internal tools from external calls (CLI/MCP)
        if self.internal_tools.contains(&tool_call.name) {
            return Err(ServerError::InvalidRequest(format!(
                "Tool '{}' is internal and not available via CLI/MCP. Use the public API instead. \
                 Run 'mill tools' to see available public tools.",
                tool_call.name
            )));
        }

        if let Some(handler) = self.handlers.get(&tool_call.name) {
            handler.handle_tool_call(context, &tool_call).await
        } else {
            Err(ServerError::Unsupported(format!(
                "No handler for tool: {}",
                tool_call.name
            )))
        }
    }

    /// Check if a tool is registered
    ///
    /// # Arguments
    ///
    /// * `tool_name` - The tool name to check
    ///
    /// # Returns
    ///
    /// Returns `true` if the tool has a registered handler, `false` otherwise.
    pub fn has_tool(&self, tool_name: &str) -> bool {
        self.handlers.contains_key(tool_name)
    }

    /// Get all public (non-internal) registered tool names
    ///
    /// This method filters out internal tools and is used for MCP tool discovery.
    /// Internal tools are still callable via `handle_tool()` but hidden from
    /// AI agents and MCP clients.
    ///
    /// # Returns
    ///
    /// Returns a vector of public tool names, sorted alphabetically.
    pub fn list_tools(&self) -> Vec<String> {
        let mut tools: Vec<String> = self
            .handlers
            .keys()
            .filter(|name| !self.internal_tools.contains(*name))
            .cloned()
            .collect();
        tools.sort();
        tools
    }

    /// Get all internal (hidden) tool names
    ///
    /// Internal tools are hidden from MCP listings but still callable for backend use.
    /// This method is useful for diagnostics and documentation.
    ///
    /// # Returns
    ///
    /// Returns a vector of internal tool names, sorted alphabetically.
    pub fn list_internal_tools(&self) -> Vec<String> {
        let mut tools: Vec<String> = self.internal_tools.iter().cloned().collect();
        tools.sort();
        tools
    }

    /// Get all registered tools with their handler information
    ///
    /// Returns a mapping of tool names to handler type names for diagnostics
    /// and CLI tools. This is useful for the `mill list-tools` command.
    ///
    /// # Returns
    ///
    /// Returns a vector of `(tool_name, handler_type)` tuples, sorted by tool name.
    ///
    /// # Example Output
    ///
    /// ```text
    /// [
    ///     ("find_definition", "NavigationHandler"),
    ///     ("health_check", "SystemHandler"),
    ///     ("read_file", "FileOpsHandler"),
    /// ]
    /// ```
    pub fn list_tools_with_handlers(&self) -> Vec<(String, String)> {
        let mut result: Vec<(String, String)> = self
            .handlers
            .keys()
            .map(|tool_name| {
                let handler_name = self
                    .handler_names
                    .get(tool_name)
                    .cloned()
                    .unwrap_or_else(|| "UnknownHandler".to_string());
                (tool_name.clone(), handler_name)
            })
            .collect();

        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    }

    /// Get all public (non-internal) tools with their handler information
    ///
    /// Like `list_tools_with_handlers()` but filters out internal tools.
    /// Used by CLI commands to show only public tools to users.
    ///
    /// # Returns
    ///
    /// Returns a vector of `(tool_name, handler_type)` tuples for public tools only,
    /// sorted alphabetically by tool name.
    ///
    /// # Example Output
    ///
    /// ```text
    /// [
    ///     ("find_definition", "NavigationHandler"),
    ///     ("health_check", "SystemHandler"),
    ///     ("rename", "RenameHandler"),
    /// ]
    /// ```
    pub fn list_public_tools_with_handlers(&self) -> Vec<(String, String)> {
        let mut result: Vec<(String, String)> = self
            .handlers
            .keys()
            .filter(|name| !self.internal_tools.contains(*name)) // Filter out internal
            .map(|tool_name| {
                let handler_name = self
                    .handler_names
                    .get(tool_name)
                    .cloned()
                    .unwrap_or_else(|| "UnknownHandler".to_string());
                (tool_name.clone(), handler_name)
            })
            .collect();

        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::tools::{ToolHandler, ToolHandlerContext};
    use async_trait::async_trait;
    use serde_json::json;

    struct TestHandler {
        tools: Vec<&'static str>,
    }

    #[async_trait]
    impl ToolHandler for TestHandler {
        fn tool_names(&self) -> &[&str] {
            &self.tools
        }

        async fn handle_tool_call(
            &self,
            _context: &ToolHandlerContext,
            tool_call: &ToolCall,
        ) -> ServerResult<Value> {
            Ok(json!({
                "tool": tool_call.name,
                "handled": true
            }))
        }
    }

    #[test]
    fn test_registry_registration() {
        let mut registry = ToolRegistry::new();
        let handler = Arc::new(TestHandler {
            tools: vec!["tool1", "tool2"],
        });

        registry.register_with_name(handler, "TestHandler");

        assert!(registry.has_tool("tool1"));
        assert!(registry.has_tool("tool2"));
        assert!(!registry.has_tool("tool3"));
    }

    #[test]
    fn test_list_tools() {
        let mut registry = ToolRegistry::new();
        let handler1 = Arc::new(TestHandler {
            tools: vec!["b_tool", "a_tool"],
        });
        let handler2 = Arc::new(TestHandler {
            tools: vec!["c_tool"],
        });

        registry.register_with_name(handler1, "TestHandler1");
        registry.register_with_name(handler2, "TestHandler2");

        let tools = registry.list_tools();
        assert_eq!(tools, vec!["a_tool", "b_tool", "c_tool"]);
    }

    #[test]
    fn test_list_public_tools_with_handlers() {
        let mut registry = ToolRegistry::new();

        // Register public handler
        struct PublicHandler;
        #[async_trait]
        impl ToolHandler for PublicHandler {
            fn tool_names(&self) -> &[&str] {
                &["public_tool"]
            }
            fn is_internal(&self) -> bool {
                false
            }
            async fn handle_tool_call(
                &self,
                _context: &ToolHandlerContext,
                _tool_call: &ToolCall,
            ) -> ServerResult<Value> {
                Ok(json!({}))
            }
        }

        // Register internal handler
        struct InternalHandler;
        #[async_trait]
        impl ToolHandler for InternalHandler {
            fn tool_names(&self) -> &[&str] {
                &["internal_tool"]
            }
            fn is_internal(&self) -> bool {
                true
            }
            async fn handle_tool_call(
                &self,
                _context: &ToolHandlerContext,
                _tool_call: &ToolCall,
            ) -> ServerResult<Value> {
                Ok(json!({}))
            }
        }

        registry.register_with_name(Arc::new(PublicHandler), "PublicHandler");
        registry.register_with_name(Arc::new(InternalHandler), "InternalHandler");

        let public_tools = registry.list_public_tools_with_handlers();

        // Should only include public tool
        assert_eq!(public_tools.len(), 1);
        assert_eq!(public_tools[0].0, "public_tool");
        assert_eq!(public_tools[0].1, "PublicHandler");

        // Internal tool should not appear
        assert!(!public_tools.iter().any(|(name, _)| name == "internal_tool"));

        // Verify internal tool is still registered in the system
        assert!(registry.has_tool("internal_tool"));

        // Verify list_tools_with_handlers still shows both
        let all_tools = registry.list_tools_with_handlers();
        assert_eq!(all_tools.len(), 2);
    }
}