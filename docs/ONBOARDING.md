# Contributor Onboarding Guide

Welcome to Codebuddy! This guide will help you get started with contributing to the project by walking you through the process of adding new tools and handlers.

## Table of Contents

- [Quick Start](#quick-start)
- [Adding a New Tool to an Existing Handler](#adding-a-new-tool-to-an-existing-handler)
- [Creating a New Handler](#creating-a-new-handler)
- [Testing Your Changes](#testing-your-changes)
- [Best Practices](#best-practices)

## Quick Start

Before you begin, make sure you have:

1. **Rust toolchain installed** (rustc, cargo)
2. **Cloned the repository**
3. **Built the project**: `cargo build`
4. **Run the tests**: `cargo test`

## Adding a New Tool to an Existing Handler

Adding a new tool to an existing handler is straightforward and requires modifying just one file.

### Step 1: Choose the Appropriate Handler

First, determine which handler should own your new tool. Handlers are organized by functionality:

| Handler | Location | Purpose | Example Tools |
|---------|----------|---------|---------------|
| **SystemHandler** | `crates/cb-server/src/handlers/tools/system.rs` | System operations | `health_check`, `web_fetch` |
| **LifecycleHandler** | `crates/cb-server/src/handlers/tools/lifecycle.rs` | File lifecycle events | `notify_file_opened` |
| **NavigationHandler** | `crates/cb-server/src/handlers/tools/navigation.rs` | Code navigation | `find_definition`, `find_references` |
| **EditingHandler** | `crates/cb-server/src/handlers/tools/editing.rs` | Code editing | `rename_symbol`, `format_document` |
| **RefactoringHandler** | `crates/cb-server/src/handlers/tools/advanced.rs` | Advanced refactoring | `extract_function`, `inline_variable` |
| **FileOpsHandler** | `crates/cb-server/src/handlers/tools/file_ops.rs` | File operations | `read_file`, `write_file` |
| **WorkspaceHandler** | `crates/cb-server/src/handlers/tools/workspace.rs` | Workspace operations | `list_files`, `find_dead_code` |

### Step 2: Add the Tool Name

Open the appropriate handler file and add your tool name to the `TOOL_NAMES` constant array.

**Example** (adding `get_call_graph` to NavigationHandler):

```rust
// crates/cb-server/src/handlers/tools/navigation.rs

const TOOL_NAMES: &[&str] = &[
    "find_definition",
    "find_references",
    "get_document_symbols",
    "find_implementations",
    "find_type_definition",
    "search_workspace_symbols",
    "get_call_graph", // ← Add your new tool here
];
```

### Step 3: Implement the Handler Logic

Add a new match arm in the `handle_tool_call` method:

```rust
impl NavigationHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ToolHandler for NavigationHandler {
    fn tool_names(&self) -> &[&str] {
        TOOL_NAMES
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        match tool_call.name.as_str() {
            "find_definition" => self.find_definition(context, tool_call).await,
            "find_references" => self.find_references(context, tool_call).await,
            // ... other tools ...

            "get_call_graph" => self.get_call_graph(context, tool_call).await, // ← Add match arm

            _ => Err(ServerError::Unsupported(format!(
                "Unsupported navigation tool: {}",
                tool_call.name
            ))),
        }
    }
}
```

### Step 4: Implement the Tool Method

Add the actual implementation as a private method:

```rust
impl NavigationHandler {
    /// Get the call graph for a function
    async fn get_call_graph(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // Extract parameters
        let args = tool_call.arguments.as_ref()
            .ok_or_else(|| ServerError::InvalidRequest("Missing arguments".to_string()))?;

        let file_path = args["file_path"]
            .as_str()
            .ok_or_else(|| ServerError::InvalidRequest("Missing file_path".to_string()))?;

        let line = args["line"]
            .as_u64()
            .ok_or_else(|| ServerError::InvalidRequest("Missing line".to_string()))? as u32;

        let character = args["character"]
            .as_u64()
            .ok_or_else(|| ServerError::InvalidRequest("Missing character".to_string()))? as u32;

        // Create LSP plugin request
        let plugin_request = PluginRequest {
            method: "get_call_graph".to_string(),
            file_path: file_path.to_string(),
            params: json!({
                "line": line,
                "character": character
            }),
            request_id: None,
        };

        // Dispatch to plugin system
        match context.plugin_manager.handle_request(plugin_request).await {
            Ok(response) => Ok(json!({
                "content": response.data,
                "metadata": response.metadata
            })),
            Err(e) => Err(ServerError::Internal(format!("Plugin error: {}", e))),
        }
    }
}
```

### Step 5: Test Your Tool

Add a test to verify your tool works:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_call_graph() {
        // Create test context
        let context = create_test_context().await;

        let tool_call = ToolCall {
            name: "get_call_graph".to_string(),
            arguments: Some(json!({
                "file_path": "test.ts",
                "line": 10,
                "character": 5
            })),
        };

        let handler = NavigationHandler::new();
        let result = handler.handle_tool_call(&context, &tool_call).await;

        assert!(result.is_ok());
    }
}
```

### Step 6: Verify Registration

Run the test to ensure all tools are still registered:

```bash
cargo test test_all_42_tools_are_registered
```

Update the expected count in `crates/cb-server/tests/tool_registration_test.rs` if you added a new tool:

```rust
assert_eq!(registered_tools.len(), 43); // Updated from 42
```

That's it! Your new tool is now fully integrated and will appear in `codebuddy tools`.

---

## Creating a New Handler

Creating a new handler is required when you're adding a category of related tools that doesn't fit into existing handlers.

### Step 1: Create the Handler File

Create a new file in `crates/cb-server/src/handlers/tools/` for your handler:

```bash
touch crates/cb-server/src/handlers/tools/diagnostics.rs
```

### Step 2: Define the Handler Struct

```rust
//! Diagnostic tools for code quality and analysis

use super::{ToolHandler, ToolHandlerContext};
use crate::{ServerError, ServerResult};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_plugins::PluginRequest;
use serde_json::{json, Value};
use tracing::{debug, error};

/// Handler for diagnostic tools
pub struct DiagnosticsHandler;

const TOOL_NAMES: &[&str] = &[
    "get_diagnostics",
    "get_code_quality_metrics",
    "find_security_issues",
];

impl DiagnosticsHandler {
    pub fn new() -> Self {
        Self
    }
}
```

### Step 3: Implement the ToolHandler Trait

```rust
#[async_trait]
impl ToolHandler for DiagnosticsHandler {
    fn tool_names(&self) -> &[&str] {
        TOOL_NAMES
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        match tool_call.name.as_str() {
            "get_diagnostics" => self.get_diagnostics(context, tool_call).await,
            "get_code_quality_metrics" => self.get_code_quality_metrics(context, tool_call).await,
            "find_security_issues" => self.find_security_issues(context, tool_call).await,
            _ => Err(ServerError::Unsupported(format!(
                "Unsupported diagnostic tool: {}",
                tool_call.name
            ))),
        }
    }
}

impl DiagnosticsHandler {
    async fn get_diagnostics(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // Implementation here
        todo!("Implement get_diagnostics")
    }

    async fn get_code_quality_metrics(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // Implementation here
        todo!("Implement get_code_quality_metrics")
    }

    async fn find_security_issues(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // Implementation here
        todo!("Implement find_security_issues")
    }
}
```

### Step 4: Export the Handler

Add your handler to `crates/cb-server/src/handlers/tools/mod.rs`:

```rust
// Near the top of the file
pub mod diagnostics; // ← Add this line

// In the public exports section
pub use diagnostics::DiagnosticsHandler; // ← Add this line
```

### Step 5: Register the Handler

Add your handler to the macro call in `crates/cb-server/src/handlers/plugin_dispatcher.rs`:

```rust
pub async fn initialize(&self) -> ServerResult<()> {
    let mut registry = self.tool_registry.lock().await;

    register_handlers_with_logging!(registry, {
        SystemHandler => "SystemHandler with 3 tools: health_check, web_fetch, ping",
        LifecycleHandler => "LifecycleHandler with 3 tools: notify_file_opened, notify_file_saved, notify_file_closed",
        NavigationHandler => "NavigationHandler with 10 tools: find_definition, find_references, ...",
        EditingHandler => "EditingHandler with 9 tools: rename_symbol, format_document, ...",
        RefactoringHandler => "RefactoringHandler with 4 tools: extract_function, inline_variable, ...",
        FileOpsHandler => "FileOpsHandler with 6 tools: read_file, write_file, ...",
        WorkspaceHandler => "WorkspaceHandler with 7 tools: list_files, find_dead_code, ...",
        DiagnosticsHandler => "DiagnosticsHandler with 3 tools: get_diagnostics, get_code_quality_metrics, find_security_issues", // ← Add this line
    });

    Ok(())
}
```

### Step 6: Update Tests

Update the safety net test in `crates/cb-server/tests/tool_registration_test.rs`:

```rust
#[tokio::test]
async fn test_all_42_tools_are_registered() {
    let dispatcher = create_test_dispatcher();
    dispatcher.initialize().await.unwrap();

    let registry = dispatcher.tool_registry.lock().await;
    let registered_tools = registry.list_tools();

    // Update expected count
    assert_eq!(registered_tools.len(), 45); // Was 42, now 45 with 3 new tools

    // Add assertions for your new tools
    assert!(registered_tools.contains(&"get_diagnostics".to_string()));
    assert!(registered_tools.contains(&"get_code_quality_metrics".to_string()));
    assert!(registered_tools.contains(&"find_security_issues".to_string()));
}
```

### Step 7: Verify Everything Works

```bash
# Build the project
cargo build

# Run all tests
cargo test

# Check your tools appear in the CLI
cargo run --bin codebuddy -- tools
```

---

## Testing Your Changes

### Unit Tests

Add unit tests directly in your handler file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::create_test_context;

    #[tokio::test]
    async fn test_tool_names() {
        let handler = DiagnosticsHandler::new();
        assert_eq!(handler.tool_names().len(), 3);
        assert!(handler.tool_names().contains(&"get_diagnostics"));
    }

    #[tokio::test]
    async fn test_get_diagnostics() {
        let context = create_test_context().await;
        let handler = DiagnosticsHandler::new();

        let tool_call = ToolCall {
            name: "get_diagnostics".to_string(),
            arguments: Some(json!({
                "file_path": "test.ts"
            })),
        };

        let result = handler.handle_tool_call(&context, &tool_call).await;
        assert!(result.is_ok());
    }
}
```

### Integration Tests

The safety net test (`test_all_42_tools_are_registered`) ensures your tools are properly registered:

```bash
cargo test test_all_42_tools_are_registered --test tool_registration_test
```

### Manual Testing

Test your tool via the CLI:

```bash
# List all tools (verify yours appears)
cargo run --bin codebuddy -- tools

# Call your tool directly
cargo run --bin codebuddy -- tool get_diagnostics '{"file_path":"src/main.rs"}'
```

---

## Best Practices

### 1. Follow Naming Conventions

- **Tool names**: Use snake_case (e.g., `get_diagnostics`, `find_security_issues`)
- **Handler names**: Use PascalCase with "Handler" suffix (e.g., `DiagnosticsHandler`)
- **File names**: Use snake_case matching the handler (e.g., `diagnostics.rs`)

### 2. Use Structured Logging

Always use the `tracing` crate with structured key-value logging:

```rust
// ✅ Good - structured logging
debug!(tool_name = %tool_call.name, file_path = %path, "Processing tool call");
error!(error = %e, tool = "get_diagnostics", "Tool execution failed");

// ❌ Bad - string interpolation
debug!("Processing tool call {} for file {}", tool_call.name, path);
error!("Tool execution failed: {} - {}", "get_diagnostics", e);
```

### 3. Comprehensive Error Handling

Provide clear, actionable error messages:

```rust
// ✅ Good
let file_path = args["file_path"]
    .as_str()
    .ok_or_else(|| ServerError::InvalidRequest(
        "Missing required parameter 'file_path'"
    ))?;

// ❌ Bad
let file_path = args["file_path"].as_str().unwrap();
```

### 4. Document Public Functions

Add doc comments explaining purpose, parameters, and return values:

```rust
/// Get diagnostic information for a file
///
/// This tool analyzes a source file and returns diagnostic information
/// including errors, warnings, and hints from the language server.
///
/// # Arguments
///
/// * `context` - Handler context with access to services
/// * `tool_call` - The tool call with file_path parameter
///
/// # Returns
///
/// Returns a JSON object with diagnostic messages, or an error if the
/// file cannot be analyzed.
async fn get_diagnostics(
    &self,
    context: &ToolHandlerContext,
    tool_call: &ToolCall,
) -> ServerResult<Value> {
    // ...
}
```

### 5. Keep Handlers Focused

Each handler should have a single, clear responsibility:

- ✅ **Good**: NavigationHandler handles all code navigation tools
- ❌ **Bad**: NavigationHandler also handles file operations and diagnostics

### 6. Validate Input Early

Validate and extract parameters at the start of each method:

```rust
async fn get_diagnostics(...) -> ServerResult<Value> {
    // Extract and validate all parameters first
    let args = tool_call.arguments.as_ref()
        .ok_or_else(|| ServerError::InvalidRequest("Missing arguments"))?;

    let file_path = args["file_path"].as_str()
        .ok_or_else(|| ServerError::InvalidRequest("Missing file_path"))?;

    let severity_filter = args.get("severity")
        .and_then(|s| s.as_str())
        .unwrap_or("all");

    // Now perform the operation
    // ...
}
```

### 7. Run Tests Before Committing

Always run the full test suite before creating a PR:

```bash
# Run all tests
cargo test

# Run clippy for linting
cargo clippy

# Format code
cargo fmt

# Run benchmarks to check performance
cargo bench
```

---

## Getting Help

- **Documentation**: See [ARCHITECTURE.md](docs/architecture/ARCHITECTURE.md) for system overview
- **API Reference**: See [API.md](API.md) for complete tool API
- **Issues**: Check [GitHub Issues](https://github.com/codebuddy/codebuddy/issues) for known problems
- **Community**: Join discussions in GitHub Discussions

---

## Example: Complete Handler Implementation

Here's a complete, working example of a simple handler:

```rust
//! Example handler demonstrating best practices

use super::{ToolHandler, ToolHandlerContext};
use crate::{ServerError, ServerResult};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use serde_json::{json, Value};
use tracing::debug;

/// Handler for example tools
pub struct ExampleHandler;

const TOOL_NAMES: &[&str] = &["echo", "reverse"];

impl ExampleHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ToolHandler for ExampleHandler {
    fn tool_names(&self) -> &[&str] {
        TOOL_NAMES
    }

    async fn handle_tool_call(
        &self,
        _context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        match tool_call.name.as_str() {
            "echo" => self.echo(tool_call).await,
            "reverse" => self.reverse(tool_call).await,
            _ => Err(ServerError::Unsupported(format!(
                "Unsupported example tool: {}",
                tool_call.name
            ))),
        }
    }
}

impl ExampleHandler {
    /// Echo back the input message
    async fn echo(&self, tool_call: &ToolCall) -> ServerResult<Value> {
        let args = tool_call
            .arguments
            .as_ref()
            .ok_or_else(|| ServerError::InvalidRequest("Missing arguments".to_string()))?;

        let message = args["message"]
            .as_str()
            .ok_or_else(|| ServerError::InvalidRequest("Missing message".to_string()))?;

        debug!(message = %message, "Echoing message");

        Ok(json!({
            "echo": message
        }))
    }

    /// Reverse a string
    async fn reverse(&self, tool_call: &ToolCall) -> ServerResult<Value> {
        let args = tool_call
            .arguments
            .as_ref()
            .ok_or_else(|| ServerError::InvalidRequest("Missing arguments".to_string()))?;

        let text = args["text"]
            .as_str()
            .ok_or_else(|| ServerError::InvalidRequest("Missing text".to_string()))?;

        let reversed: String = text.chars().rev().collect();

        debug!(original = %text, reversed = %reversed, "Reversed string");

        Ok(json!({
            "original": text,
            "reversed": reversed
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_echo() {
        let handler = ExampleHandler::new();
        let tool_call = ToolCall {
            name: "echo".to_string(),
            arguments: Some(json!({"message": "hello"})),
        };

        let result = handler.echo(&tool_call).await.unwrap();
        assert_eq!(result["echo"], "hello");
    }

    #[tokio::test]
    async fn test_reverse() {
        let handler = ExampleHandler::new();
        let tool_call = ToolCall {
            name: "reverse".to_string(),
            arguments: Some(json!({"text": "hello"})),
        };

        let result = handler.reverse(&tool_call).await.unwrap();
        assert_eq!(result["reversed"], "olleh");
    }
}
```

---

Happy coding! 🚀
