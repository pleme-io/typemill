//! Test helper functions and utilities

use mill_config::AppConfig;
use codebuddy_foundation::core::model::*;
use codebuddy_foundation::protocol::{EditPlan, ImportGraph};
use serde_json::json;
use std::path::Path;

/// Create a test configuration for testing
pub fn create_test_config() -> AppConfig {
    let mut config = AppConfig::default();

    // Use test-specific settings
    config.server.host = "127.0.0.1".to_string();
    config.server.port = 3043; // Use port from allowed range for testing
    config.server.timeout_ms = 1000; // Short timeout for tests

    config.logging.level = "debug".to_string();
    config.cache.enabled = false; // Disable cache for predictable tests

    config
}

/// Create a test intent specification
pub fn create_test_intent(name: &str) -> IntentSpec {
    IntentSpec::new(
        name,
        json!({
            "sourceFile": "test.ts",
            "oldName": "oldFunction",
            "newName": "newFunction"
        }),
    )
}

/// Create a test MCP request message
pub fn create_test_mcp_request(method: &str) -> McpMessage {
    McpMessage::request(1, method)
}

/// Create a test MCP response message
pub fn create_test_mcp_response() -> McpMessage {
    McpMessage::success_response(1, json!({"result": "success"}))
}

/// Create a test import graph
pub fn create_test_import_graph(source_file: &str) -> ImportGraph {
    use codebuddy_foundation::protocol::{
        ImportGraphMetadata, ImportInfo, ImportType, SourceLocation,
    };

    ImportGraph {
        source_file: source_file.to_string(),
        imports: vec![ImportInfo {
            module_path: "./utils".to_string(),
            import_type: ImportType::EsModule,
            named_imports: vec![],
            default_import: Some("utils".to_string()),
            namespace_import: None,
            type_only: false,
            location: SourceLocation {
                start_line: 0,
                start_column: 0,
                end_line: 0,
                end_column: 25,
            },
        }],
        importers: vec![],
        metadata: ImportGraphMetadata {
            language: "typescript".to_string(),
            parsed_at: chrono::Utc::now(),
            parser_version: "0.1.0".to_string(),
            circular_dependencies: vec![],
            external_dependencies: vec![],
        },
    }
}

/// Create a test edit plan
pub fn create_test_edit_plan() -> EditPlan {
    use codebuddy_foundation::protocol::{EditLocation, EditPlanMetadata, EditType, TextEdit};

    EditPlan {
        source_file: "test.ts".to_string(),
        edits: vec![TextEdit {
            file_path: None,
            edit_type: EditType::Rename,
            location: EditLocation {
                start_line: 5,
                start_column: 10,
                end_line: 5,
                end_column: 20,
            },
            original_text: "oldFunction".to_string(),
            new_text: "newFunction".to_string(),
            priority: 100,
            description: "Rename function".to_string(),
        }],
        dependency_updates: vec![],
        validations: vec![],
        metadata: EditPlanMetadata {
            intent_name: "rename.plan".to_string(),
            intent_arguments: json!({"oldName": "oldFunction", "newName": "newFunction"}),
            created_at: chrono::Utc::now(),
            complexity: 3,
            impact_areas: vec!["functions".to_string()],
            consolidation: None,
        },
    }
}

/// Assert that two JSON values are equal (with better error messages)
pub fn assert_json_eq(actual: &serde_json::Value, expected: &serde_json::Value) {
    if actual != expected {
        panic!(
            "JSON values not equal:\nActual: {}\nExpected: {}",
            serde_json::to_string_pretty(actual).unwrap(),
            serde_json::to_string_pretty(expected).unwrap()
        );
    }
}

/// Create a temporary file with given content for testing
pub fn create_temp_file(content: &str) -> tempfile::NamedTempFile {
    use std::io::Write;

    let mut file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(content.as_bytes())
        .expect("Failed to write temp file");
    file
}

/// Extract file extension from path
pub fn get_file_extension(path: &Path) -> Option<&str> {
    path.extension().and_then(|ext| ext.to_str())
}

/// Generate a unique test ID for test isolation
pub fn generate_test_id() -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("test_{}", id)
}