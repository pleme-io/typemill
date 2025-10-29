# Test Fixtures

This directory contains JSON fixtures for contract validation. Each file represents a valid payload that can be used to test serialization/deserialization and API contracts.

## Fixture Files

- `app_config.json` - Complete AppConfig structure
- `mcp_request.json` - MCP request message example
- `mcp_response.json` - MCP response message example
- `intent_spec.json` - Intent specification with metadata
- `import_graph.json` - Import graph analysis result

## Usage in Tests

```rust
use std::fs;
use serde_json;
use cb_core::model::*;

# [test]
fn test_mcp_request_contract() {
    let fixture = fs::read_to_string("fixtures/mcp_request.json").unwrap();
    let request: McpRequest = serde_json::from_str(&fixture).unwrap();
    assert_eq!(request.method, "tools/call");
}
```text
## Validation

All fixtures are validated against the actual Rust types during tests to ensure contract compatibility.