//! Defines the JSON-RPC protocol for communication between the core
//! and external language plugins.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A request sent from the core to a language plugin.
#[derive(Debug, Serialize, Deserialize)]
pub struct PluginRequest {
    /// A unique identifier for the request.
    pub id: u64,
    /// The method to be invoked on the plugin (e.g., "parse").
    pub method: String,
    /// The parameters for the method, as a JSON value.
    pub params: Value,
}

/// A response sent from a language plugin to the core.
#[derive(Debug, Serialize, Deserialize)]
pub struct PluginResponse {
    /// The identifier of the request this response corresponds to.
    pub id: u64,
    /// The successful result of the method invocation, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// The error that occurred during method invocation, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<PluginError>,
}

/// An error that occurred within a plugin.
#[derive(Debug, Serialize, Deserialize)]
pub struct PluginError {
    /// An error code.
    pub code: i32,
    /// A human-readable error message.
    pub message: String,
    /// Additional data related to the error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl PluginResponse {
    /// Creates a new error response.
    pub fn error(id: u64, code: i32, message: &str) -> Self {
        Self {
            id,
            result: None,
            error: Some(PluginError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }
}