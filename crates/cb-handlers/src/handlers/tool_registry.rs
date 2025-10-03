//! Tool handler registry
//!
//! Central registry for all tool handlers with automatic routing based on tool names.

use super::tools::{ToolHandler, ToolHandlerContext};
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
use cb_core::model::mcp::ToolCall;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, warn};

/// Registry for tool handlers providing automatic routing
///
/// The ToolRegistry maintains a mapping from tool names to their handlers,
/// enabling automatic dispatch without hardcoded routing logic.
pub struct ToolRegistry {
    /// Map from tool name to handler
    handlers: HashMap<String, Arc<dyn ToolHandler>>,
    /// Map from tool name to handler type name (for diagnostics)
    handler_names: HashMap<String, String>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            handler_names: HashMap::new(),
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
        for tool_name in handler.tool_names() {
            debug!(tool_name = %tool_name, handler_name = %handler_name, "Registering tool handler");
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
            self.handler_names.insert(tool_name.to_string(), handler_name.to_string());
        }
    }

    /// Register a tool handler (legacy method for backward compatibility)
    ///
    /// All tools returned by `handler.tool_names()` will be registered.
    /// If a tool name is already registered, it will be replaced and a warning logged.
    ///
    /// # Arguments
    ///
    /// * `handler` - The handler to register
    pub fn register(&mut self, handler: Arc<dyn ToolHandler>) {
        self.register_with_name(handler, "UnknownHandler")
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
    pub async fn handle_tool(
        &self,
        tool_call: ToolCall,
        context: &ToolHandlerContext,
    ) -> ServerResult<Value> {
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

    /// Get all registered tool names
    ///
    /// # Returns
    ///
    /// Returns a vector of all registered tool names, sorted alphabetically.
    pub fn list_tools(&self) -> Vec<String> {
        let mut tools: Vec<String> = self.handlers.keys().cloned().collect();
        tools.sort();
        tools
    }

    /// Get all registered tools with their handler information
    ///
    /// Returns a mapping of tool names to handler type names for diagnostics
    /// and CLI tools. This is useful for the `codebuddy list-tools` command.
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

        registry.register(handler);

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

        registry.register(handler1);
        registry.register(handler2);

        let tools = registry.list_tools();
        assert_eq!(tools, vec!["a_tool", "b_tool", "c_tool"]);
    }
}
