use crate::mcp::types::{McpMessage, McpPayload, McpResponse, McpError};
use anyhow::Result;
use serde_json::Value;

pub struct McpProtocol;

impl McpProtocol {
    pub fn serialize(message: &McpMessage) -> Result<String> {
        Ok(serde_json::to_string(message)?)
    }

    pub fn deserialize(data: &str) -> Result<McpMessage> {
        Ok(serde_json::from_str(data)?)
    }

    pub fn create_response(id: String, result: Result<Value, (i32, String)>) -> McpMessage {
        let payload = match result {
            Ok(value) => McpPayload::Response(McpResponse {
                result: Ok(value),
            }),
            Err((code, message)) => McpPayload::Response(McpResponse {
                result: Err(McpError {
                    code,
                    message,
                    data: None,
                }),
            }),
        };
        McpMessage { id, payload }
    }
}