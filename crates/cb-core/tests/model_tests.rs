//! Tests for protocol models

use cb_core::model::*;
use serde_json::json;

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
