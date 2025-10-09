use crate::{
    config::Config,
    handlers,
    mcp::{McpMessage, McpPayload, McpProtocol, StdioTransport},
    tools::ToolBox,
};
use anyhow::Result;
use jules_api::JulesClient;
use std::sync::Arc;
use tracing::{error, info, span, Level};

/// The main MCP server.
pub struct JulesMcpServer {
    toolbox: Arc<ToolBox>,
}

impl JulesMcpServer {
    /// Creates a new `JulesMcpServer`.
    pub fn new(config: Config) -> Result<Self> {
        let api_client = JulesClient::new(jules_api::Config::new(config.api_key.clone()));
        let toolbox = Arc::new(ToolBox::new(api_client));
        Self::register_tools(&toolbox);
        Ok(Self { toolbox })
    }

    /// Registers all available tools with the toolbox.
    fn register_tools(toolbox: &ToolBox) {
        toolbox.add_tool(
            "jules_list_sources",
            Arc::new(handlers::sources::ListSources),
        );
        toolbox.add_tool(
            "jules_get_source",
            Arc::new(handlers::sources::GetSource),
        );
        toolbox.add_tool(
            "jules_create_session",
            Arc::new(handlers::sessions::CreateSession),
        );
        toolbox.add_tool(
            "jules_list_sessions",
            Arc::new(handlers::sessions::ListSessions),
        );
        toolbox.add_tool(
            "jules_get_session",
            Arc::new(handlers::sessions::GetSession),
        );
        toolbox.add_tool(
            "jules_delete_session",
            Arc::new(handlers::sessions::DeleteSession),
        );
        toolbox.add_tool(
            "jules_list_activities",
            Arc::new(handlers::activities::ListActivities),
        );
        toolbox.add_tool(
            "jules_send_message",
            Arc::new(handlers::activities::SendMessage),
        );
        toolbox.add_tool(
            "jules_approve_plan",
            Arc::new(handlers::plans::ApprovePlan),
        );
    }

    /// Runs the main server loop.
    pub async fn run(&self) -> Result<()> {
        info!("Jules MCP server started. Listening for messages...");
        while let Some(msg) = StdioTransport::read_message().await? {
            let toolbox = self.toolbox.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::handle_message(msg, toolbox).await {
                    error!("Failed to handle message: {}", e);
                }
            });
        }
        info!("Input stream closed. Shutting down.");
        Ok(())
    }

    /// Handles a single incoming MCP message.
    async fn handle_message(msg: McpMessage, toolbox: Arc<ToolBox>) -> Result<()> {
        let span = span!(Level::INFO, "request", request_id = %msg.id);
        let _enter = span.enter();

        match msg.payload {
            McpPayload::Request(req) => {
                info!("Received tool call: {}", req.tool_name);
                let result = toolbox.run_tool(&req.tool_name, req.params).await;
                let response = McpProtocol::create_response(msg.id, result);
                StdioTransport::write_message(&response).await?;
            }
            _ => {
                // For now, we only handle requests.
                // Notifications and responses from the client are ignored.
                info!("Received non-request message. Ignoring.");
            }
        }
        Ok(())
    }
}