//! WebSocket transport implementation

use crate::McpDispatcher;
use cb_api::{ApiError, ApiResult};
use cb_core::config::AppConfig;
use cb_core::model::mcp::{McpError, McpMessage, McpRequest, McpResponse};
use futures_util::{SinkExt, StreamExt};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};

/// JWT Claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Claims {
    exp: usize,
    iat: usize,
    project_id: Option<String>,
}

/// Initialize message payload structure
#[derive(Debug, Deserialize)]
struct InitializePayload {
    /// Optional JWT token for authentication
    token: Option<String>,
    /// Project identifier
    project: Option<String>,
    /// Project root directory
    #[serde(rename = "projectRoot")]
    project_root: Option<String>,
}

/// Initialize response structure
#[derive(Debug, Serialize)]
struct InitializeResponse {
    /// Session identifier
    #[serde(rename = "sessionId")]
    session_id: String,
    /// Server capabilities
    capabilities: ServerCapabilities,
}

/// Server capabilities structure
#[derive(Debug, Serialize)]
struct ServerCapabilities {
    /// Supported tool methods
    tools: Vec<String>,
}

/// WebSocket connection session
#[derive(Debug)]
pub struct Session {
    /// Unique session identifier
    pub id: String,
    /// Project identifier
    pub project_id: Option<String>,
    /// Project root directory
    pub project_root: Option<String>,
    /// Whether the session is initialized
    pub initialized: bool,
}

impl Session {
    fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: None,
            project_root: None,
            initialized: false,
        }
    }
}

/// Simple JWT validation function
fn validate_token_with_project(
    token: &str,
    secret: &str,
    project_id: &str,
) -> Result<bool, String> {
    let key = DecodingKey::from_secret(secret.as_bytes());
    let mut validation = Validation::default();
    // Don't require aud claim
    validation.validate_aud = false;

    match decode::<Claims>(token, &key, &validation) {
        Ok(token_data) => {
            let claims = token_data.claims;

            // Check if project matches (if specified in claims)
            if let Some(token_project) = claims.project_id {
                if token_project != project_id {
                    return Err(format!(
                        "Token project '{}' does not match expected project '{}'",
                        token_project, project_id
                    ));
                }
            }

            Ok(true)
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Start the WebSocket server
pub async fn start_ws_server(
    config: Arc<AppConfig>,
    dispatcher: Arc<dyn McpDispatcher>,
) -> ApiResult<()> {
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| ApiError::bootstrap(format!("Failed to bind to {}: {}", addr, e)))?;

    tracing::info!("WebSocket server listening on {}", addr);

    while let Ok((stream, addr)) = listener.accept().await {
        tracing::debug!(
            client_addr = %addr,
            "New connection"
        );
        let config = config.clone();
        let dispatcher = dispatcher.clone();
        tokio::spawn(handle_connection(stream, config, dispatcher));
    }

    Ok(())
}

/// Handle a single WebSocket connection
async fn handle_connection(
    stream: TcpStream,
    config: Arc<AppConfig>,
    dispatcher: Arc<dyn McpDispatcher>,
) {
    let addr = stream
        .peer_addr()
        .unwrap_or_else(|_| "unknown".parse().unwrap());

    // Perform WebSocket handshake
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            tracing::error!(
                client_addr = %addr,
                error = %e,
                "WebSocket handshake failed"
            );
            return;
        }
    };

    tracing::info!("WebSocket connection established");
    let (mut write, mut read) = ws_stream.split();
    let mut session = Session::new();

    // Message processing loop with idle timeout
    const IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300); // 5 minutes
    loop {
        let msg = match tokio::time::timeout(IDLE_TIMEOUT, read.next()).await {
            Ok(Some(msg)) => msg,
            Ok(None) => {
                tracing::info!("WebSocket connection closed by client");
                break;
            }
            Err(_) => {
                tracing::warn!(
                    client_addr = %addr,
                    timeout_secs = 300,
                    "WebSocket connection idle timeout, closing connection"
                );
                break;
            }
        };
        match msg {
            Ok(Message::Text(text)) => {
                let request_id = uuid::Uuid::new_v4();
                tracing::debug!(
                    request_id = %request_id,
                    message_size = text.len(),
                    "Received message"
                );

                // Parse MCP message
                let mcp_message: McpMessage = match serde_json::from_str(&text) {
                    Ok(msg) => msg,
                    Err(e) => {
                        tracing::error!(
                            request_id = %request_id,
                            error = %e,
                            "Failed to parse MCP message"
                        );
                        let error_response = json!({
                            "error": {
                                "code": -32700,
                                "message": "Parse error"
                            }
                        });
                        if let Err(e) = write.send(Message::Text(error_response.to_string().into())).await
                        {
                            tracing::error!("Failed to send error response: {}", e);
                            break;
                        }
                        continue;
                    }
                };

                // Handle the message
                let response =
                    match handle_message(&mut session, mcp_message, &config, dispatcher.as_ref())
                        .await
                    {
                        Ok(response) => response,
                        Err(e) => {
                            // Convert to structured API error
                            let api_error = e.to_api_response();

                            tracing::error!(
                                request_id = %request_id,
                                error_code = %api_error.code,
                                error = %e,
                                "Failed to handle message"
                            );

                            // Serialize the structured error to JSON for the data field
                            let error_data = serde_json::to_value(&api_error).ok();

                            McpMessage::Response(McpResponse {
                                jsonrpc: "2.0".to_string(),
                                id: None,
                                result: None,
                                error: Some(McpError {
                                    code: -1,
                                    message: api_error.message.clone(),
                                    data: error_data,
                                }),
                            })
                        }
                    };

                // Send response
                let response_text = match serde_json::to_string(&response) {
                    Ok(text) => text,
                    Err(e) => {
                        tracing::error!(
                            request_id = %request_id,
                            error = %e,
                            "Failed to serialize response"
                        );
                        continue;
                    }
                };

                if let Err(e) = write.send(Message::Text(response_text.into())).await {
                    tracing::error!(
                        request_id = %request_id,
                        error = %e,
                        "Failed to send response"
                    );
                    break;
                }
            }
            Ok(Message::Close(_)) => {
                tracing::info!("WebSocket connection closed by client");
                break;
            }
            Ok(_) => {
                // Ignore other message types (binary, ping, pong)
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "WebSocket error"
                );
                break;
            }
        }
    }

    tracing::info!("WebSocket connection closed");
}

/// Handle a single MCP message
async fn handle_message(
    session: &mut Session,
    message: McpMessage,
    config: &AppConfig,
    dispatcher: &dyn McpDispatcher,
) -> ApiResult<McpMessage> {
    match message {
        McpMessage::Request(request) => {
            if request.method == "initialize" {
                handle_initialize(session, request, config).await
            } else if !session.initialized {
                // Reject non-initialize requests before initialization
                Ok(McpMessage::Response(McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(McpError {
                        code: -1,
                        message: "Session not initialized. Send initialize message first."
                            .to_string(),
                        data: None,
                    }),
                }))
            } else {
                // Dispatch to regular handler
                dispatcher.dispatch(McpMessage::Request(request)).await
            }
        }
        other => {
            // Forward other message types to dispatcher
            dispatcher.dispatch(other).await
        }
    }
}

/// Handle initialize request
async fn handle_initialize(
    session: &mut Session,
    request: McpRequest,
    config: &AppConfig,
) -> ApiResult<McpMessage> {
    // Parse initialize payload
    let payload: InitializePayload = if let Some(params) = request.params {
        serde_json::from_value(params)
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid initialize params: {}", e)))?
    } else {
        InitializePayload {
            token: None,
            project: None,
            project_root: None,
        }
    };

    // Validate authentication if required
    if let Some(auth_config) = &config.server.auth {
        if let Some(token) = &payload.token {
            let project_id = payload.project.as_deref().unwrap_or("default");

            validate_token_with_project(token, &auth_config.jwt_secret, project_id)
                .map_err(|e| ApiError::Auth(format!("Authentication failed: {}", e)))?;

            tracing::info!("Authentication successful for project: {}", project_id);
        } else {
            return Ok(McpMessage::Response(McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(McpError {
                    code: -32000,
                    message: "Authentication required: No token provided".to_string(),
                    data: None,
                }),
            }));
        }
    }

    // Update session
    session.project_id = payload.project;
    session.project_root = payload.project_root;
    session.initialized = true;

    tracing::info!(
        "Session {} initialized for project: {:?}",
        session.id,
        session.project_id
    );

    // Create response
    let response = InitializeResponse {
        session_id: session.id.clone(),
        capabilities: ServerCapabilities {
            tools: vec![
                "find_definition".to_string(),
                "find_references".to_string(),
                "rename_symbol".to_string(),
                "get_completions".to_string(),
                "get_diagnostics".to_string(),
                "format_document".to_string(),
            ],
        },
    };

    Ok(McpMessage::Response(McpResponse {
        jsonrpc: "2.0".to_string(),
        id: request.id,
        result: Some(serde_json::to_value(response)?),
        error: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cb_core::config::{AuthConfig, CacheConfig, LoggingConfig, LspConfig, ServerConfig};

    fn create_test_config(with_auth: bool) -> AppConfig {
        AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3040,
                max_clients: Some(10),
                timeout_ms: 30000,
                tls: None,
                auth: if with_auth {
                    Some(AuthConfig {
                        jwt_secret: "test_secret".to_string(),
                        jwt_expiry_seconds: 3600,
                        jwt_issuer: "test".to_string(),
                        jwt_audience: "test".to_string(),
                    })
                } else {
                    None
                },
            },
            lsp: LspConfig::default(),
            fuse: None,
            logging: LoggingConfig::default(),
            cache: CacheConfig::default(),
        }
    }

    #[tokio::test]
    async fn test_initialize_with_auth_missing_token() {
        let config = create_test_config(true);
        let mut session = Session::new();

        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(serde_json::Number::from(1))),
            method: "initialize".to_string(),
            params: Some(json!({
                "project": "test_project"
            })),
        };

        let response = handle_initialize(&mut session, request, &config)
            .await
            .unwrap();

        assert!(!session.initialized);

        if let McpMessage::Response(resp) = response {
            assert!(resp.result.is_none());
            assert!(resp.error.is_some());
        } else {
            panic!("Expected Response message");
        }
    }
}
