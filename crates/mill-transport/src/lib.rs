//! Transport layer implementations for WebSocket and stdio communication
//!
//! This crate provides transport mechanisms for the mill server,
//! enabling communication via WebSocket (for production) and stdio
//! (for MCP clients like Claude Code).

use async_trait::async_trait;
use mill_foundation::core::model::mcp::McpMessage;
use mill_foundation::errors::MillResult;

pub mod admin;
pub mod session;
pub mod stdio;
#[cfg(unix)]
pub mod unix_socket;
pub mod ws;

pub use admin::start_admin_server;
pub use session::SessionInfo;
pub use stdio::start_stdio_server;
#[cfg(unix)]
pub use unix_socket::{default_socket_path, is_daemon_running, UnixSocketClient, UnixSocketServer};
pub use ws::{start_ws_server, Session};

/// MCP message dispatcher trait for transport layer
#[async_trait]
pub trait McpDispatcher: Send + Sync {
    /// Dispatch an MCP message and return response, including session context.
    async fn dispatch(
        &self,
        message: McpMessage,
        session_info: &SessionInfo,
    ) -> MillResult<McpMessage>;
}
