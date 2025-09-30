//! Mock LSP service for predictable testing without heavy mocking

use async_trait::async_trait;
// No longer need cb_core imports since we use cb_api::Message
use cb_api::{ApiError, LspService, Message};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A mock implementation of LspService that returns predictable responses
pub struct MockLspService {
    /// Configured responses for specific LSP methods
    responses: Arc<Mutex<HashMap<String, Value>>>,
    /// Track requests for verification
    requests: Arc<Mutex<Vec<Message>>>,
    /// Simulate errors for specific methods
    error_methods: Arc<Mutex<HashMap<String, String>>>,
}

impl MockLspService {
    /// Create a new mock LSP service
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(HashMap::new())),
            requests: Arc::new(Mutex::new(Vec::new())),
            error_methods: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Configure a response for a specific LSP method
    pub fn set_response(&self, method: &str, response: Value) {
        let mut responses = self.responses.lock()
            .expect("Response lock poisoned - previous test panicked");
        responses.insert(method.to_string(), response);
    }

    /// Configure an error response for a specific LSP method
    pub fn set_error(&self, method: &str, error_message: &str) {
        let mut errors = self.error_methods.lock()
            .expect("Error methods lock poisoned - previous test panicked");
        errors.insert(method.to_string(), error_message.to_string());
    }

    /// Get all requests that were sent to this service
    pub fn get_requests(&self) -> Vec<Message> {
        let requests = self.requests.lock()
            .expect("Requests lock poisoned - previous test panicked");
        requests.clone()
    }

    /// Get the last request sent to this service
    pub fn get_last_request(&self) -> Option<Message> {
        let requests = self.requests.lock()
            .expect("Requests lock poisoned - previous test panicked");
        requests.last().cloned()
    }

    /// Clear all recorded requests
    pub fn clear_requests(&self) {
        let mut requests = self.requests.lock()
            .expect("Requests lock poisoned - previous test panicked");
        requests.clear();
    }

    /// Set up common LSP responses for navigation testing
    pub fn setup_navigation_responses(&self) {
        // textDocument/definition response
        self.set_response(
            "textDocument/definition",
            json!([
                {
                    "uri": "file:///test/example.ts",
                    "range": {
                        "start": {"line": 10, "character": 5},
                        "end": {"line": 10, "character": 15}
                    }
                }
            ]),
        );

        // textDocument/references response
        self.set_response(
            "textDocument/references",
            json!([
                {
                    "uri": "file:///test/example.ts",
                    "range": {
                        "start": {"line": 5, "character": 0},
                        "end": {"line": 5, "character": 10}
                    }
                },
                {
                    "uri": "file:///test/other.ts",
                    "range": {
                        "start": {"line": 20, "character": 8},
                        "end": {"line": 20, "character": 18}
                    }
                }
            ]),
        );

        // workspace/symbol response
        self.set_response(
            "workspace/symbol",
            json!([
                {
                    "name": "TestFunction",
                    "kind": 12,
                    "location": {
                        "uri": "file:///test/example.ts",
                        "range": {
                            "start": {"line": 15, "character": 0},
                            "end": {"line": 20, "character": 1}
                        }
                    }
                }
            ]),
        );
    }

    /// Set up common LSP responses for editing testing
    pub fn setup_editing_responses(&self) {
        // textDocument/rename response
        self.set_response(
            "textDocument/rename",
            json!({
                "changes": {
                    "file:///test/example.ts": [
                        {
                            "range": {
                                "start": {"line": 5, "character": 10},
                                "end": {"line": 5, "character": 20}
                            },
                            "newText": "newVariableName"
                        }
                    ]
                }
            }),
        );
    }

    /// Set up common LSP responses for intelligence testing
    pub fn setup_intelligence_responses(&self) {
        // textDocument/hover response
        self.set_response("textDocument/hover", json!({
            "contents": {
                "kind": "markdown",
                "value": "```typescript\nfunction testFunction(): void\n```\n\nA test function for demonstration"
            },
            "range": {
                "start": {"line": 10, "character": 5},
                "end": {"line": 10, "character": 17}
            }
        }));
    }
}

#[async_trait]
impl LspService for MockLspService {
    async fn request(&self, message: Message) -> Result<Message, ApiError> {
        // Store the request for verification
        {
            let mut requests = self.requests.lock()
                .expect("Requests lock poisoned - previous test panicked");
            requests.push(message.clone());
        }

        // Check if we should simulate an error
        {
            let errors = self.error_methods.lock()
                .expect("Error methods lock poisoned - previous test panicked");
            if let Some(error_msg) = errors.get(&message.method) {
                return Err(ApiError::lsp(error_msg.clone()));
            }
        }

        // Look up configured response
        let responses = self.responses.lock()
            .expect("Response lock poisoned - previous test panicked");
        let result_value = responses.get(&message.method).cloned().unwrap_or_else(|| {
            // Default response for unknown methods
            json!({
                "method": message.method,
                "message": "Default test response"
            })
        });

        // Create response message
        let response = Message {
            id: message.id,
            method: format!("{}_response", message.method),
            params: result_value,
        };

        Ok(response)
    }

    async fn is_available(&self, _extension: &str) -> bool {
        // Always available for testing
        true
    }

    async fn restart_servers(&self, _extensions: Option<Vec<String>>) -> Result<(), ApiError> {
        // No-op for testing
        Ok(())
    }

    async fn notify_file_opened(&self, _file_path: &std::path::Path) -> Result<(), ApiError> {
        // No-op for testing - the mock LSP service doesn't need actual file notifications
        Ok(())
    }
}

impl Default for MockLspService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_lsp_service_basic_request() {
        let service = MockLspService::new();
        service.set_response("test/method", json!({"result": "success"}));

        let request = Message {
            id: Some("1".to_string()),
            method: "test/method".to_string(),
            params: json!({"param": "value"}),
        };

        let response = service.request(request.clone()).await.unwrap();

        assert_eq!(response.id, Some("1".to_string()));
        assert_eq!(response.method, "test/method_response");
        assert_eq!(response.params, json!({"result": "success"}));

        // Verify request was recorded
        let requests = service.get_requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "test/method");
    }

    #[tokio::test]
    async fn test_mock_lsp_service_error_response() {
        let service = MockLspService::new();
        service.set_error("error/method", "Simulated error");

        let request = Message {
            id: Some("2".to_string()),
            method: "error/method".to_string(),
            params: json!({}),
        };

        let result = service.request(request).await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Simulated error"));
    }

    #[tokio::test]
    async fn test_mock_lsp_service_is_available() {
        let service = MockLspService::new();
        assert!(service.is_available("ts").await);
        assert!(service.is_available("js").await);
        assert!(service.is_available("py").await);
    }

    #[tokio::test]
    async fn test_mock_lsp_service_navigation_setup() {
        let service = MockLspService::new();
        service.setup_navigation_responses();

        // Test definition response
        let def_request = Message {
            id: Some("1".to_string()),
            method: "textDocument/definition".to_string(),
            params: json!({}),
        };

        let response = service.request(def_request).await.unwrap();

        assert!(response.params.is_array());
    }
}
