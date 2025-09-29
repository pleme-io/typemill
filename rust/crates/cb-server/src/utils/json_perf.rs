//! High-performance JSON utilities using simd-json for optimization

use crate::error::{ServerError, ServerResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// High-performance JSON deserializer using SIMD optimizations
pub struct SimdJsonParser;

impl SimdJsonParser {
    /// Deserialize JSON from a mutable byte slice using SIMD acceleration
    ///
    /// This function is optimized for performance-critical paths where large
    /// JSON responses need to be processed quickly (e.g., find_references responses)
    pub fn from_slice<T>(mut bytes: Vec<u8>) -> ServerResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        // Use simd-json for SIMD-accelerated parsing
        match simd_json::from_slice(&mut bytes) {
            Ok(value) => Ok(value),
            Err(e) => {
                tracing::warn!(
                    "SIMD JSON parsing failed, falling back to serde_json: {}",
                    e
                );
                // Fallback to standard serde_json for compatibility
                Self::fallback_from_slice(&bytes)
            }
        }
    }

    /// Fast deserialization for serde_json::Value objects
    pub fn value_from_slice(mut bytes: Vec<u8>) -> ServerResult<Value> {
        match simd_json::to_owned_value(&mut bytes) {
            Ok(value) => {
                // Convert simd_json::OwnedValue to serde_json::Value
                let json_str = value.to_string();
                serde_json::from_str(&json_str)
                    .map_err(|e| ServerError::runtime(format!("JSON conversion error: {}", e)))
            }
            Err(e) => {
                tracing::warn!(
                    "SIMD value parsing failed, falling back to serde_json: {}",
                    e
                );
                Self::fallback_value_from_slice(&bytes)
            }
        }
    }

    /// Convert JSON Value to a specific type with SIMD optimization
    pub fn from_value<T>(value: Value) -> ServerResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        // For serde_json::Value to T conversion, we still use serde_json
        // as simd-json works with raw bytes, not Value objects
        serde_json::from_value(value)
            .map_err(|e| ServerError::InvalidRequest(format!("JSON deserialization error: {}", e)))
    }

    /// Fast serialization for performance-critical responses
    pub fn to_string<T>(value: &T) -> ServerResult<String>
    where
        T: Serialize,
    {
        // For serialization, we use serde_json as simd-json is primarily for parsing
        serde_json::to_string(value)
            .map_err(|e| ServerError::runtime(format!("JSON serialization error: {}", e)))
    }

    /// Fallback method using standard serde_json
    fn fallback_from_slice<T>(bytes: &[u8]) -> ServerResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let json_str = std::str::from_utf8(bytes)
            .map_err(|e| ServerError::InvalidRequest(format!("Invalid UTF-8: {}", e)))?;

        serde_json::from_str(json_str)
            .map_err(|e| ServerError::InvalidRequest(format!("JSON parsing error: {}", e)))
    }

    /// Fallback for Value parsing
    fn fallback_value_from_slice(bytes: &[u8]) -> ServerResult<Value> {
        let json_str = std::str::from_utf8(bytes)
            .map_err(|e| ServerError::InvalidRequest(format!("Invalid UTF-8: {}", e)))?;

        serde_json::from_str(json_str)
            .map_err(|e| ServerError::InvalidRequest(format!("JSON parsing error: {}", e)))
    }
}

/// Utility function for paginated responses to reduce JSON processing overhead
pub fn create_paginated_response<T>(
    items: Vec<T>,
    page_size: usize,
    page: usize,
    total_count: usize,
) -> Value
where
    T: Serialize,
{
    let start = page * page_size;
    let end = std::cmp::min(start + page_size, items.len());
    let page_items = &items[start..end];

    serde_json::json!({
        "items": page_items,
        "pagination": {
            "page": page,
            "page_size": page_size,
            "total_pages": total_count.div_ceil(page_size),
            "total_count": total_count,
            "has_next": end < total_count,
            "has_prev": page > 0
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestStruct {
        name: String,
        value: i32,
    }

    #[test]
    fn test_simd_json_parsing() {
        let test_data = TestStruct {
            name: "test".to_string(),
            value: 42,
        };

        let json_str = serde_json::to_string(&test_data).unwrap();
        let bytes = json_str.into_bytes();

        let parsed: TestStruct = SimdJsonParser::from_slice(bytes).unwrap();
        assert_eq!(parsed, test_data);
    }

    #[test]
    fn test_paginated_response() {
        let items = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let response = create_paginated_response(items, 3, 1, 10);

        let page_items = response["items"].as_array().unwrap();
        assert_eq!(page_items.len(), 3);
        assert_eq!(page_items[0], 4); // Second page starts at index 3

        let pagination = &response["pagination"];
        assert_eq!(pagination["page"], 1);
        assert_eq!(pagination["total_pages"], 4);
        assert_eq!(pagination["has_next"], true);
        assert_eq!(pagination["has_prev"], true);
    }
}
