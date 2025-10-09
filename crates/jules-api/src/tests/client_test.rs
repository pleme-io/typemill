use crate::{Config, JulesClient};
use wiremock::matchers::{method, path, header, body_json};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn setup() -> (MockServer, JulesClient) {
    let server = MockServer::start().await;
    let config = Config {
        api_base_url: server.uri(),
        api_key: "test-key".to_string(),
        request_timeout: 5,
        max_retries: 1,
    };
    let client = JulesClient::new(config);
    (server, client)
}

#[tokio::test]
async fn test_list_sources_success() {
    let (server, client) = setup().await;

    let response_body = serde_json::json!({
        "sources": [
            { "id": "source1", "name": "Source One", "description": "First source", "language": "Rust" }
        ],
        "next_page_token": null
    });

    Mock::given(method("GET"))
        .and(path("/sources"))
        .and(header("Authorization", "Bearer test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&server)
        .await;

    let result = client.list_sources(None, None).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.sources.len(), 1);
    assert_eq!(response.sources[0].name, "Source One");
}

#[tokio::test]
async fn test_get_source_success() {
    let (server, client) = setup().await;

    let response_body = serde_json::json!({
        "id": "source1",
        "name": "Source One",
        "description": "First source",
        "language": "Rust"
    });

    Mock::given(method("GET"))
        .and(path("/sources/source1"))
        .and(header("Authorization", "Bearer test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&server)
        .await;

    let result = client.get_source("source1").await;
    assert!(result.is_ok());
    let source = result.unwrap();
    assert_eq!(source.id, "source1");
}

#[tokio::test]
async fn test_create_session_success() {
    let (server, client) = setup().await;

    let request_body = crate::types::CreateSessionRequest {
        source_id: "source1".to_string(),
    };
    let response_body = serde_json::json!({
        "id": "session1",
        "source_id": "source1",
        "state": "active",
        "created_at": "2023-01-01T12:00:00Z"
    });

    Mock::given(method("POST"))
        .and(path("/sessions"))
        .and(header("Authorization", "Bearer test-key"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(201).set_body_json(response_body))
        .mount(&server)
        .await;

    let result = client.create_session(request_body).await;
    assert!(result.is_ok());
    let session = result.unwrap();
    assert_eq!(session.id, "session1");
}

#[tokio::test]
async fn test_delete_session_success() {
    let (server, client) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/sessions/session1"))
        .and(header("Authorization", "Bearer test-key"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let result = client.delete_session("session1").await;
    assert!(result.is_ok());
}