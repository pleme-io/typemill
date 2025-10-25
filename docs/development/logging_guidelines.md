# Structured Logging Guidelines

Your guide to structured logging in TypeMill using the `tracing` crate. Follow these patterns to keep logs consistent, machine-readable, and easy to debug in production.

## Overview

All logging in this codebase uses **structured tracing** with key-value pairs to enable:
- Machine-readable logs for production debugging
- Efficient log querying and analysis
- Consistent observability across all components
- Enhanced production monitoring capabilities

## Core Principles

### 1. Use Structured Key-Value Format

**✅ CORRECT:**
```rust
error!(error = %e, file_path = %path, operation = "read", "Failed to read file");
info!(user_id = %user.id, action = "login", duration_ms = elapsed, "User authenticated");
debug!(request_id = %req_id, tool_name = %tool, "Processing tool request");
```

**❌ INCORRECT:**
```rust
error!("Failed to read file {}: {}", path, e);
info!("User {} authenticated in {}ms", user.id, elapsed);
debug!("Processing tool request {} with {}", req_id, tool);
```

### 2. Consistent Field Naming

Use these standardized field names across the codebase:

- **Errors**: `error = %e`
- **File paths**: `file_path = %path.display()` or `path = ?path`
- **Identifiers**: `user_id = %id`, `request_id = %req_id`, `plugin_name = %name`
- **Counts**: `files_count = count`, `items_processed = num`, `total_plugins = total`
- **Durations**: `duration_ms = elapsed`, `timeout_seconds = timeout`
- **Operations**: `operation = "read"`, `action = "login"`, `method = %method`
- **Network**: `url = %url`, `port = port`, `addr = %addr`

### 3. Field Formatting Rules

#### Use `%` for Display formatting:
```rust
error!(error = %e, file_path = %path.display(), "Failed to read file");
info!(url = %url, port = %port, "Server started");
```

#### Use `?` for Debug formatting:
```rust
debug!(result = ?result, params = ?params, "Function completed");
warn!(extensions = ?server_config.extensions, "No extensions configured");
```

#### Use bare values for simple types:
```rust
info!(count = files.len(), success = true, "Files processed");
debug!(timeout_seconds = 30, retry_attempts = 3, "Configuration loaded");
```

## Log Levels

### `error!` - Critical Issues
Use for runtime errors, failed operations, and system failures that require immediate attention:

```rust
error!(error = %e, plugin_name = %name, "Failed to register plugin");
error!(file_path = %path, error = %e, "Failed to write file");
error!(bind_addr = %addr, error = %e, "Failed to bind to address");
```

### `warn!` - Recoverable Issues
Use for issues that don't break functionality but need attention:

```rust
warn!(extension = %ext, "No LSP server configured for extension");
warn!(file_path = %path, error = %e, "Could not read file for dependency update");
warn!(operation_id = %id, wait_time = ?time, "Operation timed out");
```

### `info!` - Important Events
Use for significant business events, service lifecycle, and user actions:

```rust
info!(addr = %addr, "Server listening");
info!(plugin_name = %name, "Plugin registered successfully");
info!(files_count = count, "Files processed successfully");
```

### `debug!` - Detailed Flow
Use for detailed execution flow and internal state (disabled in production by default):

```rust
debug!(method = %method, "LSP request received");
debug!(file_path = %path, "Invalidated AST cache");
debug!(lock_type = ?lock_type, file_path = %path, "Acquiring lock");
```

## Context Patterns

### Error Context
Always include relevant context with errors:

```rust
// Good - includes operation context
error!(
    source_file = %plan.source_file,
    error = %e,
    "Failed to apply edits to main file"
);

// Good - includes request context
error!(
    request_id = %request_id,
    tool_name = %tool,
    error = %e,
    "Tool execution failed"
);
```

### Request Tracing
Use request IDs for tracing requests across components:

```rust
debug!(
    request_id = %request_id,
    line_length = trimmed.len(),
    "Received line"
);

error!(
    request_id = %request_id,
    error = %e,
    "Failed to handle message"
);
```

### Performance Context
Include timing information for performance analysis:

```rust
debug!(
    processing_time_ms = elapsed,
    tool_name = %tool,
    "Tool request processed"
);

info!(
    duration_ms = elapsed,
    files_modified = count,
    "Batch operation completed"
);
```

## Anti-Patterns

### ❌ String Interpolation
```rust
// Wrong - not machine readable
error!("Failed to process {} files in {}ms", count, elapsed);
```

### ❌ Mixing Structured and Unstructured
```rust
// Wrong - inconsistent format
error!(error = %e, "Failed to read file: {}", path);
```

### ❌ Redundant Information in Message
```rust
// Wrong - data duplicated in message and fields
error!(error = %e, "Error occurred: {}", e);
```

### ❌ Non-Standard Field Names
```rust
// Wrong - use standard field names
error!(err = %e, filepath = %path, "Failed to read file");
// Correct
error!(error = %e, file_path = %path, "Failed to read file");
```

## Testing Considerations

### Test Code Logging
- Test helper functions may use `println!` for debugging during development
- Production code should never use `println!`/`eprintln!` for logging
- Test assertions can validate log output using tracing test utilities

### Log Level Testing
```rust
#[cfg(test)]
mod tests {
    use tracing_test::traced_test;

    #[traced_test]
    fn test_error_logging() {
        // Test code that produces logs
        // Assertions can verify log content
    }
}
```

## Production Configuration

### Environment Variables
```bash
# Set log level
export RUST_LOG=info

# Enable JSON formatting for production
export LOG_FORMAT=json

# Enable debug logging for specific modules
export RUST_LOG=cb_server::handlers=debug,cb_plugins=info
```

### Configuration Example
```json
{
  "logging": {
    "level": "info",
    "format": "json"
  }
}
```

### Centralized Initialization

All logging is initialized through `cb_core::logging::initialize()` which provides:

**Configuration Sources** (in priority order):
1. `RUST_LOG` environment variable (most powerful, module-level filtering)
2. `LOG_LEVEL` environment variable (simple level override)
3. `LOG_FORMAT` environment variable (json or pretty)
4. Application configuration file (`config.logging`)

**Standard Environment Variables**:
- `RUST_LOG`: Module-level filtering (e.g., `cb_handlers=debug,cb_lsp=info`)
- `LOG_LEVEL`: Simple level control (trace, debug, info, warn, error)
- `LOG_FORMAT`: Format override (json, pretty, human)

**Examples**:
```bash
# Development with debug logging
LOG_LEVEL=debug cargo run

# Production with JSON logs
LOG_LEVEL=info LOG_FORMAT=json ./mill serve

# Module-specific filtering (most control)
RUST_LOG=cb_handlers=debug,cb_lsp=info cargo run
```

### Request Context Propagation (via Spans)

Use `cb_core::logging::request_span()` at transport layer to create spans with request context:

```rust
// In transport layer (ws.rs, stdio.rs)
let request_id = uuid::Uuid::new_v4();
let span = cb_core::logging::request_span(&request_id.to_string(), "websocket");
let _enter = span.enter();

// All logs within this scope automatically include:
// - request_id: Unique identifier for the request
// - transport: "websocket" or "stdio"

handle_request().await;
```

**Benefits**:
- Automatic context inheritance for all nested operations
- No manual field addition in every log statement
- Consistent request tracing across components
- Compatible with distributed tracing systems

**Standard Span Fields**:
- `request_id`: Unique identifier for the request
- `transport`: Transport type (websocket, stdio)
- Additional fields can be added via `span.record()`

### Log Output

**Note:** TypeMill writes all logs to standard error (`stderr`). Native file logging (e.g., via a `LOG_FILE` variable) is not currently supported.

To save logs to a file, use your shell's redirection capabilities:

```bash
# Redirect only stderr (where logs go) to app.log
./mill serve 2> app.log

# To redirect both stdout and stderr (useful for development)
./mill serve &> app.log
```

## Migration Guidelines

When updating existing code:

1. **Read the current logging calls** to understand the context
2. **Identify key information** that should become structured fields
3. **Apply standard field names** from this guide
4. **Test the changes** to ensure they compile and work correctly
5. **Verify log output** in both development and production formats

## Tools and Utilities

### Tracing Macros
- `error!`, `warn!`, `info!`, `debug!`, `trace!`
- `#[instrument]` for automatic function tracing
- `tracing::span!` for creating custom spans

### Viewing Logs
```bash
# Pretty format (development)
RUST_LOG=debug cargo run

# JSON format (production)
RUST_LOG=info LOG_FORMAT=json cargo run

# Filter by module
RUST_LOG=cb_server::handlers=debug cargo run
```

This structured approach ensures consistent, queryable, and maintainable logging across the entire TypeMill codebase.

---

## See Also

- **[CLAUDE.md](../../CLAUDE.md)** - Project documentation referencing these logging standards