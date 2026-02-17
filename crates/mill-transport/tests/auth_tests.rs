
use mill_auth::jwt::generate_token;
use mill_config::config::{AuthConfig, CacheConfig, LoggingConfig, LspConfig, ServerConfig};
use mill_config::AppConfig;
use mill_transport::ws::start_ws_server;
use mill_transport::{McpDispatcher, SessionInfo};
use mill_foundation::core::model::mcp::McpMessage;
use mill_foundation::errors::MillResult;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use futures_util::{SinkExt, StreamExt};

struct MockDispatcher;

#[async_trait::async_trait]
impl McpDispatcher for MockDispatcher {
    async fn dispatch(&self, _message: McpMessage, _session_info: &SessionInfo) -> MillResult<McpMessage> {
        Ok(McpMessage::Response(mill_foundation::core::model::mcp::McpResponse {
            jsonrpc: "2.0".to_string(),
            id: None,
            result: Some(json!({"status": "ok"})),
            error: None,
        }))
    }
}

fn create_test_config() -> AppConfig {
    AppConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0, // Let OS choose port
            max_clients: Some(10),
            timeout_ms: 30000,
            tls: None,
            auth: Some(AuthConfig {
                jwt_secret: "test_secret".to_string(),
                jwt_expiry_seconds: 3600,
                jwt_issuer: "test".to_string(),
                jwt_audience: "test".to_string(),
                validate_audience: false,
                jwt_audience_override: None,
            }),
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
async fn test_auth_project_isolation() {
    // 1. Setup server with auth
    let config = Arc::new(create_test_config());
    let dispatcher = Arc::new(MockDispatcher);

    // Pick a port
    let port = 3045;
    let mut config_mut = Arc::unwrap_or_clone(config);
    config_mut.server.port = port;
    let config = Arc::new(config_mut);

    let config_clone = config.clone();
    let dispatcher_clone = dispatcher.clone();

    tokio::spawn(async move {
        let _ = start_ws_server(config_clone, dispatcher_clone).await;
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 2. Generate token for "project_A"
    let token = generate_token(
        "test_secret",
        3600,
        "test",
        "test",
        Some("project_A".to_string()),
        Some("user_1".to_string()),
    ).unwrap();

    // 3. Connect with token
    let url_str = format!("ws://127.0.0.1:{}", port);

    // Create request manually using the trait method
    let mut request = IntoClientRequest::into_client_request(url_str).expect("Failed to create request");
    request.headers_mut().insert(
        "Authorization",
        format!("Bearer {}", token).parse().unwrap()
    );

    let (mut ws_stream, _) = connect_async(request).await.expect("Failed to connect");

    // 4. Send initialize request for "project_B"
    let init_msg = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "project": "project_B", // requesting DIFFERENT project
            "projectRoot": "/tmp"
        }
    });

    ws_stream.send(Message::Text(init_msg.to_string().into())).await.expect("Failed to send");

    // 5. Read response
    if let Some(msg) = ws_stream.next().await {
        let msg = msg.expect("Failed to read message");
        if let Message::Text(text) = msg {
            let response: serde_json::Value = serde_json::from_str(&text).unwrap();

            // Check if it succeeded (regression check)
            if let Some(_result) = response.get("result") {
                panic!("Regression: Session initialization succeeded with project mismatch");
            } else if let Some(error) = response.get("error") {
                // Verify error code/message roughly
                let code = error.get("code").and_then(|c| c.as_i64());
                assert_eq!(code, Some(-1), "Expected error code -1");

                let message = error.get("message").and_then(|m| m.as_str()).unwrap_or("");
                assert!(message.contains("Project mismatch"), "Expected error message to contain 'Project mismatch'");
            }
        }
    }
}
