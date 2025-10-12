//! WebSocket transport implementation

use crate::{McpDispatcher, SessionInfo};
use cb_core::{
    auth::jwt::{decode, Claims, DecodingKey, Validation},
    config::AppConfig,
    model::mcp::{McpError, McpMessage, McpRequest, McpResponse},
};
use cb_protocol::{ApiError, ApiResult};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        handshake::server::{Request, Response},
        http::{Response as HttpResponse, StatusCode},
        Message,
    },
};

/// Initialize message payload structure
#[derive(Debug, Deserialize)]
struct InitializePayload {
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
    /// The ID of the user for this session.
    pub user_id: Option<String>,
}

impl Session {
    fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: None,
            project_root: None,
            initialized: false,
            user_id: None,
        }
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

    let mut user_id_from_token: Option<String> = None;
    let config_clone = config.clone();

    // Perform WebSocket handshake with authorization header validation
    let ws_stream = match accept_hdr_async(stream, |req: &Request, response: Response| {
        // Check if authentication is required
        if let Some(auth_config) = &config_clone.server.auth {
            // Extract Authorization header
            let auth_header = req
                .headers()
                .get("Authorization")
                .and_then(|h| h.to_str().ok());

            if let Some(auth_value) = auth_header {
                // Check for Bearer token
                if let Some(token) = auth_value.strip_prefix("Bearer ") {
                    // Decode token to validate and extract user_id
                    let key = DecodingKey::from_secret(auth_config.jwt_secret.as_ref());
                    let mut validation = Validation::default();
                    validation.validate_aud = false;

                    match decode::<Claims>(token, &key, &validation) {
                        Ok(token_data) => {
                            tracing::debug!("WebSocket connection authenticated");
                            user_id_from_token = token_data.claims.user_id;
                            return Ok(response);
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "WebSocket connection rejected: token validation failed");
                        }
                    }
                } else {
                    tracing::warn!("WebSocket connection rejected: malformed Authorization header");
                }
            } else {
                tracing::warn!("WebSocket connection rejected: missing Authorization header");
            }

            // Reject with 401 Unauthorized
            let mut error_response: HttpResponse<Option<String>> = HttpResponse::new(Some("Unauthorized".to_string()));
            *error_response.status_mut() = StatusCode::UNAUTHORIZED;
            error_response.headers_mut().insert(
                "WWW-Authenticate",
                "Bearer realm=\"WebSocket\"".parse().unwrap(),
            );
            return Err(error_response);
        }

        // No authentication required
        Ok(response)
    })
    .await
    {
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
    session.user_id = user_id_from_token;

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

                // Create request span for automatic context propagation
                let span = cb_core::logging::request_span(&request_id.to_string(), "websocket");
                let _enter = span.enter();

                tracing::debug!(message_size = text.len(), "Received message");

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
                        if let Err(e) = write
                            .send(Message::Text(error_response.to_string().into()))
                            .await
                        {
                            tracing::error!("Failed to send error response: {}", e);
                            break;
                        }
                        continue;
                    }
                };

                // Create session info for this request
                let session_info = SessionInfo {
                    user_id: session.user_id.clone(),
                };

                // Handle the message
                let response = match handle_message(
                    &mut session,
                    mcp_message,
                    &config,
                    dispatcher.as_ref(),
                    &session_info,
                )
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
    session_info: &SessionInfo,
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
                dispatcher
                    .dispatch(McpMessage::Request(request), session_info)
                    .await
            }
        }
        other => {
            // Forward other message types to dispatcher
            dispatcher.dispatch(other, session_info).await
        }
    }
}

/// Handle initialize request
async fn handle_initialize(
    session: &mut Session,
    request: McpRequest,
    _config: &AppConfig,
) -> ApiResult<McpMessage> {
    // Parse initialize payload
    let payload: InitializePayload = if let Some(params) = request.params {
        serde_json::from_value(params)
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid initialize params: {}", e)))?
    } else {
        InitializePayload {
            project: None,
            project_root: None,
        }
    };

    // Update session (authentication already done at connection level)
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
            plugin_selection: Default::default(),
            git: Default::default(),
            validation: Default::default(),
            language_plugins: Default::default(),
            #[cfg(feature = "mcp-proxy")]
            external_mcp: None,
        }
    }

    #[tokio::test]
    async fn test_initialize_success() {
        let config = create_test_config(false);
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

        assert!(session.initialized);
        assert_eq!(session.project_id, Some("test_project".to_string()));

        if let McpMessage::Response(resp) = response {
            assert!(resp.result.is_some());
            assert!(resp.error.is_none());
        } else {
            panic!("Expected Response message");
        }
    }
}
