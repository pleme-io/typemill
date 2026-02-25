//! WebSocket transport implementation

use crate::{McpDispatcher, SessionInfo};
use futures_util::{SinkExt, StreamExt};
use mill_auth::jwt::{decode, Claims, DecodingKey, Validation};
use mill_config::AppConfig;
use mill_foundation::core::model::mcp::{McpError, McpMessage, McpRequest, McpResponse};
use mill_foundation::errors::{ErrorResponse, MillError, MillResult};
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
    /// The project ID authenticated via token (if any)
    pub authenticated_project_id: Option<String>,
}

impl Session {
    fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: None,
            project_root: None,
            initialized: false,
            user_id: None,
            authenticated_project_id: None,
        }
    }
}

/// Connection guard that tracks active connections
///
/// Increments counter on creation, decrements on drop.
/// Used to enforce max_clients limit.
struct ConnectionGuard {
    counter: Arc<std::sync::atomic::AtomicUsize>,
}

impl ConnectionGuard {
    fn new(counter: Arc<std::sync::atomic::AtomicUsize>) -> Self {
        counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Self { counter }
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.counter
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Start the WebSocket server
pub async fn start_ws_server(
    config: Arc<AppConfig>,
    dispatcher: Arc<dyn McpDispatcher>,
) -> MillResult<()> {
    // Enforce TLS for non-loopback hosts
    if !config.server.is_loopback_host() {
        if config.server.tls.is_none() {
            return Err(MillError::bootstrap(format!(
                "TLS is required when binding to non-loopback address '{}'. \
                     Configure server.tls or bind to 127.0.0.1",
                config.server.host
            )));
        }
        tracing::info!(
            host = %config.server.host,
            "TLS enabled for non-loopback host"
        );
    } else if config.server.tls.is_none() {
        tracing::warn!(
            host = %config.server.host,
            "WebSocket server running without TLS on loopback. \
             Enable TLS in production environments."
        );
    }

    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| MillError::bootstrap(format!("Failed to bind to {}: {}", addr, e)))?;

    // Connection tracking for max_clients enforcement
    let active_connections = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    tracing::info!("WebSocket server listening on {}", addr);

    while let Ok((stream, addr)) = listener.accept().await {
        // Check max_clients limit before accepting connection
        if let Some(max_clients) = config.server.max_clients {
            let current = active_connections.load(std::sync::atomic::Ordering::SeqCst);
            if current >= max_clients {
                tracing::warn!(
                    current_connections = current,
                    max_clients = max_clients,
                    client_addr = %addr,
                    "Max clients limit reached, rejecting connection"
                );
                // Connection will be dropped, closing the socket
                continue;
            }
        }

        tracing::debug!(
            client_addr = %addr,
            "New connection"
        );
        let config = config.clone();
        let dispatcher = dispatcher.clone();
        let connection_counter = active_connections.clone();

        tokio::spawn(async move {
            let _guard = ConnectionGuard::new(connection_counter);
            handle_connection(stream, config, dispatcher).await;
        });
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
    let mut project_id_from_token: Option<String> = None;
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

                    // Use config to determine if audience validation is enabled
                    validation.validate_aud = auth_config.validate_audience;

                    if auth_config.validate_audience {
                        let audience = auth_config
                            .jwt_audience_override
                            .as_ref()
                            .unwrap_or(&auth_config.jwt_audience);
                        validation.set_audience(&[audience]);
                    }

                    match decode::<Claims>(token, &key, &validation) {
                        Ok(token_data) => {
                            tracing::debug!("WebSocket connection authenticated");

                            // Warn if project_id is missing (deprecation path)
                            if token_data.claims.project_id.is_none() {
                                tracing::warn!(
                                    "WebSocket connection authenticated but token missing project_id claim - this will be required in future versions"
                                );
                            }

                            user_id_from_token = token_data.claims.user_id;
                            project_id_from_token = token_data.claims.project_id;
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
    session.authenticated_project_id = project_id_from_token;

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
                let span = mill_config::logging::request_span(&request_id.to_string(), "websocket");
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
                        let api_error: ErrorResponse = e.into();

                        tracing::error!(
                            request_id = %request_id,
                            error_code = %api_error.code,
                            error = %api_error.message,
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
) -> MillResult<McpMessage> {
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
) -> MillResult<McpMessage> {
    // Parse initialize payload
    let payload: InitializePayload = if let Some(params) = request.params {
        serde_json::from_value(params)
            .map_err(|e| MillError::invalid_request(format!("Invalid initialize params: {}", e)))?
    } else {
        InitializePayload {
            project: None,
            project_root: None,
        }
    };

    // Enforce project access if token is scoped
    if let Some(auth_project_id) = &session.authenticated_project_id {
        if let Some(requested_project) = &payload.project {
            if auth_project_id != requested_project {
                return Err(MillError::permission_denied(format!(
                    "Token is scoped to project '{}' but requested project '{}'",
                    auth_project_id, requested_project
                )));
            }
        }
    }

    // Update session (authentication already done at connection level)
    // If authenticated_project_id is present, default to it if no project requested
    session.project_id = payload
        .project
        .or(session.authenticated_project_id.clone());
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
                // Magnificent Seven
                "inspect_code".to_string(),
                "search_code".to_string(),
                "rename_all".to_string(),
                "relocate".to_string(),
                "prune".to_string(),
                "refactor".to_string(),
                "workspace".to_string(),
                // System tools
                "health_check".to_string(),
                "notify_file_opened".to_string(),
                "notify_file_saved".to_string(),
                "notify_file_closed".to_string(),
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
    use mill_config::config::{AuthConfig, CacheConfig, LoggingConfig, LspConfig, ServerConfig};

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
                        validate_audience: false,
                        jwt_audience_override: None,
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

    #[test]
    fn test_connection_guard_increments_on_creation() {
        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 0);

        {
            let _guard = ConnectionGuard::new(counter.clone());
            assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
        }

        // Guard dropped, should decrement
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    #[test]
    fn test_connection_guard_multiple_connections() {
        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let _guard1 = ConnectionGuard::new(counter.clone());
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);

        let _guard2 = ConnectionGuard::new(counter.clone());
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 2);

        let _guard3 = ConnectionGuard::new(counter.clone());
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 3);

        drop(_guard1);
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 2);

        drop(_guard2);
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);

        drop(_guard3);
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    #[test]
    fn test_is_loopback_host_valid() {
        let mut config = create_test_config(false);

        config.server.host = "127.0.0.1".to_string();
        assert!(
            config.server.is_loopback_host(),
            "127.0.0.1 should be loopback"
        );

        config.server.host = "::1".to_string();
        assert!(config.server.is_loopback_host(), "::1 should be loopback");

        config.server.host = "localhost".to_string();
        assert!(
            config.server.is_loopback_host(),
            "localhost should be loopback"
        );
    }

    #[test]
    fn test_is_loopback_host_invalid() {
        let mut config = create_test_config(false);

        config.server.host = "0.0.0.0".to_string();
        assert!(
            !config.server.is_loopback_host(),
            "0.0.0.0 should NOT be loopback (binds all interfaces)"
        );

        config.server.host = "192.168.1.1".to_string();
        assert!(
            !config.server.is_loopback_host(),
            "192.168.1.1 should NOT be loopback"
        );

        config.server.host = "10.0.0.1".to_string();
        assert!(
            !config.server.is_loopback_host(),
            "10.0.0.1 should NOT be loopback"
        );

        config.server.host = "example.com".to_string();
        assert!(
            !config.server.is_loopback_host(),
            "example.com should NOT be loopback"
        );
    }

    #[tokio::test]
    async fn test_initialize_enforces_project_id() {
        let config = create_test_config(false);
        let mut session = Session::new();
        // Simulate authenticated session with project ID
        session.authenticated_project_id = Some("auth-project".to_string());

        // 1. Test mismatch
        let request_mismatch = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(serde_json::Number::from(1))),
            method: "initialize".to_string(),
            params: Some(json!({
                "project": "other-project"
            })),
        };

        let result = handle_initialize(&mut session, request_mismatch, &config).await;
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("Token is scoped to project"));

        // 2. Test match
        let request_match = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(serde_json::Number::from(2))),
            method: "initialize".to_string(),
            params: Some(json!({
                "project": "auth-project"
            })),
        };

        let response = handle_initialize(&mut session, request_match, &config).await;
        assert!(response.is_ok());
        assert_eq!(session.project_id, Some("auth-project".to_string()));

        // 3. Test auto-bind (no project in payload)
        // Reset session initialized state for re-init (though handle_initialize doesn't check initialized state itself, handle_message does)
        session.initialized = false;

        let request_none = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(serde_json::Number::from(3))),
            method: "initialize".to_string(),
            params: Some(json!({})),
        };

        let response = handle_initialize(&mut session, request_none, &config).await;
        assert!(response.is_ok());
        assert_eq!(session.project_id, Some("auth-project".to_string()));
    }

    #[tokio::test]
    async fn test_initialize_allows_setting_project_if_not_bound() {
        let config = create_test_config(false);
        let mut session = Session::new();
        // No authenticated_project_id

        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(serde_json::Number::from(1))),
            method: "initialize".to_string(),
            params: Some(json!({
                "project": "any-project"
            })),
        };

        let response = handle_initialize(&mut session, request, &config).await;
        assert!(response.is_ok());
        assert_eq!(session.project_id, Some("any-project".to_string()));
    }
}
