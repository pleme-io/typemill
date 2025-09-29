//! WebSocket transport implementation

use crate::auth::jwt::validate_token_with_project;
use crate::error::{ServerError, ServerResult};
use crate::handlers::PluginDispatcher;
use cb_core::config::AppConfig;
use cb_core::model::mcp::{McpMessage, McpRequest, McpResponse, McpError};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};

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

/// Start the WebSocket server
pub async fn start_ws_server(
    config: Arc<AppConfig>,
    dispatcher: Arc<PluginDispatcher>,
) -> ServerResult<()> {
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| ServerError::bootstrap(format!("Failed to bind to {}: {}", addr, e)))?;

    tracing::info!("WebSocket server listening on {}", addr);

    while let Ok((stream, addr)) = listener.accept().await {
        tracing::debug!("New connection from {}", addr);
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
    dispatcher: Arc<PluginDispatcher>,
) {
    // Perform WebSocket handshake
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            tracing::error!("WebSocket handshake failed: {}", e);
            return;
        }
    };

    tracing::info!("WebSocket connection established");
    let (mut write, mut read) = ws_stream.split();
    let mut session = Session::new();

    // Message processing loop
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                tracing::debug!("Received message: {}", text);

                // Parse MCP message
                let mcp_message: McpMessage = match serde_json::from_str(&text) {
                    Ok(msg) => msg,
                    Err(e) => {
                        tracing::error!("Failed to parse MCP message: {}", e);
                        let error_response = json!({
                            "error": {
                                "code": -32700,
                                "message": "Parse error"
                            }
                        });
                        if let Err(e) = write
                            .send(Message::Text(error_response.to_string()))
                            .await
                        {
                            tracing::error!("Failed to send error response: {}", e);
                            break;
                        }
                        continue;
                    }
                };

                // Handle the message
                let response = match handle_message(&mut session, mcp_message, &config, &dispatcher).await {
                    Ok(response) => response,
                    Err(e) => {
                        tracing::error!("Failed to handle message: {}", e);
                        McpMessage::Response(McpResponse {
            jsonrpc: "2.0".to_string(),
                            id: None,
                            result: None,
                            error: Some(McpError {
                                code: -1,
                                message: e.to_string(),
                                data: None,
                            }),
                        })
                    }
                };

                // Send response
                let response_text = match serde_json::to_string(&response) {
                    Ok(text) => text,
                    Err(e) => {
                        tracing::error!("Failed to serialize response: {}", e);
                        continue;
                    }
                };

                if let Err(e) = write.send(Message::Text(response_text)).await {
                    tracing::error!("Failed to send response: {}", e);
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
                tracing::error!("WebSocket error: {}", e);
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
    dispatcher: &PluginDispatcher,
) -> ServerResult<McpMessage> {
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
                        message: "Session not initialized. Send initialize message first.".to_string(),
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
) -> ServerResult<McpMessage> {
    // Parse initialize payload
    let payload: InitializePayload = if let Some(params) = request.params {
        serde_json::from_value(params)
            .map_err(|e| ServerError::InvalidRequest(format!("Invalid initialize params: {}", e)))?
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
                .map_err(|e| ServerError::Auth(format!("Authentication failed: {}", e)))?;

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

    tracing::info!("Session {} initialized for project: {:?}",
                   session.id, session.project_id);

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
    use cb_core::config::{AuthConfig, ServerConfig, LoggingConfig, CacheConfig, LspConfig};

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
    async fn test_initialize_without_auth() {
        let config = create_test_config(false);
        let project_root = std::path::PathBuf::from(".");
        let ast_service = Arc::new(crate::services::DefaultAstService::new());
        let file_service = Arc::new(crate::services::FileService::new(&project_root));
        let lock_manager = Arc::new(crate::services::LockManager::new());
        let operation_queue = Arc::new(crate::services::OperationQueue::new(lock_manager.clone()));
        let app_state = Arc::new(crate::handlers::AppState {
            ast_service,
            file_service,
            project_root,
            lock_manager,
            operation_queue,
        });
        let _dispatcher = Arc::new(PluginDispatcher::new(app_state));
        let mut session = Session::new();

        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(serde_json::Number::from(1))),
            method: "initialize".to_string(),
            params: Some(json!({
                "project": "test_project",
                "projectRoot": "/path/to/project"
            })),
        };

        let response = handle_initialize(&mut session, request, &config).await.unwrap();

        assert!(session.initialized);
        assert_eq!(session.project_id, Some("test_project".to_string()));
        assert_eq!(session.project_root, Some("/path/to/project".to_string()));

        if let McpMessage::Response(resp) = response {
            assert!(resp.result.is_some());
            assert!(resp.error.is_none());
        } else {
            panic!("Expected Response message");
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

        let response = handle_initialize(&mut session, request, &config).await.unwrap();

        assert!(!session.initialized);

        if let McpMessage::Response(resp) = response {
            assert!(resp.result.is_none());
            assert!(resp.error.is_some());
        } else {
            panic!("Expected Response message");
        }
    }
}