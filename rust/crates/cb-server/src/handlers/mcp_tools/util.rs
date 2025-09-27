//! Utility functions for MCP tool handlers

use crate::error::ServerError;
use crate::interfaces::LspService;
use cb_core::model::mcp::{McpMessage, McpRequest, McpResponse};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::debug;

/// Global request ID generator for unique IDs across all handlers
static REQUEST_ID_GENERATOR: AtomicU64 = AtomicU64::new(1);

/// Generate a unique request ID
pub fn generate_request_id() -> u64 {
    REQUEST_ID_GENERATOR.fetch_add(1, Ordering::SeqCst)
}

/// Forward an LSP request and handle the response uniformly
///
/// This helper function eliminates boilerplate code across tool handlers by:
/// 1. Generating a unique request ID automatically
/// 2. Creating the MCP request structure
/// 3. Sending the request to the LSP service
/// 4. Processing the response with consistent error handling
///
/// # Arguments
/// * `lsp_service` - The LSP service to send the request to
/// * `method` - The LSP method to call (e.g., "textDocument/definition")
/// * `params` - The parameters for the LSP method
///
/// # Returns
/// * `Ok(Value)` - The successful result from the LSP service
/// * `Err(ServerError)` - An error if the request failed
pub async fn forward_lsp_request(
    lsp_service: &dyn LspService,
    method: String,
    params: Option<Value>,
) -> Result<Value, ServerError> {
    let request_id = generate_request_id();

    debug!(
        "Forwarding LSP request: method={}, id={}",
        method, request_id
    );

    // Create LSP request with unique ID
    let lsp_request = McpRequest {
        id: Some(Value::Number(serde_json::Number::from(request_id))),
        method,
        params,
    };

    // Send request to LSP service
    match lsp_service.request(McpMessage::Request(lsp_request)).await {
        Ok(McpMessage::Response(response)) => {
            if let Some(result) = response.result {
                Ok(result)
            } else if let Some(error) = response.error {
                Err(ServerError::runtime(format!("LSP error: {}", error.message)))
            } else {
                Err(ServerError::runtime("Empty LSP response"))
            }
        }
        Ok(_) => Err(ServerError::runtime("Unexpected LSP message type")),
        Err(e) => Err(ServerError::runtime(format!("LSP request failed: {}", e))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::thread;
    use proptest::prelude::*;

    #[test]
    fn test_unique_request_ids() {
        let id1 = generate_request_id();
        let id2 = generate_request_id();
        let id3 = generate_request_id();

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
        assert!(id1 < id2);
        assert!(id2 < id3);
    }

    #[test]
    fn test_concurrent_request_id_generation() {
        let num_threads = 10;
        let ids_per_thread = 100;
        let mut handles = vec![];

        // Spawn multiple threads to generate IDs concurrently
        for _ in 0..num_threads {
            let handle = thread::spawn(move || {
                let mut thread_ids = Vec::new();
                for _ in 0..ids_per_thread {
                    thread_ids.push(generate_request_id());
                }
                thread_ids
            });
            handles.push(handle);
        }

        // Collect all IDs from all threads
        let mut all_ids = HashSet::new();
        for handle in handles {
            let thread_ids = handle.join().unwrap();
            for id in thread_ids {
                // Each ID should be unique across all threads
                assert!(all_ids.insert(id), "Duplicate ID found: {}", id);
            }
        }

        // Verify we got the expected number of unique IDs
        assert_eq!(all_ids.len(), num_threads * ids_per_thread);
    }

    #[test]
    fn test_request_id_atomicity() {
        // Test that the atomic operations work correctly under stress
        let num_iterations = 1000;
        let mut handles = vec![];

        for _ in 0..10 {
            let handle = thread::spawn(move || {
                let mut ids = Vec::new();
                for _ in 0..num_iterations {
                    ids.push(generate_request_id());
                }
                ids
            });
            handles.push(handle);
        }

        let mut all_ids = Vec::new();
        for handle in handles {
            all_ids.extend(handle.join().unwrap());
        }

        // Check that all IDs are unique
        let mut id_set = HashSet::new();
        for id in &all_ids {
            assert!(id_set.insert(*id), "Duplicate ID: {}", id);
        }

        // Check that IDs are generally increasing (allowing for threading)
        all_ids.sort();
        for window in all_ids.windows(2) {
            assert!(window[0] < window[1], "IDs should be strictly increasing");
        }
    }

    #[tokio::test]
    async fn test_forward_lsp_request_success() {
        use cb_core::model::mcp::{McpMessage, McpResponse};
        use cb_server::interfaces::LspService;
        use async_trait::async_trait;
        use cb_core::CoreError;

        // Create a simple test LSP service
        struct TestLspService {
            response: Value,
        }

        #[async_trait]
        impl LspService for TestLspService {
            async fn request(&self, _message: McpMessage) -> Result<McpMessage, CoreError> {
                let response = McpResponse {
                    id: Some(Value::Number(serde_json::Number::from(1))),
                    result: Some(self.response.clone()),
                    error: None,
                };
                Ok(McpMessage::Response(response))
            }

            async fn is_available(&self, _extension: &str) -> bool {
                true
            }

            async fn restart_servers(&self, _extensions: Option<Vec<String>>) -> Result<(), CoreError> {
                Ok(())
            }
        }

        let test_service = TestLspService {
            response: serde_json::json!({"success": true, "data": "test"}),
        };

        let result = forward_lsp_request(
            &test_service,
            "test/method".to_string(),
            Some(serde_json::json!({"param": "value"}))
        ).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response["success"], true);
        assert_eq!(response["data"], "test");
    }

    #[tokio::test]
    async fn test_forward_lsp_request_error() {
        use cb_core::model::mcp::{McpMessage, McpResponse, McpError};
        use cb_server::interfaces::LspService;
        use async_trait::async_trait;
        use cb_core::CoreError;

        struct ErrorLspService;

        #[async_trait]
        impl LspService for ErrorLspService {
            async fn request(&self, _message: McpMessage) -> Result<McpMessage, CoreError> {
                let response = McpResponse {
                    id: Some(Value::Number(serde_json::Number::from(1))),
                    result: None,
                    error: Some(McpError {
                        code: -32603,
                        message: "Test error".to_string(),
                        data: None,
                    }),
                };
                Ok(McpMessage::Response(response))
            }

            async fn is_available(&self, _extension: &str) -> bool {
                true
            }

            async fn restart_servers(&self, _extensions: Option<Vec<String>>) -> Result<(), CoreError> {
                Ok(())
            }
        }

        let error_service = ErrorLspService;

        let result = forward_lsp_request(
            &error_service,
            "error/method".to_string(),
            None
        ).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Test error"));
    }

    proptest! {
        #[test]
        fn test_request_ids_always_unique_property(
            num_threads in 1..20usize,
            ids_per_thread in 1..100usize
        ) {
            let mut handles = vec![];

            for _ in 0..num_threads {
                let handle = thread::spawn(move || {
                    let mut thread_ids = Vec::new();
                    for _ in 0..ids_per_thread {
                        thread_ids.push(generate_request_id());
                    }
                    thread_ids
                });
                handles.push(handle);
            }

            let mut all_ids = HashSet::new();
            for handle in handles {
                let thread_ids = handle.join().unwrap();
                for id in thread_ids {
                    prop_assert!(all_ids.insert(id), "Duplicate ID found: {}", id);
                }
            }

            prop_assert_eq!(all_ids.len(), num_threads * ids_per_thread);
        }

        #[test]
        fn test_request_ids_are_increasing_property(count in 1..1000usize) {
            let mut ids = Vec::new();
            for _ in 0..count {
                ids.push(generate_request_id());
            }

            // IDs should be strictly increasing when generated sequentially
            for window in ids.windows(2) {
                prop_assert!(window[0] < window[1], "IDs should increase: {} >= {}", window[0], window[1]);
            }
        }
    }

    #[test]
    fn test_request_id_wraparound_safety() {
        // Test behavior near potential overflow (though unlikely in practice)
        // This is more of a documentation test since we'd need to generate
        // 2^64 IDs to actually test overflow

        // At least verify the atomic counter continues to work after many operations
        let initial_id = generate_request_id();

        // Generate many IDs quickly
        for _ in 0..10000 {
            generate_request_id();
        }

        let later_id = generate_request_id();
        assert!(later_id > initial_id + 10000);
    }

    #[test]
    fn test_request_id_thread_safety_stress() {
        // Stress test with many threads doing rapid ID generation
        let num_threads = 50;
        let iterations = 200;

        let handles: Vec<_> = (0..num_threads).map(|_| {
            thread::spawn(move || {
                let mut max_id = 0;
                for _ in 0..iterations {
                    let id = generate_request_id();
                    max_id = max_id.max(id);
                }
                max_id
            })
        }).collect();

        let max_ids: Vec<u64> = handles.into_iter()
            .map(|h| h.join().unwrap())
            .collect();

        // Verify that we got reasonable results from all threads
        assert_eq!(max_ids.len(), num_threads);

        // All max IDs should be different and reasonable
        let mut id_set = HashSet::new();
        for &max_id in &max_ids {
            assert!(max_id > 0);
            assert!(id_set.insert(max_id), "Duplicate max ID: {}", max_id);
        }
    }
}