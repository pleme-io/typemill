# Tool Architecture: Magnificent Seven API

Understanding the TypeMill public API - the "Magnificent Seven" tools.

## Overview

TypeMill uses a streamlined public API with exactly **7 tools** exposed to AI agents via MCP. All legacy tools have been removed from the public API and consolidated into these intent-oriented tools.

## Public API: Magnificent Seven

The complete public API consists of exactly 7 tools:

| Tool | Purpose |
|------|---------|
| `inspect_code` | Aggregate code intelligence (definition, references, types, diagnostics) |
| `search_code` | Search workspace symbols |
| `rename_all` | Rename symbols, files, directories (updates all references) |
| `relocate` | Move symbols, files, directories |
| `prune` | Delete symbols, files, directories with cleanup |
| `refactor` | Extract, inline, reorder, transform code |
| `workspace` | Package management, find/replace, dependency extraction |

### Key Features

- **Unified interface**: Each tool handles its entire domain
- **Safe defaults**: All refactoring tools default to `dryRun: true` (preview mode)
- **Intent-oriented**: Tools are designed around user goals, not implementation details

## Architecture

### Handler Structure

```
Magnificent Seven Handlers (Public API)
├── InspectHandler           → inspect_code
├── SearchHandler            → search_code
├── RenameAllHandler         → rename_all (delegates to internal services)
├── RelocateHandler          → relocate (delegates to internal services)
├── PruneHandler             → prune (delegates to internal services)
├── RefactorHandler          → refactor (delegates to internal services)
└── WorkspaceHandler         → workspace
```

### ToolHandler Trait

The trait is simplified - no visibility flags:

```rust
pub trait ToolHandler: Send + Sync {
    fn tool_names(&self) -> &[&str];

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value>;
}
```

### ToolRegistry Behavior

- `list_tools()` - Returns all 7 registered tools
- `list_tools_with_handlers()` - Returns tools with handler info
- `handle_tool()` - Executes tool calls

## CLI Tool Listing

The `mill tools` command lists all registered tools:

```bash
$ mill tools
Registered tools (7 total):
  - inspect_code
  - search_code
  - rename_all
  - relocate
  - prune
  - refactor
  - workspace
```

## Testing

Verify the tool count:

```rust
let registry = dispatcher.tool_registry.lock().await;
let tools = registry.list_tools();
assert_eq!(tools.len(), 7, "Expected exactly 7 Magnificent Seven tools");
```

## See Also

- **[Tools Reference](../tools/README.md)** - Complete tool documentation
- **[Tool Definitions](../../crates/mill-handlers/src/handlers/tool_definitions.rs)** - Schema definitions
- **[Tool Registration Tests](../../crates/mill-server/tests/tool_registration_test.rs)** - Verification tests
