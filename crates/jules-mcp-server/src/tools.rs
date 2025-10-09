use anyhow::Result;
use async_trait::async_trait;
use jules_api::JulesClient;
use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// A trait for a tool that can be executed by the MCP server.
#[async_trait]
pub trait Tool: Send + Sync {
    async fn execute(&self, client: &JulesClient, params: Value) -> Result<Value, (i32, String)>;
}

/// A container for all registered tools.
pub struct ToolBox {
    client: JulesClient,
    tools: RwLock<HashMap<String, Arc<dyn Tool>>>,
}

impl ToolBox {
    /// Creates a new `ToolBox`.
    pub fn new(client: JulesClient) -> Self {
        Self {
            client,
            tools: RwLock::new(HashMap::new()),
        }
    }

    /// Adds a tool to the toolbox.
    pub fn add_tool(&self, name: &str, tool: Arc<dyn Tool>) {
        self.tools.write().insert(name.to_string(), tool);
    }

    /// Runs a tool by name with the given parameters.
    pub async fn run_tool(&self, name: &str, params: Value) -> Result<Value, (i32, String)> {
        let tool = {
            let tools = self.tools.read();
            tools.get(name).cloned()
        };

        if let Some(tool) = tool {
            tool.execute(&self.client, params).await
        } else {
            Err((-32601, format!("Tool not found: {}", name)))
        }
    }
}