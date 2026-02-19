use async_trait::async_trait;
use mill_auth::jwt::generate_token;
use mill_config::config::{AuthConfig, CacheConfig, LoggingConfig, LspConfig, ServerConfig};
use mill_config::AppConfig;
use mill_foundation::core::model::mcp::{McpMessage, McpResponse};
use mill_foundation::errors::MillResult;
use mill_transport::{McpDispatcher, SessionInfo, start_ws_server};
use serde_json::json;
use std::sync::Arc;
use tokio_tungstenite::{connect_async, tungstenite::client::IntoClientRequest, tungstenite::http::HeaderValue};
use futures_util::{SinkExt, StreamExt};

struct MockDispatcher;

#[async_trait]
impl McpDispatcher for MockDispatcher {
    async fn dispatch(
        &self,
        message: McpMessage,
        _session_info: &SessionInfo,
    ) -> MillResult<McpMessage> {
        match message {
            McpMessage::Request(req) => Ok(McpMessage::Response(McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(json!({"status": "ok"})),
                error: None,
            })),
            _ => panic!("Unexpected message type"),
        }
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
async fn test_project_isolation_bypass() {
    // 1. Setup server
    let mut config = create_test_config();
    // Bind to a random port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    config.server.port = port;

    let config = Arc::new(config);
    let dispatcher = Arc::new(MockDispatcher);

    // Spawn server in background
    let server_config = config.clone();
    let server_dispatcher = dispatcher.clone();

    // We need to reimplement start_ws_server slightly because it binds its own listener
    // But start_ws_server takes config and binds itself.
    // So we should just call start_ws_server but with the port we found available or let it fail if port is taken?
    // Actually start_ws_server binds the listener itself.

    // Let's try to find a free port and use it.
    // The port is already set in config.server.port

    // We need to drop the listener we created to check for free port
    drop(listener);

    let server_handle = tokio::spawn(async move {
        start_ws_server(server_config, server_dispatcher).await.unwrap();
    });

    // Wait for server to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 2. Generate token for "project-A"
    let token = generate_token(
        "test_secret",
        3600,
        "test",
        "test",
        Some("project-A".to_string()),
        Some("user-1".to_string()),
    ).unwrap();

    // 3. Connect
    let url = format!("ws://127.0.0.1:{}", port);
    let mut request = url.into_client_request().unwrap();
    request.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
    );

    let (mut ws_stream, _) = connect_async(request).await.expect("Failed to connect");

    // 4. Initialize session with "project-B"
    let init_msg = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "project": "project-B",
            "projectRoot": "/tmp/project-b"
        }
    });

    ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(init_msg.to_string().into())).await.unwrap();

    // 5. Read response
    let response = ws_stream.next().await.unwrap().unwrap();
    match response {
        tokio_tungstenite::tungstenite::Message::Text(text) => {
            let resp: serde_json::Value = serde_json::from_str(&text).unwrap();

            // If the vulnerability exists, we get a result.
            // If fixed, we should get an error or connection close.

            if let Some(error) = resp.get("error") {
                println!("Got error as expected: {:?}", error);
            } else {
                println!("Got success response: {:?}", resp);
                // Check if project-B was accepted
                // The initialize response doesn't echo back the project, but if it succeeded, it means we bypassed the check.
                // In a secure implementation, this should fail because token project-A != requested project-B
                panic!("VULNERABILITY CONFIRMED: Successfully initialized session for project-B with token for project-A");
            }
        }
        _ => panic!("Unexpected response type"),
    }

    server_handle.abort();
}
