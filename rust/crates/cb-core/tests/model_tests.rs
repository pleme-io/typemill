//! Tests for protocol models

use cb_core::model::*;
use serde_json::json;

#[test]
fn test_mcp_message_request_serialization() {
    let request = McpMessage::request(1, "test_method");

    let json = serde_json::to_value(&request).unwrap();

    if let Some(content) = json.as_object() {
        if let Some(method) = content.get("method") {
            assert_eq!(method, "test_method");
        }
        if let Some(id) = content.get("id") {
            assert_eq!(id, 1);
        }
    }
}

#[test]
fn test_mcp_message_request_with_params() {
    let params = json!({"key": "value", "number": 42});
    let request = McpMessage::request_with_params(1, "test_method", params.clone());

    let json = serde_json::to_value(&request).unwrap();

    // The message should be able to deserialize back
    let deserialized: McpMessage = serde_json::from_value(json).unwrap();
    match deserialized {
        McpMessage::Request(req) => {
            assert_eq!(req.method, "test_method");
            assert_eq!(req.id, Some(json!(1)));
            assert_eq!(req.params, Some(params));
        }
        _ => panic!("Expected request message"),
    }
}

#[test]
fn test_mcp_message_response_success() {
    let result = json!({"success": true, "data": "test"});
    let response = McpMessage::success_response(1, result.clone());

    let json = serde_json::to_value(&response).unwrap();

    let deserialized: McpMessage = serde_json::from_value(json).unwrap();
    match deserialized {
        McpMessage::Response(resp) => {
            assert_eq!(resp.id, Some(json!(1)));
            assert_eq!(resp.result, Some(result));
            assert!(resp.error.is_none());
        }
        _ => panic!("Expected response message"),
    }
}

#[test]
fn test_mcp_message_response_error() {
    let error = McpError::internal_error("Test error");
    let response = McpMessage::error_response(1, error.clone());

    let json = serde_json::to_value(&response).unwrap();

    let deserialized: McpMessage = serde_json::from_value(json).unwrap();
    match deserialized {
        McpMessage::Response(resp) => {
            assert_eq!(resp.id, Some(json!(1)));
            assert!(resp.result.is_none());
            assert_eq!(resp.error, Some(error));
        }
        _ => panic!("Expected response message"),
    }
}

#[test]
fn test_mcp_error_codes() {
    let parse_error = McpError::parse_error("Parse error");
    assert_eq!(parse_error.code, McpError::PARSE_ERROR);
    assert_eq!(parse_error.message, "Parse error");

    let invalid_request = McpError::invalid_request("Invalid request");
    assert_eq!(invalid_request.code, McpError::INVALID_REQUEST);

    let method_not_found = McpError::method_not_found("unknown_method");
    assert_eq!(method_not_found.code, McpError::METHOD_NOT_FOUND);
    assert!(method_not_found.message.contains("unknown_method"));

    let invalid_params = McpError::invalid_params("Invalid parameters");
    assert_eq!(invalid_params.code, McpError::INVALID_PARAMS);

    let internal_error = McpError::internal_error("Internal error");
    assert_eq!(internal_error.code, McpError::INTERNAL_ERROR);
}

#[test]
fn test_lsp_request_creation() {
    let request = LspRequest::new(1, "textDocument/definition");
    assert_eq!(request.jsonrpc, "2.0");
    assert_eq!(request.id, json!(1));
    assert_eq!(request.method, "textDocument/definition");
    assert!(request.params.is_none());

    let params = json!({"textDocument": {"uri": "file:///test.rs"}, "position": {"line": 10, "character": 5}});
    let request_with_params = LspRequest::with_params(2, "textDocument/definition", params.clone());
    assert_eq!(request_with_params.params, Some(params));
}

#[test]
fn test_lsp_response_creation() {
    let result = json!({"uri": "file:///test.rs", "range": {"start": {"line": 5, "character": 0}, "end": {"line": 5, "character": 10}}});
    let response = LspResponse::success(1, result.clone());
    assert_eq!(response.jsonrpc, "2.0");
    assert_eq!(response.id, json!(1));
    assert_eq!(response.result, Some(result));
    assert!(response.error.is_none());

    let error = LspError {
        code: -32602,
        message: "Invalid params".to_string(),
        data: None,
    };
    let error_response = LspResponse::error(2, error.clone());
    assert_eq!(error_response.error, Some(error));
    assert!(error_response.result.is_none());
}

#[test]
fn test_lsp_position_serialization() {
    let position = LspPosition {
        line: 10,
        character: 5,
    };
    let json = serde_json::to_value(&position).unwrap();

    assert_eq!(json["line"], 10);
    assert_eq!(json["character"], 5);

    let deserialized: LspPosition = serde_json::from_value(json).unwrap();
    assert_eq!(deserialized, position);
}

#[test]
fn test_lsp_range_serialization() {
    let range = LspRange {
        start: LspPosition {
            line: 0,
            character: 0,
        },
        end: LspPosition {
            line: 0,
            character: 10,
        },
    };

    let json = serde_json::to_value(&range).unwrap();
    assert_eq!(json["start"]["line"], 0);
    assert_eq!(json["start"]["character"], 0);
    assert_eq!(json["end"]["line"], 0);
    assert_eq!(json["end"]["character"], 10);

    let deserialized: LspRange = serde_json::from_value(json).unwrap();
    assert_eq!(deserialized, range);
}

#[test]
fn test_fuse_config_default() {
    let config = FuseConfig::default();
    assert_eq!(config.mount_point.to_str().unwrap(), "/tmp/codebuddy");
    assert!(config.read_only);
    assert_eq!(config.cache_timeout_seconds, 60);
    assert_eq!(config.max_file_size_bytes, 10 * 1024 * 1024);
    assert!(!config.debug);
    assert_eq!(config.mount_options.len(), 2);
    assert!(config.mount_options.contains(&"auto_unmount".to_string()));
    assert!(config
        .mount_options
        .contains(&"default_permissions".to_string()));
}

#[test]
fn test_fuse_config_serialization() {
    let config = FuseConfig::default();
    let json = serde_json::to_value(&config).unwrap();

    let deserialized: FuseConfig = serde_json::from_value(json).unwrap();
    assert_eq!(deserialized, config);
}

#[test]
fn test_fuse_result_conversion() {
    let ok_result: FuseResult<String> = FuseResult::Ok("test".to_string());
    assert!(ok_result.is_ok());
    assert!(!ok_result.is_err());

    let err_result: FuseResult<String> = FuseResult::Err(FuseError::NotFound);
    assert!(!err_result.is_ok());
    assert!(err_result.is_err());

    // Test conversion to Result
    let std_result = ok_result.into_result();
    assert!(std_result.is_ok());
    assert_eq!(std_result.unwrap(), "test");

    let std_err_result = err_result.into_result();
    assert!(std_err_result.is_err());
    matches!(std_err_result.unwrap_err(), FuseError::NotFound);
}

#[test]
fn test_intent_spec_creation() {
    let args = json!({"file": "test.rs", "method": "rename"});
    let intent = IntentSpec::new("refactor", args.clone());

    assert_eq!(intent.name(), "refactor");
    assert_eq!(intent.arguments(), &args);
    assert!(intent.metadata().is_none());
    assert!(intent.correlation_id().is_none());
    assert!(intent.source().is_none());
    assert!(intent.priority().is_none());
}

#[test]
fn test_intent_spec_with_metadata() {
    let args = json!({"action": "test"});
    let metadata = IntentMetadata::new("user")
        .with_correlation_id("req-123")
        .with_priority(8)
        .with_context("session_id", json!("session-456"));

    let intent = IntentSpec::with_metadata("test_intent", args, metadata);

    assert_eq!(intent.name(), "test_intent");
    assert_eq!(intent.source().unwrap(), "user");
    assert_eq!(intent.correlation_id().unwrap(), "req-123");
    assert_eq!(intent.priority().unwrap(), 8);

    let metadata = intent.metadata().unwrap();
    assert!(metadata.timestamp.is_some());
    assert_eq!(
        metadata.context.get("session_id").unwrap(),
        &json!("session-456")
    );
}

#[test]
fn test_intent_result_creation() {
    let success_result = IntentResult::success();
    assert!(success_result.success);
    assert!(success_result.error.is_none());

    let data = json!({"result": "completed"});
    let success_with_data = IntentResult::success_with_data(data.clone());
    assert!(success_with_data.success);
    assert_eq!(success_with_data.data.unwrap(), data);

    let error = IntentError::new("EXEC_FAILED", "Execution failed");
    let failure_result = IntentResult::failure(error.clone());
    assert!(!failure_result.success);
    assert_eq!(failure_result.error.unwrap(), error);
}

#[test]
fn test_intent_context_management() {
    let intent = IntentSpec::new("test", json!({}));
    let mut context = IntentContext::new("exec-123", intent)
        .with_status(IntentStatus::Running)
        .with_parent("parent-exec-456");

    assert_eq!(context.execution_id, "exec-123");
    assert_eq!(context.status, IntentStatus::Running);
    assert_eq!(
        context.parent_execution_id.as_ref().unwrap(),
        "parent-exec-456"
    );
    assert!(context.child_execution_ids.is_empty());

    context.add_child("child-exec-789");
    assert_eq!(context.child_execution_ids.len(), 1);
    assert_eq!(context.child_execution_ids[0], "child-exec-789");
}

#[test]
fn test_intent_serialization() {
    let metadata = IntentMetadata::new("system")
        .with_correlation_id("corr-123")
        .with_priority(5);

    let intent = IntentSpec::with_metadata("test_action", json!({"param": "value"}), metadata);

    // Serialize to JSON
    let json = serde_json::to_value(&intent).unwrap();

    // Verify camelCase naming
    assert!(json.get("metadata").is_some());
    let metadata_json = &json["metadata"];
    assert_eq!(metadata_json["source"], "system");
    assert_eq!(metadata_json["correlationId"], "corr-123");
    assert_eq!(metadata_json["priority"], 5);

    // Deserialize back
    let deserialized: IntentSpec = serde_json::from_value(json).unwrap();
    assert_eq!(deserialized.name, intent.name);
    assert_eq!(deserialized.arguments, intent.arguments);

    let deserialized_metadata = deserialized.metadata.unwrap();
    let original_metadata = intent.metadata.unwrap();
    assert_eq!(deserialized_metadata.source, original_metadata.source);
    assert_eq!(
        deserialized_metadata.correlation_id,
        original_metadata.correlation_id
    );
    assert_eq!(deserialized_metadata.priority, original_metadata.priority);
}
