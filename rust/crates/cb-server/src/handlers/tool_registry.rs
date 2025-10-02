//! Tool handler registry
//!
//! Central registry for all tool handlers with automatic routing based on tool names.

use super::tool_handler::{ToolContext, ToolHandler};
use crate::{ServerError, ServerResult};
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
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a tool handler
    ///
    /// All tools returned by `handler.supported_tools()` will be registered.
    /// If a tool name is already registered, it will be replaced and a warning logged.
    ///
    /// # Arguments
    ///
    /// * `handler` - The handler to register
    pub fn register(&mut self, handler: Arc<dyn ToolHandler>) {
        for tool_name in handler.supported_tools() {
            debug!(tool_name = %tool_name, "Registering tool handler");
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
    pub async fn handle_tool(
        &self,
        tool_call: ToolCall,
        context: &ToolContext,
    ) -> ServerResult<Value> {
        if let Some(handler) = self.handlers.get(&tool_call.name) {
            handler.handle_tool(tool_call, context).await
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
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;

    struct TestHandler {
        tools: Vec<&'static str>,
    }

    #[async_trait]
    impl ToolHandler for TestHandler {
        fn supported_tools(&self) -> Vec<&'static str> {
            self.tools.clone()
        }

        async fn handle_tool(
            &self,
            tool_call: ToolCall,
            _context: &ToolContext,
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
