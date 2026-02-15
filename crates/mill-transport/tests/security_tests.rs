use mill_config::config::AppConfig;
use mill_transport::start_admin_server;
use mill_workspaces::WorkspaceManager;
use reqwest::Client;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::test]
async fn test_generate_token_endpoint_security() {
    // 1. Setup config and workspace manager
    let mut config = AppConfig::default();
    config.server.auth = Some(mill_config::config::AuthConfig {
        jwt_secret: "secret".to_string(),
        jwt_expiry_seconds: 3600,
        jwt_issuer: "test".to_string(),
        jwt_audience: "test".to_string(),
        validate_audience: false,
        jwt_audience_override: None,
    });
    let config = Arc::new(config);
    let workspace_manager = Arc::new(WorkspaceManager::new());

    // 2. Find a free port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener); // Close listener so server can bind

    // 3. Start admin server in background
    let config_clone = config.clone();
    let wm_clone = workspace_manager.clone();
    tokio::spawn(async move {
        start_admin_server(port, config_clone, wm_clone).await.unwrap();
    });

    // Give it a moment to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 4. Try to generate token (unauthenticated)
    let client = Client::new();
    let response = client
        .post(format!("http://127.0.0.1:{}/auth/generate-token", port))
        .json(&serde_json::json!({
            "project_id": "p1",
            "user_id": "u1"
        }))
        .send()
        .await
        .unwrap();

    // The fix: endpoint should be removed (404 Not Found)
    assert_eq!(response.status(), 404, "Should return 404 Not Found (fixed)");
}
