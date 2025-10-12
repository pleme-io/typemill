//! Transport layer implementations for WebSocket and stdio communication
//!
//! This crate provides transport mechanisms for the codebuddy server,
//! enabling communication via WebSocket (for production) and stdio
//! (for MCP clients like Claude Code).

use async_trait::async_trait;
use cb_core::model::mcp::McpMessage;
use cb_protocol::ApiResult;

pub mod admin;
pub mod session;
pub mod stdio;
pub mod ws;

pub use admin::start_admin_server;
pub use session::SessionInfo;
pub use stdio::start_stdio_server;
pub use ws::{start_ws_server, Session};

/// MCP message dispatcher trait for transport layer
#[async_trait]
pub trait McpDispatcher: Send + Sync {
    /// Dispatch an MCP message and return response, including session context.
    async fn dispatch(
        &self,
        message: McpMessage,
        session_info: &SessionInfo,
    ) -> ApiResult<McpMessage>;
}
