use crate::client_config::ClientConfig;
use crate::error::{ClientError, ClientResult};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};
use url::Url;

// Type alias for complex type
type PendingRequests = Arc<Mutex<HashMap<String, oneshot::Sender<ClientResult<MCPResponse>>>>>;

/// MCP request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPRequest {
    pub id: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

/// MCP response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPResponse {
    pub id: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<MCPError>,
}

/// MCP error structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Connection state
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Authenticating,
    Authenticated,
    Reconnecting,
    Failed,
}

/// WebSocket client for MCP communication
pub struct WebSocketClient {
    config: ClientConfig,
    state: Arc<Mutex<ConnectionState>>,
    next_id: AtomicU64,
    pending_requests: Arc<Mutex<HashMap<String, oneshot::Sender<ClientResult<MCPResponse>>>>>,
    connection: Arc<Mutex<Option<Connection>>>,
}

/// Internal connection wrapper
struct Connection {
    sender: mpsc::UnboundedSender<Message>,
    _handle: tokio::task::JoinHandle<()>,
}

impl WebSocketClient {
    /// Create a new WebSocket client
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            next_id: AtomicU64::new(1),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            connection: Arc::new(Mutex::new(None)),
        }
    }

    /// Connect to the WebSocket server
    pub async fn connect(&self) -> ClientResult<()> {
        let url = self.config.get_url()?;

        {
            let mut state = self.state.lock().await;
            if *state == ConnectionState::Connected || *state == ConnectionState::Authenticated {
                return Ok(());
            }
            *state = ConnectionState::Connecting;
        }

        info!(url = %url, "Connecting to server");

        // Parse URL and establish connection
        let url = Url::parse(url)
            .map_err(|e| ClientError::ConnectionError(format!("Invalid URL: {}", e)))?;

        let (ws_stream, _) = connect_async(url.as_str())
            .await
            .map_err(|e| ClientError::ConnectionError(format!("Failed to connect: {}", e)))?;

        info!("WebSocket connection established");

        // Split the stream for reading and writing
        let (mut write, mut read) = ws_stream.split();

        // Create a channel for sending messages
        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

        // Clone Arcs for the background tasks
        let state_clone = Arc::clone(&self.state);
        let pending_requests_clone = Arc::clone(&self.pending_requests);

        // Spawn write task
        let write_handle = {
            let state = Arc::clone(&state_clone);
            tokio::spawn(async move {
                while let Some(message) = rx.recv().await {
                    if let Err(e) = write.send(message).await {
                        error!(error = %e, "Failed to send message");
                        break;
                    }
                }
                debug!("Write task ending");
                let mut state = state.lock().await;
                if *state == ConnectionState::Connected || *state == ConnectionState::Authenticated
                {
                    *state = ConnectionState::Disconnected;
                }
            })
        };

        // Spawn read task
        let read_handle = {
            let state = Arc::clone(&state_clone);
            let pending_requests = Arc::clone(&pending_requests_clone);
            tokio::spawn(async move {
                while let Some(message) = read.next().await {
                    match message {
                        Ok(Message::Text(text)) => {
                            if let Err(e) = Self::handle_message(&text, &pending_requests).await {
                                warn!(error = %e, "Failed to handle message");
                            }
                        }
                        Ok(Message::Close(_)) => {
                            info!("WebSocket connection closed by server");
                            break;
                        }
                        Err(e) => {
                            error!(error = %e, "WebSocket error");
                            break;
                        }
                        _ => {}
                    }
                }
                debug!("Read task ending");
                let mut state = state.lock().await;
                if *state == ConnectionState::Connected || *state == ConnectionState::Authenticated
                {
                    *state = ConnectionState::Disconnected;
                }
            })
        };

        // Combine both handles
        let combined_handle = tokio::spawn(async move {
            tokio::select! {
                _ = write_handle => {},
                _ = read_handle => {},
            }
        });

        // Store the connection
        {
            let mut connection = self.connection.lock().await;
            *connection = Some(Connection {
                sender: tx,
                _handle: combined_handle,
            });
        }

        // Update state to connected
        {
            let mut state = self.state.lock().await;
            *state = ConnectionState::Connected;
        }

        // Authenticate if token is available
        if self.config.has_token() {
            self.authenticate().await?;
        }

        Ok(())
    }

    /// Authenticate with the server using JWT token
    async fn authenticate(&self) -> ClientResult<()> {
        let token = self.config.get_token().ok_or_else(|| {
            ClientError::AuthError("No authentication token configured".to_string())
        })?;

        {
            let mut state = self.state.lock().await;
            *state = ConnectionState::Authenticating;
        }

        debug!("Authenticating with server");

        // Send authentication request
        let auth_request = MCPRequest {
            id: self.generate_id(),
            method: "auth".to_string(),
            params: Some(serde_json::json!({ "token": token })),
        };

        let response = self.send_request(auth_request).await?;

        if response.error.is_some() {
            let mut state = self.state.lock().await;
            *state = ConnectionState::Connected;
            return Err(ClientError::AuthError("Authentication failed".to_string()));
        }

        {
            let mut state = self.state.lock().await;
            *state = ConnectionState::Authenticated;
        }

        info!("Successfully authenticated");
        Ok(())
    }

    /// Send an MCP request and wait for response
    pub async fn send_request(&self, request: MCPRequest) -> ClientResult<MCPResponse> {
        // Check connection state
        {
            let state = self.state.lock().await;
            match *state {
                ConnectionState::Disconnected | ConnectionState::Failed => {
                    return Err(ClientError::ConnectionError("Not connected".to_string()));
                }
                ConnectionState::Connecting | ConnectionState::Reconnecting => {
                    return Err(ClientError::ConnectionError(
                        "Connection in progress".to_string(),
                    ));
                }
                _ => {}
            }
        }

        let timeout_duration = Duration::from_millis(self.config.get_timeout_ms());
        let request_id = request.id.clone();

        // Create a oneshot channel for the response
        let (tx, rx) = oneshot::channel();

        // Register the pending request
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(request_id.clone(), tx);
        }

        // Send the request
        let message = serde_json::to_string(&request).map_err(|e| {
            ClientError::SerializationError(format!("Failed to serialize request: {}", e))
        })?;

        {
            let connection = self.connection.lock().await;
            if let Some(conn) = connection.as_ref() {
                if let Err(e) = conn.sender.send(Message::Text(message.into())) {
                    // Clean up pending request
                    let mut pending = self.pending_requests.lock().await;
                    pending.remove(&request_id);
                    return Err(ClientError::ConnectionError(format!(
                        "Failed to send message: {}",
                        e
                    )));
                }
            } else {
                // Clean up pending request
                let mut pending = self.pending_requests.lock().await;
                pending.remove(&request_id);
                return Err(ClientError::ConnectionError(
                    "No active connection".to_string(),
                ));
            }
        }

        // Wait for response with timeout
        match timeout(timeout_duration, rx).await {
            Ok(Ok(response)) => response,
            Ok(Err(_)) => {
                // Clean up pending request
                let mut pending = self.pending_requests.lock().await;
                pending.remove(&request_id);
                Err(ClientError::RequestError("Request cancelled".to_string()))
            }
            Err(_) => {
                // Clean up pending request
                let mut pending = self.pending_requests.lock().await;
                pending.remove(&request_id);
                Err(ClientError::TimeoutError("Request timed out".to_string()))
            }
        }
    }

    /// Call an MCP tool
    pub async fn call_tool(
        &self,
        tool: &str,
        params: Option<serde_json::Value>,
    ) -> ClientResult<MCPResponse> {
        // Format as MCP tools/call request
        let tool_params = serde_json::json!({
            "name": tool,
            "arguments": params.unwrap_or(serde_json::json!({}))
        });

        let request = MCPRequest {
            id: self.generate_id(),
            method: "tools/call".to_string(),
            params: Some(tool_params),
        };

        self.send_request(request).await
    }

    /// Get connection state
    pub async fn get_state(&self) -> ConnectionState {
        let state = self.state.lock().await;
        state.clone()
    }

    /// Check if connected and authenticated
    pub async fn is_ready(&self) -> bool {
        let state = self.state.lock().await;
        matches!(
            *state,
            ConnectionState::Connected | ConnectionState::Authenticated
        )
    }

    /// Disconnect from the server
    pub async fn disconnect(&self) -> ClientResult<()> {
        info!("Disconnecting from server");

        {
            let mut state = self.state.lock().await;
            *state = ConnectionState::Disconnected;
        }

        // Close connection
        {
            let mut connection = self.connection.lock().await;
            if let Some(conn) = connection.take() {
                let _ = conn.sender.send(Message::Close(None));
                // The connection will be cleaned up when the task ends
            }
        }

        // Cancel all pending requests
        {
            let mut pending = self.pending_requests.lock().await;
            for (_, sender) in pending.drain() {
                let _ = sender.send(Err(ClientError::ConnectionError(
                    "Connection closed".to_string(),
                )));
            }
        }

        Ok(())
    }

    /// Generate a unique request ID
    fn generate_id(&self) -> String {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        format!("req_{}", id)
    }

    /// Handle incoming message
    async fn handle_message(text: &str, pending_requests: &PendingRequests) -> ClientResult<()> {
        debug!(message = %text, "Received message");

        let response: MCPResponse = serde_json::from_str(text).map_err(|e| {
            ClientError::SerializationError(format!("Failed to parse response: {}", e))
        })?;

        // Find and complete the pending request
        let mut pending = pending_requests.lock().await;
        if let Some(sender) = pending.remove(&response.id) {
            let _ = sender.send(Ok(response));
        } else {
            warn!(request_id = %response.id, "Received response for unknown request ID");
        }

        Ok(())
    }

    /// Ping the server to check connectivity
    pub async fn ping(&self) -> ClientResult<Duration> {
        let start = std::time::Instant::now();

        let request = MCPRequest {
            id: self.generate_id(),
            method: "ping".to_string(),
            params: None,
        };

        self.send_request(request).await?;
        Ok(start.elapsed())
    }

    /// Get server capabilities
    pub async fn get_capabilities(&self) -> ClientResult<serde_json::Value> {
        let request = MCPRequest {
            id: self.generate_id(),
            method: "capabilities".to_string(),
            params: None,
        };

        let response = self.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(ClientError::RequestError(format!(
                "Server error: {}",
                error.message
            )));
        }

        Ok(response.result.unwrap_or_default())
    }
}

impl Drop for WebSocketClient {
    fn drop(&mut self) {
        // Note: We can't call async methods in Drop, but the connection tasks
        // will clean themselves up when the channels are dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_request_serialization() {
        let request = MCPRequest {
            id: "test_id".to_string(),
            method: "inspect_code".to_string(),
            params: Some(serde_json::json!({
                "filePath": "src/main.rs",
                "symbolName": "main",
                "include": ["definition"]
            })),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: MCPRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request.id, deserialized.id);
        assert_eq!(request.method, deserialized.method);
        assert_eq!(request.params, deserialized.params);
    }

    #[test]
    fn test_mcp_response_serialization() {
        let response = MCPResponse {
            id: "test_id".to_string(),
            result: Some(serde_json::json!({"status": "success"})),
            error: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: MCPResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response.id, deserialized.id);
        assert_eq!(response.result, deserialized.result);
        assert!(response.error.is_none());
    }

    #[test]
    fn test_generate_unique_ids() {
        let config = ClientConfig::new();
        let client = WebSocketClient::new(config);

        let id1 = client.generate_id();
        let id2 = client.generate_id();
        let id3 = client.generate_id();

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }
}
