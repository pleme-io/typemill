//! Test LSP service for predictable testing without heavy mocking

use async_trait::async_trait;
use cb_core::{model::mcp::{McpMessage, McpRequest, McpResponse, McpError}, CoreError};
use cb_server::interfaces::LspService;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A test implementation of LspService that returns predictable responses
/// This is not a mock but a simple implementation for testing purposes
pub struct TestLspService {
    /// Configured responses for specific LSP methods
    responses: Arc<Mutex<HashMap<String, Value>>>,
    /// Track requests for verification
    requests: Arc<Mutex<Vec<McpRequest>>>,
    /// Simulate errors for specific methods
    error_methods: Arc<Mutex<HashMap<String, String>>>,
}

impl TestLspService {
    /// Create a new test LSP service
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(HashMap::new())),
            requests: Arc::new(Mutex::new(Vec::new())),
            error_methods: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Configure a response for a specific LSP method
    pub fn set_response(&self, method: &str, response: Value) {
        let mut responses = self.responses.lock().unwrap();
        responses.insert(method.to_string(), response);
    }

    /// Configure an error response for a specific LSP method
    pub fn set_error(&self, method: &str, error_message: &str) {
        let mut errors = self.error_methods.lock().unwrap();
        errors.insert(method.to_string(), error_message.to_string());
    }

    /// Get all requests that were sent to this service
    pub fn get_requests(&self) -> Vec<McpRequest> {
        let requests = self.requests.lock().unwrap();
        requests.clone()
    }

    /// Get the last request sent to this service
    pub fn get_last_request(&self) -> Option<McpRequest> {
        let requests = self.requests.lock().unwrap();
        requests.last().cloned()
    }

    /// Clear all recorded requests
    pub fn clear_requests(&self) {
        let mut requests = self.requests.lock().unwrap();
        requests.clear();
    }

    /// Set up common LSP responses for navigation testing
    pub fn setup_navigation_responses(&self) {
        // textDocument/definition response
        self.set_response("textDocument/definition", json!([
            {
                "uri": "file:///test/example.ts",
                "range": {
                    "start": {"line": 10, "character": 5},
                    "end": {"line": 10, "character": 15}
                }
            }
        ]));

        // textDocument/references response
        self.set_response("textDocument/references", json!([
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
        ]));

        // workspace/symbol response
        self.set_response("workspace/symbol", json!([
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
        ]));
    }

    /// Set up common LSP responses for editing testing
    pub fn setup_editing_responses(&self) {
        // textDocument/rename response
        self.set_response("textDocument/rename", json!({
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
        }));
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
impl LspService for TestLspService {
    async fn request(&self, message: McpMessage) -> Result<McpMessage, CoreError> {
        match message {
            McpMessage::Request(request) => {
                // Store the request for verification
                {
                    let mut requests = self.requests.lock().unwrap();
                    requests.push(request.clone());
                }

                // Check if we should simulate an error
                {
                    let errors = self.error_methods.lock().unwrap();
                    if let Some(error_msg) = errors.get(&request.method) {
                        let error_response = McpResponse {
                            id: request.id.clone(),
                            result: None,
                            error: Some(McpError {
                                code: -32603,
                                message: error_msg.clone(),
                                data: None,
                            }),
                        };
                        return Ok(McpMessage::Response(error_response));
                    }
                }

                // Look up configured response
                let responses = self.responses.lock().unwrap();
                let result = responses.get(&request.method).cloned()
                    .unwrap_or_else(|| {
                        // Default response for unknown methods
                        json!({
                            "method": request.method,
                            "message": "Default test response"
                        })
                    });

                let response = McpResponse {
                    id: request.id,
                    result: Some(result),
                    error: None,
                };

                Ok(McpMessage::Response(response))
            }
            _ => Err(CoreError::InvalidRequest("Expected request message".to_string())),
        }
    }

    async fn is_available(&self, _extension: &str) -> bool {
        // Always available for testing
        true
    }

    async fn restart_servers(&self, _extensions: Option<Vec<String>>) -> Result<(), CoreError> {
        // No-op for testing
        Ok(())
    }
}

impl Default for TestLspService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lsp_service_basic_request() {
        let service = TestLspService::new();
        service.set_response("test/method", json!({"result": "success"}));

        let request = McpRequest {
            id: Some(json!(1)),
            method: "test/method".to_string(),
            params: Some(json!({"param": "value"})),
        };

        let response = service.request(McpMessage::Request(request.clone())).await.unwrap();

        if let McpMessage::Response(resp) = response {
            assert_eq!(resp.id, Some(json!(1)));
            assert_eq!(resp.result, Some(json!({"result": "success"})));
            assert!(resp.error.is_none());
        } else {
            panic!("Expected response message");
        }

        // Verify request was recorded
        let requests = service.get_requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "test/method");
    }

    #[tokio::test]
    async fn test_lsp_service_error_response() {
        let service = TestLspService::new();
        service.set_error("error/method", "Simulated error");

        let request = McpRequest {
            id: Some(json!(2)),
            method: "error/method".to_string(),
            params: None,
        };

        let response = service.request(McpMessage::Request(request)).await.unwrap();

        if let McpMessage::Response(resp) = response {
            assert_eq!(resp.id, Some(json!(2)));
            assert!(resp.result.is_none());
            assert!(resp.error.is_some());
            assert_eq!(resp.error.unwrap().message, "Simulated error");
        } else {
            panic!("Expected response message");
        }
    }

    #[tokio::test]
    async fn test_lsp_service_is_available() {
        let service = TestLspService::new();
        assert!(service.is_available("ts").await);
        assert!(service.is_available("js").await);
        assert!(service.is_available("py").await);
    }

    #[tokio::test]
    async fn test_lsp_service_navigation_setup() {
        let service = TestLspService::new();
        service.setup_navigation_responses();

        // Test definition response
        let def_request = McpRequest {
            id: Some(json!(1)),
            method: "textDocument/definition".to_string(),
            params: None,
        };

        let response = service.request(McpMessage::Request(def_request)).await.unwrap();
        if let McpMessage::Response(resp) = response {
            assert!(resp.result.is_some());
            let result = resp.result.unwrap();
            assert!(result.is_array());
        }
    }
}