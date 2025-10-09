# Contributing to Codebuddy

> **📌 New to the project?** This guide is for developers building from source.
> End users: see [README.md](README.md) for installation instructions.

First off, thank you for considering contributing! It's people like you that make Codebuddy such a great tool.

## Getting Started

### Prerequisites

Building the full project requires the following tools. You can verify them all at once by running `make check-parser-deps`.

**Note:** A complete build and a passing test suite require the installation of external SDKs (Java, .NET, Node.js). Without these, parser builds will fail, which will cause tests for the corresponding language plugins to fail.

- **Rust Toolchain:** Get it from [rustup.rs](https://rustup.rs/).
- **Java SDK & Maven:** Required to build the Java parser.
- **.NET SDK:** Required to build the C# parser.
- **Node.js & npm:** Required to build the TypeScript parser.
- **Git:** For cloning the repository.
- **(Optional) SourceKitten:** For Swift language support.

### Setup Tools Explained

We use a few different tools for setup. Here's what each one is for:

| Tool | Who is it for? | Purpose |
|---|---|---|
| `install.sh` | **End Users** | Automated installer. Builds from source and copies the `codebuddy` binary to your system. |
| `make first-time-setup` | **Developers** | **THE complete setup command**. Installs all dev tools (cargo-nextest, sccache, mold), builds parsers, builds project, installs LSP servers, validates everything (~3-5 min). |
| `make build-parsers` | **Developers** | Builds only the external language parsers (Java, C#, TypeScript). Usually not needed directly—included in `first-time-setup`. |
| `make build` | **Developers** | Builds the core Rust project only. |
| `codebuddy setup` | **Both** | A runtime configuration wizard that helps you configure Language Server Protocol (LSP) servers for your projects. |

### Developer Setup Workflow

For the best first-time setup experience, we recommend using the `Makefile` targets.

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/goobits/codebuddy.git
    cd codebuddy
    ```

2.  **Run the first-time setup command:**
    ```bash
    make first-time-setup
    ```
    This single command will:
    a. Check that you have all the necessary prerequisites.
    b. Install recommended development tools (`sccache`, `mold`).
    c. Build all the external language parsers.
    d. Build the main Rust project.

3.  **Configure Language Servers:**
    ```bash
    # Run the interactive setup wizard
    codebuddy setup
    ```
    This will detect your project languages and help you set up the necessary LSP servers.

## Running Tests

This project uses [cargo-nextest](https://nexte.st/) for running tests. It's faster, provides better output, and has become the standard for modern Rust projects.

### Installation

If you ran `make first-time-setup`, `cargo-nextest` is already installed. Otherwise, `make test` will auto-install it for you.

### Usage

The easiest way to run tests is with the `Makefile`:

```bash
# Run fast tests (recommended for local development)
make test

# Run the full test suite, including skipped tests
make test-full

# Run tests that require LSP servers
make test-lsp
```

You can also run `cargo-nextest` directly for more granular control:

```bash
# Run all workspace tests
cargo nextest run --workspace

# Run a specific test file
cargo nextest run --test lsp_features

# Run ignored/skipped tests
cargo nextest run --status-level skip
```

## Code Style and Linting

We use the standard Rust formatting and linting tools to maintain a consistent codebase.

- **Formatting:** Before committing your changes, please format your code with `cargo fmt`.
  ```bash
  cargo fmt --all
  ```

- **Linting:** We use `clippy` for catching common mistakes and improving code quality.
  ```bash
  cargo clippy --all-targets -- -D warnings
  # Or use Makefile
  make clippy
  ```

- **Code Quality Checks:**
  ```bash
  make check                # Run fmt + clippy + test
  make check-duplicates     # Detect duplicate code & complexity
  ```

## Pull Request Process

1.  **Create a Feature Branch:**
    ```bash
    git checkout -b your-feature-name
    ```

2.  **Commit Your Changes:** Make your changes and commit them with a descriptive message.
    ```bash
    git commit -m "feat: Add new feature" -m "Detailed description of the changes."
    ```

3.  **Ensure Tests Pass:** Run the tests one last time to make sure everything is working correctly.
    ```bash
    make test
    ```

4.  **Push to Your Branch:**
    ```bash
    git push origin your-feature-name
    ```

5.  **Open a Pull Request:** Go to the repository on GitHub and open a new pull request. Provide a clear title and description of your changes.

## Adding New Language Plugins

To add support for a new programming language, see the **[Language Plugins Guide](crates/languages/README.md)** which provides:

- Complete plugin structure and schema requirements
- Required trait implementations (`LanguagePlugin`)
- Data types (ParsedSource, Symbol, ManifestData)
- Plugin registration steps
- Implementation patterns (dual-mode vs pure Rust)
- Reference implementations (Rust, Go, TypeScript)

## Adding New MCP Tools

This section explains how to add new tools and handlers to the system.

### Adding a Tool to an Existing Handler

Adding a new tool to an existing handler requires modifying just one file.

#### Step 1: Choose the Appropriate Handler

Handlers are organized by functionality:

| Handler | Location | Purpose | Example Tools |
|---------|----------|---------|---------------|
| **AnalysisHandler** | `crates/cb-handlers/src/handlers/tools/analysis.rs` | Code analysis | `find_unused_imports`, `analyze_complexity` |
| **AdvancedHandler** | `crates/cb-handlers/src/handlers/tools/advanced.rs` | Advanced operations | `apply_edits`, `batch_execute` |
| **EditingHandler** | `crates/cb-handlers/src/handlers/tools/editing.rs` | Code editing | `rename_symbol`, `format_document`, `optimize_imports` |
| **FileOpsHandler** | `crates/cb-handlers/src/handlers/tools/file_ops.rs` | File operations | `create_file`, `read_file`, `write_file`, `delete_file`, `rename_file`, `list_files` |
| **LifecycleHandler** | `crates/cb-handlers/src/handlers/tools/lifecycle.rs` | File lifecycle events | `notify_file_opened`, `notify_file_saved`, `notify_file_closed` |
| **NavigationHandler** | `crates/cb-handlers/src/handlers/tools/navigation.rs` | Code navigation | `find_definition`, `find_references` |
| **SystemHandler** | `crates/cb-handlers/src/handlers/tools/system.rs` | System operations | `health_check`, `web_fetch`, `system_status` |
| **WorkspaceHandler** | `crates/cb-handlers/src/handlers/tools/workspace.rs` | Workspace operations | `rename_directory`, `analyze_imports`, `find_dead_code` |

#### Step 2: Add the Tool Name

Open the appropriate handler file and add your tool name to the `TOOL_NAMES` constant:

```rust
// crates/cb-handlers/src/handlers/tools/navigation.rs

const TOOL_NAMES: &[&str] = &[
    "find_definition",
    "find_references",
    "get_call_graph", // ← Add your new tool here
];
```

#### Step 3: Implement the Handler Logic

Add a new match arm in the `handle_tool_call` method:

```rust
async fn handle_tool_call(
    &self,
    context: &ToolHandlerContext,
    tool_call: &ToolCall,
) -> ServerResult<Value> {
    match tool_call.name.as_str() {
        "find_definition" => self.find_definition(context, tool_call).await,
        "get_call_graph" => self.get_call_graph(context, tool_call).await, // ← Add match arm
        _ => Err(ServerError::Unsupported(format!(
            "Unsupported navigation tool: {}",
            tool_call.name
        ))),
    }
}
```

#### Step 4: Implement the Tool Method

Add the implementation as a private method:

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

        // Dispatch to plugin system
        let plugin_request = PluginRequest {
            method: "get_call_graph".to_string(),
            file_path: file_path.to_string(),
            params: json!({ /* parameters */ }),
            request_id: None,
        };

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

### Creating a New Handler

Create a new handler when adding a category of related tools that doesn't fit existing handlers.

#### Step 1: Create the Handler File

```bash
touch crates/cb-handlers/src/handlers/tools/diagnostics.rs
```

#### Step 2: Define the Handler Struct

```rust
//! Diagnostic tools for code quality and analysis

use super::{ToolHandler, ToolHandlerContext};
use crate::{ServerError, ServerResult};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use serde_json::{json, Value};
use tracing::{debug, error};

/// Handler for diagnostic tools
pub struct DiagnosticsHandler;

const TOOL_NAMES: &[&str] = &[
    "get_diagnostics",
    "get_code_quality_metrics",
];

impl DiagnosticsHandler {
    pub fn new() -> Self {
        Self
    }
}
```

#### Step 3: Implement the ToolHandler Trait

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
            _ => Err(ServerError::Unsupported(format!(
                "Unsupported diagnostic tool: {}",
                tool_call.name
            ))),
        }
    }
}
```

#### Step 4: Register the Handler

Add to `crates/cb-handlers/src/handlers/tools/mod.rs`:

```rust
pub mod diagnostics;
pub use diagnostics::DiagnosticsHandler;
```

Add to the dispatcher in `crates/cb-handlers/src/handlers/plugin_dispatcher.rs`:

```rust
register_handlers_with_logging!(registry, {
    SystemHandler => "SystemHandler with 3 tools...",
    DiagnosticsHandler => "DiagnosticsHandler with 2 tools...", // ← Add this
});
```

### Best Practices

#### Naming Conventions
- **Tool names**: snake_case (e.g., `get_diagnostics`)
- **Handler names**: PascalCase with "Handler" suffix (e.g., `DiagnosticsHandler`)
- **File names**: snake_case matching handler (e.g., `diagnostics.rs`)

#### Structured Logging
Always use structured key-value logging (see [docs/development/LOGGING_GUIDELINES.md](docs/development/LOGGING_GUIDELINES.md)):

```rust
// ✅ Good - structured logging
debug!(tool_name = %tool_call.name, file_path = %path, "Processing tool call");
error!(error = %e, tool = "get_diagnostics", "Tool execution failed");

// ❌ Bad - string interpolation
debug!("Processing tool call {} for file {}", tool_call.name, path);
```

#### Error Handling
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

#### Documentation
Add doc comments explaining purpose, parameters, and return values:

```rust
/// Get diagnostic information for a file
///
/// # Arguments
///
/// * `context` - Handler context with access to services
/// * `tool_call` - The tool call with file_path parameter
///
/// # Returns
///
/// Returns diagnostic messages, or an error if the file cannot be analyzed.
async fn get_diagnostics(...) -> ServerResult<Value> {
    // ...
}
```

#### Testing
Add tests for your tools (see [integration-tests/TESTING_GUIDE.md](integration-tests/TESTING_GUIDE.md)):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_diagnostics() {
        let context = create_test_context().await;
        let handler = DiagnosticsHandler::new();

        let tool_call = ToolCall {
            name: "get_diagnostics".to_string(),
            arguments: Some(json!({"file_path": "test.ts"})),
        };

        let result = handler.handle_tool_call(&context, &tool_call).await;
        assert!(result.is_ok());
    }
}
```

## Build Performance Tips

### Optimization Tools (Configured Automatically)

The project uses several build optimizations configured in `.cargo/config.toml`:

- **sccache**: Compilation cache that dramatically speeds up rebuilds
- **mold**: Modern, fast linker (3-10x faster than traditional linkers)
- **Dependency optimization**: Dependencies compiled with `-O2` in dev mode

### Quick Commands

```bash
# Check sccache statistics
sccache --show-stats

# Clear sccache (if having cache issues)
sccache --zero-stats

# Fast feedback during development (doesn't build binaries)
cargo check

# Build only changed code (fastest)
cargo build

# Full rebuild (slow, use only when necessary)
cargo clean && cargo build
```

### Build Times Reference

With sccache and mold installed:

| Build Type | Time (First) | Time (Incremental) |
|------------|--------------|-------------------|
| `cargo check` | ~30s | 2-5s |
| `cargo build` | ~2m | 5-20s |
| `cargo build --release` | ~3m | 30-60s |
| `cargo nextest run` (`make test`) | ~2m | 8-25s |

**Note:** Times vary based on:
- CPU cores (6+ cores recommended)
- SSD vs HDD (SSD strongly recommended)
- Changes scope (few files vs many files)

### Troubleshooting Slow Builds

If builds are slower than expected:

1. **Verify sccache is working:**
   ```bash
   sccache --show-stats
   # Should show cache hits on second build
   ```

2. **Check mold is being used:**
   ```bash
   grep -r "fuse-ld=mold" .cargo/config.toml
   # Should show linker configuration
   ```

3. **Monitor build parallelism:**
   ```bash
   # Check CPU usage during builds
   # Should use 80-100% of all cores
   ```

4. **Clear cache if corrupted:**
   ```bash
   sccache --zero-stats
   rm -rf target/
   cargo build
   ```
