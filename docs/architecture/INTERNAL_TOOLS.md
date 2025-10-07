# Internal Tools Policy

## Overview

CodeBuddy distinguishes between **public tools** (exposed to AI agents via MCP) and **internal tools** (backend-only, hidden from MCP listings).

## Internal vs Public Tools

### Public Tools (Visible to AI Agents)
- Listed in MCP `tools/list` requests
- Designed for direct use by AI agents
- Well-documented, user-friendly APIs
- Examples: `find_definition`, `rename_symbol`, `read_file`

### Internal Tools (Hidden from AI Agents)
- **Hidden** from MCP `tools/list` requests
- **Still callable** via direct tool invocation (for backend use)
- Used for system plumbing, protocol interop, and backend coordination
- Examples: `notify_file_opened`, `apply_workspace_edit`

## Current Internal Tools

### Lifecycle Hooks (3 tools)
**Handler**: `LifecycleHandler`

- `notify_file_opened`
- `notify_file_saved`
- `notify_file_closed`

**Rationale**: These are backend hooks for editors/IDEs to notify LSP servers and trigger plugin lifecycle events. AI agents don't "open/close files" in the editor sense - they directly read/write files. No value for AI code modification tasks.

## Marking Tools as Internal

### In Code

Implement `is_internal()` in your `ToolHandler`:

```rust
#[async_trait]
impl ToolHandler for MyHandler {
    fn tool_names(&self) -> &[&str] {
        &["my_internal_tool"]
    }

    fn is_internal(&self) -> bool {
        true  // Hide from MCP listings
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // Implementation
    }
}
```

### Documentation Requirements

When marking a tool as internal, you **must**:

1. **Add an entry to this file** under "Current Internal Tools"
2. **Document the rationale** - why is it internal?
3. **Add inline code comments** explaining the decision
4. **Update CLAUDE.md** tool count if necessary

## When to Make a Tool Internal

A tool should be internal if it meets **any** of these criteria:

### 1. Backend Plumbing
- Used for internal system coordination
- No direct value to end users
- Examples: lifecycle hooks, health check internals

### 2. Protocol Leakage
- Exposes low-level protocol details (LSP, language-specific internals)
- AI agents should use higher-level abstractions
- Example: `apply_workspace_edit` (LSP protocol format)

### 3. Redundancy
- Functionality fully covered by other public tools
- Internal convenience for specific backend use cases
- Example: `rename_symbol_with_imports` (covered by `rename_symbol` + `optimize_imports`)

### 4. Dangerous/Unstable
- Experimental features not ready for general use
- Could cause data loss or corruption if misused
- Should be reviewed before making public

## Candidates Under Review

These tools are being evaluated for internal status:

### Potential Internal Tools (4-7)
- `rename_symbol_with_imports` - Redundant with `rename_symbol` + `optimize_imports`
- `update_dependency` - Redundant with `update_dependencies`
- `batch_update_dependencies` - Redundant with `update_dependencies`
- `apply_workspace_edit` - LSP protocol interop (low-level)

**Status**: Under discussion - need to verify if used by other backend functionality.

## Implementation Details

### ToolHandler Trait

```rust
pub trait ToolHandler: Send + Sync {
    fn tool_names(&self) -> &[&str];

    fn is_internal(&self) -> bool {
        false  // Default: tools are public
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value>;
}
```

### ToolRegistry Behavior

- `list_tools()` - Returns **only public tools** (for MCP)
- `list_internal_tools()` - Returns internal tools (for diagnostics)
- `handle_tool()` - Executes **all tools** (public and internal)

This ensures:
- AI agents only see relevant, user-friendly tools
- Backend code can still call internal tools when needed
- Clear separation between public API and internal plumbing

## Testing

### Public Tool Tests
Test that tools are **listed** in MCP responses:
```rust
let tools = registry.list_tools();
assert!(tools.contains(&"find_definition".to_string()));
```

### Internal Tool Tests
Test that internal tools are:
1. **Not listed** in public listings
2. **Still callable** via handle_tool

```rust
let tools = registry.list_tools();
assert!(!tools.contains(&"notify_file_opened".to_string()));

let internal_tools = registry.list_internal_tools();
assert!(internal_tools.contains(&"notify_file_opened".to_string()));

// But still callable
let result = registry.handle_tool(
    ToolCall {
        name: "notify_file_opened".to_string(),
        arguments: Some(json!({"file_path": "test.rs"})),
    },
    &context,
).await;
assert!(result.is_ok());
```

## Migration Guide

### Making an Existing Tool Internal

1. **Add `is_internal()` override** to the handler
2. **Update this documentation** with rationale
3. **Update test expectations** (tool count, listings)
4. **Update CLAUDE.md** if tool count changed
5. **Announce in changelog** if user-facing

### Making an Internal Tool Public

Same steps, but document why it's now useful for AI agents.

## FAQ

**Q: Can internal tools still be called by backend code?**
A: Yes! Internal tools are only hidden from MCP `tools/list`. They're still fully functional and callable via `handle_tool()`.

**Q: What happens if an AI agent tries to call an internal tool?**
A: It works! The tool executes normally. We just don't advertise it in the tool list.

**Q: Should I hide tools that are rarely used?**
A: No. "Internal" is about purpose, not frequency. If a tool is legitimately useful for AI agents, keep it public even if rarely used.

**Q: Can I have a handler with both public and internal tools?**
A: No. `is_internal()` applies to all tools in a handler. If you need mixed visibility, split into separate handlers.

## See Also

- [`ToolHandler` trait documentation](../../crates/cb-handlers/src/handlers/tools/mod.rs)
- [`ToolRegistry` implementation](../../crates/cb-handlers/src/handlers/tool_registry.rs)
- [Tool registration tests](../../crates/cb-server/tests/tool_registration_test.rs)
