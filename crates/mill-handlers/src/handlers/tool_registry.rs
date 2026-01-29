//! Tool handler registry
//!
//! Central registry for all tool handlers with automatic routing based on tool names.
//! Only the Magnificent Seven tools are registered - no legacy or internal tools.

use super::tools::ToolHandler;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, warn};

/// Registry for tool handlers providing automatic routing
///
/// The ToolRegistry maintains a mapping from tool names to their handlers,
/// enabling automatic dispatch without hardcoded routing logic.
///
/// Only the Magnificent Seven tools are supported:
/// - inspect_code, search_code, rename_all, relocate, prune, refactor, workspace
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
    pub fn register_with_name(&mut self, handler: Arc<dyn ToolHandler>, handler_name: &str) {
        for tool_name in handler.tool_names() {
            debug!(
                tool_name = %tool_name,
                handler_name = %handler_name,
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
        }
    }

    /// Route a tool call to the appropriate handler
    pub async fn handle_tool(
        &self,
        tool_call: ToolCall,
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<Value> {
        if let Some(handler) = self.handlers.get(&tool_call.name) {
            handler.handle_tool_call(context, &tool_call).await
        } else {
            Err(ServerError::not_supported(format!(
                "Unknown tool: '{}'. Available tools: inspect_code, search_code, rename_all, relocate, prune, refactor, workspace",
                tool_call.name
            )))
        }
    }

    /// Check if a tool is registered
    pub fn has_tool(&self, tool_name: &str) -> bool {
        self.handlers.contains_key(tool_name)
    }

    /// Get all registered tool names
    pub fn list_tools(&self) -> Vec<String> {
        let mut tools: Vec<String> = self.handlers.keys().cloned().collect();
        tools.sort();
        tools
    }

    /// Get all registered tools with their handler information
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
    use crate::handlers::tools::ToolHandler;
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
            _context: &mill_handler_api::ToolHandlerContext,
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
}
