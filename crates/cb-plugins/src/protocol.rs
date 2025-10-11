//! Protocol abstraction layer
//!
//! This module provides protocol-agnostic request/response types that hide
//! LSP-specific details from plugins.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

/// Protocol-agnostic request sent to plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRequest {
    /// The method being requested (e.g., "find_definition", "rename.plan")
    pub method: String,
    /// The file path being operated on
    pub file_path: PathBuf,
    /// Position information (line, column) if relevant
    pub position: Option<Position>,
    /// Range information if relevant
    pub range: Option<Range>,
    /// Method-specific parameters
    pub params: Value,
    /// Request correlation ID for tracing
    pub request_id: Option<String>,
}

/// Protocol-agnostic response from plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResponse {
    /// Success or error indicator
    pub success: bool,
    /// Response data for successful requests
    pub data: Option<Value>,
    /// Error message for failed requests
    pub error: Option<String>,
    /// Request correlation ID
    pub request_id: Option<String>,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

/// Position in a text document (0-indexed)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Position {
    /// Line number (0-indexed)
    pub line: u32,
    /// Character/column number (0-indexed)
    pub character: u32,
}

/// Range in a text document
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Range {
    /// Start position
    pub start: Position,
    /// End position
    pub end: Position,
}

/// Response metadata for debugging and monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// Plugin that handled the request
    pub plugin_name: String,
    /// Processing time in milliseconds
    pub processing_time_ms: Option<u64>,
    /// Whether response was cached
    pub cached: bool,
    /// Additional plugin-specific metadata
    pub plugin_metadata: Value,
}

impl PluginRequest {
    /// Create a new plugin request
    pub fn new(method: impl Into<String>, file_path: PathBuf) -> Self {
        Self {
            method: method.into(),
            file_path,
            position: None,
            range: None,
            params: Value::Null,
            request_id: None,
        }
    }

    /// Set position for the request
    pub fn with_position(mut self, line: u32, character: u32) -> Self {
        self.position = Some(Position { line, character });
        self
    }

    /// Set range for the request
    pub fn with_range(
        mut self,
        start_line: u32,
        start_char: u32,
        end_line: u32,
        end_char: u32,
    ) -> Self {
        self.range = Some(Range {
            start: Position {
                line: start_line,
                character: start_char,
            },
            end: Position {
                line: end_line,
                character: end_char,
            },
        });
        self
    }

    /// Set parameters for the request
    pub fn with_params(mut self, params: Value) -> Self {
        self.params = params;
        self
    }

    /// Set request ID for correlation
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    /// Get a parameter by key
    pub fn get_param(&self, key: &str) -> Option<&Value> {
        self.params.get(key)
    }

    /// Get a string parameter
    pub fn get_string_param(&self, key: &str) -> Option<&str> {
        self.params.get(key)?.as_str()
    }

    /// Get a boolean parameter
    pub fn get_bool_param(&self, key: &str) -> Option<bool> {
        self.params.get(key)?.as_bool()
    }

    /// Get a number parameter
    pub fn get_number_param(&self, key: &str) -> Option<f64> {
        self.params.get(key)?.as_f64()
    }
}

impl PluginResponse {
    /// Create a successful response
    pub fn success(data: Value, plugin_name: impl Into<String>) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            request_id: None,
            metadata: ResponseMetadata {
                plugin_name: plugin_name.into(),
                processing_time_ms: None,
                cached: false,
                plugin_metadata: Value::Null,
            },
        }
    }

    /// Create an error response
    pub fn error(error: impl Into<String>, plugin_name: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.into()),
            request_id: None,
            metadata: ResponseMetadata {
                plugin_name: plugin_name.into(),
                processing_time_ms: None,
                cached: false,
                plugin_metadata: Value::Null,
            },
        }
    }

    /// Create an empty successful response
    pub fn empty() -> Self {
        Self {
            success: true,
            data: Some(Value::Null),
            error: None,
            request_id: None,
            metadata: ResponseMetadata {
                plugin_name: "unknown".to_string(),
                processing_time_ms: None,
                cached: false,
                plugin_metadata: Value::Null,
            },
        }
    }

    /// Set request correlation ID
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    /// Set processing time
    pub fn with_processing_time(mut self, time_ms: u64) -> Self {
        self.metadata.processing_time_ms = Some(time_ms);
        self
    }

    /// Mark response as cached
    pub fn with_cached(mut self, cached: bool) -> Self {
        self.metadata.cached = cached;
        self
    }

    /// Set plugin-specific metadata
    pub fn with_plugin_metadata(mut self, metadata: Value) -> Self {
        self.metadata.plugin_metadata = metadata;
        self
    }
}

impl Position {
    /// Create a new position
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }

    /// Convert to LSP position format
    pub fn to_lsp_position(&self) -> Value {
        serde_json::json!({
            "line": self.line,
            "character": self.character
        })
    }

    /// Create from LSP position format
    pub fn from_lsp_position(value: &Value) -> Option<Self> {
        let line = value.get("line")?.as_u64()? as u32;
        let character = value.get("character")?.as_u64()? as u32;
        Some(Self { line, character })
    }
}

impl Range {
    /// Create a new range
    pub fn new(start_line: u32, start_char: u32, end_line: u32, end_char: u32) -> Self {
        Self {
            start: Position::new(start_line, start_char),
            end: Position::new(end_line, end_char),
        }
    }

    /// Convert to LSP range format
    pub fn to_lsp_range(&self) -> Value {
        serde_json::json!({
            "start": self.start.to_lsp_position(),
            "end": self.end.to_lsp_position()
        })
    }

    /// Create from LSP range format
    pub fn from_lsp_range(value: &Value) -> Option<Self> {
        let start = Position::from_lsp_position(value.get("start")?)?;
        let end = Position::from_lsp_position(value.get("end")?)?;
        Some(Self { start, end })
    }

    /// Check if this range contains a position
    pub fn contains(&self, position: Position) -> bool {
        (self.start.line < position.line
            || (self.start.line == position.line && self.start.character <= position.character))
            && (self.end.line > position.line
                || (self.end.line == position.line && self.end.character >= position.character))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn test_plugin_request_builder() {
        let request = PluginRequest::new("find_definition", PathBuf::from("test.ts"))
            .with_position(10, 20)
            .with_params(json!({"symbol": "test"}))
            .with_request_id("req-123");

        assert_eq!(request.method, "find_definition");
        assert_eq!(request.file_path, PathBuf::from("test.ts"));
        assert_eq!(
            request.position,
            Some(Position {
                line: 10,
                character: 20
            })
        );
        assert_eq!(request.get_string_param("symbol"), Some("test"));
        assert_eq!(request.request_id, Some("req-123".to_string()));
    }

    #[test]
    fn test_plugin_response_success() {
        let response = PluginResponse::success(json!({"locations": []}), "typescript-plugin")
            .with_processing_time(150);

        assert!(response.success);
        assert!(response.error.is_none());
        assert_eq!(response.metadata.plugin_name, "typescript-plugin");
        assert_eq!(response.metadata.processing_time_ms, Some(150));
    }

    #[test]
    fn test_plugin_response_error() {
        let response = PluginResponse::error("File not found", "typescript-plugin");

        assert!(!response.success);
        assert_eq!(response.error, Some("File not found".to_string()));
        assert_eq!(response.metadata.plugin_name, "typescript-plugin");
    }

    #[test]
    fn test_position_lsp_conversion() {
        let pos = Position::new(10, 20);
        let lsp_pos = pos.to_lsp_position();
        let converted_back = Position::from_lsp_position(&lsp_pos).unwrap();

        assert_eq!(pos.line, converted_back.line);
        assert_eq!(pos.character, converted_back.character);
    }

    #[test]
    fn test_range_contains() {
        let range = Range::new(5, 10, 10, 20);

        assert!(range.contains(Position::new(7, 15)));
        assert!(range.contains(Position::new(5, 10))); // Start boundary
        assert!(range.contains(Position::new(10, 20))); // End boundary
        assert!(!range.contains(Position::new(3, 5))); // Before range
        assert!(!range.contains(Position::new(15, 25))); // After range
    }

    #[test]
    fn test_request_param_accessors() {
        let request = PluginRequest::new("test", PathBuf::from("test.ts")).with_params(json!({
            "text": "hello",
            "enabled": true,
            "count": 42.5
        }));

        assert_eq!(request.get_string_param("text"), Some("hello"));
        assert_eq!(request.get_bool_param("enabled"), Some(true));
        assert_eq!(request.get_number_param("count"), Some(42.5));
        assert_eq!(request.get_string_param("missing"), None);
    }
}
